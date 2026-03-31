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
  };
}
