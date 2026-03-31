package nsc

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"os/exec"
	"strings"
	"time"

	"github.com/google/uuid"
	"golang.org/x/sync/semaphore"
)

type Options struct {
	BinaryPath       string
	DefaultImage     string
	DefaultMachine   string
	DefaultDuration  time.Duration
	WorkDir          string
	MaxParallel      int64
	RunnerNamePrefix string
	Executor         string
	Network          string
	ComputeBaseURL   string
	MacosBaseImageID string
	MacosMachineArch string
	Logger           *slog.Logger
}

type LaunchRequest struct {
	Token       string
	InstanceURL string
	Labels      []string
	Duration    time.Duration
	MachineType string
	Image       string
	ExtraEnv    map[string]string
}

type Dispatcher struct {
	opts Options
	sem  *semaphore.Weighted
	log  *slog.Logger
}

func NewDispatcher(opts Options) (*Dispatcher, error) {
	if opts.BinaryPath == "" {
		return nil, errors.New("nsc binary path is required")
	}
	if opts.DefaultImage == "" {
		return nil, errors.New("default Namespace runner image is required")
	}
	if opts.RunnerNamePrefix == "" {
		opts.RunnerNamePrefix = "nscloud-"
	}
	if opts.Executor == "" {
		opts.Executor = "shell"
	}
	if opts.MacosBaseImageID == "" {
		opts.MacosBaseImageID = "tahoe"
	}
	if opts.MacosMachineArch == "" {
		opts.MacosMachineArch = "arm64"
	}
	if opts.MaxParallel <= 0 {
		opts.MaxParallel = 4
	}
	if opts.DefaultDuration == 0 {
		opts.DefaultDuration = 30 * time.Minute
	}
	logger := opts.Logger
	if logger == nil {
		logger = slog.New(slog.NewTextHandler(io.Discard, nil))
	}

	return &Dispatcher{
		opts: opts,
		sem:  semaphore.NewWeighted(opts.MaxParallel),
		log:  logger,
	}, nil
}

func (d *Dispatcher) LaunchRunner(ctx context.Context, req LaunchRequest) (string, error) {
	if req.Token == "" {
		return "", errors.New("registration token is required")
	}
	if req.InstanceURL == "" {
		return "", errors.New("forgejo instance url is required")
	}
	if err := d.sem.Acquire(ctx, 1); err != nil {
		return "", err
	}
	defer d.sem.Release(1)

	runnerName := d.generateName()
	duration := req.Duration
	if duration == 0 {
		duration = d.opts.DefaultDuration
	}
	machineType := choose(req.MachineType, d.opts.DefaultMachine)
	image := choose(req.Image, d.opts.DefaultImage)

	if hasWindowsLabel(req.Labels) {
		if err := d.launchWindowsRunnerViaWinRM(ctx, runnerName, req, duration, machineType); err != nil {
			return "", err
		}
		return runnerName, nil
	}

	if hasMacOSLabel(req.Labels) {
		// Compute macOS shapes differ from the Linux "run" defaults. If the request
		// didn't specify a machine type, ensure we pick a macOS-valid default.
		if machineType == "" || machineType == d.opts.DefaultMachine {
			machineType = "12x28"
		}

		// Prefer the Compute API path because it uses the service token (NSC_TOKEN_FILE)
		// and does not require an interactive `nsc login` session.
		if err := d.launchMacOSRunner(ctx, runnerName, req, duration, machineType); err != nil {
			d.log.Warn("macos compute launch failed; falling back to nsc create+ssh", "runner", runnerName, "err", err)
			if err := d.launchMacOSRunnerViaNSC(ctx, runnerName, req, duration, machineType); err != nil {
				return "", err
			}
		}
		return runnerName, nil
	}

	env := map[string]string{
		"FORGEJO_INSTANCE_URL":  req.InstanceURL,
		"FORGEJO_RUNNER_TOKEN":  req.Token,
		"FORGEJO_RUNNER_NAME":   runnerName,
		"FORGEJO_RUNNER_LABELS": strings.Join(req.Labels, ","),
		"FORGEJO_RUNNER_EXEC":   d.opts.Executor,
	}
	for k, v := range req.ExtraEnv {
		env[k] = v
	}
	if _, ok := env["NSC_CACHE_PATH"]; !ok {
		env["NSC_CACHE_PATH"] = "/nix/store"
	}

	script := d.bootstrapScript()
	args := []string{
		"run",
		"--wait",
		"--output",
		"json",
		"--duration", duration.String(),
		"--image", image,
		"--name", runnerName,
		"--user", "root",
	}
	if machineType != "" {
		args = append(args, "--machine_type", machineType)
	}
	if d.opts.Network != "" {
		args = append(args, "--network", d.opts.Network)
	}
	for key, value := range env {
		if value == "" {
			continue
		}
		args = append(args, "-e", fmt.Sprintf("%s=%s", key, value))
	}
	if d.opts.WorkDir != "" {
		args = append(args, "-e", fmt.Sprintf("FORGEJO_RUNNER_WORKDIR=%s", d.opts.WorkDir))
	}

	args = append(args, "--", "/bin/sh", "-c", script)

	cmd := exec.CommandContext(ctx, d.opts.BinaryPath, args...)
	var buf bytes.Buffer
	cmd.Stdout = &buf
	cmd.Stderr = &buf

	start := time.Now()
	d.log.Info("launching Namespace runner",
		"runner", runnerName,
		"machine_type", machineType,
		"image", image,
	)
	err := cmd.Run()
	if err != nil {
		return "", fmt.Errorf("nsc run failed: %w\n%s", err, buf.String())
	}

	if output := strings.TrimSpace(buf.String()); output != "" {
		d.log.Info("runner output", "runner", runnerName, "output", output)
	}

	d.log.Info("runner completed",
		"runner", runnerName,
		"duration", time.Since(start),
	)

	if instanceID := parseInstanceID(buf.String()); instanceID != "" {
		waitCtx, cancel := context.WithTimeout(context.Background(), duration)
		defer cancel()
		stopped := d.waitForInstanceStop(waitCtx, runnerName, instanceID, duration)
		if !stopped {
			d.log.Warn("runner did not stop before timeout", "runner", runnerName, "instance", instanceID)
		}
		d.destroyInstance(waitCtx, runnerName, instanceID)
	}

	return runnerName, nil
}

func (d *Dispatcher) generateName() string {
	id := strings.ReplaceAll(uuid.NewString(), "-", "")
	return d.opts.RunnerNamePrefix + id[:12]
}

func parseInstanceID(output string) string {
	if jsonBlob := extractJSON(output); jsonBlob != "" {
		var payload struct {
			ClusterID string `json:"cluster_id"`
		}
		if err := json.Unmarshal([]byte(jsonBlob), &payload); err == nil && payload.ClusterID != "" {
			return payload.ClusterID
		}
	}
	const marker = "ID:"
	idx := strings.Index(output, marker)
	if idx == -1 {
		return ""
	}
	rest := strings.TrimSpace(output[idx+len(marker):])
	if rest == "" {
		return ""
	}
	fields := strings.Fields(rest)
	if len(fields) == 0 {
		return ""
	}
	return fields[0]
}

func extractJSON(output string) string {
	trimmed := strings.TrimSpace(output)
	if trimmed == "" {
		return ""
	}
	start := strings.IndexAny(trimmed, "[{")
	if start == -1 {
		return ""
	}
	end := strings.LastIndexAny(trimmed, "]}")
	if end == -1 || end < start {
		return ""
	}
	return trimmed[start : end+1]
}

type describeResponse struct {
	Resource    string                    `json:"resource"`
	PerResource map[string]describeTarget `json:"per_resource"`
}

type describeTarget struct {
	Tombstone string              `json:"tombstone"`
	Container []describeContainer `json:"container"`
}

type describeContainer struct {
	Status       string `json:"status"`
	TerminatedAt string `json:"terminated_at"`
}

func instanceStopped(output string) bool {
	jsonBlob := extractJSON(output)
	if jsonBlob == "" {
		return false
	}
	var payload []describeResponse
	if err := json.Unmarshal([]byte(jsonBlob), &payload); err != nil {
		return false
	}
	if len(payload) == 0 {
		return false
	}
	for _, entry := range payload {
		for _, target := range entry.PerResource {
			if target.Tombstone != "" {
				return true
			}
			if len(target.Container) == 0 {
				continue
			}
			for _, container := range target.Container {
				if container.Status != "stopped" && container.TerminatedAt == "" {
					return false
				}
			}
		}
	}
	return true
}

func (d *Dispatcher) waitForInstanceStop(ctx context.Context, runnerName, instanceID string, timeout time.Duration) bool {
	if timeout <= 0 {
		timeout = d.opts.DefaultDuration
	}
	deadline := time.Now().Add(timeout)
	ticker := time.NewTicker(10 * time.Second)
	defer ticker.Stop()

	for {
		stopped, err := d.checkInstanceStopped(ctx, instanceID)
		if err != nil {
			d.log.Warn("runner stop check failed", "runner", runnerName, "instance", instanceID, "err", err)
			return false
		}
		if stopped {
			return true
		}
		if time.Now().After(deadline) {
			return false
		}
		select {
		case <-ctx.Done():
			return false
		case <-ticker.C:
		}
	}
}

func (d *Dispatcher) checkInstanceStopped(ctx context.Context, instanceID string) (bool, error) {
	cmd := exec.CommandContext(ctx, d.opts.BinaryPath, "describe", "--output", "json", instanceID)
	var buf bytes.Buffer
	cmd.Stdout = &buf
	cmd.Stderr = &buf
	if err := cmd.Run(); err != nil {
		output := strings.ToLower(buf.String())
		if strings.Contains(output, "destroyed") || strings.Contains(output, "not found") {
			return true, nil
		}
		return false, fmt.Errorf("nsc describe failed: %w\n%s", err, strings.TrimSpace(buf.String()))
	}
	return instanceStopped(buf.String()), nil
}

func (d *Dispatcher) destroyInstance(ctx context.Context, runnerName, instanceID string) {
	cmd := exec.CommandContext(ctx, d.opts.BinaryPath, "destroy", "--force", instanceID)
	var buf bytes.Buffer
	cmd.Stdout = &buf
	cmd.Stderr = &buf
	if err := cmd.Run(); err != nil {
		d.log.Warn("runner destroy failed", "runner", runnerName, "instance", instanceID, "err", err, "output", strings.TrimSpace(buf.String()))
		return
	}
	if output := strings.TrimSpace(buf.String()); output != "" {
		d.log.Info("runner destroyed", "runner", runnerName, "instance", instanceID, "output", output)
	} else {
		d.log.Info("runner destroyed", "runner", runnerName, "instance", instanceID)
	}
}

func choose(values ...string) string {
	for _, v := range values {
		if strings.TrimSpace(v) != "" {
			return v
		}
	}
	return ""
}

func (d *Dispatcher) bootstrapScript() string {
	var builder strings.Builder
	builder.WriteString(`set -euo pipefail
mkdir -p "${FORGEJO_RUNNER_WORKDIR:-/tmp/forgejo-runner}"
cd "${FORGEJO_RUNNER_WORKDIR:-/tmp/forgejo-runner}"

if ! command -v node >/dev/null 2>&1; then
  apk add --no-cache nodejs npm >/dev/null
fi
if ! command -v sudo >/dev/null 2>&1; then
  apk add --no-cache sudo bash >/dev/null
fi
if ! command -v curl >/dev/null 2>&1; then
  apk add --no-cache curl >/dev/null
fi
if ! command -v xz >/dev/null 2>&1; then
  apk add --no-cache xz >/dev/null
fi
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
node --version >/dev/null

cat > runner.yaml <<'EOF'
log:
  level: info
runner:
  file: .runner
  capacity: 1
  name: ${FORGEJO_RUNNER_NAME}
  labels:
EOF
`)
	builder.WriteString(`runner_exec="${FORGEJO_RUNNER_EXEC:-host}"
if [ "$runner_exec" = "shell" ]; then
  runner_exec="host"
fi

resolved_labels=""
for label in ${FORGEJO_RUNNER_LABELS//,/ } ; do
  if [ -z "${label}" ]; then
    continue
  fi
  case "${label}" in
    *:*) resolved="${label}" ;;
    *)
      if [ "$runner_exec" = "host" ]; then
        resolved="${label}:host"
      else
        resolved="${label}:${runner_exec}"
      fi
      ;;
  esac
  echo "  - ${resolved}" >> runner.yaml
  if [ -z "${resolved_labels}" ]; then
    resolved_labels="${resolved}"
  else
    resolved_labels="${resolved_labels},${resolved}"
  fi
done
`)
	builder.WriteString(`cat >> runner.yaml <<'EOF'
cache:
  enabled: false
EOF

forgejo-runner register \
  --no-interactive \
  --instance "${FORGEJO_INSTANCE_URL}" \
  --token "${FORGEJO_RUNNER_TOKEN}" \
  --name "${FORGEJO_RUNNER_NAME}" \
  --labels "${resolved_labels}" \
  --config runner.yaml

runner_mode="${FORGEJO_RUNNER_MODE:-one-job}"
case "$runner_mode" in
  one-job)
    forgejo-runner one-job --config runner.yaml
    ;;
  daemon)
    forgejo-runner daemon --config runner.yaml
    ;;
  *)
    echo "Unknown FORGEJO_RUNNER_MODE: ${runner_mode}" >&2
    exit 1
    ;;
esac
`)
	return builder.String()
}
