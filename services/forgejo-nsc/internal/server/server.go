package server

import (
	"context"
	"encoding/json"
	"errors"
	"log/slog"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"

	"github.com/burrow/forgejo-nsc/internal/app"
)

type Server struct {
	httpServer *http.Server
	app        *app.Service
	log        *slog.Logger
}

func New(listen string, svc *app.Service, logger *slog.Logger) *Server {
	if logger == nil {
		logger = slog.Default()
	}

	router := chi.NewRouter()
	router.Use(middleware.RequestID)
	router.Use(middleware.RealIP)
	router.Use(middleware.Logger)
	router.Use(middleware.Recoverer)

	s := &Server{
		app: svc,
		log: logger,
		httpServer: &http.Server{
			Addr:        listen,
			Handler:     router,
			ReadTimeout: 30 * time.Second,
			// Dispatch requests can legitimately run for the duration of a build.
			// A short WriteTimeout will kill the request context mid-provisioning.
			WriteTimeout: 2 * time.Hour,
			IdleTimeout:  60 * time.Second,
		},
	}

	router.Get("/healthz", s.handleHealthz)
	router.Post("/api/v1/dispatch", s.handleDispatch)

	return s
}

func (s *Server) ListenAndServe() error {
	return s.httpServer.ListenAndServe()
}

func (s *Server) Shutdown(ctx context.Context) error {
	return s.httpServer.Shutdown(ctx)
}

// Handler exposes the underlying HTTP handler for tests.
func (s *Server) Handler() http.Handler {
	return s.httpServer.Handler
}

type dispatchRequest struct {
	Count   int               `json:"count"`
	Labels  []string          `json:"labels"`
	Scope   *dispatchScope    `json:"scope"`
	TTL     string            `json:"ttl"`
	Machine string            `json:"machine_type"`
	Image   string            `json:"image"`
	Env     map[string]string `json:"env"`
}

type dispatchScope struct {
	Level string `json:"level"`
	Owner string `json:"owner"`
	Name  string `json:"name"`
}

func (s *Server) handleDispatch(w http.ResponseWriter, r *http.Request) {
	var payload dispatchRequest
	if err := json.NewDecoder(r.Body).Decode(&payload); err != nil {
		s.writeError(w, http.StatusBadRequest, err)
		return
	}

	duration, err := parseDuration(payload.TTL)
	if err != nil {
		s.writeError(w, http.StatusBadRequest, err)
		return
	}

	var scope *app.Scope
	if payload.Scope != nil {
		scope = &app.Scope{
			Level: payload.Scope.Level,
			Owner: payload.Scope.Owner,
			Name:  payload.Scope.Name,
		}
	}

	resp, err := s.app.Dispatch(r.Context(), app.DispatchRequest{
		Count:    payload.Count,
		Labels:   payload.Labels,
		Scope:    scope,
		TTL:      duration,
		Machine:  payload.Machine,
		Image:    payload.Image,
		ExtraEnv: payload.Env,
	})
	if err != nil {
		s.writeError(w, http.StatusInternalServerError, err)
		return
	}

	s.writeJSON(w, http.StatusOK, resp)
}

func parseDuration(value string) (time.Duration, error) {
	if value == "" {
		return 0, nil
	}
	dur, err := time.ParseDuration(value)
	if err != nil {
		return 0, err
	}
	if dur <= 0 {
		return 0, errors.New("ttl must be positive")
	}
	return dur, nil
}

func (s *Server) handleHealthz(w http.ResponseWriter, _ *http.Request) {
	s.writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func (s *Server) writeError(w http.ResponseWriter, code int, err error) {
	s.log.Error("request failed", "err", err, "status", code)
	s.writeJSON(w, code, map[string]string{
		"error": err.Error(),
	})
}

func (s *Server) writeJSON(w http.ResponseWriter, code int, payload any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(payload)
}
