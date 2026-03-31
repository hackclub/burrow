## forgejo-nsc-dispatcher

This service exposes a simple HTTP API that tells Namespace Cloud to start
ephemeral Forgejo Actions runners on demand. It glues together three pieces:

1. **Forgejo Actions** – the service requests a scoped registration token
   for the repository/organization/instance where you want to run jobs.
2. **Namespace (`nsc`)** – the dispatcher shells out to the `nsc` CLI to create
   a short‑lived environment, runs the `forgejo-runner` container inside it,
   and exits after a single job (`forgejo-runner one-job`). The Namespace TTL is
   the hard cap, not the typical lifetime.
3. **Your automation** – you call the service via HTTP (directly, through Caddy,
   via Forgejo webhooks, etc.) whenever a new runner is needed.

### Directory layout

```
.
├── cmd/forgejo-nsc-dispatcher   # main entry point
├── internal/                    # service packages (config, forgejo client, nsc dispatcher, HTTP server)
├── config.example.yaml          # starter config referenced by README
├── flake.nix / flake.lock       # reproducible builds (Go binary + container image)
└── .forgejo/workflows           # CI that runs go test/build and publishes manifests
```

### Configuration

Copy `config.example.yaml` and update it for your Forgejo instance and Namespace
profile. The important knobs are:

- `forgejo.base_url` – HTTPS endpoint of your Forgejo server. A PAT with
  `actions:runner` scope is required in `forgejo.token`.
- `forgejo.instance_url` – URL that spawned runners use to register back to Forgejo.
  This must be reachable from the runner (typically the public URL like
  `https://git.burrow.net`). On the forge host it commonly differs from `base_url`
  (which may be `http://127.0.0.1:3000`).
- `forgejo.default_scope` – where new runners register
  (`instance`, `organization`, or `repository`).
- `forgejo.default_labels` – labels applied to every spawned runner. GateForge
  workflows via `runs-on: ["namespace-profile-linux-medium"]` (or other
  `namespace-profile-linux-*` labels).
- `namespace.nsc_binary` – path to the `nsc` binary (the Nix container ships one
  compiled from `namespacelabs/foundation` so `/app/bin/nsc` works out of the box).
- `namespace.image` – OCI image containing `forgejo-runner`.
- `namespace.machine_type` / `namespace.duration` – shape + TTL for the ephemeral
  Namespace environment. The dispatcher destroys the instance after a job so the
  TTL acts as a hard cap, not an idle timeout.

### Running locally

```shell
# Ensure nsc is available (e.g. `go build ./foundation/cmd/nsc`)
cp config.example.yaml config.yaml
nix develop   # optional dev shell with Go toolchain
go run ./cmd/forgejo-nsc-dispatcher --config config.yaml
```

API example:

```shell
curl -X POST http://localhost:8080/api/v1/dispatch \
  -H 'Content-Type: application/json' \
  -d '{
    "count": 1,
    "ttl": "20m",
    "labels": ["namespace-profile-linux-medium"],
    "scope": {"level": "repository", "owner": "example", "name": "app"}
  }'
```

### Deploying with Nix + GHCR

- `nix build .#packages.x86_64-linux.container-amd64` produces a deterministic
  tarball containing the service, the `nsc` binary, BusyBox, and `forgejo-runner`.
- The included `Build Container` workflow builds both `amd64` and `arm64` images
  on Namespace runners and pushes them to `ghcr.io/<owner>/<repo>`.
  No Fly.io manifests are emitted – the multi‑arch manifest points only at GHCR.

### How this fits behind Caddy (last-mile networking)

The dispatcher is just an HTTP server. You can:

1. Run it anywhere that can reach Forgejo and Namespace: bare metal, Namespace
   cluster, Kubernetes, Fly, etc.
2. Put Caddy (or any reverse proxy) in front to terminate TLS, do auth, or
   rewrite URLs. For example:

   ```
   forgejo-dispatcher.example.com {
     reverse_proxy 127.0.0.1:8080
     basicauth /api/* {
       user JDJhJDE...
     }
   }
   ```

The service doesn’t assume Caddy, nor does it manipulate HTTP clients
directly – it simply waits for POST requests. As long as the dispatcher can
reach Forgejo’s REST API and run the `nsc` binary, you can drop it anywhere.

### Autoscaling (webhook + poller)

If you don’t want to call `/api/v1/dispatch` manually, there’s a companion
autoscaler (`cmd/forgejo-nsc-autoscaler`) that watches Forgejo job queues and
triggers the dispatcher for you. It operates in two modes simultaneously:

1. **Polling** – every instance polls `GET /api/v1/.../actions/runners` to keep a
   minimum number of idle Namespace runners per label. This continues until a
   webhook is successfully processed, so the system is self-bootstrapping.
2. **Webhooks** – once Forgejo reaches the autoscaler via the `/webhook/{name}`
   endpoint, the autoscaler stops polling and reacts to `workflow_job` events in
   real time. Each payload is mapped to a target label set and results in a
   dispatch call.

You can manage multiple Forgejo instances by listing them under `instances` in
`autoscaler.example.yaml`:

```
listen: ":8090"
dispatcher:
  url: "http://dispatcher:8080"

instances:
- name: burrow
  forgejo:
    base_url: "https://git.burrow.net"
    token: "PENDING-FORGEJO-PAT"
  scope:
    level: "repository"
    owner: "burrow"
    name: "burrow"
    disable_polling: true   # webhook-only mode
    poll_interval: "30s"
    webhook_secret: "supersecret"
    webhook:
      url: "https://nsc-autoscaler.burrow.net/webhook/burrow"
      content_type: "json"
      events: ["workflow_job"]
      active: true
    targets:
      - labels: ["namespace-profile-linux-medium"]
        min_idle: 0  # set to 0 to scale-to-zero between jobs
        ttl: "20m"
      - labels: ["namespace-profile-windows-large"]
        min_idle: 0
        ttl: "45m"
        machine_type: "windows/amd64:8x16"
```

For Burrow, use `Scripts/provision-forgejo-nsc.sh` to mint the Forgejo PAT,
generate a Namespace token from the logged-in namespace account, and render the
dispatcher/autoscaler configs into `intake/forgejo_nsc_{dispatcher,autoscaler}.yaml`
plus `intake/forgejo_nsc_token.txt`.

For ongoing operations, use `Scripts/sync-forgejo-nsc-config.sh`:

- `Scripts/sync-forgejo-nsc-config.sh` copies the intake-backed configs and
  Namespace token onto `/var/lib/burrow/intake/` on the forge host, reapplies
  file ownership for `forgejo-nsc`, and restarts the dispatcher/autoscaler.
- `Scripts/sync-forgejo-nsc-config.sh --rotate-pat` additionally mints a new
  Forgejo PAT on the Burrow forge host and refreshes the local intake files.

Run it next to the dispatcher:

```bash
go run ./cmd/forgejo-nsc-autoscaler --config autoscaler.yaml
# or build the binary/container via `nix build .#forgejo-nsc-autoscaler`
```

If your Forgejo build doesn’t expose the runner listing API, set
`disable_polling: true` and rely on `webhook` entries. The autoscaler will
auto-create/update the webhook (using the PAT) so that new `workflow_job` events
immediately call the dispatcher even if the service isn’t publicly reachable yet.

In Forgejo add a webhook pointing to `https://nsc-autoscaler.burrow.net/webhook/burrow`
with the shared secret (or let the autoscaler create it by specifying `webhook.url`
in config). The autoscaler continues polling until it receives the first valid
webhook (unless disabled), so you get capacity immediately even if outbound
webhooks from Forgejo aren’t yet configured.
