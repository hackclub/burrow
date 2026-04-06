{ config, lib, pkgs, self, ... }:

let
  cfg = config.services.burrow.namespacePortal;
  burrowExe = lib.getExe self.packages.${pkgs.system}.burrow;
  nscExe = lib.getExe self.packages.${pkgs.system}.nsc;
in
{
  options.services.burrow.namespacePortal = {
    enable = lib.mkEnableOption "the Burrow Namespace authentication portal";

    domain = lib.mkOption {
      type = lib.types.str;
      default = "nsc.burrow.net";
      description = "Public domain for the Namespace portal.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 9080;
      description = "Local listen port for the Namespace portal.";
    };

    baseUrl = lib.mkOption {
      type = lib.types.str;
      default = "https://nsc.burrow.net";
      description = "Public base URL for redirects.";
    };

    oidcProviderSlug = lib.mkOption {
      type = lib.types.str;
      default = "namespace";
      description = "Authentik provider slug used for the portal.";
    };

    oidcClientId = lib.mkOption {
      type = lib.types.str;
      default = "nsc.burrow.net";
      description = "OIDC client ID used by the portal.";
    };

    oidcClientSecretFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Optional host-local OIDC client secret for the portal.";
    };

    adminGroup = lib.mkOption {
      type = lib.types.str;
      default = "burrow-admins";
      description = "Authentik group required to access the portal.";
    };

    stateDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/burrow/namespace-portal";
      description = "Persistent state directory for the portal-owned NSC session.";
    };

    tokenOutputPath = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/burrow/intake/forgejo_nsc_token.txt";
      description = "Path where refreshed NSC tokens should be written.";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = config.services.forgejo-nsc.enable;
        message = "services.burrow.namespacePortal requires services.forgejo-nsc.enable";
      }
    ];

    systemd.tmpfiles.rules = [
      "d ${cfg.stateDir} 0750 forgejo-nsc forgejo-nsc -"
      "d ${cfg.stateDir}/nsc 0750 forgejo-nsc forgejo-nsc -"
    ];

    systemd.services.burrow-namespace-portal = {
      description = "Burrow Namespace authentication portal";
      after = [
        "network-online.target"
        "burrow-authentik-ready.service"
      ];
      wants = [
        "network-online.target"
        "burrow-authentik-ready.service"
      ];
      wantedBy = [ "multi-user.target" ];
      path = [
        self.packages.${pkgs.system}.burrow
        self.packages.${pkgs.system}.nsc
        pkgs.coreutils
      ];
      serviceConfig = {
        Type = "simple";
        User = "forgejo-nsc";
        Group = "forgejo-nsc";
        WorkingDirectory = cfg.stateDir;
        Restart = "on-failure";
        RestartSec = "2s";
      };
      script = ''
        set -euo pipefail
        export BURROW_NAMESPACE_PORTAL_LISTEN=127.0.0.1:${toString cfg.port}
        export BURROW_NAMESPACE_PORTAL_BASE_URL=${lib.escapeShellArg cfg.baseUrl}
        export BURROW_NAMESPACE_PORTAL_OIDC_DISCOVERY_URL=${lib.escapeShellArg "https://${config.services.burrow.authentik.domain}/application/o/${cfg.oidcProviderSlug}/.well-known/openid-configuration"}
        export BURROW_NAMESPACE_PORTAL_OIDC_CLIENT_ID=${lib.escapeShellArg cfg.oidcClientId}
        export BURROW_NAMESPACE_PORTAL_ALLOWED_GROUP=${lib.escapeShellArg cfg.adminGroup}
        export BURROW_NAMESPACE_PORTAL_NSC_BIN=${lib.escapeShellArg nscExe}
        export BURROW_NAMESPACE_PORTAL_NSC_STATE_DIR=${lib.escapeShellArg "${cfg.stateDir}/nsc"}
        export BURROW_NAMESPACE_PORTAL_TOKEN_OUTPUT_PATH=${lib.escapeShellArg cfg.tokenOutputPath}
        ${lib.optionalString (cfg.oidcClientSecretFile != null) ''
          export BURROW_NAMESPACE_PORTAL_OIDC_CLIENT_SECRET="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.oidcClientSecretFile})"
        ''}
        exec ${burrowExe} namespace-portal
      '';
    };

    services.caddy.virtualHosts."${cfg.domain}".extraConfig = ''
      encode gzip zstd
      reverse_proxy 127.0.0.1:${toString cfg.port}
    '';
  };
}
