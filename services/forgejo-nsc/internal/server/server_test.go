package server

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"sync"
	"testing"
	"time"

	"github.com/burrow/forgejo-nsc/internal/app"
	"github.com/burrow/forgejo-nsc/internal/forgejo"
	"github.com/burrow/forgejo-nsc/internal/nsc"
)

type serverForgejoMock struct {
	mu     sync.Mutex
	token  string
	scopes []forgejo.Scope
}

func (m *serverForgejoMock) RegistrationToken(ctx context.Context, scope forgejo.Scope) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.scopes = append(m.scopes, scope)
	return m.token, nil
}

type serverDispatcherMock struct {
	mu       sync.Mutex
	requests []nsc.LaunchRequest
	result   string
}

func (m *serverDispatcherMock) LaunchRunner(ctx context.Context, req nsc.LaunchRequest) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.requests = append(m.requests, req)
	if m.result != "" {
		return m.result, nil
	}
	return "runner", nil
}

func TestDispatchEndpoint(t *testing.T) {
	forgejoMock := &serverForgejoMock{token: "token"}
	dispatcherMock := &serverDispatcherMock{result: "runner-http"}

	cfg := app.Config{
		DefaultScope:  forgejo.Scope{Level: forgejo.ScopeInstance},
		DefaultLabels: []string{"fallback"},
		InstanceURL:   "https://forgejo.example.com",
		DefaultTTL:    30 * time.Minute,
	}

	service := app.NewService(cfg, forgejoMock, dispatcherMock, nil)
	srv := New(":0", service, nil)
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	body := map[string]any{
		"count":        1,
		"ttl":          "45m",
		"labels":       []string{"nscloud-arm"},
		"scope":        map[string]string{"level": string(forgejo.ScopeOrganization), "owner": "acme"},
		"machine_type": "8x16",
		"image":        "runner:http",
		"env":          map[string]string{"FOO": "bar"},
	}

	payload, _ := json.Marshal(body)

	resp, err := http.Post(ts.URL+"/api/v1/dispatch", "application/json", bytes.NewReader(payload))
	if err != nil {
		t.Fatalf("POST failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 OK, got %d", resp.StatusCode)
	}

	var decoded app.DispatchResponse
	if err := json.NewDecoder(resp.Body).Decode(&decoded); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if len(decoded.Runners) != 1 || decoded.Runners[0].Name != "runner-http" {
		t.Fatalf("unexpected response: %+v", decoded)
	}

	if len(forgejoMock.scopes) != 1 || forgejoMock.scopes[0].Level != forgejo.ScopeOrganization {
		t.Fatalf("expected organization scope, got %+v", forgejoMock.scopes)
	}

	if len(dispatcherMock.requests) != 1 {
		t.Fatalf("expected dispatcher call")
	}
	call := dispatcherMock.requests[0]
	if call.Duration != 45*time.Minute {
		t.Fatalf("expected ttl override, got %v", call.Duration)
	}
	if call.Labels[0] != "nscloud-arm" {
		t.Fatalf("expected labels passthrough, got %v", call.Labels)
	}
	if call.ExtraEnv["FOO"] != "bar" {
		t.Fatalf("expected env passthrough")
	}
}
