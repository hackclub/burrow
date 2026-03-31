package app

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"strings"
	"time"

	"golang.org/x/sync/errgroup"

	"github.com/burrow/forgejo-nsc/internal/forgejo"
	"github.com/burrow/forgejo-nsc/internal/nsc"
)

type Dispatcher interface {
	LaunchRunner(ctx context.Context, req nsc.LaunchRequest) (string, error)
}

type ForgejoClient interface {
	RegistrationToken(ctx context.Context, scope forgejo.Scope) (string, error)
}

type Service struct {
	forgejo    ForgejoClient
	dispatcher Dispatcher
	logger     *slog.Logger

	defaultScope  forgejo.Scope
	defaultLabels []string
	instanceURL   string
	defaultTTL    time.Duration

	allowLabels map[string]struct{}
	allowScopes map[string]struct{}
}

type Config struct {
	DefaultScope  forgejo.Scope
	DefaultLabels []string
	InstanceURL   string
	DefaultTTL    time.Duration
	AllowLabels   []string
	AllowScopes   []string
}

func NewService(cfg Config, forgejo ForgejoClient, dispatcher Dispatcher, logger *slog.Logger) *Service {
	if logger == nil {
		logger = slog.Default()
	}
	allowLabels := make(map[string]struct{}, len(cfg.AllowLabels))
	for _, label := range cfg.AllowLabels {
		allowLabels[normalizeLabel(label)] = struct{}{}
	}
	allowScopes := make(map[string]struct{}, len(cfg.AllowScopes))
	for _, scope := range cfg.AllowScopes {
		allowScopes[scope] = struct{}{}
	}
	return &Service{
		defaultScope:  cfg.DefaultScope,
		defaultLabels: cfg.DefaultLabels,
		instanceURL:   cfg.InstanceURL,
		defaultTTL:    cfg.DefaultTTL,
		forgejo:       forgejo,
		dispatcher:    dispatcher,
		logger:        logger,
		allowLabels:   allowLabels,
		allowScopes:   allowScopes,
	}
}

type DispatchRequest struct {
	Count    int
	Labels   []string
	Scope    *Scope
	TTL      time.Duration
	Machine  string
	Image    string
	ExtraEnv map[string]string
}

type Scope struct {
	Level string
	Owner string
	Name  string
}

type DispatchResponse struct {
	Runners []RunnerHandle `json:"runners"`
}

type RunnerHandle struct {
	Name string `json:"name"`
}

func (s *Service) Dispatch(ctx context.Context, req DispatchRequest) (DispatchResponse, error) {
	count := req.Count
	if count <= 0 {
		count = 1
	}

	scope, err := s.mergeScope(req.Scope)
	if err != nil {
		return DispatchResponse{}, err
	}

	labels, err := s.mergeLabels(req.Labels)
	if err != nil {
		return DispatchResponse{}, err
	}
	if len(labels) == 0 {
		return DispatchResponse{}, errors.New("no runner labels resolved")
	}

	ttl := req.TTL
	if ttl == 0 {
		ttl = s.defaultTTL
	}

	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	res := DispatchResponse{
		Runners: make([]RunnerHandle, count),
	}
	eg, egCtx := errgroup.WithContext(ctx)

	for i := 0; i < count; i++ {
		index := i
		eg.Go(func() error {
			token, err := s.forgejo.RegistrationToken(egCtx, scope)
			if err != nil {
				return fmt.Errorf("fetching registration token: %w", err)
			}

			name, err := s.dispatcher.LaunchRunner(egCtx, nsc.LaunchRequest{
				Token:       token,
				InstanceURL: s.instanceURL,
				Labels:      labels,
				Duration:    ttl,
				MachineType: req.Machine,
				Image:       req.Image,
				ExtraEnv:    req.ExtraEnv,
			})
			if err != nil {
				return err
			}

			res.Runners[index] = RunnerHandle{Name: name}
			return nil
		})
	}

	if err := eg.Wait(); err != nil {
		return DispatchResponse{}, err
	}

	return res, nil
}

func (s *Service) mergeScope(value *Scope) (forgejo.Scope, error) {
	if value == nil {
		return s.defaultScope, nil
	}

	scope := forgejo.Scope{
		Level: forgejo.ScopeLevel(value.Level),
		Owner: value.Owner,
		Name:  value.Name,
	}
	if scope.Level == "" {
		return forgejo.Scope{}, errors.New("scope level is required")
	}
	switch scope.Level {
	case forgejo.ScopeInstance:
		if !s.scopeAllowed(scope) {
			return forgejo.Scope{}, fmt.Errorf("scope %q not allowed", scopeKey(scope))
		}
		return scope, nil
	case forgejo.ScopeOrganization:
		if scope.Owner == "" {
			return forgejo.Scope{}, errors.New("organization scope requires owner")
		}
		if !s.scopeAllowed(scope) {
			return forgejo.Scope{}, fmt.Errorf("scope %q not allowed", scopeKey(scope))
		}
		return scope, nil
	case forgejo.ScopeRepository:
		if scope.Owner == "" || scope.Name == "" {
			return forgejo.Scope{}, errors.New("repository scope requires owner and name")
		}
		if !s.scopeAllowed(scope) {
			return forgejo.Scope{}, fmt.Errorf("scope %q not allowed", scopeKey(scope))
		}
		return scope, nil
	default:
		return forgejo.Scope{}, fmt.Errorf("unsupported scope %q", scope.Level)
	}
}

func (s *Service) mergeLabels(labels []string) ([]string, error) {
	var resolved []string
	if len(labels) == 0 {
		resolved = append([]string{}, s.defaultLabels...)
	} else {
		resolved = labels
	}
	if len(s.allowLabels) == 0 {
		return resolved, nil
	}
	for _, label := range resolved {
		norm := normalizeLabel(label)
		if _, ok := s.allowLabels[norm]; !ok {
			return nil, fmt.Errorf("label %q not allowed", label)
		}
	}
	return resolved, nil
}

func normalizeLabel(label string) string {
	trimmed := strings.TrimSpace(label)
	if trimmed == "" {
		return ""
	}
	// Ignore any explicit executor suffix ("label:host"), since workflows
	// and config allowlists typically deal in base label names.
	if before, _, ok := strings.Cut(trimmed, ":"); ok {
		return before
	}
	return trimmed
}

func scopeKey(scope forgejo.Scope) string {
	switch scope.Level {
	case forgejo.ScopeInstance:
		return "instance"
	case forgejo.ScopeOrganization:
		return fmt.Sprintf("organization:%s", scope.Owner)
	case forgejo.ScopeRepository:
		return fmt.Sprintf("repository:%s/%s", scope.Owner, scope.Name)
	default:
		return string(scope.Level)
	}
}

func (s *Service) scopeAllowed(scope forgejo.Scope) bool {
	if len(s.allowScopes) == 0 {
		return true
	}
	_, ok := s.allowScopes[scopeKey(scope)]
	return ok
}
