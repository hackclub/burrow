package autoscaler

import (
	"fmt"
	"os"
	"time"

	"gopkg.in/yaml.v3"

	"github.com/burrow/forgejo-nsc/internal/config"
)

type Config struct {
	Listen     string           `yaml:"listen"`
	Dispatcher DispatcherConfig `yaml:"dispatcher"`
	Instances  []InstanceConfig `yaml:"instances"`
}

type DispatcherConfig struct {
	URL     string          `yaml:"url"`
	Timeout config.Duration `yaml:"timeout"`
}

type InstanceConfig struct {
	Name           string             `yaml:"name"`
	Forgejo        ForgejoInstance    `yaml:"forgejo"`
	Scope          config.ScopeConfig `yaml:"scope"`
	PollInterval   config.Duration    `yaml:"poll_interval"`
	DisablePolling bool               `yaml:"disable_polling"`
	WebhookSecret  string             `yaml:"webhook_secret"`
	Webhook        WebhookConfig      `yaml:"webhook"`
	Dispatcher     *DispatcherConfig  `yaml:"dispatcher"`
	Targets        []TargetConfig     `yaml:"targets"`
}

type ForgejoInstance struct {
	BaseURL string `yaml:"base_url"`
	Token   string `yaml:"token"`
}

type WebhookConfig struct {
	URL         string   `yaml:"url"`
	ContentType string   `yaml:"content_type"`
	Events      []string `yaml:"events"`
	Active      *bool    `yaml:"active"`
}

type TargetConfig struct {
	Labels      []string          `yaml:"labels"`
	MinIdle     int               `yaml:"min_idle"`
	TTL         config.Duration   `yaml:"ttl"`
	MachineType string            `yaml:"machine_type"`
	Image       string            `yaml:"image"`
	Env         map[string]string `yaml:"env"`
}

func LoadConfig(path string) (Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return Config{}, err
	}
	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return Config{}, err
	}
	if cfg.Listen == "" {
		cfg.Listen = ":8090"
	}
	if cfg.Dispatcher.URL == "" {
		return Config{}, fmt.Errorf("dispatcher.url is required")
	}
	if cfg.Dispatcher.Timeout.Duration == 0 {
		cfg.Dispatcher.Timeout = config.Duration{Duration: 15 * time.Second}
	}
	if len(cfg.Instances) == 0 {
		return Config{}, fmt.Errorf("at least one instance must be configured")
	}
	for i := range cfg.Instances {
		inst := &cfg.Instances[i]
		if inst.Name == "" {
			return Config{}, fmt.Errorf("instance[%d] missing name", i)
		}
		if inst.Forgejo.BaseURL == "" || inst.Forgejo.Token == "" {
			return Config{}, fmt.Errorf("instance %s missing forgejo.base_url or token", inst.Name)
		}
		if inst.PollInterval.Duration == 0 {
			inst.PollInterval = config.Duration{Duration: 30 * time.Second}
		}
		if len(inst.Webhook.Events) == 0 {
			inst.Webhook.Events = []string{"workflow_job"}
		}
		if inst.Webhook.ContentType == "" {
			inst.Webhook.ContentType = "json"
		}
		if len(inst.Targets) == 0 {
			return Config{}, fmt.Errorf("instance %s requires at least one target", inst.Name)
		}
		for ti, tgt := range inst.Targets {
			if len(tgt.Labels) == 0 {
				return Config{}, fmt.Errorf("instance %s target[%d] missing labels", inst.Name, ti)
			}
			if tgt.MinIdle < 0 {
				return Config{}, fmt.Errorf("instance %s target[%d] min_idle must be >= 0", inst.Name, ti)
			}
		}
	}
	return cfg, nil
}
