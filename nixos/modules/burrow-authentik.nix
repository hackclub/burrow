{ config, lib, pkgs, ... }:

let
  cfg = config.services.burrow.authentik;
  runtimeDir = "/run/burrow-authentik";
  envFile = "${runtimeDir}/authentik.env";
  blueprintDir = "${runtimeDir}/blueprints";
  blueprintFile = "${blueprintDir}/burrow-authentik.yaml";
  postgresVolume = "burrow-authentik-postgresql:/var/lib/postgresql/data";
  dataVolume = "burrow-authentik-data:/data";
  directorySyncScript = ../../Scripts/authentik-sync-burrow-directory.sh;
  forgejoOidcSyncScript = ../../Scripts/authentik-sync-forgejo-oidc.sh;
  namespacePortalOidcSyncScript = ../../Scripts/authentik-sync-namespace-portal-oidc.sh;
  tailscaleOidcSyncScript = ../../Scripts/authentik-sync-tailscale-oidc.sh;
  googleSourceSyncScript = ../../Scripts/authentik-sync-google-source.sh;
  tailnetAuthFlowSyncScript = ../../Scripts/authentik-sync-tailnet-auth-flow.sh;
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

      - model: authentik_providers_oauth2.scopemapping
        id: burrow-oidc-groups
        identifiers:
          name: Burrow OIDC Groups
        attrs:
          name: Burrow OIDC Groups
          scope_name: groups
          description: Group membership mapping for Burrow
          expression: |
            return {
                "groups": [group.name for group in request.user.ak_groups.all()],
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
            - !KeyOf burrow-oidc-groups
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

    forgejoDomain = lib.mkOption {
      type = lib.types.str;
      default = "git.burrow.net";
      description = "Forgejo public domain used for the bundled OIDC client.";
    };

    forgejoProviderSlug = lib.mkOption {
      type = lib.types.str;
      default = "git";
      description = "Authentik application slug for Forgejo.";
    };

    tailscaleProviderSlug = lib.mkOption {
      type = lib.types.str;
      default = "tailscale";
      description = "Authentik application slug for Tailscale custom OIDC sign-in.";
    };

    namespacePortalDomain = lib.mkOption {
      type = lib.types.str;
      default = "nsc.burrow.net";
      description = "Public domain for the Burrow Namespace portal.";
    };

    namespacePortalProviderSlug = lib.mkOption {
      type = lib.types.str;
      default = "namespace";
      description = "Authentik application slug for the Namespace portal.";
    };

    namespacePortalClientId = lib.mkOption {
      type = lib.types.str;
      default = "nsc.burrow.net";
      description = "Client ID Authentik should present to the Namespace portal.";
    };

    namespacePortalClientSecretFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Optional host-local file containing the Authentik Namespace portal OIDC client secret.";
    };

    tailscaleClientId = lib.mkOption {
      type = lib.types.str;
      default = "tailscale.burrow.net";
      description = "Client ID Authentik should present to Tailscale.";
    };

    tailscaleClientSecretFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Host-local file containing the Authentik Tailscale OIDC client secret.";
    };

    forgejoClientId = lib.mkOption {
      type = lib.types.str;
      default = "git.burrow.net";
      description = "Client ID Authentik should present to Forgejo.";
    };

    forgejoClientSecretFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Host-local file containing the Authentik Forgejo OIDC client secret.";
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

    headscaleAuthenticationFlowSlug = lib.mkOption {
      type = lib.types.str;
      default = "burrow-tailnet-authentication";
      description = "Authentik authentication flow slug used for Burrow Tailnet sign-in.";
    };

    headscaleAuthenticationFlowName = lib.mkOption {
      type = lib.types.str;
      default = "Burrow Tailnet Authentication";
      description = "Authentik authentication flow name used for Burrow Tailnet sign-in.";
    };

    headscaleIdentificationStageName = lib.mkOption {
      type = lib.types.str;
      default = "burrow-tailnet-identification-stage";
      description = "Authentik identification stage used for Burrow Tailnet sign-in.";
    };

    headscalePasswordStageName = lib.mkOption {
      type = lib.types.str;
      default = "burrow-tailnet-password-stage";
      description = "Authentik password stage used for Burrow Tailnet sign-in.";
    };

    headscaleUserLoginStageName = lib.mkOption {
      type = lib.types.str;
      default = "burrow-tailnet-user-login-stage";
      description = "Authentik user-login stage used for Burrow Tailnet sign-in.";
    };

    userGroupName = lib.mkOption {
      type = lib.types.str;
      default = "burrow-users";
      description = "Authentik group granted baseline Burrow access.";
    };

    adminGroupName = lib.mkOption {
      type = lib.types.str;
      default = "burrow-admins";
      description = "Authentik group granted Burrow administrator access.";
    };

    bootstrapUsers = lib.mkOption {
      type = with lib.types; listOf (submodule {
        options = {
          username = lib.mkOption {
            type = str;
            description = "Authentik username.";
          };
          name = lib.mkOption {
            type = str;
            description = "Display name for the user.";
          };
          email = lib.mkOption {
            type = str;
            description = "Canonical email stored in Authentik.";
          };
          sourceEmail = lib.mkOption {
            type = nullOr str;
            default = null;
            description = "External Google account email that should map onto this Authentik user.";
          };
          groups = lib.mkOption {
            type = listOf str;
            default = [ ];
            description = "Additional Authentik groups for this user.";
          };
          isAdmin = lib.mkOption {
            type = bool;
            default = false;
            description = "Whether this user should be in the Burrow admin group.";
          };
          passwordFile = lib.mkOption {
            type = nullOr str;
            default = null;
            description = "Optional host-local file containing a bootstrap password for this user.";
          };
        };
      });
      default = [ ];
      description = "Declarative Burrow users to create in Authentik.";
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

        ${lib.optionalString (cfg.forgejoClientSecretFile != null) ''
          if [ ! -s ${lib.escapeShellArg cfg.forgejoClientSecretFile} ]; then
            echo "Forgejo client secret missing: ${cfg.forgejoClientSecretFile}" >&2
            exit 1
          fi
        ''}

        ${lib.optionalString (cfg.tailscaleClientSecretFile != null) ''
          if [ ! -s ${lib.escapeShellArg cfg.tailscaleClientSecretFile} ]; then
            echo "Tailscale client secret missing: ${cfg.tailscaleClientSecretFile}" >&2
            exit 1
          fi
        ''}

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
${lib.optionalString (cfg.forgejoClientSecretFile != null) "AUTHENTIK_BURROW_FORGEJO_CLIENT_SECRET=$(read_secret ${lib.escapeShellArg cfg.forgejoClientSecretFile})"}
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

    systemd.services.podman-burrow-authentik-server.restartTriggers = [
      blueprintFile
      envFile
    ];

    systemd.services.podman-burrow-authentik-worker.restartTriggers = [
      blueprintFile
      envFile
    ];

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
        export AUTHENTIK_GOOGLE_ACCOUNT_MAP_JSON='${builtins.toJSON (map (user: {
          source_email = user.sourceEmail;
          username = user.username;
          email = user.email;
          name = user.name;
        }) (lib.filter (user: user.sourceEmail != null) cfg.bootstrapUsers))}'

        ${pkgs.bash}/bin/bash ${googleSourceSyncScript}
      '';
    };

    systemd.services.burrow-authentik-directory = lib.mkIf (cfg.bootstrapUsers != [ ]) {
      description = "Reconcile Burrow Authentik users and groups";
      after =
        [
          "burrow-authentik-ready.service"
          "network-online.target"
        ]
        ++ lib.optionals (cfg.forgejoClientSecretFile != null) [ "burrow-authentik-forgejo-oidc.service" ];
      wants =
        [
          "burrow-authentik-ready.service"
          "network-online.target"
        ]
        ++ lib.optionals (cfg.forgejoClientSecretFile != null) [ "burrow-authentik-forgejo-oidc.service" ];
      wantedBy = [ "multi-user.target" ];
      restartTriggers = [
        directorySyncScript
        cfg.envFile
      ] ++ lib.concatMap (user: lib.optional (user.passwordFile != null) user.passwordFile) cfg.bootstrapUsers;
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
      };
      script = ''
        set -euo pipefail
        set -a
        source ${lib.escapeShellArg cfg.envFile}
        set +a

        export AUTHENTIK_URL=https://${cfg.domain}
        export AUTHENTIK_BURROW_USERS_GROUP=${lib.escapeShellArg cfg.userGroupName}
        export AUTHENTIK_BURROW_ADMINS_GROUP=${lib.escapeShellArg cfg.adminGroupName}
        export AUTHENTIK_FORGEJO_APPLICATION_SLUG=${lib.escapeShellArg cfg.forgejoProviderSlug}
        export AUTHENTIK_BURROW_DIRECTORY_JSON='${builtins.toJSON (map (user: {
          inherit (user) username name email isAdmin passwordFile;
          groups = user.groups;
        }) cfg.bootstrapUsers)}'

        ${pkgs.bash}/bin/bash ${directorySyncScript}
      '';
    };

    systemd.services.burrow-authentik-tailnet-auth-flow = {
      description = "Reconcile the Burrow Tailnet authentication flow";
      after =
        [
          "burrow-authentik-ready.service"
          "network-online.target"
        ]
        ++ lib.optionals (
          cfg.googleClientIDFile != null && cfg.googleClientSecretFile != null
        ) [ "burrow-authentik-google-source.service" ];
      wants =
        [
          "burrow-authentik-ready.service"
          "network-online.target"
        ]
        ++ lib.optionals (
          cfg.googleClientIDFile != null && cfg.googleClientSecretFile != null
        ) [ "burrow-authentik-google-source.service" ];
      wantedBy = [ "multi-user.target" ];
      restartTriggers = [
        tailnetAuthFlowSyncScript
        cfg.envFile
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
      };
      script = ''
        set -euo pipefail
        set -a
        source ${lib.escapeShellArg cfg.envFile}
        set +a

        export AUTHENTIK_URL=https://${cfg.domain}
        export AUTHENTIK_TAILNET_PROVIDER_SLUG=${lib.escapeShellArg cfg.headscaleProviderSlug}
        export AUTHENTIK_TAILNET_PROVIDER_SLUGS_JSON='["${cfg.headscaleProviderSlug}","${cfg.tailscaleProviderSlug}"]'
        export AUTHENTIK_TAILNET_AUTHENTICATION_FLOW_NAME=${lib.escapeShellArg cfg.headscaleAuthenticationFlowName}
        export AUTHENTIK_TAILNET_AUTHENTICATION_FLOW_SLUG=${lib.escapeShellArg cfg.headscaleAuthenticationFlowSlug}
        export AUTHENTIK_TAILNET_IDENTIFICATION_STAGE_NAME=${lib.escapeShellArg cfg.headscaleIdentificationStageName}
        export AUTHENTIK_TAILNET_PASSWORD_STAGE_NAME=${lib.escapeShellArg cfg.headscalePasswordStageName}
        export AUTHENTIK_TAILNET_USER_LOGIN_STAGE_NAME=${lib.escapeShellArg cfg.headscaleUserLoginStageName}
        export AUTHENTIK_TAILNET_GOOGLE_SOURCE_SLUG=${lib.escapeShellArg cfg.googleSourceSlug}

        ${pkgs.bash}/bin/bash ${tailnetAuthFlowSyncScript}
      '';
    };

    systemd.services.burrow-authentik-forgejo-oidc = lib.mkIf (cfg.forgejoClientSecretFile != null) {
      description = "Reconcile the Burrow Authentik Forgejo OIDC application";
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
        forgejoOidcSyncScript
        cfg.envFile
        cfg.forgejoClientSecretFile
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
      };
      script = ''
        set -euo pipefail
        set -a
        source ${lib.escapeShellArg cfg.envFile}
        set +a

        export AUTHENTIK_URL=https://${cfg.domain}
        export AUTHENTIK_FORGEJO_APPLICATION_SLUG=${lib.escapeShellArg cfg.forgejoProviderSlug}
        export AUTHENTIK_FORGEJO_APPLICATION_NAME=burrow.net
        export AUTHENTIK_FORGEJO_PROVIDER_NAME=burrow.net
        export AUTHENTIK_FORGEJO_CLIENT_ID=${lib.escapeShellArg cfg.forgejoClientId}
        export AUTHENTIK_FORGEJO_CLIENT_SECRET="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.forgejoClientSecretFile})"
        export AUTHENTIK_FORGEJO_LAUNCH_URL=https://${cfg.forgejoDomain}/
        export AUTHENTIK_FORGEJO_REDIRECT_URIS_JSON='["https://${cfg.forgejoDomain}/user/oauth2/burrow.net/callback","https://${cfg.forgejoDomain}/user/oauth2/authentik/callback","https://${cfg.forgejoDomain}/user/oauth2/GitHub/callback"]'

        ${pkgs.bash}/bin/bash ${forgejoOidcSyncScript}
      '';
    };

    systemd.services.burrow-authentik-tailscale-oidc = lib.mkIf (cfg.tailscaleClientSecretFile != null) {
      description = "Reconcile the Burrow Authentik Tailscale OIDC application";
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
        tailscaleOidcSyncScript
        cfg.envFile
        cfg.tailscaleClientSecretFile
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
      };
      script = ''
        set -euo pipefail
        set -a
        source ${lib.escapeShellArg cfg.envFile}
        set +a

        export AUTHENTIK_URL=https://${cfg.domain}
        export AUTHENTIK_TAILSCALE_APPLICATION_SLUG=${lib.escapeShellArg cfg.tailscaleProviderSlug}
        export AUTHENTIK_TAILSCALE_APPLICATION_NAME=Tailscale
        export AUTHENTIK_TAILSCALE_PROVIDER_NAME=Tailscale
        export AUTHENTIK_TAILSCALE_TEMPLATE_SLUG=${lib.escapeShellArg cfg.headscaleProviderSlug}
        export AUTHENTIK_TAILSCALE_CLIENT_ID=${lib.escapeShellArg cfg.tailscaleClientId}
        export AUTHENTIK_TAILSCALE_CLIENT_SECRET="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.tailscaleClientSecretFile})"
        export AUTHENTIK_TAILSCALE_LAUNCH_URL=https://login.tailscale.com/start/oidc
        export AUTHENTIK_TAILSCALE_REDIRECT_URIS_JSON='["https://login.tailscale.com/a/oauth_response"]'

        ${pkgs.bash}/bin/bash ${tailscaleOidcSyncScript}
      '';
    };

    systemd.services.burrow-authentik-namespace-portal-oidc = {
      description = "Reconcile the Burrow Authentik Namespace portal OIDC application";
      after = [
        "burrow-authentik-ready.service"
        "network-online.target"
      ];
      wants = [
        "burrow-authentik-ready.service"
        "network-online.target"
      ];
      wantedBy = [ "multi-user.target" ];
      restartTriggers =
        [
          namespacePortalOidcSyncScript
          cfg.envFile
        ]
        ++ lib.optionals (cfg.namespacePortalClientSecretFile != null) [ cfg.namespacePortalClientSecretFile ];
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
      };
      script = ''
        set -euo pipefail
        set -a
        source ${lib.escapeShellArg cfg.envFile}
        set +a

        export AUTHENTIK_URL=https://${cfg.domain}
        export AUTHENTIK_NAMESPACE_PORTAL_APPLICATION_SLUG=${lib.escapeShellArg cfg.namespacePortalProviderSlug}
        export AUTHENTIK_NAMESPACE_PORTAL_APPLICATION_NAME="Namespace Portal"
        export AUTHENTIK_NAMESPACE_PORTAL_PROVIDER_NAME="Namespace Portal"
        export AUTHENTIK_NAMESPACE_PORTAL_TEMPLATE_SLUG=${lib.escapeShellArg cfg.headscaleProviderSlug}
        export AUTHENTIK_NAMESPACE_PORTAL_CLIENT_ID=${lib.escapeShellArg cfg.namespacePortalClientId}
        ${lib.optionalString (cfg.namespacePortalClientSecretFile != null) ''
          export AUTHENTIK_NAMESPACE_PORTAL_CLIENT_SECRET="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.namespacePortalClientSecretFile})"
        ''}
        export AUTHENTIK_NAMESPACE_PORTAL_LAUNCH_URL=https://${cfg.namespacePortalDomain}/
        export AUTHENTIK_NAMESPACE_PORTAL_REDIRECT_URIS_JSON='["https://${cfg.namespacePortalDomain}/oauth/callback"]'

        ${pkgs.bash}/bin/bash ${namespacePortalOidcSyncScript}
      '';
    };

    services.caddy.virtualHosts."${cfg.domain}".extraConfig = ''
      encode gzip zstd
      reverse_proxy 127.0.0.1:${toString cfg.port}
    '';
  };
}
