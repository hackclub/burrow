package main

import (
	"context"
	"flag"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/burrow/forgejo-nsc/internal/app"
	"github.com/burrow/forgejo-nsc/internal/config"
	"github.com/burrow/forgejo-nsc/internal/forgejo"
	"github.com/burrow/forgejo-nsc/internal/nsc"
	"github.com/burrow/forgejo-nsc/internal/server"
)

func main() {
	var configPath string
	flag.StringVar(&configPath, "config", "config.yaml", "Path to the dispatcher config file.")
	flag.Parse()

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))

	cfg, err := config.Load(configPath)
	if err != nil {
		logger.Error("failed to load config", "error", err)
		os.Exit(1)
	}

	scope, err := cfg.Forgejo.DefaultScope.ToScope()
	if err != nil {
		logger.Error("invalid default scope", "error", err)
		os.Exit(1)
	}

	forgejoClient, err := forgejo.NewClient(cfg.Forgejo.BaseURL, cfg.Forgejo.Token)
	if err != nil {
		logger.Error("failed to create forgejo client", "error", err)
		os.Exit(1)
	}

	dispatcher, err := nsc.NewDispatcher(nsc.Options{
		BinaryPath:       cfg.Namespace.NSCBinary,
		ComputeBaseURL:   cfg.Namespace.ComputeBaseURL,
		DefaultImage:     cfg.Namespace.Image,
		DefaultMachine:   cfg.Namespace.MachineType,
		MacosBaseImageID: cfg.Namespace.MacosBaseImageID,
		MacosMachineArch: cfg.Namespace.MacosMachineArch,
		DefaultDuration:  cfg.Namespace.Duration.Duration,
		WorkDir:          cfg.Namespace.WorkDir,
		MaxParallel:      cfg.Namespace.MaxParallel,
		RunnerNamePrefix: cfg.Runner.NamePrefix,
		Executor:         cfg.Runner.Executor,
		Network:          cfg.Namespace.Network,
		Logger:           logger,
	})
	if err != nil {
		logger.Error("failed to create dispatcher", "error", err)
		os.Exit(1)
	}

	service := app.NewService(app.Config{
		DefaultScope:  scope,
		DefaultLabels: cfg.Forgejo.DefaultLabels,
		InstanceURL:   cfg.Forgejo.InstanceURL,
		DefaultTTL:    cfg.Namespace.Duration.Duration,
		AllowLabels:   cfg.Namespace.AllowLabels,
		AllowScopes:   cfg.Namespace.AllowScopes,
	}, forgejoClient, dispatcher, logger)

	srv := server.New(cfg.Listen, service, logger)

	go func() {
		logger.Info("dispatcher listening", "addr", cfg.Listen)
		if err := srv.ListenAndServe(); err != nil && err != context.Canceled && err != http.ErrServerClosed {
			logger.Error("server terminated", "error", err)
		}
	}()

	interrupt := make(chan os.Signal, 1)
	signal.Notify(interrupt, syscall.SIGTERM, syscall.SIGINT)
	<-interrupt

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()
	_ = srv.Shutdown(ctx)
}
