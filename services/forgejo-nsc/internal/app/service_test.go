package app

import (
	"context"
	"sync"
	"testing"
	"time"

	"github.com/burrow/forgejo-nsc/internal/forgejo"
	"github.com/burrow/forgejo-nsc/internal/nsc"
)

type mockForgejo struct {
	mu      sync.Mutex
	tokens  []string
	scopes  []forgejo.Scope
	err     error
	counter int
}

func (m *mockForgejo) RegistrationToken(ctx context.Context, scope forgejo.Scope) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.scopes = append(m.scopes, scope)
	if m.err != nil {
		return "", m.err
	}
	if m.counter >= len(m.tokens) {
		return "", context.Canceled
	}
	tok := m.tokens[m.counter]
	m.counter++
	return tok, nil
}

type mockDispatcher struct {
	mu        sync.Mutex
	requests  []nsc.LaunchRequest
	responses []string
	err       error
}

func (m *mockDispatcher) LaunchRunner(ctx context.Context, req nsc.LaunchRequest) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.err != nil {
		return "", m.err
	}
	m.requests = append(m.requests, req)
	idx := len(m.requests) - 1
	if idx < len(m.responses) {
		return m.responses[idx], nil
	}
	return "runner", nil
}

func TestServiceDispatchUsesDefaults(t *testing.T) {
	forgejoMock := &mockForgejo{tokens: []string{"token"}}
	dispatcherMock := &mockDispatcher{responses: []string{"runner-default"}}

	cfg := Config{
		DefaultScope:  forgejo.Scope{Level: forgejo.ScopeInstance},
		DefaultLabels: []string{"nscloud"},
		InstanceURL:   "https://forgejo.example.com",
		DefaultTTL:    15 * time.Minute,
	}

	service := NewService(cfg, forgejoMock, dispatcherMock, nil)

	resp, err := service.Dispatch(context.Background(), DispatchRequest{})
	if err != nil {
		t.Fatalf("Dispatch returned error: %v", err)
	}
	if len(resp.Runners) != 1 || resp.Runners[0].Name != "runner-default" {
		t.Fatalf("unexpected dispatch response: %+v", resp)
	}

	if len(forgejoMock.scopes) != 1 || forgejoMock.scopes[0].Level != forgejo.ScopeInstance {
		t.Fatalf("expected default scope, got %+v", forgejoMock.scopes)
	}

	if len(dispatcherMock.requests) != 1 {
		t.Fatalf("expected one dispatcher call, got %d", len(dispatcherMock.requests))
	}
	req := dispatcherMock.requests[0]
	if req.InstanceURL != cfg.InstanceURL {
		t.Fatalf("expected instance URL %s, got %s", cfg.InstanceURL, req.InstanceURL)
	}
	if got := req.Labels; len(got) != 1 || got[0] != "nscloud" {
		t.Fatalf("expected default labels, got %v", got)
	}
	if req.Duration != cfg.DefaultTTL {
		t.Fatalf("expected duration %v, got %v", cfg.DefaultTTL, req.Duration)
	}
}

func TestServiceDispatchCustomScopeAndCount(t *testing.T) {
	forgejoMock := &mockForgejo{tokens: []string{"token-1", "token-2"}}
	dispatcherMock := &mockDispatcher{responses: []string{"runner-1", "runner-2"}}

	cfg := Config{
		DefaultScope:  forgejo.Scope{Level: forgejo.ScopeInstance},
		DefaultLabels: []string{"default"},
		InstanceURL:   "https://forgejo.example.com",
		DefaultTTL:    10 * time.Minute,
	}

	service := NewService(cfg, forgejoMock, dispatcherMock, nil)

	reqScope := &Scope{Level: string(forgejo.ScopeRepository), Owner: "acme", Name: "repo"}
	res, err := service.Dispatch(context.Background(), DispatchRequest{
		Count:    2,
		Labels:   []string{"custom"},
		Scope:    reqScope,
		TTL:      5 * time.Minute,
		Machine:  "4x8",
		Image:    "runner:latest",
		ExtraEnv: map[string]string{"FOO": "bar"},
	})
	if err != nil {
		t.Fatalf("Dispatch returned error: %v", err)
	}
	if len(res.Runners) != 2 {
		t.Fatalf("expected two runners, got %+v", res)
	}

	if len(forgejoMock.scopes) != 2 {
		t.Fatalf("expected two scope calls, got %d", len(forgejoMock.scopes))
	}
	for _, scope := range forgejoMock.scopes {
		if scope.Level != forgejo.ScopeRepository || scope.Owner != "acme" || scope.Name != "repo" {
			t.Fatalf("unexpected scope: %+v", scope)
		}
	}

	if len(dispatcherMock.requests) != 2 {
		t.Fatalf("expected two dispatcher calls, got %d", len(dispatcherMock.requests))
	}
	for _, call := range dispatcherMock.requests {
		if call.MachineType != "4x8" || call.Image != "runner:latest" {
			t.Fatalf("unexpected machine/image in %+v", call)
		}
		if call.Duration != 5*time.Minute {
			t.Fatalf("expected TTL to override default, got %v", call.Duration)
		}
		if call.Labels[0] != "custom" {
			t.Fatalf("expected custom labels, got %v", call.Labels)
		}
		if call.ExtraEnv["FOO"] != "bar" {
			t.Fatalf("expected env passthrough, got %v", call.ExtraEnv)
		}
	}
}

func TestServiceDispatchErrorsWithoutLabels(t *testing.T) {
	service := NewService(Config{DefaultScope: forgejo.Scope{Level: forgejo.ScopeInstance}}, &mockForgejo{}, &mockDispatcher{}, nil)
	if _, err := service.Dispatch(context.Background(), DispatchRequest{}); err == nil {
		t.Fatalf("expected error when no labels are available")
	}
}
