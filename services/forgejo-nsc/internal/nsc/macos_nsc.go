package nsc

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

func normalizeMacOSNSCMachineType(machineType string) (normalized string, changed bool, err error) {
	vcpu, memoryMB, err := parseMachineTypeCPUxMemGB(machineType)
	if err != nil {
		return "", false, err
	}
	memGB := memoryMB / 1024
	if memGB <= 0 || vcpu <= 0 {
		return "", false, fmt.Errorf("invalid machine_type %q after parse: vcpu=%d memGB=%d", machineType, vcpu, memGB)
	}

	// NSC CLI (and the underlying InstanceService) enforce discrete cpu/mem sets
	// for macOS. Normalize requested values by rounding up to the closest allowed
	// values to keep provisioning stable even when configs drift.
	//
	// Observed allowed sets from Namespace API error output for macos/arm64:
	// cpu: [4 6 8 12]
	// mem: [7 14 28 56] (GB)
	allowedCPU := []int32{4, 6, 8, 12}
	allowedMemGB := []int32{7, 14, 28, 56}

	roundUp := func(v int32, allowed []int32) (int32, bool) {
		for _, a := range allowed {
			if v <= a {
				return a, a != v
			}
		}
		// Clamp to max if above all allowed values.
		return allowed[len(allowed)-1], true
	}

	newCPU, cpuChanged := roundUp(vcpu, allowedCPU)
	newMemGB, memChanged := roundUp(memGB, allowedMemGB)

	normalized = fmt.Sprintf("%dx%d", newCPU, newMemGB)
	changed = cpuChanged || memChanged
	return normalized, changed, nil
}

func (d *Dispatcher) launchMacOSRunnerViaNSC(ctx context.Context, runnerName string, req LaunchRequest, ttl time.Duration, machineType string) error {
	if machineType == "" {
		return errors.New("machine_type is required for macos runners")
	}
	if strings.TrimSpace(os.Getenv("NSC_TOKEN_FILE")) == "" {
		// The Burrow forge host feeds NSC_TOKEN_FILE from the intake-backed runtime token.
		return errors.New("NSC_TOKEN_FILE is required for macos runners")
	}

	selectors := macosSelectorsArg(d.opts.MacosBaseImageID)
	if selectors == "" {
		return errors.New("macos selectors resolved empty")
	}

	normalizedMachineType := machineType
	if n, changed, err := normalizeMacOSNSCMachineType(machineType); err != nil {
		return err
	} else if changed {
		normalizedMachineType = n
	}

	// If capacity is constrained for the requested (large) shape, try a small
	// set of progressively smaller shapes before failing the dispatch request.
	// This keeps macOS builds flowing even when large runners are scarce.
	candidates := []string{normalizedMachineType, "8x28", "6x14", "4x7"}
	seen := map[string]struct{}{}
	var uniq []string
	for _, c := range candidates {
		c = strings.TrimSpace(c)
		if c == "" {
			continue
		}
		if _, ok := seen[c]; ok {
			continue
		}
		seen[c] = struct{}{}
		uniq = append(uniq, c)
	}
	candidates = uniq

	type attemptCfg struct {
		waitTimeout   time.Duration
		createTimeout time.Duration
	}
	attempts := []attemptCfg{
		{waitTimeout: 6 * time.Minute, createTimeout: 8 * time.Minute},
		{waitTimeout: 4 * time.Minute, createTimeout: 6 * time.Minute},
		{waitTimeout: 3 * time.Minute, createTimeout: 5 * time.Minute},
	}

	createInstance := func(mt string, a attemptCfg) (instanceID string, out string, err error) {
		tmpDir, err := os.MkdirTemp("", "forgejo-nsc-macos-*")
		if err != nil {
			return "", "", fmt.Errorf("mktemp: %w", err)
		}
		defer os.RemoveAll(tmpDir)

		metaPath := filepath.Join(tmpDir, "create.json")
		cidPath := filepath.Join(tmpDir, "create.cid")

		arch := strings.TrimSpace(d.opts.MacosMachineArch)
		if arch == "" {
			arch = "arm64"
		}
		// Namespace CLI requires the "os/arch:" prefix to create a macOS instance.
		// Without it, `nsc create` defaults to Linux even if selectors include macos.*.
		machineType := fmt.Sprintf("macos/%s:%s", arch, mt)

		args := []string{
			"create",
			"--duration", ttl.String(),
			"--machine_type", machineType,
			"--selectors", selectors,
			"--bare",
			"--cidfile", cidPath,
			"--log_actions",
			"--purpose", fmt.Sprintf("burrow forgejo runner %s", runnerName),
			// Prefer plain output for debuggability (progress, capacity errors, etc).
			"--output", "plain",
			"--output_json_to", metaPath,
			// macOS instances can take a while to become ready.
			"--wait_timeout", a.waitTimeout.String(),
		}
		args = prependNSCRegionArgs(args, d.opts.ComputeBaseURL)

		createCtx, cancel := context.WithTimeout(ctx, a.createTimeout)
		defer cancel()

		cmd := exec.CommandContext(createCtx, d.opts.BinaryPath, args...)
		var buf bytes.Buffer
		cmd.Stdout = &buf
		cmd.Stderr = &buf

		if err := cmd.Run(); err != nil {
			// Best-effort cleanup: if the instance ID was written before the command failed
			// (or before we timed it out), attempt to destroy it to avoid idling machines.
			if instanceID := strings.TrimSpace(mustReadFile(cidPath)); instanceID != "" {
				d.destroyNSCInstance(context.Background(), runnerName, instanceID)
			}
			if errors.Is(createCtx.Err(), context.DeadlineExceeded) {
				return "", buf.String(), fmt.Errorf("nsc create timed out after %s", a.createTimeout)
			}
			return "", buf.String(), fmt.Errorf("nsc create failed: %w", err)
		}

		instanceID, err = readNSCCreateInstanceID(metaPath)
		if err != nil {
			return "", buf.String(), fmt.Errorf("nsc create output parse failed: %w", err)
		}
		if instanceID == "" {
			return "", buf.String(), fmt.Errorf("nsc create returned empty instance id")
		}
		return instanceID, buf.String(), nil
	}

	var (
		instanceID string
		lastOut    string
		lastErr    error
	)
	for i, mt := range candidates {
		a := attempts[i]
		if i >= len(attempts) {
			a = attempts[len(attempts)-1]
		}

		d.log.Info("launching Namespace macos runner via nsc",
			"runner", runnerName,
			"attempt", i+1,
			"machine_type", mt,
			"requested_machine_type", machineType,
			"selectors", selectors,
		)

		id, out, err := createInstance(mt, a)
		lastOut = out
		lastErr = err
		if err != nil {
			// Timeouts are treated as retryable (capacity constrained).
			if strings.Contains(err.Error(), "timed out") || strings.Contains(strings.ToLower(out), "capacity") {
				continue
			}
			return fmt.Errorf("%w\n%s", err, out)
		}
		instanceID = id
		break
	}
	if instanceID == "" {
		if lastErr != nil {
			return fmt.Errorf("%w\n%s", lastErr, lastOut)
		}
		return fmt.Errorf("nsc create failed without producing an instance id\n%s", lastOut)
	}

	// Always attempt cleanup even if the runner fails.
	defer d.destroyNSCInstance(context.Background(), runnerName, instanceID)

	script := macosBootstrapWrapperScript(runnerName, req, d.opts.Executor, d.opts.WorkDir)
	// Use the Compute SSH config endpoint (direct TCP) instead of `nsc ssh`, which
	// relies on a websocket-based SSH proxy that is not supported by the
	// revokable tenant token we run the dispatcher with.
	if err := d.runMacOSComputeSSHScript(ctx, runnerName, instanceID, script); err != nil {
		return err
	}
	return nil
}

func mustReadFile(path string) string {
	raw, err := os.ReadFile(path)
	if err != nil {
		return ""
	}
	return string(raw)
}

func macosSelectorsArg(baseImageID string) string {
	id := strings.TrimSpace(baseImageID)
	if id == "" {
		id = "tahoe"
	}
	// Allow passing selectors directly via config, e.g. "macos.version=26.x,image.with=xcode-26".
	if strings.Contains(id, "=") {
		return id
	}
	switch strings.ToLower(id) {
	case "sonoma", "macos-14", "macos14", "14":
		return "macos.version=14.x"
	case "sequoia", "macos-15", "macos15", "15":
		return "macos.version=15.x"
	case "tahoe", "macos-26", "macos26", "26":
		return "macos.version=26.x,image.with=xcode-26"
	default:
		return "macos.version=26.x"
	}
}

type nscCreateMetadata struct {
	InstanceID string `json:"instance_id"`
	ClusterID  string `json:"cluster_id"`
	ID         string `json:"id"`
}

func readNSCCreateInstanceID(path string) (string, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("read %s: %w", path, err)
	}
	var meta nscCreateMetadata
	if err := json.Unmarshal(raw, &meta); err != nil {
		return "", err
	}
	if meta.InstanceID != "" {
		return meta.InstanceID, nil
	}
	if meta.ClusterID != "" {
		return meta.ClusterID, nil
	}
	if meta.ID != "" {
		return meta.ID, nil
	}
	return "", nil
}

func (d *Dispatcher) destroyNSCInstance(ctx context.Context, runnerName, instanceID string) {
	if ctx == nil {
		ctx = context.Background()
	}
	ctx, cancel := context.WithTimeout(ctx, 2*time.Minute)
	defer cancel()

	args := []string{"destroy", "--force", instanceID}
	args = prependNSCRegionArgs(args, d.opts.ComputeBaseURL)
	cmd := exec.CommandContext(ctx, d.opts.BinaryPath, args...)
	var buf bytes.Buffer
	cmd.Stdout = &buf
	cmd.Stderr = &buf
	if err := cmd.Run(); err != nil {
		d.log.Warn("nsc destroy failed", "runner", runnerName, "instance", instanceID, "err", err, "output", strings.TrimSpace(buf.String()))
		return
	}
	d.log.Info("nsc instance destroyed", "runner", runnerName, "instance", instanceID)
}

func macosBootstrapWrapperScript(runnerName string, req LaunchRequest, executor, workdir string) string {
	if strings.TrimSpace(workdir) == "" {
		workdir = "/tmp/forgejo-runner"
	}

	// Pass all values via stdin script so secrets do not appear in the nsc ssh argv.
	env := map[string]string{
		"FORGEJO_INSTANCE_URL":   req.InstanceURL,
		"FORGEJO_RUNNER_TOKEN":   req.Token,
		"FORGEJO_RUNNER_NAME":    runnerName,
		"FORGEJO_RUNNER_LABELS":  strings.Join(req.Labels, ","),
		"FORGEJO_RUNNER_EXEC":    executor,
		"FORGEJO_RUNNER_WORKDIR": workdir,
	}
	for k, v := range req.ExtraEnv {
		env[k] = v
	}

	var b strings.Builder
	b.WriteString("set -euo pipefail\n")
	for k, v := range env {
		if strings.TrimSpace(k) == "" {
			continue
		}
		// Single-quote shell escaping: safe for arbitrary tokens.
		b.WriteString("export ")
		b.WriteString(k)
		b.WriteString("=")
		b.WriteString(shellSingleQuote(v))
		b.WriteString("\n")
	}
	b.WriteString("\n")
	b.WriteString(macosBootstrapScript())
	return b.String()
}

func shellSingleQuote(value string) string {
	// 'foo' -> '\'' within single quotes: '"'"'
	return "'" + strings.ReplaceAll(value, "'", `'\"'\"'`) + "'"
}

func prependNSCRegionArgs(args []string, computeBaseURL string) []string {
	region := strings.TrimSpace(os.Getenv("NSC_REGION"))
	if region == "" {
		region = regionFromComputeBaseURL(computeBaseURL)
	}
	if region == "" {
		// Default to the burrow region used for other Namespace integrations.
		region = "ord4"
	}
	return append([]string{"--region", region}, args...)
}

func regionFromComputeBaseURL(raw string) string {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return ""
	}
	u, err := url.Parse(raw)
	if err != nil {
		return ""
	}
	host := u.Hostname()
	if host == "" {
		return ""
	}
	parts := strings.Split(host, ".")
	if len(parts) == 0 {
		return ""
	}
	// ord4.compute.namespaceapis.com -> ord4
	if strings.HasSuffix(host, ".compute.namespaceapis.com") || strings.Contains(host, ".compute.") {
		return parts[0]
	}
	return ""
}
