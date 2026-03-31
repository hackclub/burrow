package nsc

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"strconv"
	"strings"
	"sync"
	"time"

	computev1betaconnect "buf.build/gen/go/namespace/cloud/connectrpc/go/proto/namespace/cloud/compute/v1beta/computev1betaconnect"
	computev1beta "buf.build/gen/go/namespace/cloud/protocolbuffers/go/proto/namespace/cloud/compute/v1beta"
	stdlib "buf.build/gen/go/namespace/cloud/protocolbuffers/go/proto/namespace/stdlib"
	"connectrpc.com/connect"
	"golang.org/x/crypto/ssh"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func hasMacOSLabel(labels []string) bool {
	for _, label := range labels {
		l := strings.TrimSpace(label)
		if l == "" {
			continue
		}
		if strings.HasPrefix(l, "namespace-profile-macos-") {
			return true
		}
	}
	return false
}

type lockedBuffer struct {
	mu sync.Mutex
	b  bytes.Buffer
}

func (lb *lockedBuffer) Write(p []byte) (int, error) {
	lb.mu.Lock()
	defer lb.mu.Unlock()
	return lb.b.Write(p)
}

func (lb *lockedBuffer) Len() int {
	lb.mu.Lock()
	defer lb.mu.Unlock()
	return lb.b.Len()
}

func (lb *lockedBuffer) String() string {
	lb.mu.Lock()
	defer lb.mu.Unlock()
	return lb.b.String()
}

func macosSupportDiskSelectors(baseImageID string) []*stdlib.Label {
	id := strings.TrimSpace(baseImageID)
	if id == "" {
		id = "tahoe"
	}

	// Allow specifying selectors directly, e.g. "macos.version=26.x,image.with=xcode-26".
	if strings.Contains(id, "=") {
		var out []*stdlib.Label
		for _, part := range strings.Split(id, ",") {
			part = strings.TrimSpace(part)
			if part == "" {
				continue
			}
			name, value, ok := strings.Cut(part, "=")
			name = strings.TrimSpace(name)
			value = strings.TrimSpace(value)
			if !ok || name == "" || value == "" {
				continue
			}
			out = append(out, &stdlib.Label{Name: name, Value: value})
		}
		if len(out) > 0 {
			return out
		}
	}

	// Human-friendly presets used by burrow config.
	switch strings.ToLower(id) {
	case "sonoma", "macos-14", "macos14", "14":
		return []*stdlib.Label{{Name: "macos.version", Value: "14.x"}}
	case "sequoia", "macos-15", "macos15", "15":
		return []*stdlib.Label{{Name: "macos.version", Value: "15.x"}}
	case "tahoe", "macos-26", "macos26", "26":
		// Constrain to the Xcode 26 support disk explicitly, since Apple builds
		// depend on Xcode being present and Compute currently errors if it can't
		// resolve a support disk selection.
		return []*stdlib.Label{{Name: "macos.version", Value: "26.x"}, {Name: "image.with", Value: "xcode-26"}}
	default:
		return []*stdlib.Label{{Name: "macos.version", Value: "26.x"}}
	}
}

func macosComputeBaseImageID(baseImageID string) string {
	id := strings.TrimSpace(baseImageID)
	if id == "" {
		return "tahoe"
	}
	// If selectors were provided directly, we cannot safely infer a canonical
	// base image ID from them.
	if strings.Contains(id, "=") {
		return ""
	}
	switch strings.ToLower(id) {
	case "sonoma", "macos-14", "macos14", "14":
		return "sonoma"
	case "sequoia", "macos-15", "macos15", "15":
		return "sequoia"
	case "tahoe", "macos-26", "macos26", "26":
		return "tahoe"
	default:
		return id
	}
}

type nscBearerTokenFile struct {
	BearerToken string `json:"bearer_token"`
}

func readNSCBearerToken() (string, error) {
	path := os.Getenv("NSC_TOKEN_FILE")
	if path == "" {
		return "", errors.New("NSC_TOKEN_FILE is required for macos runners")
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("read NSC_TOKEN_FILE: %w", err)
	}
	trimmed := strings.TrimSpace(string(raw))
	if trimmed == "" {
		return "", errors.New("NSC_TOKEN_FILE is empty")
	}
	// Support the on-host format used by burrow: {"bearer_token":"..."}.
	var parsed nscBearerTokenFile
	if err := json.Unmarshal([]byte(trimmed), &parsed); err == nil && parsed.BearerToken != "" {
		return parsed.BearerToken, nil
	}
	// Fallback: allow a raw bearer token.
	return trimmed, nil
}

func parseMachineTypeCPUxMemGB(machineType string) (vcpu int32, memoryMB int32, err error) {
	parts := strings.Split(machineType, "x")
	if len(parts) != 2 {
		return 0, 0, fmt.Errorf("invalid machine_type %q: expected CPUxMemoryGB (e.g. 12x28)", machineType)
	}
	cpu64, err := strconv.ParseInt(parts[0], 10, 32)
	if err != nil {
		return 0, 0, fmt.Errorf("invalid machine_type %q: cpu: %w", machineType, err)
	}
	memGB64, err := strconv.ParseInt(parts[1], 10, 32)
	if err != nil {
		return 0, 0, fmt.Errorf("invalid machine_type %q: memory: %w", machineType, err)
	}
	return int32(cpu64), int32(memGB64 * 1024), nil
}

func (d *Dispatcher) launchMacOSRunner(ctx context.Context, runnerName string, req LaunchRequest, ttl time.Duration, machineType string) error {
	if machineType == "" {
		return errors.New("machine_type is required for macos runners")
	}
	vcpu, memoryMB, err := parseMachineTypeCPUxMemGB(machineType)
	if err != nil {
		return err
	}
	bearer, err := readNSCBearerToken()
	if err != nil {
		return err
	}

	httpClient := &http.Client{Timeout: 60 * time.Second}
	client := computev1betaconnect.NewComputeServiceClient(httpClient, d.opts.ComputeBaseURL)

	workdir := d.opts.WorkDir
	if strings.TrimSpace(workdir) == "" {
		workdir = "/tmp/forgejo-runner"
	}

	env := map[string]string{
		"FORGEJO_INSTANCE_URL":   req.InstanceURL,
		"FORGEJO_RUNNER_TOKEN":   req.Token,
		"FORGEJO_RUNNER_NAME":    runnerName,
		"FORGEJO_RUNNER_LABELS":  strings.Join(req.Labels, ","),
		"FORGEJO_RUNNER_EXEC":    d.opts.Executor,
		"FORGEJO_RUNNER_WORKDIR": workdir,
	}
	for k, v := range req.ExtraEnv {
		env[k] = v
	}
	// Best-effort caching: workflows call Scripts/nscloud-cache.sh, which is a
	// no-op unless NSC_CACHE_PATH is set. This may still be skipped if spacectl
	// lacks credentials, but setting the path is harmless and keeps behavior
	// consistent across macOS / Linux runners.
	if _, ok := env["NSC_CACHE_PATH"]; !ok {
		env["NSC_CACHE_PATH"] = "/Users/runner/.cache/nscloud"
	}

	deadline := timestamppb.New(time.Now().Add(ttl))

	createReq := &computev1beta.CreateInstanceRequest{
		Shape: &computev1beta.InstanceShape{
			VirtualCpu:      vcpu,
			MemoryMegabytes: memoryMB,
			MachineArch:     d.opts.MacosMachineArch,
			Os:              "macos",
			// Namespace macOS compute requires selectors to pick the base image
			// ("support disk"), otherwise instance creation fails.
			Selectors: macosSupportDiskSelectors(d.opts.MacosBaseImageID),
		},
		DocumentedPurpose: fmt.Sprintf("burrow forgejo runner %s", runnerName),
		Deadline:          deadline,
		Labels: []*stdlib.Label{
			{Name: "nsc.source", Value: "forgejo-nsc"},
			{Name: "burrow.service", Value: "forgejo-runner"},
			{Name: "burrow.runner", Value: runnerName},
		},
		Applications: []*computev1beta.ApplicationRequest{
			{
				Name:         "forgejo-runner",
				Command:      "/bin/bash",
				Args:         []string{"-lc", macosBootstrapScript()},
				Environment:  env,
				WorkloadType: computev1beta.ApplicationRequest_JOB,
			},
		},
	}
	if imageID := macosComputeBaseImageID(d.opts.MacosBaseImageID); imageID != "" {
		createReq.Experimental = &computev1beta.CreateInstanceRequest_ExperimentalFeatures{
			MacosBaseImageId: imageID,
		}
	}

	d.log.Info("launching Namespace macos runner",
		"runner", runnerName,
		"compute_base_url", d.opts.ComputeBaseURL,
		"macos_base_image_id", d.opts.MacosBaseImageID,
		"shape", fmt.Sprintf("%dx%d", vcpu, memoryMB/1024),
		"arch", d.opts.MacosMachineArch,
	)

	reqCreate := connect.NewRequest(createReq)
	reqCreate.Header().Set("Authorization", "Bearer "+bearer)
	resp, err := client.CreateInstance(ctx, reqCreate)
	if err != nil {
		return fmt.Errorf("compute create instance failed: %w", err)
	}
	if resp.Msg == nil || resp.Msg.Metadata == nil {
		return errors.New("compute create instance returned no metadata")
	}
	instanceID := resp.Msg.Metadata.InstanceId

	waitErr := d.waitForMacOSRunnerStop(ctx, client, bearer, runnerName, instanceID, ttl)
	d.destroyComputeInstance(context.Background(), client, bearer, runnerName, instanceID)
	return waitErr
}

func (d *Dispatcher) runMacOSComputeSSHScript(ctx context.Context, runnerName, instanceID, script string) error {
	bearer, err := readNSCBearerToken()
	if err != nil {
		return err
	}

	httpClient := &http.Client{Timeout: 60 * time.Second}
	client := computev1betaconnect.NewComputeServiceClient(httpClient, d.opts.ComputeBaseURL)

	getReq := connect.NewRequest(&computev1beta.GetSSHConfigRequest{
		InstanceId: instanceID,
		// TargetContainer is optional. Keep it empty to run commands in the default instance environment.
	})
	getReq.Header().Set("Authorization", "Bearer "+bearer)

	resp, err := client.GetSSHConfig(ctx, getReq)
	if err != nil {
		return fmt.Errorf("compute get ssh config failed: %w", err)
	}
	if resp.Msg == nil {
		return errors.New("compute get ssh config returned empty response")
	}
	if resp.Msg.Endpoint == "" {
		return errors.New("compute get ssh config returned empty endpoint")
	}
	if len(resp.Msg.SshPrivateKey) == 0 {
		return errors.New("compute get ssh config returned empty ssh private key")
	}
	if strings.TrimSpace(resp.Msg.Username) == "" {
		return errors.New("compute get ssh config returned empty username")
	}

	signer, err := ssh.ParsePrivateKey(resp.Msg.SshPrivateKey)
	if err != nil {
		return fmt.Errorf("parse ssh private key: %w", err)
	}

	addr := fmt.Sprintf("%s:22", resp.Msg.Endpoint)
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		return fmt.Errorf("dial ssh endpoint: %w", err)
	}
	defer conn.Close()

	sshCfg := &ssh.ClientConfig{
		User:            resp.Msg.Username,
		Auth:            []ssh.AuthMethod{ssh.PublicKeys(signer)},
		HostKeyCallback: ssh.InsecureIgnoreHostKey(), // Endpoint is short-lived and key is delivered out-of-band.
		Timeout:         30 * time.Second,
	}

	c, chans, reqs, err := ssh.NewClientConn(conn, addr, sshCfg)
	if err != nil {
		return fmt.Errorf("ssh client conn: %w", err)
	}
	clientSSH := ssh.NewClient(c, chans, reqs)
	defer clientSSH.Close()

	session, err := clientSSH.NewSession()
	if err != nil {
		return fmt.Errorf("ssh new session: %w", err)
	}
	defer session.Close()

	var buf bytes.Buffer
	session.Stdout = &buf
	session.Stderr = &buf
	session.Stdin = strings.NewReader(script)

	// Feed the bootstrap script via stdin so we don't need to quote/escape it.
	//
	// Note: Some SSH servers do not reliably parse exec strings with arguments.
	// Running bare `/bin/bash` still reads from stdin and avoids argument parsing.
	if err := session.Run("/bin/bash"); err != nil {
		outRaw := buf.String()
		out := strings.TrimSpace(outRaw)

		// Some SSH servers reject exec requests and only allow interactive shells,
		// and others will "succeed" but still interpret stdin under the default
		// login shell (showing the zsh banner / prompts).
		//
		// In those cases, retry via Shell() with a PTY.
		exitStatus := 0
		exitErr, isExitErr := err.(*ssh.ExitError)
		if isExitErr {
			exitStatus = exitErr.ExitStatus()
		}

		looksInteractive := strings.Contains(outRaw, "The default interactive shell is now zsh") ||
			strings.Contains(outRaw, " runner$ ") ||
			strings.Contains(outRaw, "bash-3.2$")
		shouldFallback := !isExitErr || looksInteractive

		if shouldFallback {
			d.log.Warn("compute ssh exec bootstrap failed; retrying via interactive shell",
				"runner", runnerName,
				"instance", instanceID,
				"exit_status", exitStatus,
			)

			session2, err2 := clientSSH.NewSession()
			if err2 != nil {
				return fmt.Errorf("ssh new session (fallback): %w", err2)
			}
			defer session2.Close()

			// bytes.Buffer isn't safe for concurrent writes + reads; the SSH session
			// writes from background goroutines. Wrap it so we can poll for a prompt
			// before sending commands.
			lb := &lockedBuffer{}
			session2.Stdout = lb
			session2.Stderr = lb

			in, err2 := session2.StdinPipe()
			if err2 != nil {
				return fmt.Errorf("ssh stdin pipe (fallback): %w", err2)
			}

			// Request a PTY to match interactive semantics even when the caller
			// doesn't have a local terminal.
			_ = session2.RequestPty("xterm", 24, 80, nil)

			if err2 := session2.Shell(); err2 != nil {
				return fmt.Errorf("ssh shell (fallback): %w", err2)
			}

			// Wait briefly for the prompt/banner so the first command isn't dropped.
			// We also emit a sentinel `echo` to verify the TTY is live.
			deadline := time.Now().Add(3 * time.Second)
			for time.Now().Before(deadline) {
				n := lb.Len()
				if n > 0 {
					break
				}
				time.Sleep(50 * time.Millisecond)
			}

			// Stream the script then exit. Prefer LF line endings; macOS shells and
			// PTYs can treat CRLF as literal CR characters (breaking heredoc
			// delimiters and quoting).
			writeTTY := func(s string) {
				if s == "" {
					return
				}
				s = strings.ReplaceAll(s, "\r\n", "\n")
				_, _ = io.WriteString(in, s)
			}

			scriptTTY := strings.ReplaceAll(script, "\r\n", "\n")

			// Cut down noise in logs and reduce the chance of ZSH line-editing
			// behavior corrupting long inputs.
			writeTTY("stty -echo 2>/dev/null || true\n")
			writeTTY("echo BURROW_BOOTSTRAP_TTY_OK\n")

			// Avoid heredocs for the script itself (PTY newline handling is fragile).
			// Instead, stream base64 in short chunks to a file, then decode and run it.
			enc := base64.StdEncoding.EncodeToString([]byte(scriptTTY))
			idSafe := strings.ReplaceAll(instanceID, "-", "_")
			b64Path := "/tmp/burrow-bootstrap-" + idSafe + ".b64"
			shPath := "/tmp/burrow-bootstrap-" + idSafe + ".sh"

			writeTTY("rm -f " + b64Path + " " + shPath + "\n")
			writeTTY(": > " + b64Path + "\n")

			const chunkSize = 80
			for i := 0; i < len(enc); i += chunkSize {
				j := i + chunkSize
				if j > len(enc) {
					j = len(enc)
				}
				chunk := enc[i:j]
				// Base64 chunks contain only [A-Za-z0-9+/=], which are safe to pass
				// unquoted. Avoid quotes entirely so a truncated line can't leave
				// the remote shell in a multi-line continuation state.
				writeTTY("printf %s " + chunk + " >> " + b64Path + "\n")
				time.Sleep(5 * time.Millisecond)
			}

			// macOS uses `base64 -D` (BSD), some environments use `-d` (GNU).
			writeTTY("base64 -D " + b64Path + " > " + shPath + " 2>/dev/null || base64 -d " + b64Path + " > " + shPath + "\n")
			writeTTY("/bin/bash " + shPath + "\n")
			writeTTY("exit\n")
			_ = in.Close()

			if err2 := session2.Wait(); err2 != nil {
				out2 := strings.TrimSpace(lb.String())
				if len(out2) > 16*1024 {
					out2 = out2[len(out2)-16*1024:]
				}
				return fmt.Errorf("compute ssh runner bootstrap failed (shell fallback): %w\n%s", err2, out2)
			}

			d.log.Info("macos runner bootstrap completed via compute ssh shell", "runner", runnerName, "instance", instanceID)
			return nil
		}

		if len(out) > 16*1024 {
			out = out[len(out)-16*1024:]
		}
		return fmt.Errorf("compute ssh runner bootstrap failed: %w\n%s", err, out)
	}

	d.log.Info("macos runner bootstrap completed via compute ssh", "runner", runnerName, "instance", instanceID)
	return nil
}

func (d *Dispatcher) waitForMacOSRunnerStop(ctx context.Context, client computev1betaconnect.ComputeServiceClient, bearer, runnerName, instanceID string, ttl time.Duration) error {
	if ttl <= 0 {
		ttl = d.opts.DefaultDuration
	}
	deadline := time.Now().Add(ttl)
	ticker := time.NewTicker(15 * time.Second)
	defer ticker.Stop()

	for {
		stopped, err := d.checkComputeInstanceStopped(ctx, client, bearer, instanceID)
		if err != nil {
			d.log.Warn("macos runner stop check failed", "runner", runnerName, "instance", instanceID, "err", err)
		} else if stopped {
			return nil
		}

		if time.Now().After(deadline) {
			return fmt.Errorf("macos runner exceeded ttl (%s) without stopping", ttl)
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-ticker.C:
		}
	}
}

func (d *Dispatcher) checkComputeInstanceStopped(ctx context.Context, client computev1betaconnect.ComputeServiceClient, bearer, instanceID string) (bool, error) {
	describeReq := connect.NewRequest(&computev1beta.DescribeInstanceRequest{InstanceId: instanceID})
	describeReq.Header().Set("Authorization", "Bearer "+bearer)
	resp, err := client.DescribeInstance(ctx, describeReq)
	if err != nil {
		// NotFound means the instance is already gone.
		if connect.CodeOf(err) == connect.CodeNotFound {
			return true, nil
		}
		return false, err
	}
	if resp.Msg == nil || resp.Msg.Metadata == nil {
		return false, errors.New("describe instance returned no metadata")
	}
	switch resp.Msg.Metadata.Status {
	case computev1beta.InstanceMetadata_DESTROYED:
		return true, nil
	case computev1beta.InstanceMetadata_ERROR:
		// Best-effort include shutdown reasons; do not include unbounded output.
		var b strings.Builder
		for _, reason := range resp.Msg.ShutdownReasons {
			if reason == nil {
				continue
			}
			if b.Len() > 0 {
				b.WriteString("; ")
			}
			b.WriteString(reason.String())
			if b.Len() > 1024 {
				break
			}
		}
		msg := strings.TrimSpace(b.String())
		if msg == "" {
			msg = "unknown shutdown reason"
		}
		return true, fmt.Errorf("instance entered error state: %s", msg)
	default:
		if resp.Msg.Metadata.DestroyedAt != nil {
			return true, nil
		}
		return false, nil
	}
}

func (d *Dispatcher) destroyComputeInstance(ctx context.Context, client computev1betaconnect.ComputeServiceClient, bearer, runnerName, instanceID string) {
	if ctx == nil {
		ctx = context.Background()
	}
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()

	destroyReq := connect.NewRequest(&computev1beta.DestroyInstanceRequest{InstanceId: instanceID})
	destroyReq.Header().Set("Authorization", "Bearer "+bearer)
	if _, err := client.DestroyInstance(ctx, destroyReq); err != nil {
		if connect.CodeOf(err) == connect.CodeNotFound {
			d.log.Info("macos runner destroyed", "runner", runnerName, "instance", instanceID, "status", "not_found")
			return
		}
		d.log.Warn("macos runner destroy failed", "runner", runnerName, "instance", instanceID, "err", err)
		return
	}
	d.log.Info("macos runner destroyed", "runner", runnerName, "instance", instanceID)
}

func macosBootstrapScript() string {
	// Keep this script self-contained: it runs on a fresh macOS VM base image.
	var b strings.Builder
	b.WriteString(`set -euo pipefail

workdir="${FORGEJO_RUNNER_WORKDIR:-/tmp/forgejo-runner}"
mkdir -p "${workdir}"
cd "${workdir}"

export PATH="/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

if ! command -v nix >/dev/null 2>&1; then
  echo "Installing nix (Determinate Systems installer)..."
  installer="/tmp/nix-installer.$$"
  curl -fsSL -o "${installer}" https://install.determinate.systems/nix
  chmod +x "${installer}"

  if command -v sudo >/dev/null 2>&1; then
    if sudo -n true 2>/dev/null; then
      sudo -n sh "${installer}" install --no-confirm
    else
      sudo sh "${installer}" install --no-confirm
    fi
  else
    sh "${installer}" install --no-confirm
  fi

  rm -f "${installer}"
fi

if [[ -f /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh ]]; then
  # shellcheck disable=SC1091
  . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh
fi

export PATH="/nix/var/nix/profiles/default/bin:/nix/var/nix/profiles/default/sbin:${PATH}"

# Flake builds need nix-command + flakes enabled. Workflows may layer additional
# config, but ensure a sane default exists.
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/nix"
cat > "${XDG_CONFIG_HOME:-$HOME/.config}/nix/nix.conf" <<'EOF'
experimental-features = nix-command flakes
sandbox = true
fallback = true
substituters = https://cache.nixos.org
trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
EOF

mkdir -p bin
export PATH="${PWD}/bin:${PATH}"

runner_version="v12.6.4"
runner_src_tgz="forgejo-runner-${runner_version}.tar.gz"
runner_src_url="https://code.forgejo.org/forgejo/runner/archive/${runner_version}.tar.gz"
runner_src_dir="forgejo-runner-src"

if ! command -v forgejo-runner >/dev/null 2>&1; then
  rm -rf "${runner_src_dir}"
  mkdir -p "${runner_src_dir}"
  curl -fsSL "${runner_src_url}" -o "${runner_src_tgz}"
  tar -xzf "${runner_src_tgz}" -C "${runner_src_dir}" --strip-components=1

  toolchain="$(grep -E '^toolchain ' "${runner_src_dir}/go.mod" | awk '{print $2}' | head -n 1 || true)"
  if [ -z "${toolchain}" ]; then
    toolchain="go1.25.7"
  fi

  if ! command -v go >/dev/null 2>&1; then
    go_tgz="${toolchain}.darwin-arm64.tar.gz"
    go_url="https://go.dev/dl/${go_tgz}"
    curl -fsSL "${go_url}" -o "${go_tgz}"
    tar -xzf "${go_tgz}"
    export GOROOT="${PWD}/go"
    export PATH="${GOROOT}/bin:${PATH}"
  fi

  export GOPATH="${PWD}/.gopath"
  export GOMODCACHE="${PWD}/.gomodcache"
  export GOCACHE="${PWD}/.gocache"
  mkdir -p "${GOPATH}" "${GOMODCACHE}" "${GOCACHE}"

  (cd "${runner_src_dir}" && go build -o "${workdir}/bin/forgejo-runner" .)
  chmod +x "${workdir}/bin/forgejo-runner"
fi

cat > runner.yaml <<'EOF'
log:
  level: info
runner:
  file: .runner
  capacity: 1
  name: ${FORGEJO_RUNNER_NAME}
  labels:
EOF

runner_exec="${FORGEJO_RUNNER_EXEC:-host}"
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
      resolved="${label}:host"
      ;;
  esac
  echo "  - ${resolved}" >> runner.yaml
  if [ -z "${resolved_labels}" ]; then
    resolved_labels="${resolved}"
  else
    resolved_labels="${resolved_labels},${resolved}"
  fi
done

cat >> runner.yaml <<'EOF'
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

forgejo-runner one-job --config runner.yaml
`)
	return b.String()
}
