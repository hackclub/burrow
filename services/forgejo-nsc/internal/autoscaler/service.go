package autoscaler

import (
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/go-chi/chi/v5"

	"namespacelabs.dev/foundation/std/tasks"

	"github.com/burrow/forgejo-nsc/internal/forgejo"
)

type Service struct {
	listen      string
	controllers map[string]*InstanceController
	router      chi.Router
}

func NewService(cfg Config) (*Service, error) {
	controllers := make(map[string]*InstanceController)
	for _, inst := range cfg.Instances {
		scope, err := inst.Scope.ToScope()
		if err != nil {
			return nil, err
		}
		forgejoClient, err := forgejo.NewClient(inst.Forgejo.BaseURL, inst.Forgejo.Token)
		if err != nil {
			return nil, err
		}
		dispCfg := cfg.Dispatcher
		if inst.Dispatcher != nil && inst.Dispatcher.URL != "" {
			dispCfg = *inst.Dispatcher
			if dispCfg.Timeout.Duration == 0 {
				dispCfg.Timeout = cfg.Dispatcher.Timeout
			}
		}
		dClient := newDispatcherClient(dispCfg.URL, dispCfg.Timeout.Duration)
		webhookActive := true
		if inst.Webhook.Active != nil {
			webhookActive = *inst.Webhook.Active
		}
		controller := &InstanceController{
			name:       inst.Name,
			cfg:        inst,
			scope:      scope,
			forgejo:    forgejoClient,
			dispatcher: dClient,
			webhook: forgejo.WebhookConfig{
				URL:         inst.Webhook.URL,
				ContentType: inst.Webhook.ContentType,
				Events:      inst.Webhook.Events,
				Active:      webhookActive,
			},
			secret: inst.WebhookSecret,
		}
		controllers[inst.Name] = controller
	}

	router := chi.NewRouter()
	service := &Service{
		listen:      cfg.Listen,
		controllers: controllers,
		router:      router,
	}

	router.Get("/healthz", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("ok"))
	})
	router.Post("/webhook/{instance}", service.handleWebhook)

	return service, nil
}

func (s *Service) Start(ctx context.Context) error {
	for _, controller := range s.controllers {
		if err := controller.EnsureWebhook(ctx); err != nil {
			return err
		}
	}

	var wg sync.WaitGroup
	for _, controller := range s.controllers {
		wg.Add(1)
		go func(c *InstanceController) {
			defer wg.Done()
			c.Run(ctx)
		}(controller)
	}

	srv := &http.Server{
		Addr:    s.listen,
		Handler: s.router,
	}

	go func() {
		<-ctx.Done()
		_ = srv.Shutdown(context.Background())
	}()

	if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
		return err
	}
	wg.Wait()
	return nil
}

func (s *Service) handleWebhook(w http.ResponseWriter, r *http.Request) {
	name := chi.URLParam(r, "instance")
	controller, ok := s.controllers[name]
	if !ok {
		http.Error(w, "unknown instance", http.StatusNotFound)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "invalid body", http.StatusBadRequest)
		return
	}
	if controller.cfg.WebhookSecret != "" {
		signature := r.Header.Get("X-Gitea-Signature")
		if signature == "" {
			http.Error(w, "missing signature", http.StatusUnauthorized)
			return
		}
		if !verifySignature(controller.cfg.WebhookSecret, signature, body) {
			http.Error(w, "invalid signature", http.StatusUnauthorized)
			return
		}
	}

	var payload workflowJobPayload
	if err := json.Unmarshal(body, &payload); err != nil {
		http.Error(w, "bad payload", http.StatusBadRequest)
		return
	}

	controller.MarkWebhookSeen()
	if payload.Action == "queued" {
		controller.DispatchForJob(r.Context(), payload)
	}

	w.WriteHeader(http.StatusAccepted)
}

type workflowJobPayload struct {
	Action      string `json:"action"`
	WorkflowJob struct {
		Labels []string `json:"labels"`
	} `json:"workflow_job"`
}

type InstanceController struct {
	name       string
	cfg        InstanceConfig
	scope      forgejo.Scope
	forgejo    *forgejo.Client
	dispatcher *dispatcherClient
	ready      atomic.Bool
	webhook    forgejo.WebhookConfig
	secret     string
}

func (c *InstanceController) EnsureWebhook(ctx context.Context) error {
	if c.webhook.URL == "" {
		return nil
	}
	return tasks.Action("autoscaler.ensure-webhook").Arg("instance", c.name).Run(ctx, func(ctx context.Context) error {
		return c.forgejo.EnsureWebhook(ctx, c.scope, c.webhook, c.secret)
	})
}

func (c *InstanceController) Run(ctx context.Context) {
	if c.cfg.DisablePolling {
		<-ctx.Done()
		return
	}
	ticker := time.NewTicker(c.cfg.PollInterval.Duration)
	defer ticker.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			_ = tasks.Action("autoscaler.poll").Arg("instance", c.name).Run(ctx, func(ctx context.Context) error {
				return c.reconcile(ctx)
			})
		}
	}
}

func (c *InstanceController) reconcile(ctx context.Context) error {
	runners, err := c.forgejo.ListRunners(ctx, c.scope)
	if err != nil {
		// Keep polling even if runner listing fails; we can still dispatch based on queued jobs.
		runners = nil
	}

	for _, target := range c.cfg.Targets {
		idle := countIdle(runners, target.Labels)

		need := 0
		if idle < target.MinIdle {
			need = target.MinIdle - idle
		}

		jobs, jobErr := c.forgejo.ListRunJobs(ctx, c.scope, target.Labels)
		if jobErr != nil {
			return jobErr
		}
		waiting := countWaitingJobs(jobs, target.Labels)
		// Scale-to-zero friendly: if anything is waiting and there are no idle runners
		// for that label set, dispatch exactly one runner to unblock the queue.
		if waiting > 0 && idle == 0 && need < 1 {
			need = 1
		}

		if need <= 0 {
			continue
		}
		if err := c.dispatch(ctx, target, need, "poll"); err != nil {
			return err
		}
	}
	return nil
}

func (c *InstanceController) dispatch(ctx context.Context, target TargetConfig, count int, reason string) error {
	if count <= 0 {
		return nil
	}
	req := dispatcherRequest{
		Count:  count,
		Labels: target.Labels,
	}
	if target.TTL.Duration > 0 {
		req.TTL = target.TTL.Duration.String()
	}
	if target.MachineType != "" {
		req.MachineType = target.MachineType
	}
	if target.Image != "" {
		req.Image = target.Image
	}
	if len(target.Env) > 0 {
		req.Env = target.Env
	}
	return tasks.Action("autoscaler.dispatch").Arg("instance", c.name).Arg("reason", reason).Arg("labels", strings.Join(target.Labels, ",")).Run(ctx, func(ctx context.Context) error {
		return c.dispatcher.Dispatch(ctx, req)
	})
}

func (c *InstanceController) DispatchForJob(ctx context.Context, payload workflowJobPayload) {
	action := strings.ToLower(payload.Action)
	if action != "queued" && action != "waiting" {
		return
	}
	jobLabels := payload.WorkflowJob.Labels
	for _, target := range c.cfg.Targets {
		if labelsMatch(jobLabels, target.Labels) {
			_ = c.dispatch(ctx, target, 1, "webhook")
			return
		}
	}
}

func (c *InstanceController) MarkWebhookSeen() {
	c.ready.Store(true)
}

func countIdle(runners []forgejo.Runner, labels []string) int {
	count := 0
	for _, runner := range runners {
		if strings.ToLower(runner.Status) != "online" || runner.Busy {
			continue
		}
		if labelsMatch(extractLabels(runner.Labels), labels) {
			count++
		}
	}
	return count
}

func countWaitingJobs(jobs []forgejo.RunJob, labels []string) int {
	count := 0
	for _, job := range jobs {
		if status := strings.ToLower(job.Status); status != "waiting" && status != "queued" {
			continue
		}
		if labelsMatch(job.RunsOn, labels) {
			count++
		}
	}
	return count
}

func extractLabels(src []forgejo.RunnerLabel) []string {
	result := make([]string, 0, len(src))
	for _, lbl := range src {
		result = append(result, lbl.Name)
	}
	return result
}

func labelsMatch(have, want []string) bool {
	set := make(map[string]struct{}, len(have))
	for _, label := range have {
		set[label] = struct{}{}
	}
	for _, label := range want {
		if _, ok := set[label]; !ok {
			return false
		}
	}
	return true
}

func verifySignature(secret, signature string, body []byte) bool {
	parts := strings.SplitN(signature, "=", 2)
	if len(parts) == 2 {
		signature = parts[1]
	}
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write(body)
	expected := hex.EncodeToString(mac.Sum(nil))
	return hmac.Equal([]byte(expected), []byte(signature))
}

type dispatcherClient struct {
	url    string
	client *http.Client
}

type dispatcherRequest struct {
	Count       int               `json:"count"`
	Labels      []string          `json:"labels"`
	TTL         string            `json:"ttl,omitempty"`
	MachineType string            `json:"machine_type,omitempty"`
	Image       string            `json:"image,omitempty"`
	Env         map[string]string `json:"env,omitempty"`
}

func newDispatcherClient(url string, timeout time.Duration) *dispatcherClient {
	if timeout == 0 {
		timeout = 30 * time.Second
	}
	return &dispatcherClient{
		url: url,
		client: &http.Client{
			Timeout: timeout,
		},
	}
}

func (d *dispatcherClient) Dispatch(ctx context.Context, req dispatcherRequest) error {
	body, _ := json.Marshal(req)
	endpoint := strings.TrimSuffix(d.url, "/") + "/api/v1/dispatch"
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, endpoint, bytes.NewReader(body))
	if err != nil {
		return err
	}
	httpReq.Header.Set("Content-Type", "application/json")
	resp, err := d.client.Do(httpReq)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 300 {
		return fmt.Errorf("dispatcher returned %s", resp.Status)
	}
	return nil
}
