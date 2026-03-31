package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"time"

	"tailscale.com/client/local"
	"tailscale.com/ipn"
	"tailscale.com/tsnet"
)

type statusResponse struct {
	BackendState   string   `json:"backend_state"`
	AuthURL        string   `json:"auth_url,omitempty"`
	Running        bool     `json:"running"`
	NeedsLogin     bool     `json:"needs_login"`
	TailnetName    string   `json:"tailnet_name,omitempty"`
	MagicDNSSuffix string   `json:"magic_dns_suffix,omitempty"`
	SelfDNSName    string   `json:"self_dns_name,omitempty"`
	TailscaleIPs   []string `json:"tailscale_ips,omitempty"`
	Health         []string `json:"health,omitempty"`
}

func main() {
	listen := flag.String("listen", "127.0.0.1:0", "local listen address")
	stateDir := flag.String("state-dir", "", "persistent state directory")
	hostname := flag.String("hostname", "burrow-apple", "tailnet hostname")
	controlURL := flag.String("control-url", "", "optional control URL")
	flag.Parse()

	if *stateDir == "" {
		log.Fatal("--state-dir is required")
	}

	if err := os.MkdirAll(*stateDir, 0o755); err != nil {
		log.Fatalf("create state dir: %v", err)
	}

	server := &tsnet.Server{
		Dir:      *stateDir,
		Hostname: *hostname,
		UserLogf: log.Printf,
	}
	if *controlURL != "" {
		server.ControlURL = *controlURL
	}
	defer server.Close()

	if err := server.Start(); err != nil {
		log.Fatalf("start tsnet: %v", err)
	}

	localClient, err := server.LocalClient()
	if err != nil {
		log.Fatalf("local client: %v", err)
	}

	ln, err := net.Listen("tcp", *listen)
	if err != nil {
		log.Fatalf("listen: %v", err)
	}
	defer ln.Close()

	fmt.Printf("{\"listen_addr\":%q}\n", ln.Addr().String())
	_ = os.Stdout.Sync()

	mux := http.NewServeMux()
	mux.HandleFunc("/status", func(w http.ResponseWriter, r *http.Request) {
		status, err := snapshot(r.Context(), localClient)
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadGateway)
			return
		}
		w.Header().Set("content-type", "application/json")
		_ = json.NewEncoder(w).Encode(status)
	})
	mux.HandleFunc("/shutdown", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNoContent)
		go func() {
			_ = server.Close()
			time.Sleep(100 * time.Millisecond)
			os.Exit(0)
		}()
	})

	httpServer := &http.Server{
		Handler: mux,
	}
	log.Fatal(httpServer.Serve(ln))
}

func snapshot(ctx context.Context, localClient *local.Client) (*statusResponse, error) {
	status, err := localClient.StatusWithoutPeers(ctx)
	if err != nil {
		return nil, err
	}
	if (status.BackendState == ipn.NeedsLogin.String() || status.BackendState == ipn.NoState.String()) && status.AuthURL == "" {
		if err := localClient.StartLoginInteractive(ctx); err != nil {
			return nil, err
		}
		status, err = localClient.StatusWithoutPeers(ctx)
		if err != nil {
			return nil, err
		}
	}

	response := &statusResponse{
		BackendState: status.BackendState,
		AuthURL:      status.AuthURL,
		Running:      status.BackendState == ipn.Running.String(),
		NeedsLogin:   status.BackendState == ipn.NeedsLogin.String(),
		Health:       append([]string(nil), status.Health...),
	}

	if status.CurrentTailnet != nil {
		response.TailnetName = status.CurrentTailnet.Name
		response.MagicDNSSuffix = status.CurrentTailnet.MagicDNSSuffix
	}
	if status.Self != nil {
		response.SelfDNSName = status.Self.DNSName
	}
	for _, ip := range status.TailscaleIPs {
		response.TailscaleIPs = append(response.TailscaleIPs, ip.String())
	}
	return response, nil
}
