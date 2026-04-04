package main

import (
	"context"
	"encoding/binary"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"net"
	"net/netip"
	"net/http"
	"os"
	"strconv"
	"sync"
	"time"

	"github.com/tailscale/wireguard-go/tun"
	"tailscale.com/client/local"
	"tailscale.com/ipn"
	"tailscale.com/ipn/ipnstate"
	"tailscale.com/tailcfg"
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
	Peers          []peerSummary `json:"peers,omitempty"`
}

type peerSummary struct {
	Name         string   `json:"name,omitempty"`
	DNSName      string   `json:"dns_name,omitempty"`
	TailscaleIPs []string `json:"tailscale_ips,omitempty"`
	Online       bool     `json:"online"`
	Active       bool     `json:"active"`
	Relay        string   `json:"relay,omitempty"`
	CurAddr      string   `json:"cur_addr,omitempty"`
	LastSeenUnix int64    `json:"last_seen_unix,omitempty"`
}

type pingResponse struct {
	Result *ipnstate.PingResult `json:"result,omitempty"`
}

type helperHello struct {
	ListenAddr   string `json:"listen_addr"`
	PacketSocket string `json:"packet_socket,omitempty"`
}

type helperState struct {
	mu      sync.RWMutex
	authURL string
}

func (s *helperState) authURLSnapshot() string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.authURL
}

func (s *helperState) setAuthURL(url string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.authURL = url
}

func (s *helperState) clearAuthURL() {
	s.setAuthURL("")
}

// chanTUN is a tun.Device backed by channels so another process can feed and
// consume raw IP packets while tsnet handles the Tailnet control/data plane.
type chanTUN struct {
	Inbound  chan []byte
	Outbound chan []byte
	closed   chan struct{}
	events   chan tun.Event
}

func newChanTUN() *chanTUN {
	t := &chanTUN{
		Inbound:  make(chan []byte, 1024),
		Outbound: make(chan []byte, 1024),
		closed:   make(chan struct{}),
		events:   make(chan tun.Event, 1),
	}
	t.events <- tun.EventUp
	return t
}

func (t *chanTUN) File() *os.File { return nil }

func (t *chanTUN) Close() error {
	select {
	case <-t.closed:
	default:
		close(t.closed)
		close(t.Inbound)
	}
	return nil
}

func (t *chanTUN) Read(bufs [][]byte, sizes []int, offset int) (int, error) {
	select {
	case <-t.closed:
		return 0, io.EOF
	case pkt, ok := <-t.Outbound:
		if !ok {
			return 0, io.EOF
		}
		sizes[0] = copy(bufs[0][offset:], pkt)
		return 1, nil
	}
}

func (t *chanTUN) Write(bufs [][]byte, offset int) (int, error) {
	for _, buf := range bufs {
		pkt := buf[offset:]
		if len(pkt) == 0 {
			continue
		}
		select {
		case <-t.closed:
			return 0, errors.New("closed")
		case t.Inbound <- append([]byte(nil), pkt...):
		default:
		}
	}
	return len(bufs), nil
}

func (t *chanTUN) MTU() (int, error)        { return 1280, nil }
func (t *chanTUN) Name() (string, error)    { return "burrow-tailnet", nil }
func (t *chanTUN) Events() <-chan tun.Event { return t.events }
func (t *chanTUN) BatchSize() int           { return 1 }

func main() {
	listen := flag.String("listen", "127.0.0.1:0", "local listen address")
	stateDir := flag.String("state-dir", "", "persistent state directory")
	hostname := flag.String("hostname", "burrow-apple", "tailnet hostname")
	controlURL := flag.String("control-url", "", "optional control URL")
	packetSocket := flag.String("packet-socket", "", "optional unix socket path for raw packet bridging")
	udpEchoPort := flag.Int("udp-echo-port", 0, "optional tailnet UDP echo port")
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

	var tunDevice *chanTUN
	var packetListener net.Listener
	if *packetSocket != "" {
		_ = os.Remove(*packetSocket)
		ln, err := net.Listen("unix", *packetSocket)
		if err != nil {
			log.Fatalf("packet listen: %v", err)
		}
		packetListener = ln
		defer func() {
			packetListener.Close()
			_ = os.Remove(*packetSocket)
		}()

		tunDevice = newChanTUN()
		server.Tun = tunDevice
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
	state := &helperState{}

	ln, err := net.Listen("tcp", *listen)
	if err != nil {
		log.Fatalf("listen: %v", err)
	}
	defer ln.Close()

	if packetListener != nil {
		go servePacketBridge(packetListener, tunDevice)
	}
	if *udpEchoPort > 0 {
		go serveUDPEcho(context.Background(), server, localClient, *udpEchoPort)
	}

	hello := helperHello{
		ListenAddr: ln.Addr().String(),
	}
	if *packetSocket != "" {
		hello.PacketSocket = *packetSocket
	}
	if err := json.NewEncoder(os.Stdout).Encode(hello); err != nil {
		log.Fatalf("write hello: %v", err)
	}
	_ = os.Stdout.Sync()

	mux := http.NewServeMux()
	mux.HandleFunc("/status", func(w http.ResponseWriter, r *http.Request) {
		status, err := snapshot(r.Context(), localClient, state)
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadGateway)
			return
		}
		w.Header().Set("content-type", "application/json")
		_ = json.NewEncoder(w).Encode(status)
	})
	mux.HandleFunc("/ping", func(w http.ResponseWriter, r *http.Request) {
		ip := r.URL.Query().Get("ip")
		if ip == "" {
			http.Error(w, "missing ip", http.StatusBadRequest)
			return
		}
		target, err := netip.ParseAddr(ip)
		if err != nil {
			http.Error(w, fmt.Sprintf("invalid ip: %v", err), http.StatusBadRequest)
			return
		}

		pingType := tailcfg.PingTSMP
		switch r.URL.Query().Get("type") {
		case "", "tsmp", "TSMP":
			pingType = tailcfg.PingTSMP
		case "icmp", "ICMP":
			pingType = tailcfg.PingICMP
		case "peerapi":
			pingType = tailcfg.PingPeerAPI
		default:
			http.Error(w, "unsupported ping type", http.StatusBadRequest)
			return
		}

		result, err := localClient.Ping(r.Context(), target, pingType)
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadGateway)
			return
		}

		w.Header().Set("content-type", "application/json")
		_ = json.NewEncoder(w).Encode(&pingResponse{Result: result})
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

func servePacketBridge(listener net.Listener, device *chanTUN) {
	for {
		conn, err := listener.Accept()
		if err != nil {
			if errors.Is(err, net.ErrClosed) {
				return
			}
			log.Printf("packet accept: %v", err)
			continue
		}
		log.Printf("packet bridge connected")
		if err := bridgePacketConn(conn, device); err != nil && !errors.Is(err, io.EOF) {
			log.Printf("packet bridge error: %v", err)
		}
		_ = conn.Close()
		log.Printf("packet bridge disconnected")
	}
}

func bridgePacketConn(conn net.Conn, device *chanTUN) error {
	errCh := make(chan error, 2)

	go func() {
		for {
			pkt, err := readFrame(conn)
			if err != nil {
				errCh <- err
				return
			}
			select {
			case <-device.closed:
				errCh <- io.EOF
				return
			case device.Outbound <- pkt:
			}
		}
	}()

	go func() {
		for {
			select {
			case <-device.closed:
				errCh <- io.EOF
				return
			case pkt, ok := <-device.Inbound:
				if !ok {
					errCh <- io.EOF
					return
				}
				if err := writeFrame(conn, pkt); err != nil {
					errCh <- err
					return
				}
			}
		}
	}()

	return <-errCh
}

func readFrame(r io.Reader) ([]byte, error) {
	var size [4]byte
	if _, err := io.ReadFull(r, size[:]); err != nil {
		return nil, err
	}
	length := binary.BigEndian.Uint32(size[:])
	if length == 0 {
		return []byte{}, nil
	}
	packet := make([]byte, length)
	if _, err := io.ReadFull(r, packet); err != nil {
		return nil, err
	}
	return packet, nil
}

func writeFrame(w io.Writer, packet []byte) error {
	var size [4]byte
	binary.BigEndian.PutUint32(size[:], uint32(len(packet)))
	if _, err := w.Write(size[:]); err != nil {
		return err
	}
	if len(packet) == 0 {
		return nil
	}
	_, err := w.Write(packet)
	return err
}

func snapshot(ctx context.Context, localClient *local.Client, state *helperState) (*statusResponse, error) {
	status, err := localClient.Status(ctx)
	if err != nil {
		return nil, err
	}

	authURL := status.AuthURL
	if authURL == "" {
		authURL = state.authURLSnapshot()
	}
	if status.BackendState == ipn.Running.String() {
		state.clearAuthURL()
		authURL = ""
	} else if (status.BackendState == ipn.NeedsLogin.String() || status.BackendState == ipn.NoState.String()) && authURL == "" {
		authURL, err = awaitAuthURL(ctx, localClient, state)
		if err != nil {
			return nil, err
		}
	}

	response := &statusResponse{
		BackendState: status.BackendState,
		AuthURL:      authURL,
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
	for _, key := range status.Peers() {
		peer := status.Peer[key]
		if peer == nil {
			continue
		}
		summary := peerSummary{
			Name:         peer.HostName,
			DNSName:      peer.DNSName,
			Online:       peer.Online,
			Active:       peer.Active,
			Relay:        peer.Relay,
			CurAddr:      peer.CurAddr,
			LastSeenUnix: peer.LastSeen.Unix(),
		}
		for _, ip := range peer.TailscaleIPs {
			summary.TailscaleIPs = append(summary.TailscaleIPs, ip.String())
		}
		response.Peers = append(response.Peers, summary)
	}
	return response, nil
}

func serveUDPEcho(ctx context.Context, server *tsnet.Server, localClient *local.Client, port int) {
	ip, err := awaitTailscaleIP(ctx, localClient)
	if err != nil {
		log.Printf("udp echo setup failed: %v", err)
		return
	}

	listenAddr := net.JoinHostPort(ip.String(), strconv.Itoa(port))
	pc, err := server.ListenPacket("udp", listenAddr)
	if err != nil {
		log.Printf("udp echo listen failed on %s: %v", listenAddr, err)
		return
	}
	defer pc.Close()

	log.Printf("udp echo listening on %s", pc.LocalAddr())
	buf := make([]byte, 64<<10)
	for {
		n, addr, err := pc.ReadFrom(buf)
		if err != nil {
			if errors.Is(err, net.ErrClosed) || errors.Is(err, io.EOF) {
				return
			}
			log.Printf("udp echo read failed: %v", err)
			return
		}
		if _, err := pc.WriteTo(buf[:n], addr); err != nil {
			log.Printf("udp echo write failed: %v", err)
			return
		}
	}
}

func awaitTailscaleIP(ctx context.Context, localClient *local.Client) (netip.Addr, error) {
	for range 60 {
		status, err := localClient.StatusWithoutPeers(ctx)
		if err == nil {
			for _, ip := range status.TailscaleIPs {
				if ip.Is4() {
					return ip, nil
				}
			}
			for _, ip := range status.TailscaleIPs {
				if ip.Is6() {
					return ip, nil
				}
			}
		}
		select {
		case <-ctx.Done():
			return netip.Addr{}, ctx.Err()
		case <-time.After(250 * time.Millisecond):
		}
	}
	return netip.Addr{}, errors.New("timed out waiting for tailscale IP")
}

func awaitAuthURL(ctx context.Context, localClient *local.Client, state *helperState) (string, error) {
	watchCtx, cancel := context.WithTimeout(ctx, 8*time.Second)
	defer cancel()

	watcher, err := localClient.WatchIPNBus(watchCtx, ipn.NotifyInitialState)
	if err != nil {
		return "", err
	}
	defer watcher.Close()

	if err := localClient.StartLoginInteractive(ctx); err != nil {
		return "", err
	}

	for {
		notify, err := watcher.Next()
		if err != nil {
			if errors.Is(err, context.DeadlineExceeded) || errors.Is(err, context.Canceled) {
				return state.authURLSnapshot(), nil
			}
			return "", err
		}
		if notify.BrowseToURL != nil && *notify.BrowseToURL != "" {
			state.setAuthURL(*notify.BrowseToURL)
			return *notify.BrowseToURL, nil
		}
		if notify.State != nil && *notify.State == ipn.Running {
			state.clearAuthURL()
			return "", nil
		}
	}
}
