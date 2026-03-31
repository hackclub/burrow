package nsc

import (
	"bufio"
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

type windowsProxyOutput struct {
	Endpoint string `json:"endpoint"`
	RDP      struct {
		Credentials struct {
			Username string `json:"username"`
			Password string `json:"password"`
		} `json:"credentials"`
	} `json:"rdp"`
}

func (d *Dispatcher) launchWindowsRunnerViaWinRM(ctx context.Context, runnerName string, req LaunchRequest, ttl time.Duration, machineType string) error {
	script := windowsBootstrapScript(runnerName, req, d.opts.Executor, d.opts.WorkDir)
	return d.launchWindowsScriptViaWinRM(ctx, runnerName, ttl, machineType, req.Labels, script)
}

func (d *Dispatcher) launchWindowsScriptViaWinRM(ctx context.Context, runnerName string, ttl time.Duration, machineType string, labels []string, script string) error {
	if ttl <= 0 {
		ttl = d.opts.DefaultDuration
	}

	mt := normalizeWindowsMachineType(machineType, labels)
	instanceID, createOutput, err := d.createWindowsInstance(ctx, runnerName, ttl, mt)
	if err != nil {
		return fmt.Errorf("windows create failed: %w\n%s", err, createOutput)
	}
	defer d.destroyNSCInstance(context.Background(), runnerName, instanceID)

	username, password, err := d.resolveWindowsCredentials(ctx, instanceID)
	if err != nil {
		return err
	}

	if err := d.probeWindowsWinRMService(ctx, instanceID); err != nil {
		return err
	}

	endpoint, stopForward, err := d.startWindowsWinRMPortForward(ctx, instanceID)
	if err != nil {
		return err
	}
	defer stopForward()

	if err := d.runWindowsWinRMPowerShell(ctx, endpoint, username, password, script); err != nil {
		return err
	}

	return nil
}

func (d *Dispatcher) createWindowsInstance(ctx context.Context, runnerName string, ttl time.Duration, machineType string) (instanceID string, output string, err error) {
	tmpDir, err := os.MkdirTemp("", "forgejo-nsc-windows-*")
	if err != nil {
		return "", "", fmt.Errorf("mktemp: %w", err)
	}
	defer os.RemoveAll(tmpDir)

	metaPath := filepath.Join(tmpDir, "create.json")
	cidPath := filepath.Join(tmpDir, "create.cid")

	args := []string{
		"create",
		"--duration", ttl.String(),
		"--machine_type", machineType,
		"--cidfile", cidPath,
		"--purpose", fmt.Sprintf("burrow forgejo runner %s", runnerName),
		"--output", "plain",
		"--output_json_to", metaPath,
		"--wait_timeout", "6m",
	}
	args = prependNSCRegionArgs(args, d.opts.ComputeBaseURL)

	createCtx, cancel := context.WithTimeout(ctx, 8*time.Minute)
	defer cancel()

	cmd := exec.CommandContext(createCtx, d.opts.BinaryPath, args...)
	var buf bytes.Buffer
	cmd.Stdout = &buf
	cmd.Stderr = &buf

	if err := cmd.Run(); err != nil {
		if created := strings.TrimSpace(mustReadFile(cidPath)); created != "" {
			d.destroyNSCInstance(context.Background(), runnerName, created)
		}
		if errors.Is(createCtx.Err(), context.DeadlineExceeded) {
			return "", buf.String(), fmt.Errorf("nsc create timed out after %s", 8*time.Minute)
		}
		return "", buf.String(), fmt.Errorf("nsc create failed: %w", err)
	}

	instanceID, err = readNSCCreateInstanceID(metaPath)
	if err != nil {
		return "", buf.String(), fmt.Errorf("nsc create output parse failed: %w", err)
	}
	if instanceID == "" {
		return "", buf.String(), errors.New("nsc create returned empty instance id")
	}
	return instanceID, buf.String(), nil
}

func (d *Dispatcher) resolveWindowsCredentials(ctx context.Context, instanceID string) (username string, password string, err error) {
	tmpDir, err := os.MkdirTemp("", "forgejo-nsc-winproxy-*")
	if err != nil {
		return "", "", fmt.Errorf("mktemp: %w", err)
	}
	defer os.RemoveAll(tmpDir)

	outPath := filepath.Join(tmpDir, "proxy.json")
	outFile, err := os.Create(outPath)
	if err != nil {
		return "", "", fmt.Errorf("create proxy output file: %w", err)
	}
	defer outFile.Close()

	var stderr bytes.Buffer
	args := []string{"instance", "proxy", instanceID, "-s", "rdp", "-o", "json"}
	args = prependNSCRegionArgs(args, d.opts.ComputeBaseURL)

	proxyCtx, cancel := context.WithTimeout(ctx, 90*time.Second)
	defer cancel()

	cmd := exec.CommandContext(proxyCtx, d.opts.BinaryPath, args...)
	cmd.Stdout = outFile
	cmd.Stderr = &stderr

	if err := cmd.Start(); err != nil {
		return "", "", fmt.Errorf("start nsc instance proxy: %w", err)
	}

	waitDone := make(chan struct{})
	var waitErr error
	go func() {
		waitErr = cmd.Wait()
		close(waitDone)
	}()

	var payload windowsProxyOutput
	deadline := time.Now().Add(45 * time.Second)
	for time.Now().Before(deadline) {
		raw, _ := os.ReadFile(outPath)
		jsonBlob := extractJSON(string(raw))
		if jsonBlob != "" {
			if err := json.Unmarshal([]byte(jsonBlob), &payload); err == nil {
				username = strings.TrimSpace(payload.RDP.Credentials.Username)
				password = strings.TrimSpace(payload.RDP.Credentials.Password)
				if username != "" && password != "" {
					break
				}
			}
		}
		select {
		case <-waitDone:
			if waitErr != nil {
				return "", "", fmt.Errorf("nsc instance proxy exited before credentials were available: %w\n%s", waitErr, stderr.String())
			}
		default:
		}
		time.Sleep(1 * time.Second)
	}

	if cmd.Process != nil {
		_ = cmd.Process.Kill()
	}
	<-waitDone

	if username == "" || password == "" {
		raw, _ := os.ReadFile(outPath)
		return "", "", fmt.Errorf("failed to resolve windows credentials from nsc instance proxy output\nstdout=%s\nstderr=%s", strings.TrimSpace(string(raw)), strings.TrimSpace(stderr.String()))
	}
	return username, password, nil
}

func (d *Dispatcher) probeWindowsWinRMService(ctx context.Context, instanceID string) error {
	args := []string{"instance", "proxy", instanceID, "-s", "winrm", "-o", "json", "--once"}
	args = prependNSCRegionArgs(args, d.opts.ComputeBaseURL)

	probeCtx, cancel := context.WithTimeout(ctx, 15*time.Second)
	defer cancel()

	cmd := exec.CommandContext(probeCtx, d.opts.BinaryPath, args...)
	var out bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &out

	err := cmd.Run()
	raw := strings.TrimSpace(out.String())
	if endpoint, ok := parseProxyEndpoint(raw); ok && endpoint != "" {
		return nil
	}

	if indicatesMissingProxyService(raw, "winrm") {
		return fmt.Errorf("namespace windows non-interactive channel unavailable: instance does not expose winrm service (rdp-only)\n%s", raw)
	}

	if errors.Is(probeCtx.Err(), context.DeadlineExceeded) {
		return fmt.Errorf("timed out probing Namespace winrm service before bootstrap\n%s", raw)
	}

	if err != nil {
		return fmt.Errorf("nsc winrm service probe failed: %w\n%s", err, raw)
	}
	return fmt.Errorf("nsc winrm service probe did not yield endpoint output\n%s", raw)
}

func parseProxyEndpoint(raw string) (string, bool) {
	jsonBlob := extractJSON(raw)
	if jsonBlob == "" {
		return "", false
	}
	var payload struct {
		Endpoint string `json:"endpoint"`
	}
	if err := json.Unmarshal([]byte(jsonBlob), &payload); err != nil {
		return "", false
	}
	endpoint := strings.TrimSpace(payload.Endpoint)
	if endpoint == "" {
		return "", false
	}
	return endpoint, true
}

func indicatesMissingProxyService(raw string, service string) bool {
	service = strings.TrimSpace(service)
	if service == "" {
		return false
	}
	token := fmt.Sprintf("does not have service %q", service)
	return strings.Contains(raw, token)
}

func (d *Dispatcher) startWindowsWinRMPortForward(ctx context.Context, instanceID string) (endpoint string, stop func(), err error) {
	args := []string{"instance", "port-forward", instanceID, "--target_port", "5985"}
	args = prependNSCRegionArgs(args, d.opts.ComputeBaseURL)

	forwardCtx, cancel := context.WithCancel(ctx)
	cmd := exec.CommandContext(forwardCtx, d.opts.BinaryPath, args...)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		cancel()
		return "", nil, fmt.Errorf("port-forward stdout pipe: %w", err)
	}
	var stderr bytes.Buffer
	cmd.Stderr = &stderr

	if err := cmd.Start(); err != nil {
		cancel()
		return "", nil, fmt.Errorf("start nsc port-forward: %w", err)
	}

	waitDone := make(chan struct{})
	var waitErr error
	go func() {
		waitErr = cmd.Wait()
		close(waitDone)
	}()

	endpointCh := make(chan string, 1)
	scanErrCh := make(chan error, 1)
	go func() {
		scanner := bufio.NewScanner(stdout)
		for scanner.Scan() {
			line := strings.TrimSpace(scanner.Text())
			if strings.HasPrefix(line, "Listening on ") {
				endpointCh <- strings.TrimSpace(strings.TrimPrefix(line, "Listening on "))
				return
			}
		}
		if err := scanner.Err(); err != nil {
			scanErrCh <- err
		}
	}()

	select {
	case endpoint = <-endpointCh:
		stop = func() {
			cancel()
			if cmd.Process != nil {
				_ = cmd.Process.Kill()
			}
			<-waitDone
		}
		return endpoint, stop, nil
	case err := <-scanErrCh:
		cancel()
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
		<-waitDone
		return "", nil, fmt.Errorf("failed reading port-forward output: %w", err)
	case <-waitDone:
		cancel()
		if waitErr != nil {
			return "", nil, fmt.Errorf("nsc port-forward exited early: %w\n%s", waitErr, stderr.String())
		}
		return "", nil, fmt.Errorf("nsc port-forward exited without endpoint\n%s", stderr.String())
	case <-time.After(45 * time.Second):
		cancel()
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
		<-waitDone
		return "", nil, fmt.Errorf("timed out waiting for WinRM port-forward endpoint\n%s", stderr.String())
	case <-ctx.Done():
		cancel()
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
		<-waitDone
		return "", nil, ctx.Err()
	}
}

func (d *Dispatcher) runWindowsWinRMPowerShell(ctx context.Context, endpoint, username, password, script string) error {
	pythonPath, err := exec.LookPath("python3")
	if err != nil {
		return fmt.Errorf("python3 is required for windows WinRM bootstrap: %w", err)
	}

	workdir := strings.TrimSpace(d.opts.WorkDir)
	if workdir == "" {
		workdir = "/tmp/forgejo-runner"
	}
	if err := os.MkdirAll(workdir, 0o755); err != nil {
		return fmt.Errorf("create workdir %s: %w", workdir, err)
	}

	venvPath := filepath.Join(workdir, ".winrm-venv")
	venvPython := filepath.Join(venvPath, "bin", "python")
	if _, err := os.Stat(venvPython); err != nil {
		cmd := exec.CommandContext(ctx, pythonPath, "-m", "venv", venvPath)
		var out bytes.Buffer
		cmd.Stdout = &out
		cmd.Stderr = &out
		if err := cmd.Run(); err != nil {
			return fmt.Errorf("create python venv for winrm failed: %w\n%s", err, out.String())
		}
	}

	ensurePyWinRM := `
import importlib.util, subprocess, sys
if importlib.util.find_spec("winrm") is None:
    subprocess.check_call([sys.executable, "-m", "pip", "install", "--quiet", "pywinrm"])
`
	ensureCmd := exec.CommandContext(ctx, venvPython, "-c", ensurePyWinRM)
	var ensureOut bytes.Buffer
	ensureCmd.Stdout = &ensureOut
	ensureCmd.Stderr = &ensureOut
	if err := ensureCmd.Run(); err != nil {
		return fmt.Errorf("install pywinrm failed: %w\n%s", err, ensureOut.String())
	}

	runScript := `
import base64, os, sys, time, traceback, winrm

endpoint = os.environ["WINRM_ENDPOINT"]
user = os.environ["WINRM_USER"]
password = os.environ["WINRM_PASS"]
script = base64.b64decode(os.environ["WINRM_SCRIPT_B64"]).decode("utf-8")

deadline = time.time() + 300.0
last_err = None

while time.time() < deadline:
    try:
        session = winrm.Session(f"http://{endpoint}/wsman", auth=(user, password), transport="ntlm")
        result = session.run_ps(script)
        sys.stdout.write(result.std_out.decode("utf-8", errors="replace"))
        sys.stderr.write(result.std_err.decode("utf-8", errors="replace"))
        print(f"winrm_exit={result.status_code}")
        sys.exit(result.status_code)
    except Exception as err:
        last_err = err
        time.sleep(5.0)

sys.stderr.write("timed out waiting for WinRM connectivity after 300s\\n")
if last_err is not None:
    traceback.print_exception(last_err, file=sys.stderr)
sys.exit(111)
`
	runCmd := exec.CommandContext(ctx, venvPython, "-c", runScript)
	runCmd.Env = append(os.Environ(),
		"WINRM_ENDPOINT="+endpoint,
		"WINRM_USER="+username,
		"WINRM_PASS="+password,
		"WINRM_SCRIPT_B64="+base64.StdEncoding.EncodeToString([]byte(script)),
	)
	var runOut bytes.Buffer
	runCmd.Stdout = &runOut
	runCmd.Stderr = &runOut
	if err := runCmd.Run(); err != nil {
		return fmt.Errorf("windows winrm bootstrap command failed: %w\n%s", err, runOut.String())
	}
	return nil
}

func windowsBootstrapScript(runnerName string, req LaunchRequest, executor, workdir string) string {
	if strings.TrimSpace(workdir) == "" {
		workdir = `C:\burrow\forgejo-runner`
	}

	runnerExec := strings.TrimSpace(executor)
	if runnerExec == "" || runnerExec == "shell" {
		runnerExec = "host"
	}

	safeName := strings.NewReplacer(`\`, "-", ":", "-", "/", "-", " ", "-").Replace(runnerName)
	workRoot := strings.TrimRight(workdir, `\`) + `\` + safeName

	var b strings.Builder
	b.WriteString("$ErrorActionPreference = 'Stop'\n")
	b.WriteString("$ProgressPreference = 'SilentlyContinue'\n")
	b.WriteString("[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12\n")
	b.WriteString("$runnerName = " + powershellSingleQuote(runnerName) + "\n")
	b.WriteString("$runnerToken = " + powershellSingleQuote(req.Token) + "\n")
	b.WriteString("$instanceURL = " + powershellSingleQuote(req.InstanceURL) + "\n")
	b.WriteString("$labelsCsv = " + powershellSingleQuote(strings.Join(req.Labels, ",")) + "\n")
	b.WriteString("$runnerExec = " + powershellSingleQuote(runnerExec) + "\n")
	b.WriteString("$workRoot = " + powershellSingleQuote(workRoot) + "\n")
	b.WriteString(`
New-Item -Path $workRoot -ItemType Directory -Force | Out-Null
Set-Location $workRoot

$runnerVersion = "12.6.4"
$zipUrl = "https://code.forgejo.org/forgejo/runner/releases/download/v${runnerVersion}/forgejo-runner-${runnerVersion}-windows-amd64.zip"
$zipPath = Join-Path $workRoot "forgejo-runner.zip"
$extractDir = Join-Path $workRoot "forgejo-runner"

if (Test-Path $extractDir) {
  Remove-Item -Path $extractDir -Recurse -Force
}

Invoke-WebRequest -Uri $zipUrl -OutFile $zipPath
Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

$runnerExe = Join-Path $extractDir "forgejo-runner.exe"
if (-not (Test-Path $runnerExe)) {
  throw "Missing forgejo-runner.exe after extract: $runnerExe"
}

$labels = @()
foreach ($label in ($labelsCsv -split ",")) {
  $trimmed = $label.Trim()
  if ([string]::IsNullOrWhiteSpace($trimmed)) { continue }
  if ($trimmed.Contains(":")) {
    $labels += $trimmed
  } else {
    $labels += ("{0}:{1}" -f $trimmed, $runnerExec)
  }
}
if ($labels.Count -eq 0) {
  throw "No runner labels resolved for windows bootstrap"
}

$labelLines = ($labels | ForEach-Object { "  - $_" }) -join [Environment]::NewLine
$configPath = Join-Path $workRoot "runner.yaml"
$runnerYaml = @"
log:
  level: info
runner:
  file: .runner
  capacity: 1
  name: $runnerName
  labels:
$labelLines
cache:
  enabled: false
"@
Set-Content -Path $configPath -Value $runnerYaml -Encoding UTF8

$labelsArg = ($labels -join ",")
& $runnerExe register --no-interactive --instance $instanceURL --token $runnerToken --name $runnerName --labels $labelsArg --config $configPath
if ($LASTEXITCODE -ne 0) {
  throw ("forgejo-runner register failed: {0}" -f $LASTEXITCODE)
}

& $runnerExe one-job --config $configPath
if ($LASTEXITCODE -ne 0) {
  throw ("forgejo-runner one-job failed: {0}" -f $LASTEXITCODE)
}
`)
	return b.String()
}
