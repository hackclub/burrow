package main

import (
	"context"
	"flag"
	"log/slog"
	"os"
	"os/signal"
	"syscall"

	"namespacelabs.dev/foundation/std/tasks"
	"namespacelabs.dev/foundation/std/tasks/simplelog"

	"github.com/burrow/forgejo-nsc/internal/autoscaler"
)

func main() {
	var configPath string
	flag.StringVar(&configPath, "config", "autoscaler.yaml", "Path to the autoscaler config file")
	flag.Parse()

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))

	cfg, err := autoscaler.LoadConfig(configPath)
	if err != nil {
		logger.Error("failed to load config", "error", err)
		os.Exit(1)
	}

	service, err := autoscaler.NewService(cfg)
	if err != nil {
		logger.Error("failed to initialize autoscaler", "error", err)
		os.Exit(1)
	}

	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()
	ctx = tasks.WithSink(ctx, simplelog.NewSink(os.Stdout, 0))

	if err := tasks.Action("autoscaler.run").Run(ctx, func(ctx context.Context) error {
		return service.Start(ctx)
	}); err != nil {
		logger.Error("autoscaler exited", "error", err)
		os.Exit(1)
	}
}
