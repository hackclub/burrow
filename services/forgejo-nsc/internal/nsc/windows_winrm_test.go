package nsc

import "testing"

func TestParseProxyEndpoint(t *testing.T) {
	t.Parallel()

	tests := []struct {
		name   string
		raw    string
		want   string
		wantOK bool
	}{
		{
			name:   "plain json payload",
			raw:    `{"endpoint":"127.0.0.1:61234"}`,
			want:   "127.0.0.1:61234",
			wantOK: true,
		},
		{
			name: "json wrapped with extra output",
			raw: `Connected.
{"endpoint":"127.0.0.1:61235","rdp":{"credentials":{"username":"runneradmin","password":"runneradmin"}}}`,
			want:   "127.0.0.1:61235",
			wantOK: true,
		},
		{
			name:   "missing endpoint field",
			raw:    `{"rdp":{"credentials":{"username":"runneradmin"}}}`,
			wantOK: false,
		},
		{
			name:   "non-json output",
			raw:    `Failed: instance does not have service "winrm"`,
			wantOK: false,
		},
	}

	for _, tc := range tests {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()

			got, ok := parseProxyEndpoint(tc.raw)
			if ok != tc.wantOK {
				t.Fatalf("parseProxyEndpoint(%q) ok=%v, want %v", tc.raw, ok, tc.wantOK)
			}
			if got != tc.want {
				t.Fatalf("parseProxyEndpoint(%q) endpoint=%q, want %q", tc.raw, got, tc.want)
			}
		})
	}
}

func TestIndicatesMissingProxyService(t *testing.T) {
	t.Parallel()

	raw := `Failed: instance does not have service "winrm"`
	if !indicatesMissingProxyService(raw, "winrm") {
		t.Fatalf("indicatesMissingProxyService should return true for missing winrm message")
	}
	if indicatesMissingProxyService(raw, "ssh") {
		t.Fatalf("indicatesMissingProxyService should be false when service name does not match")
	}
}
