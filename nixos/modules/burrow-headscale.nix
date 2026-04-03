{ config, lib, pkgs, ... }:

let
  cfg = config.services.burrow.headscale;
  policyFile = ./burrow-headscale-policy.hujson;
  landingPage = pkgs.writeTextDir "index.html" ''
    <!DOCTYPE html>
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Burrow Tailnet</title>
        <style>
          :root {
            color-scheme: light dark;
            --bg: #f7f7f5;
            --fg: #171717;
            --muted: #5c5c57;
            --card: rgba(255, 255, 255, 0.75);
            --border: rgba(23, 23, 23, 0.12);
            --accent: #0f766e;
          }
          @media (prefers-color-scheme: dark) {
            :root {
              --bg: #111311;
              --fg: #f2f3ee;
              --muted: #b2b6aa;
              --card: rgba(20, 23, 20, 0.78);
              --border: rgba(242, 243, 238, 0.12);
              --accent: #5eead4;
            }
          }
          * { box-sizing: border-box; }
          body {
            margin: 0;
            min-height: 100vh;
            font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
            background:
              radial-gradient(circle at top left, rgba(15, 118, 110, 0.12), transparent 32rem),
              linear-gradient(180deg, rgba(255,255,255,0.35), transparent 18rem),
              var(--bg);
            color: var(--fg);
            display: grid;
            place-items: center;
            padding: 2rem;
          }
          main {
            width: min(42rem, 100%);
            border: 1px solid var(--border);
            background: var(--card);
            backdrop-filter: blur(16px);
            border-radius: 1.5rem;
            padding: 2rem;
            box-shadow: 0 1.25rem 3rem rgba(0, 0, 0, 0.12);
          }
          h1 {
            margin: 0 0 0.75rem;
            font-size: clamp(2rem, 5vw, 3rem);
            line-height: 1;
          }
          p {
            margin: 0;
            color: var(--muted);
            font-size: 1rem;
            line-height: 1.6;
          }
          .stack {
            display: grid;
            gap: 1rem;
          }
          .actions {
            display: flex;
            flex-wrap: wrap;
            gap: 0.75rem;
            margin-top: 1rem;
          }
          a.button {
            text-decoration: none;
            color: inherit;
            border-radius: 999px;
            padding: 0.8rem 1.1rem;
            border: 1px solid var(--border);
            background: rgba(255, 255, 255, 0.6);
          }
          a.button.primary {
            background: var(--accent);
            border-color: transparent;
            color: #06201d;
            font-weight: 600;
          }
          code, pre {
            font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, monospace;
            font-size: 0.95rem;
          }
          pre {
            margin: 0;
            padding: 1rem;
            border-radius: 1rem;
            border: 1px solid var(--border);
            overflow-x: auto;
            background: rgba(0, 0, 0, 0.05);
          }
          .eyebrow {
            color: var(--accent);
            font-weight: 700;
            letter-spacing: 0.08em;
            text-transform: uppercase;
            font-size: 0.78rem;
          }
        </style>
      </head>
      <body>
        <main class="stack">
          <div class="eyebrow">Burrow Tailnet</div>
          <div class="stack">
            <h1>Sign-in starts from your client, not this page.</h1>
            <p>
              <code>ts.burrow.net</code> is the Burrow Headscale control plane. Headscale does not provide a built-in web UI,
              so browser authentication starts only after a Tailscale-compatible client initiates login.
            </p>
          </div>
          <pre>tailscale up --login-server https://ts.burrow.net</pre>
          <div class="actions">
            <a class="button primary" href="https://tailscale.com/download" rel="noreferrer">Download Tailscale</a>
            <a class="button" href="/health">Health Check</a>
          </div>
        </main>
      </body>
    </html>
  '';
in
{
  options.services.burrow.headscale = {
    enable = lib.mkEnableOption "the Burrow Headscale control plane";

    domain = lib.mkOption {
      type = lib.types.str;
      default = "ts.burrow.net";
      description = "Public Headscale control-plane domain.";
    };

    tailDomain = lib.mkOption {
      type = lib.types.str;
      default = "tail.burrow.net";
      description = "MagicDNS suffix served by Headscale.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8413;
      description = "Local Headscale listen port.";
    };

    oidcIssuer = lib.mkOption {
      type = lib.types.str;
      default = "https://${config.services.burrow.authentik.domain}/application/o/${config.services.burrow.authentik.headscaleProviderSlug}/";
      description = "OIDC issuer URL used by Headscale.";
    };

    oidcClientSecretFile = lib.mkOption {
      type = lib.types.str;
      default = config.services.burrow.authentik.headscaleClientSecretFile;
      description = "Host-local file containing the OIDC client secret used by Headscale.";
    };

    bootstrapUsers = lib.mkOption {
      type = with lib.types; listOf (submodule {
        options = {
          name = lib.mkOption {
            type = str;
            description = "Headscale username.";
          };
          displayName = lib.mkOption {
            type = str;
            description = "Friendly display name.";
          };
          email = lib.mkOption {
            type = str;
            description = "User email address.";
          };
        };
      });
      default = [
        {
          name = "contact";
          displayName = "Burrow";
          email = "contact@burrow.net";
        }
        {
          name = "conrad";
          displayName = "Conrad";
          email = "conrad@burrow.net";
        }
        {
          name = "agent";
          displayName = "Agent";
          email = "agent@burrow.net";
        }
        {
          name = "infra";
          displayName = "Infrastructure";
          email = "infra@burrow.net";
        }
      ];
      description = "Users to create or reconcile inside Headscale.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ pkgs.headscale ];

    systemd.services.burrow-headscale-client-secret = {
      description = "Ensure the Burrow Headscale OIDC client secret exists";
      before =
        [ "headscale.service" ]
        ++ lib.optionals config.services.burrow.authentik.enable [ "burrow-authentik-runtime.service" ];
      wantedBy =
        [ "headscale.service" ]
        ++ lib.optionals config.services.burrow.authentik.enable [ "burrow-authentik-runtime.service" ];
      path = [
        pkgs.coreutils
        pkgs.openssl
      ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
        RemainAfterExit = true;
      };
      script = ''
        set -euo pipefail

        install -d -m 0755 /var/lib/burrow/intake

        if [ ! -s ${lib.escapeShellArg cfg.oidcClientSecretFile} ]; then
          umask 077
          ${pkgs.openssl}/bin/openssl rand -base64 48 > ${lib.escapeShellArg cfg.oidcClientSecretFile}
          chown root:root ${lib.escapeShellArg cfg.oidcClientSecretFile}
          chmod 0400 ${lib.escapeShellArg cfg.oidcClientSecretFile}
        fi
      '';
    };

    services.headscale = {
      enable = true;
      address = "127.0.0.1";
      port = cfg.port;
      settings = {
        server_url = "https://${cfg.domain}";
        dns = {
          magic_dns = true;
          base_domain = cfg.tailDomain;
          nameservers.global = [
            "1.1.1.1"
            "1.0.0.1"
            "2606:4700:4700::1111"
            "2606:4700:4700::1001"
          ];
          search_domains = [ cfg.tailDomain ];
        };
        database.sqlite.write_ahead_log = true;
        log.level = "info";
        policy = {
          mode = "file";
          path = policyFile;
        };
        oidc = {
          only_start_if_oidc_is_available = true;
          issuer = cfg.oidcIssuer;
          client_id = cfg.domain;
          client_secret_path = "\${CREDENTIALS_DIRECTORY}/oidc_client_secret";
          scope = [
            "openid"
            "profile"
            "email"
          ];
          pkce = {
            enabled = true;
            method = "S256";
          };
        };
      };
    };

    systemd.services.headscale = {
      after =
        [ "burrow-headscale-client-secret.service" ]
        ++ lib.optionals config.services.burrow.authentik.enable [ "burrow-authentik-ready.service" ];
      wants =
        [ "burrow-headscale-client-secret.service" ]
        ++ lib.optionals config.services.burrow.authentik.enable [ "burrow-authentik-ready.service" ];
      requires =
        [ "burrow-headscale-client-secret.service" ]
        ++ lib.optionals config.services.burrow.authentik.enable [ "burrow-authentik-ready.service" ];
      serviceConfig.LoadCredential = [
        "oidc_client_secret:${cfg.oidcClientSecretFile}"
      ];
    };

    systemd.services.headscale-bootstrap = {
      description = "Bootstrap Burrow Headscale users";
      after = [ "headscale.service" ];
      requires = [ "headscale.service" ];
      wantedBy = [ "multi-user.target" ];
      path = [
        pkgs.coreutils
        pkgs.headscale
        pkgs.jq
      ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
      };
      script = ''
        set -euo pipefail

        list_users() {
          local users_json
          users_json="$(${pkgs.headscale}/bin/headscale users list -o json)"
          printf '%s\n' "$users_json" | ${pkgs.jq}/bin/jq -c 'if type == "array" then . else [] end'
        }

        ensure_user() {
          local name="$1"
          local display_name="$2"
          local email="$3"
          if list_users | ${pkgs.jq}/bin/jq -e --arg name "$name" 'map(select(.name == $name)) | length > 0' >/dev/null; then
            return 0
          fi
          ${pkgs.headscale}/bin/headscale users create "$name" --display-name "$display_name" --email "$email" >/dev/null
        }

        for _ in $(seq 1 60); do
          if list_users >/dev/null 2>&1; then
            break
          fi
          sleep 1
        done

        ${lib.concatMapStringsSep "\n" (user: ''
          ensure_user ${lib.escapeShellArg user.name} ${lib.escapeShellArg user.displayName} ${lib.escapeShellArg user.email}
        '') cfg.bootstrapUsers}
      '';
    };

    services.caddy.virtualHosts."${cfg.domain}".extraConfig = ''
      encode gzip zstd
      @root path /
      handle @root {
        root * ${landingPage}
        file_server
      }
      handle {
        reverse_proxy 127.0.0.1:${toString cfg.port}
      }
    '';
  };
}
