package nsc

import "testing"

func TestHasWindowsLabel(t *testing.T) {
	t.Parallel()

	cases := []struct {
		name   string
		labels []string
		want   bool
	}{
		{
			name:   "namespace windows label",
			labels: []string{"namespace-profile-windows-large"},
			want:   true,
		},
		{
			name:   "namespace windows label with host suffix",
			labels: []string{"namespace-profile-windows-large:host"},
			want:   true,
		},
		{
			name:   "non namespace windows-like label",
			labels: []string{"burrow-winrunner:host"},
			want:   false,
		},
		{
			name:   "macos label",
			labels: []string{"namespace-profile-macos-large"},
			want:   false,
		},
	}

	for _, tc := range cases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			got := hasWindowsLabel(tc.labels)
			if got != tc.want {
				t.Fatalf("hasWindowsLabel(%v) = %v, want %v", tc.labels, got, tc.want)
			}
		})
	}
}

func TestNormalizeWindowsMachineType(t *testing.T) {
	t.Parallel()

	cases := []struct {
		name       string
		machine    string
		labels     []string
		wantPrefix string
	}{
		{
			name:       "explicit windows machine type keeps value",
			machine:    "windows/amd64:8x16",
			labels:     []string{"namespace-profile-windows-large"},
			wantPrefix: "windows/amd64:8x16",
		},
		{
			name:       "shape only is normalized",
			machine:    "4x8",
			labels:     []string{"namespace-profile-windows-large"},
			wantPrefix: "windows/amd64:4x8",
		},
		{
			name:       "large label default",
			machine:    "",
			labels:     []string{"namespace-profile-windows-large"},
			wantPrefix: "windows/amd64:8x16",
		},
		{
			name:       "medium label default",
			machine:    "",
			labels:     []string{"namespace-profile-windows-medium"},
			wantPrefix: "windows/amd64:4x8",
		},
		{
			name:       "fallback default",
			machine:    "",
			labels:     []string{"namespace-profile-windows-custom"},
			wantPrefix: "windows/amd64:8x16",
		},
	}

	for _, tc := range cases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			got := normalizeWindowsMachineType(tc.machine, tc.labels)
			if got != tc.wantPrefix {
				t.Fatalf("normalizeWindowsMachineType(%q, %v) = %q, want %q", tc.machine, tc.labels, got, tc.wantPrefix)
			}
		})
	}
}
