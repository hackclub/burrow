package config

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestLoadConfig(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "config.yaml")
	content := `
listen: ":9090"
forgejo:
  base_url: https://forgejo.test
  token: abc
  default_scope:
    level: instance
namespace:
  nsc_binary: /usr/bin/nsc
  image: ghcr.io/forgejo/runner:3
  duration: 15m
runner:
  name_prefix: custom-
`
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		t.Fatal(err)
	}

	cfg, err := Load(path)
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}
	if cfg.Listen != ":9090" {
		t.Fatalf("unexpected listen addr: %s", cfg.Listen)
	}
	if cfg.Namespace.Duration.Duration != 15*time.Minute {
		t.Fatalf("duration parsing failed: %s", cfg.Namespace.Duration.Duration)
	}
}
