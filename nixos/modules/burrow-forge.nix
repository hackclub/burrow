{ config, lib, pkgs, ... }:

let
  cfg = config.services.burrow.forge;
  forgejoCfg = config.services.forgejo;
  forgejoExe = lib.getExe forgejoCfg.package;
  forgejoWorkPath = forgejoCfg.stateDir;
  forgejoCustomPath = "${forgejoWorkPath}/custom";
  forgejoConfigFile = "${forgejoCustomPath}/conf/app.ini";
  forgejoAdminArgs = "--config ${lib.escapeShellArg forgejoConfigFile} --work-path ${lib.escapeShellArg forgejoWorkPath} --custom-path ${lib.escapeShellArg forgejoCustomPath}";
  homeRepoPath = "/${cfg.homeOwner}/${cfg.homeRepo}";
  homeRepoUrl = "https://${cfg.gitDomain}${homeRepoPath}";
in
{
  options.services.burrow.forge = {
    enable = lib.mkEnableOption "the Burrow Forge host";

    gitDomain = lib.mkOption {
      type = lib.types.str;
      default = "git.burrow.net";
      description = "Public Forgejo domain.";
    };

    siteDomain = lib.mkOption {
      type = lib.types.str;
      default = "burrow.net";
      description = "Root site domain.";
    };

    homeOwner = lib.mkOption {
      type = lib.types.str;
      default = "hackclub";
      description = "Canonical Forgejo org/user for the Burrow home repository.";
    };

    homeRepo = lib.mkOption {
      type = lib.types.str;
      default = "burrow";
      description = "Canonical Forgejo repository name for the Burrow home repository.";
    };

    contactEmail = lib.mkOption {
      type = lib.types.str;
      default = "contact@burrow.net";
      description = "Operator contact email.";
    };

    nscAutoscalerDomain = lib.mkOption {
      type = lib.types.str;
      default = "nsc-autoscaler.burrow.net";
      description = "Public webhook domain for the Forgejo Namespace autoscaler.";
    };

    adminUsername = lib.mkOption {
      type = lib.types.str;
      default = "contact";
      description = "Initial Forgejo admin username.";
    };

    adminEmail = lib.mkOption {
      type = lib.types.str;
      default = "contact@burrow.net";
      description = "Initial Forgejo admin email.";
    };

    adminPasswordFile = lib.mkOption {
      type = lib.types.str;
      description = "Host-local path to the plaintext bootstrap password file for the initial Forgejo admin.";
    };

    oidcDisplayName = lib.mkOption {
      type = lib.types.str;
      default = "burrow.net";
      description = "Login button label for the Forgejo OIDC provider.";
    };

    oidcClientId = lib.mkOption {
      type = lib.types.str;
      default = "git.burrow.net";
      description = "OIDC client ID that Forgejo should use against Authentik.";
    };

    oidcClientSecretFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Host-local path to the Forgejo OIDC client secret.";
    };

    oidcDiscoveryUrl = lib.mkOption {
      type = lib.types.str;
      default = "https://auth.burrow.net/application/o/git/.well-known/openid-configuration";
      description = "OpenID Connect discovery URL for the Forgejo login source.";
    };

    oidcScopes = lib.mkOption {
      type = with lib.types; listOf str;
      default = [
        "openid"
        "profile"
        "email"
        "groups"
      ];
      description = "OIDC scopes requested from Authentik.";
    };

    oidcGroupClaimName = lib.mkOption {
      type = lib.types.str;
      default = "groups";
      description = "OIDC claim name that carries group membership.";
    };

    oidcAdminGroup = lib.mkOption {
      type = lib.types.str;
      default = "burrow-admins";
      description = "OIDC group that should grant Forgejo admin access.";
    };

    oidcRestrictedGroup = lib.mkOption {
      type = lib.types.str;
      default = "burrow-users";
      description = "OIDC group that is required to log into Forgejo.";
    };

    oidcAutoRegistration = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether Forgejo should automatically create users for new OIDC sign-ins.";
    };

    oidcAccountLinking = lib.mkOption {
      type = lib.types.enum [ "disabled" "login" "auto" ];
      default = "auto";
      description = "How Forgejo should link existing local accounts for OIDC sign-ins.";
    };

    oidcUsernameSource = lib.mkOption {
      type = lib.types.enum [ "userid" "nickname" "email" ];
      default = "email";
      description = "Which OIDC claim Forgejo should use to derive usernames for auto-registration.";
    };

    authorizedKeys = lib.mkOption {
      type = with lib.types; listOf str;
      default = [ ];
      description = "SSH keys allowed for root login and operational bootstrap.";
    };
  };

  config = lib.mkIf cfg.enable {
    networking.hostName = "burrow-forge";
    networking.useDHCP = lib.mkDefault true;

    services.qemuGuest.enable = true;

    boot.loader.grub = {
      enable = true;
      efiSupport = true;
      efiInstallAsRemovable = true;
      device = "nodev";
    };

    fileSystems."/boot".neededForBoot = true;

    services.postgresql = {
      enable = true;
      package = pkgs.postgresql_16;
    };

    services.openssh = {
      enable = true;
      settings = {
        PasswordAuthentication = false;
        KbdInteractiveAuthentication = false;
        PermitRootLogin = "prohibit-password";
      };
    };

    users.users.root.openssh.authorizedKeys.keys = cfg.authorizedKeys;

    networking.firewall.allowedTCPPorts = [
      22
      80
      443
      2222
    ];

    services.forgejo = {
      enable = true;
      database = {
        type = "postgres";
        createDatabase = true;
      };
      lfs.enable = true;
      settings = {
        server = {
          DOMAIN = cfg.gitDomain;
          ROOT_URL = "https://${cfg.gitDomain}/";
          HTTP_PORT = 3000;
          SSH_DOMAIN = cfg.gitDomain;
          SSH_PORT = 2222;
          START_SSH_SERVER = true;
        };

        service = {
          DISABLE_REGISTRATION = true;
          ENABLE_INTERNAL_SIGNIN = false;
          ENABLE_BASIC_AUTHENTICATION = false;
          SHOW_REGISTRATION_BUTTON = false;
          REQUIRE_SIGNIN_VIEW = false;
          DEFAULT_ALLOW_CREATE_ORGANIZATION = false;
          ENABLE_NOTIFY_MAIL = false;
          NO_REPLY_ADDRESS = cfg.adminEmail;
        };

        session = {
          COOKIE_SECURE = true;
          SAME_SITE = "strict";
        };

        openid = {
          ENABLE_OPENID_SIGNIN = false;
          ENABLE_OPENID_SIGNUP = false;
        };

        oauth2_client = {
          OPENID_CONNECT_SCOPES = lib.concatStringsSep " " (lib.subtractLists [ "openid" ] cfg.oidcScopes);
          ENABLE_AUTO_REGISTRATION = cfg.oidcAutoRegistration;
          ACCOUNT_LINKING = cfg.oidcAccountLinking;
          USERNAME = cfg.oidcUsernameSource;
        };

        actions = {
          ENABLED = true;
        };

        repository = {
          DEFAULT_BRANCH = "main";
          ENABLE_PUSH_CREATE_USER = false;
        };

        ui = {
          DEFAULT_THEME = "forgejo-auto";
        };
      };
    };

    services.caddy = {
      enable = true;
      email = cfg.contactEmail;
      virtualHosts =
        {
          "${cfg.gitDomain}".extraConfig = ''
            encode gzip zstd
            @root path /
            redir @root ${homeRepoPath} 308
            reverse_proxy 127.0.0.1:${toString config.services.forgejo.settings.server.HTTP_PORT}
          '';
          "${cfg.siteDomain}".extraConfig = ''
            encode gzip zstd
            @oidcConfig path /.well-known/openid-configuration
            redir @oidcConfig https://${config.services.burrow.authentik.domain}/application/o/${config.services.burrow.authentik.forgejoProviderSlug}/.well-known/openid-configuration 308
            @tailnetConfig path /.well-known/burrow-tailnet
            header @tailnetConfig Content-Type application/json
            respond @tailnetConfig "{\"domain\":\"${cfg.siteDomain}\",\"provider\":\"headscale\",\"authority\":\"https://${config.services.burrow.headscale.domain}\",\"oidc_issuer\":\"https://${config.services.burrow.authentik.domain}/application/o/${config.services.burrow.authentik.headscaleProviderSlug}/\"}" 200
            @webfinger path /.well-known/webfinger
            header @webfinger Content-Type application/jrd+json
            respond @webfinger "{\"subject\":\"{query.resource}\",\"links\":[{\"rel\":\"http://openid.net/specs/connect/1.0/issuer\",\"href\":\"https://${config.services.burrow.authentik.domain}/application/o/${config.services.burrow.authentik.forgejoProviderSlug}/\"},{\"rel\":\"https://burrow.net/rel/tailnet-control-server\",\"href\":\"https://${config.services.burrow.headscale.domain}\"}]}" 200
            @root path /
            redir @root ${homeRepoUrl} 308
            respond 404
          '';
        }
        // lib.optionalAttrs (
          config.services.burrow.forgejoNsc.enable && config.services.burrow.forgejoNsc.autoscaler.enable
        ) {
          "${cfg.nscAutoscalerDomain}".extraConfig = ''
            encode gzip zstd
            reverse_proxy 127.0.0.1:8090
          '';
        };
    };

    systemd.services.burrow-forgejo-bootstrap = {
      description = "Seed the initial Burrow Forgejo admin account";
      after = [ "forgejo.service" ];
      requires = [ "forgejo.service" ];
      wantedBy = [ "multi-user.target" ];
      path = [
        forgejoCfg.package
        pkgs.coreutils
        pkgs.gnugrep
      ];
      serviceConfig = {
        Type = "oneshot";
        User = forgejoCfg.user;
        Group = forgejoCfg.group;
        WorkingDirectory = forgejoCfg.stateDir;
      };
      script = ''
        set -euo pipefail

        if [ ! -s ${lib.escapeShellArg cfg.adminPasswordFile} ]; then
          echo "bootstrap password file is missing; skipping admin bootstrap" >&2
          exit 0
        fi

        password="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.adminPasswordFile})"
        if [ -z "$password" ]; then
          echo "bootstrap password file is empty; skipping admin bootstrap" >&2
          exit 0
        fi

        log_file="$(mktemp)"
        trap 'rm -f "$log_file"' EXIT

        if ! ${forgejoExe} admin user create \
          ${forgejoAdminArgs} \
          --admin \
          --username ${lib.escapeShellArg cfg.adminUsername} \
          --email ${lib.escapeShellArg cfg.adminEmail} \
          --password "$password" \
          --must-change-password=false >"$log_file" 2>&1; then
          if grep -qi "already exists" "$log_file"; then
            ${forgejoExe} admin user change-password \
              ${forgejoAdminArgs} \
              --username ${lib.escapeShellArg cfg.adminUsername} \
              --password "$password" \
              --must-change-password=false
          else
            cat "$log_file" >&2
            exit 1
          fi
        fi
      '';
    };

    systemd.services.burrow-forgejo-oidc-bootstrap = lib.mkIf (cfg.oidcClientSecretFile != null) {
      description = "Seed the Burrow Forgejo OIDC login source";
      after = [
        "forgejo.service"
        "postgresql.service"
      ] ++ lib.optionals config.services.burrow.authentik.enable [
        "burrow-authentik-ready.service"
      ];
      wants = lib.optionals config.services.burrow.authentik.enable [
        "burrow-authentik-ready.service"
      ];
      requires = [
        "forgejo.service"
        "postgresql.service"
      ];
      wantedBy = [ "multi-user.target" ];
      restartTriggers = [
        cfg.oidcClientSecretFile
      ];
      path = [
        pkgs.coreutils
        pkgs.gnugrep
        pkgs.jq
        pkgs.postgresql
      ];
      serviceConfig = {
        Type = "oneshot";
        User = forgejoCfg.user;
        Group = forgejoCfg.group;
        WorkingDirectory = forgejoCfg.stateDir;
      };
      script = ''
        set -euo pipefail

        if [ ! -s ${lib.escapeShellArg cfg.oidcClientSecretFile} ]; then
          echo "Forgejo OIDC client secret missing: ${cfg.oidcClientSecretFile}" >&2
          exit 1
        fi

        ready=0
        for attempt in $(seq 1 60); do
          if ${pkgs.postgresql}/bin/psql -h /run/postgresql -U forgejo forgejo -tAc \
            "SELECT 1 FROM pg_tables WHERE schemaname='public' AND tablename='login_source';" \
            | grep -q 1; then
            ready=1
            break
          fi
          sleep 1
        done

        if [ "$ready" -ne 1 ]; then
          echo "Forgejo login_source table did not become ready" >&2
          exit 1
        fi

        oidc_secret="$(${pkgs.coreutils}/bin/tr -d '\r\n' < ${lib.escapeShellArg cfg.oidcClientSecretFile})"
        if [ -z "$oidc_secret" ]; then
          echo "Forgejo OIDC client secret is empty" >&2
          exit 1
        fi

        cfg_json="$(${pkgs.jq}/bin/jq -nc \
          --arg client_id ${lib.escapeShellArg cfg.oidcClientId} \
          --arg client_secret "$oidc_secret" \
          --arg discovery_url ${lib.escapeShellArg cfg.oidcDiscoveryUrl} \
          --argjson scopes '${builtins.toJSON cfg.oidcScopes}' \
          --arg group_claim_name ${lib.escapeShellArg cfg.oidcGroupClaimName} \
          --arg admin_group ${lib.escapeShellArg cfg.oidcAdminGroup} \
          --arg restricted_group ${lib.escapeShellArg cfg.oidcRestrictedGroup} \
          '{
            Provider: "openidConnect",
            ClientID: $client_id,
            ClientSecret: $client_secret,
            OpenIDConnectAutoDiscoveryURL: $discovery_url,
            CustomURLMapping: null,
            IconURL: "",
            Scopes: $scopes,
            AttributeSSHPublicKey: "",
            RequiredClaimName: "",
            RequiredClaimValue: "",
            GroupClaimName: $group_claim_name,
            AdminGroup: $admin_group,
            GroupTeamMap: "",
            GroupTeamMapRemoval: false,
            RestrictedGroup: $restricted_group
          }')"

        ${pkgs.postgresql}/bin/psql -v ON_ERROR_STOP=1 \
          -h /run/postgresql -U forgejo forgejo \
          -v oidc_name=${lib.escapeShellArg cfg.oidcDisplayName} \
          -v cfg_json="$cfg_json" <<'SQL'
        INSERT INTO login_source (
          type, name, is_active, is_sync_enabled, cfg, created_unix, updated_unix
        ) VALUES (
          6,
          :'oidc_name',
          TRUE,
          FALSE,
          :'cfg_json',
          EXTRACT(EPOCH FROM NOW())::BIGINT,
          EXTRACT(EPOCH FROM NOW())::BIGINT
        )
        ON CONFLICT (name) DO UPDATE SET
          type = EXCLUDED.type,
          is_active = TRUE,
          is_sync_enabled = FALSE,
          cfg = EXCLUDED.cfg,
          updated_unix = EXCLUDED.updated_unix;
        SQL
      '';
    };
  };
}
