{ config, lib, pkgs, ... }:

let
  cfg = config.services.burrow.authentik;
  runtimeDir = "/run/burrow-authentik";
  envFile = "${runtimeDir}/authentik.env";
  blueprintDir = "${runtimeDir}/blueprints";
  blueprintFile = "${blueprintDir}/burrow-authentik.yaml";
  postgresVolume = "burrow-authentik-postgresql:/var/lib/postgresql/data";
  dataVolume = "burrow-authentik-data:/data";
  googleSourceSyncScript = ../../Scripts/authentik-sync-google-source.sh;
  authentikBlueprint = pkgs.writeText "burrow-authentik-blueprint.yaml" ''
    version: 1
    metadata:
      name: Burrow Authentik
      labels:
        blueprints.goauthentik.io/description: Minimal Burrow Authentik applications
    entries:
      - model: authentik_providers_oauth2.scopemapping
        id: burrow-oidc-email
        identifiers:
          name: Burrow OIDC Email
        attrs:
          name: Burrow OIDC Email
          scope_name: email
          description: Verified email mapping for Burrow
          expression: |
            return {
                "email": request.user.email,
                "email_verified": True,
            }

      - model: authentik_providers_oauth2.oauth2provider
        id: burrow-oidc-provider-ts
        identifiers:
          name: Burrow Tailnet
        attrs:
          authorization_flow: !Find [authentik_flows.flow, [slug, default-provider-authorization-implicit-consent]]
          invalidation_flow: !Find [authentik_flows.flow, [slug, default-provider-invalidation-flow]]
          issuer_mode: per_provider
          slug: ${cfg.headscaleProviderSlug}
          client_type: confidential
          client_id: ${cfg.headscaleDomain}
          client_secret: !Env [AUTHENTIK_BURROW_TS_CLIENT_SECRET, ""]
          include_claims_in_id_token: true
          redirect_uris:
            - matching_mode: strict
              url: https://${cfg.headscaleDomain}/oidc/callback
          property_mappings:
            - !Find [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-openid]]
            - !KeyOf burrow-oidc-email
            - !Find [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-profile]]
          signing_key: !Find [authentik_crypto.certificatekeypair, [name, authentik Self-signed Certificate]]

      - model: authentik_core.application
        identifiers:
          slug: ${cfg.headscaleProviderSlug}
        attrs:
          name: Burrow Tailnet
          slug: ${cfg.headscaleProviderSlug}
          provider: !KeyOf burrow-oidc-provider-ts
          meta_launch_url: https://${cfg.headscaleDomain}/
  '';
in
{
  options.services.burrow.authentik = {
    enable = lib.mkEnableOption "the Burrow Authentik identity provider";

    domain = lib.mkOption {
      type = lib.types.str;
      default = "auth.burrow.net";
      description = "Public Authentik domain.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 9002;
      description = "Local Authentik HTTP listen port.";
    };

    image = lib.mkOption {
      type = lib.types.str;
      default = "ghcr.io/goauthentik/server:2026.2.1";
      description = "Authentik container image reference.";
    };

    envFile = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/burrow/intake/authentik.env";
      description = "Host-local Authentik bootstrap environment file.";
    };

    headscaleDomain = lib.mkOption {
      type = lib.types.str;
      default = "ts.burrow.net";
      description = "Headscale public domain used for the bundled OIDC client.";
    };

    headscaleProviderSlug = lib.mkOption {
      type = lib.types.str;
      default = "ts";
      description = "Authentik provider slug for Headscale.";
    };

    headscaleClientSecretFile = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/burrow/intake/authentik_headscale_client_secret.txt";
      description = "Host-local file containing the Authentik Headscale OIDC client secret.";
    };

    googleClientIDFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Host-local file containing the Google OAuth client ID for the Authentik source.";
    };

    googleClientSecretFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Host-local file containing the Google OAuth client secret for the Authentik source.";
    };

    googleSourceSlug = lib.mkOption {
      type = lib.types.str;
      default = "google";
      description = "Authentik OAuth source slug used for Google login.";
    };

    googleLoginMode = lib.mkOption {
      type = lib.types.enum [
        "promoted"
        "redirect"
      ];
      default = "redirect";
      description = "Identification-stage behavior for the Google Authentik source.";
    };
  };

  config = lib.mkIf cfg.enable {
    virtualisation.podman.enable = true;

    systemd.tmpfiles.rules = [
      "d ${runtimeDir} 0750 root root -"
      "d ${blueprintDir} 0750 root root -"
    ];

    systemd.services.burrow-authentik-runtime = {
      description = "Render the Burrow Authentik runtime environment";
      before = [
        "podman-burrow-authentik-postgresql.service"
        "podman-burrow-authentik-server.service"
        "podman-burrow-authentik-worker.service"
      ];
      wantedBy = [
        "podman-burrow-authentik-postgresql.service"
        "podman-burrow-authentik-server.service"
        "podman-burrow-authentik-worker.service"
      ];
      after = lib.optionals config.services.burrow.headscale.enable [
        "burrow-headscale-client-secret.service"
      ];
      wants = lib.optionals config.services.burrow.headscale.enable [
        "burrow-headscale-client-secret.service"
      ];
      path = [ pkgs.coreutils ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
        RemainAfterExit = true;
      };
      script = ''
        set -euo pipefail

        if [ ! -s ${lib.escapeShellArg cfg.envFile} ]; then
          echo "Authentik env file missing: ${cfg.envFile}" >&2
          exit 1
        fi

        if [ ! -s ${lib.escapeShellArg cfg.headscaleClientSecretFile} ]; then
          echo "Headscale client secret missing: ${cfg.headscaleClientSecretFile}" >&2
          exit 1
        fi

        install -d -m 0750 -o root -g root ${runtimeDir} ${blueprintDir}
        install -m 0644 -o root -g root ${authentikBlueprint} ${blueprintFile}

        source ${lib.escapeShellArg cfg.envFile}

        read_secret() {
          tr -d '\r\n' < "$1"
        }

        cat > ${envFile} <<EOF
PG_DB=authentik
PG_USER=authentik
PG_PASS=$PG_PASS
POSTGRES_DB=authentik
POSTGRES_USER=authentik
POSTGRES_PASSWORD=$PG_PASS
AUTHENTIK_POSTGRESQL__HOST=127.0.0.1
AUTHENTIK_POSTGRESQL__PORT=5433
AUTHENTIK_POSTGRESQL__NAME=authentik
AUTHENTIK_POSTGRESQL__USER=authentik
AUTHENTIK_POSTGRESQL__PASSWORD=$PG_PASS
AUTHENTIK_LISTEN__HTTP=0.0.0.0:${toString cfg.port}
AUTHENTIK_SECRET_KEY=$AUTHENTIK_SECRET_KEY
AUTHENTIK_BOOTSTRAP_PASSWORD=$AUTHENTIK_BOOTSTRAP_PASSWORD
AUTHENTIK_BOOTSTRAP_TOKEN=$AUTHENTIK_BOOTSTRAP_TOKEN
AUTHENTIK_BURROW_TS_CLIENT_SECRET=$(read_secret ${lib.escapeShellArg cfg.headscaleClientSecretFile})
EOF
        chown root:root ${envFile}
        chmod 0600 ${envFile}
      '';
    };

    virtualisation.oci-containers.containers."burrow-authentik-postgresql" = {
      image = "docker.io/library/postgres:16-alpine";
      autoStart = true;
      environmentFiles = [ envFile ];
      cmd = [
        "-c"
        "port=5433"
        "-c"
        "listen_addresses=127.0.0.1"
      ];
      volumes = [ postgresVolume ];
      extraOptions = [
        "--network=host"
        "--pull=always"
      ];
    };

    virtualisation.oci-containers.containers."burrow-authentik-server" = {
      image = cfg.image;
      autoStart = true;
      cmd = [ "server" ];
      environmentFiles = [ envFile ];
      volumes = [
        dataVolume
        "${blueprintFile}:/blueprints/burrow-authentik.yaml:ro"
      ];
      extraOptions = [
        "--network=host"
        "--pull=always"
      ];
    };

    virtualisation.oci-containers.containers."burrow-authentik-worker" = {
      image = cfg.image;
      autoStart = true;
      cmd = [ "worker" ];
      environmentFiles = [ envFile ];
      volumes = [
        dataVolume
        "${blueprintFile}:/blueprints/burrow-authentik.yaml:ro"
      ];
      extraOptions = [
        "--network=host"
        "--pull=always"
        "--user=root"
      ];
    };

    systemd.services.burrow-authentik-ready = {
      description = "Wait for Burrow Authentik to become ready";
      after = [ "podman-burrow-authentik-server.service" ];
      wants = [ "podman-burrow-authentik-server.service" ];
      wantedBy = [ "multi-user.target" ];
      path = [
        pkgs.coreutils
        pkgs.curl
      ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
      };
      script = ''
        set -euo pipefail

        for _ in $(seq 1 90); do
          if ${pkgs.curl}/bin/curl -fsS http://127.0.0.1:${toString cfg.port}/-/health/ready/ >/dev/null; then
            exit 0
          fi
          sleep 2
        done

        echo "Authentik did not become ready on ${cfg.domain}" >&2
        exit 1
      '';
    };

    systemd.services.burrow-authentik-google-source = lib.mkIf (
      cfg.googleClientIDFile != null && cfg.googleClientSecretFile != null
    ) {
      description = "Reconcile the Burrow Authentik Google OAuth source";
      after = [
        "burrow-authentik-ready.service"
        "network-online.target"
      ];
      wants = [
        "burrow-authentik-ready.service"
        "network-online.target"
      ];
      wantedBy = [ "multi-user.target" ];
      restartTriggers = [
        googleSourceSyncScript
        cfg.envFile
        cfg.googleClientIDFile
        cfg.googleClientSecretFile
      ];
      path = [
        pkgs.bash
        pkgs.coreutils
        pkgs.curl
        pkgs.jq
      ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
        Restart = "on-failure";
        RestartSec = 5;
      };
      script = ''
        set -euo pipefail
        set -a
        source ${lib.escapeShellArg cfg.envFile}
        set +a

        export AUTHENTIK_URL=https://${cfg.domain}
        export AUTHENTIK_GOOGLE_SOURCE_SLUG=${lib.escapeShellArg cfg.googleSourceSlug}
        export AUTHENTIK_GOOGLE_LOGIN_MODE=${lib.escapeShellArg cfg.googleLoginMode}
        export AUTHENTIK_GOOGLE_USER_MATCHING_MODE=email_link
        export AUTHENTIK_GOOGLE_CLIENT_ID="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.googleClientIDFile})"
        export AUTHENTIK_GOOGLE_CLIENT_SECRET="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.googleClientSecretFile})"

        ${pkgs.bash}/bin/bash ${googleSourceSyncScript}
      '';
    };

    services.caddy.virtualHosts."${cfg.domain}".extraConfig = ''
      encode gzip zstd
      reverse_proxy 127.0.0.1:${toString cfg.port}
    '';
  };
}
