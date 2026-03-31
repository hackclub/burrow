package forgejo

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"path"
	"strings"
	"time"
)

type ScopeLevel string

const (
	ScopeInstance     ScopeLevel = "instance"
	ScopeOrganization ScopeLevel = "organization"
	ScopeRepository   ScopeLevel = "repository"
)

type Scope struct {
	Level ScopeLevel
	Owner string
	Name  string
}

type Client struct {
	baseURL *url.URL
	token   string
	client  *http.Client
}

type Runner struct {
	ID     int64         `json:"id"`
	Name   string        `json:"name"`
	Status string        `json:"status"`
	Busy   bool          `json:"busy"`
	Labels []RunnerLabel `json:"labels"`
}

type RunnerLabel struct {
	Name string `json:"name"`
}

type RunJob struct {
	ID     int64    `json:"id"`
	Name   string   `json:"name"`
	RunsOn []string `json:"runs_on"`
	Status string   `json:"status"`
	TaskID int64    `json:"task_id"`
}

type WebhookConfig struct {
	URL         string
	ContentType string
	Events      []string
	Active      bool
}

type Option func(*Client)

func WithHTTPClient(httpClient *http.Client) Option {
	return func(c *Client) {
		if httpClient != nil {
			c.client = httpClient
		}
	}
}

func NewClient(rawURL, token string, opts ...Option) (*Client, error) {
	if rawURL == "" {
		return nil, errors.New("forgejo base URL is required")
	}

	u, err := url.Parse(rawURL)
	if err != nil {
		return nil, err
	}

	client := &Client{
		baseURL: u,
		token:   strings.TrimSpace(token),
		client: &http.Client{
			Timeout: 30 * time.Second,
		},
	}

	for _, opt := range opts {
		opt(client)
	}

	if client.token == "" {
		return nil, errors.New("forgejo token is required")
	}

	return client, nil
}

type registrationTokenResponse struct {
	Token string    `json:"token"`
	TTL   time.Time `json:"expires_at"`
}

func (c *Client) RegistrationToken(ctx context.Context, scope Scope) (string, error) {
	endpoint, err := c.registrationEndpoint(scope)
	if err != nil {
		return "", err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return "", err
	}
	req.Header.Set("Authorization", fmt.Sprintf("token %s", c.token))
	req.Header.Set("Accept", "application/json")

	resp, err := c.client.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return "", fmt.Errorf("forgejo returned %s", resp.Status)
	}

	var decoded registrationTokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&decoded); err != nil {
		return "", err
	}
	if decoded.Token == "" {
		return "", errors.New("forgejo response missing token")
	}

	return decoded.Token, nil
}

func (c *Client) ListRunners(ctx context.Context, scope Scope) ([]Runner, error) {
	endpoint, err := c.runnersEndpoint(scope)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", fmt.Sprintf("token %s", c.token))
	req.Header.Set("Accept", "application/json")

	resp, err := c.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("forgejo returned %s", resp.Status)
	}

	var decoded []Runner
	if err := json.NewDecoder(resp.Body).Decode(&decoded); err != nil {
		return nil, err
	}

	return decoded, nil
}

func (c *Client) ListRunJobs(ctx context.Context, scope Scope, labels []string) ([]RunJob, error) {
	endpoint, err := c.runJobsEndpoint(scope)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, err
	}
	if len(labels) > 0 {
		query := req.URL.Query()
		query.Set("labels", strings.Join(labels, ","))
		req.URL.RawQuery = query.Encode()
	}
	req.Header.Set("Authorization", fmt.Sprintf("token %s", c.token))
	req.Header.Set("Accept", "application/json")

	resp, err := c.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("forgejo returned %s", resp.Status)
	}

	var decoded []RunJob
	if err := json.NewDecoder(resp.Body).Decode(&decoded); err != nil {
		return nil, err
	}

	if decoded == nil {
		decoded = []RunJob{}
	}
	return decoded, nil
}

func (c *Client) EnsureWebhook(ctx context.Context, scope Scope, cfg WebhookConfig, secret string) error {
	if cfg.URL == "" {
		return nil
	}

	hooks, err := c.listWebhooks(ctx, scope)
	if err != nil {
		return err
	}

	for _, hook := range hooks {
		if strings.EqualFold(hook.Config.URL, cfg.URL) {
			return c.updateWebhook(ctx, scope, hook.ID, cfg, secret)
		}
	}

	return c.createWebhook(ctx, scope, cfg, secret)
}

func (c *Client) registrationEndpoint(scope Scope) (string, error) {
	var segments []string
	switch scope.Level {
	case ScopeRepository:
		if scope.Owner == "" || scope.Name == "" {
			return "", errors.New("repository scope requires owner and name")
		}
		segments = []string{"api", "v1", "repos", scope.Owner, scope.Name, "actions", "runners", "registration-token"}
	case ScopeOrganization:
		if scope.Owner == "" {
			return "", errors.New("organization scope requires owner")
		}
		segments = []string{"api", "v1", "orgs", scope.Owner, "actions", "runners", "registration-token"}
	case ScopeInstance:
		segments = []string{"api", "v1", "admin", "actions", "runners", "registration-token"}
	default:
		return "", fmt.Errorf("unsupported scope level %q", scope.Level)
	}

	clone := *c.baseURL
	clone.Path = path.Join(append([]string{clone.Path}, segments...)...)
	return clone.String(), nil
}

type webhook struct {
	ID     int64                `json:"id"`
	Config webhookConfigPayload `json:"config"`
}

type webhookConfigPayload struct {
	URL         string `json:"url"`
	ContentType string `json:"content_type"`
}

func (c *Client) listWebhooks(ctx context.Context, scope Scope) ([]webhook, error) {
	endpoint, err := c.webhooksEndpoint(scope)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", fmt.Sprintf("token %s", c.token))
	req.Header.Set("Accept", "application/json")

	resp, err := c.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("forgejo returned %s", resp.Status)
	}

	var hooks []webhook
	if err := json.NewDecoder(resp.Body).Decode(&hooks); err != nil {
		return nil, err
	}

	return hooks, nil
}

func (c *Client) createWebhook(ctx context.Context, scope Scope, cfg WebhookConfig, secret string) error {
	payload := webhookRequestPayload{
		Type: "gitea",
		Config: map[string]string{
			"url":          cfg.URL,
			"content_type": cfg.ContentType,
			"secret":       secret,
			"insecure_ssl": "0",
		},
		Events: cfg.Events,
		Active: cfg.Active,
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return err
	}

	endpoint, err := c.webhooksEndpoint(scope)
	if err != nil {
		return err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, endpoint, bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", fmt.Sprintf("token %s", c.token))
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return fmt.Errorf("forgejo returned %s", resp.Status)
	}

	return nil
}

func (c *Client) updateWebhook(ctx context.Context, scope Scope, id int64, cfg WebhookConfig, secret string) error {
	payload := webhookRequestPayload{
		Type: "gitea",
		Config: map[string]string{
			"url":          cfg.URL,
			"content_type": cfg.ContentType,
			"secret":       secret,
			"insecure_ssl": "0",
		},
		Events: cfg.Events,
		Active: cfg.Active,
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return err
	}

	endpoint, err := c.webhooksEndpoint(scope)
	if err != nil {
		return err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPatch, fmt.Sprintf("%s/%d", endpoint, id), bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", fmt.Sprintf("token %s", c.token))
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return fmt.Errorf("forgejo returned %s", resp.Status)
	}

	return nil
}

func (c *Client) webhooksEndpoint(scope Scope) (string, error) {
	var segments []string
	switch scope.Level {
	case ScopeRepository:
		if scope.Owner == "" || scope.Name == "" {
			return "", errors.New("repository scope requires owner and name")
		}
		segments = []string{"api", "v1", "repos", scope.Owner, scope.Name, "hooks"}
	case ScopeOrganization:
		if scope.Owner == "" {
			return "", errors.New("organization scope requires owner")
		}
		segments = []string{"api", "v1", "orgs", scope.Owner, "hooks"}
	default:
		return "", fmt.Errorf("webhook management not supported for scope level %q", scope.Level)
	}

	clone := *c.baseURL
	clone.Path = path.Join(append([]string{clone.Path}, segments...)...)
	return clone.String(), nil
}

type webhookRequestPayload struct {
	Type   string            `json:"type"`
	Config map[string]string `json:"config"`
	Events []string          `json:"events"`
	Active bool              `json:"active"`
}

func (c *Client) runnersEndpoint(scope Scope) (string, error) {
	var segments []string
	switch scope.Level {
	case ScopeRepository:
		if scope.Owner == "" || scope.Name == "" {
			return "", errors.New("repository scope requires owner and name")
		}
		segments = []string{"api", "v1", "repos", scope.Owner, scope.Name, "actions", "runners"}
	case ScopeOrganization:
		if scope.Owner == "" {
			return "", errors.New("organization scope requires owner")
		}
		segments = []string{"api", "v1", "orgs", scope.Owner, "actions", "runners"}
	case ScopeInstance:
		segments = []string{"api", "v1", "actions", "runners"}
	default:
		return "", fmt.Errorf("unsupported scope level %q", scope.Level)
	}

	clone := *c.baseURL
	clone.Path = path.Join(append([]string{clone.Path}, segments...)...)
	return clone.String(), nil
}

func (c *Client) runJobsEndpoint(scope Scope) (string, error) {
	var segments []string
	switch scope.Level {
	case ScopeRepository:
		if scope.Owner == "" || scope.Name == "" {
			return "", errors.New("repository scope requires owner and name")
		}
		segments = []string{"api", "v1", "repos", scope.Owner, scope.Name, "actions", "runners", "jobs"}
	case ScopeOrganization:
		if scope.Owner == "" {
			return "", errors.New("organization scope requires owner")
		}
		segments = []string{"api", "v1", "orgs", scope.Owner, "actions", "runners", "jobs"}
	default:
		return "", fmt.Errorf("run jobs not supported for scope level %q", scope.Level)
	}

	clone := *c.baseURL
	clone.Path = path.Join(append([]string{clone.Path}, segments...)...)
	return clone.String(), nil
}
