package nsc

import (
	"regexp"
	"strings"
)

const windowsDefaultMachineType = "windows/amd64:8x16"

var cpuMemShapePattern = regexp.MustCompile(`^\d+x\d+$`)

func hasWindowsLabel(labels []string) bool {
	for _, label := range labels {
		l := strings.TrimSpace(label)
		if l == "" {
			continue
		}
		base := l
		if before, _, ok := strings.Cut(l, ":"); ok {
			base = before
		}
		if strings.HasPrefix(base, "namespace-profile-windows-") {
			return true
		}
	}
	return false
}

func normalizeWindowsMachineType(machineType string, labels []string) string {
	mt := strings.TrimSpace(machineType)
	if strings.HasPrefix(mt, "windows/") {
		return mt
	}
	if cpuMemShapePattern.MatchString(mt) {
		return "windows/amd64:" + mt
	}

	// Label-derived defaults: keep a simple shape ladder for explicit profile sizes.
	for _, label := range labels {
		base := strings.TrimSpace(label)
		if before, _, ok := strings.Cut(base, ":"); ok {
			base = before
		}
		switch {
		case strings.HasPrefix(base, "namespace-profile-windows-small"):
			return "windows/amd64:2x4"
		case strings.HasPrefix(base, "namespace-profile-windows-medium"):
			return "windows/amd64:4x8"
		case strings.HasPrefix(base, "namespace-profile-windows-large"):
			return windowsDefaultMachineType
		}
	}
	return windowsDefaultMachineType
}

func powershellSingleQuote(value string) string {
	// PowerShell single-quoted string escaping: ' -> ''
	return "'" + strings.ReplaceAll(value, "'", "''") + "'"
}
