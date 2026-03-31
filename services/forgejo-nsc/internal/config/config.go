package config

import (
	"errors"
	"fmt"
	"os"
	"strings"
	"time"

	"gopkg.in/yaml.v3"

	"github.com/burrow/forgejo-nsc/internal/forgejo"
)

// Duration wraps time.Duration to support YAML unmarshalling from strings.
type Duration struct {
	time.Duration
}

// UnmarshalYAML implements yaml.v3 unmarshalling for Duration.
func (d *Duration) UnmarshalYAML(value *yaml.Node) error {
	switch value.Tag {
	case "!!int":
		var seconds int64
		if err := value.Decode(&seconds); err != nil {
			return err
		}
		d.Duration = time.Duration(seconds) * time.Second
		return nil
	default:
		parsed, err := time.ParseDuration(value.Value)
		if err != nil {
			return err
		}
		d.Duration = parsed
		return nil
	}
}

// MarshalYAML implements yaml.v3 marshalling.
func (d Duration) MarshalYAML() (any, error) {
	return d.Duration.String(), nil
}

type Config struct {
	Listen    string          `yaml:"listen"`
	Forgejo   ForgejoConfig   `yaml:"forgejo"`
	Namespace NamespaceConfig `yaml:"namespace"`
	Runner    RunnerConfig    `yaml:"runner"`
}

type ForgejoConfig struct {
	BaseURL       string      `yaml:"base_url"`
	// InstanceURL is the URL runners should use when registering with Forgejo.
	// This must be reachable from the spawned runner (e.g. the public URL like
	// https://git.burrow.net), and may differ from BaseURL (which can be a local
	// loopback URL on the forge host).
	InstanceURL   string      `yaml:"instance_url"`
	Token         string      `yaml:"token"`
	DefaultScope  ScopeConfig `yaml:"default_scope"`
	DefaultLabels []string    `yaml:"default_labels"`
	Timeout       Duration    `yaml:"timeout"`
	ExtraHeaders  yaml.Node   `yaml:"extra_headers"`
}

type ScopeConfig struct {
	Level string `yaml:"level"`
	Owner string `yaml:"owner,omitempty"`
	Name  string `yaml:"name,omitempty"`
}

type NamespaceConfig struct {
	NSCBinary string `yaml:"nsc_binary"`
	// ComputeBaseURL is the Namespace Cloud Compute API endpoint (Connect RPC base URL).
	// This is used for macOS runners, since NSC "run" is container-based (Linux-only).
	// Example: "https://ord4.compute.namespaceapis.com"
	ComputeBaseURL string `yaml:"compute_base_url"`
	Image          string `yaml:"image"`
	MachineType    string `yaml:"machine_type"`
	// MacosBaseImageID selects which macOS base image to use (e.g. "tahoe").
	MacosBaseImageID string `yaml:"macos_base_image_id"`
	// MacosMachineArch is the architecture used for macOS instances (typically "arm64").
	MacosMachineArch string   `yaml:"macos_machine_arch"`
	Duration         Duration `yaml:"duration"`
	WorkDir          string   `yaml:"workdir"`
	MaxParallel      int64    `yaml:"max_parallel"`
	Environment      []string `yaml:"environment"`
	AllowLabels      []string `yaml:"allow_labels"`
	AllowScopes      []string `yaml:"allow_scopes"`
	Network          string   `yaml:"network"`
	InstanceTags     []string `yaml:"instance_tags"`
}

type RunnerConfig struct {
	NamePrefix string `yaml:"name_prefix"`
	Executor   string `yaml:"executor"`
}

func Load(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, err
	}

	if err := cfg.Validate(); err != nil {
		return nil, err
	}

	return &cfg, nil
}

func (c *Config) Validate() error {
	if c.Listen == "" {
		c.Listen = ":8080"
	}
	if c.Runner.NamePrefix == "" {
		c.Runner.NamePrefix = "nscloud-"
	}
	if c.Runner.Executor == "" {
		c.Runner.Executor = "shell"
	}

	if c.Forgejo.BaseURL == "" {
		return errors.New("forgejo.base_url is required")
	}
	if c.Forgejo.InstanceURL == "" {
		// Backwards-compatible default: assume runners can reach the same URL.
		c.Forgejo.InstanceURL = c.Forgejo.BaseURL
	}
	if c.Forgejo.Token == "" {
		return errors.New("forgejo.token is required")
	}
	if c.Forgejo.Timeout.Duration == 0 {
		c.Forgejo.Timeout.Duration = 30 * time.Second
	}
	if _, err := c.Forgejo.DefaultScope.ToScope(); err != nil {
		return err
	}

	if c.Namespace.NSCBinary == "" {
		c.Namespace.NSCBinary = "nsc"
	}
	if c.Namespace.Image == "" {
		c.Namespace.Image = "code.forgejo.org/forgejo/runner:11"
	}
	if c.Namespace.MacosBaseImageID == "" {
		c.Namespace.MacosBaseImageID = "tahoe"
	}
	if c.Namespace.MacosMachineArch == "" {
		c.Namespace.MacosMachineArch = "arm64"
	}
	if c.Namespace.Duration.Duration == 0 {
		c.Namespace.Duration.Duration = 30 * time.Minute
	}
	if c.Namespace.MaxParallel <= 0 {
		c.Namespace.MaxParallel = 4
	}

	return nil
}

func (s ScopeConfig) ToScope() (forgejo.Scope, error) {
	level := forgejo.ScopeLevel(strings.ToLower(s.Level))
	switch level {
	case forgejo.ScopeInstance:
		return forgejo.Scope{Level: level}, nil
	case forgejo.ScopeOrganization:
		if s.Owner == "" {
			return forgejo.Scope{}, errors.New("forgejo default scope requires owner for organization level")
		}
		return forgejo.Scope{Level: level, Owner: s.Owner}, nil
	case forgejo.ScopeRepository:
		if s.Owner == "" || s.Name == "" {
			return forgejo.Scope{}, errors.New("forgejo default scope requires owner and name for repository level")
		}
		return forgejo.Scope{Level: level, Owner: s.Owner, Name: s.Name}, nil
	default:
		return forgejo.Scope{}, fmt.Errorf("unknown scope level %q", s.Level)
	}
}
