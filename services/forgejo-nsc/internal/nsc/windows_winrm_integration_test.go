package nsc

import (
	"context"
	"io"
	"log/slog"
	"os"
	"os/exec"
	"strings"
	"testing"
	"time"
)

func TestWindowsWinRMScriptRoundTrip(t *testing.T) {
	if os.Getenv("NSC_WINDOWS_E2E") != "1" {
		t.Skip("set NSC_WINDOWS_E2E=1 to run Namespace Windows integration test")
	}

	nscBinary, err := exec.LookPath("nsc")
	if err != nil {
		t.Skipf("nsc not found in PATH: %v", err)
	}

	authCheck := exec.Command(nscBinary, "auth", "check-login")
	if out, err := authCheck.CombinedOutput(); err != nil {
		t.Skipf("nsc auth check-login failed: %v (%s)", err, strings.TrimSpace(string(out)))
	}

	machineType := strings.TrimSpace(os.Getenv("NSC_WINDOWS_E2E_MACHINE_TYPE"))
	if machineType == "" {
		machineType = "windows/amd64:4x8"
	}

	dispatcher, err := NewDispatcher(Options{
		BinaryPath:      nscBinary,
		DefaultImage:    "code.forgejo.org/forgejo/runner:11",
		DefaultMachine:  machineType,
		DefaultDuration: 20 * time.Minute,
		MaxParallel:     1,
		WorkDir:         t.TempDir(),
		ComputeBaseURL:  strings.TrimSpace(os.Getenv("NSC_COMPUTE_BASE_URL")),
		Logger:          slog.New(slog.NewTextHandler(io.Discard, nil)),
	})
	if err != nil {
		t.Fatalf("NewDispatcher() error: %v", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Minute)
	defer cancel()

	script := "Write-Output ('winrm-ok:' + $env:COMPUTERNAME)"
	labels := []string{"namespace-profile-windows-medium"}
	if err := dispatcher.launchWindowsScriptViaWinRM(ctx, "nsc-winrm-itest", 20*time.Minute, machineType, labels, script); err != nil {
		if strings.Contains(err.Error(), "does not expose winrm service (rdp-only)") {
			t.Skipf("namespace windows control channel is rdp-only: %v", err)
		}
		t.Fatalf("launchWindowsScriptViaWinRM() error: %v", err)
	}
}
