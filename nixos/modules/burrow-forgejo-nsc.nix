{ config, lib, pkgs, self, ... }:

let
  inherit (lib)
    mkEnableOption
    mkIf
    mkOption
    types
    mkAfter
    mkDefault
    optional
    optionalAttrs
    optionalString
    ;

  cfg = config.services.burrow.forgejoNsc;
  dispatcherRuntimeConfig = "${cfg.stateDir}/dispatcher.yaml";
  autoscalerRuntimeConfig = "${cfg.stateDir}/autoscaler.yaml";

  pendingCheck = configPath: pkgs.writeShellScript "forgejo-nsc-check-pending" ''
    set -euo pipefail
    if ${pkgs.gnugrep}/bin/grep -q 'PENDING-' '${configPath}'; then
      echo "forgejo-nsc config still contains placeholder values (PENDING-); update ${configPath} before starting." >&2
      exit 1
    fi
  '';

  nscTokenPath = "${cfg.stateDir}/nsc.token";
  tokenSync = optionalString (cfg.nscTokenFile != null) ''
    install -m 600 ${lib.escapeShellArg cfg.nscTokenFile} ${lib.escapeShellArg nscTokenPath}
    chown ${cfg.user}:${cfg.group} ${nscTokenPath}
    chmod 600 ${nscTokenPath}
  '';
  dispatcherConfigSync = optionalString (cfg.dispatcher.configFile != null) ''
    install -m 400 ${lib.escapeShellArg cfg.dispatcher.configFile} ${lib.escapeShellArg dispatcherRuntimeConfig}
    chown ${cfg.user}:${cfg.group} ${lib.escapeShellArg dispatcherRuntimeConfig}
    chmod 400 ${lib.escapeShellArg dispatcherRuntimeConfig}
  '';
  autoscalerConfigSync = optionalString (cfg.autoscaler.configFile != null) ''
    install -m 400 ${lib.escapeShellArg cfg.autoscaler.configFile} ${lib.escapeShellArg autoscalerRuntimeConfig}
    chown ${cfg.user}:${cfg.group} ${lib.escapeShellArg autoscalerRuntimeConfig}
    chmod 400 ${lib.escapeShellArg autoscalerRuntimeConfig}
  '';

  dispatcherEnv =
    cfg.extraEnv
    // optionalAttrs (cfg.nscTokenFile != null) { NSC_TOKEN_FILE = nscTokenPath; }
    // optionalAttrs (cfg.nscTokenSpecFile != null) { NSC_TOKEN_SPEC_FILE = cfg.nscTokenSpecFile; }
    // optionalAttrs (cfg.nscEndpoint != null) { NSC_ENDPOINT = cfg.nscEndpoint; };
in {
  options.services.burrow.forgejoNsc = {
    enable = mkEnableOption "Forgejo Namespace Cloud runner dispatcher";

    user = mkOption {
      type = types.str;
      default = "forgejo-nsc";
      description = "System user that runs the forgejo-nsc services.";
    };

    group = mkOption {
      type = types.str;
      default = "forgejo-nsc";
      description = "System group for the forgejo-nsc services.";
    };

    stateDir = mkOption {
      type = types.str;
      default = "/var/lib/forgejo-nsc";
      description = "State directory for the dispatcher/autoscaler.";
    };

    nscTokenFile = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Optional NSC token file (exported as NSC_TOKEN_FILE).";
    };

    nscTokenSpecFile = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Optional NSC token spec file (exported as NSC_TOKEN_SPEC_FILE).";
    };

    nscEndpoint = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Optional NSC endpoint override (exported as NSC_ENDPOINT).";
    };

    extraEnv = mkOption {
      type = types.attrsOf types.str;
      default = { };
      description = "Extra environment variables injected into the services.";
    };

    nscPackage = mkOption {
      type = types.nullOr types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.nsc or null;
      description = "Optional nsc CLI package added to the service PATH.";
    };

    dispatcher = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable the forgejo-nsc dispatcher service.";
      };

      package = mkOption {
        type = types.package;
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.forgejo-nsc-dispatcher;
        description = "Package providing the forgejo-nsc dispatcher binary.";
      };

      configFile = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Host-local YAML config file for the dispatcher.";
      };

      allowPending = mkOption {
        type = types.bool;
        default = false;
        description = "Allow placeholder values (PENDING-) in the dispatcher config.";
      };
    };

    autoscaler = {
      enable = mkOption {
        type = types.bool;
        default = false;
        description = "Enable the forgejo-nsc autoscaler service.";
      };

      package = mkOption {
        type = types.package;
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.forgejo-nsc-autoscaler;
        description = "Package providing the forgejo-nsc autoscaler binary.";
      };

      configFile = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Host-local YAML config file for the autoscaler.";
      };

      allowPending = mkOption {
        type = types.bool;
        default = false;
        description = "Allow placeholder values (PENDING-) in the autoscaler config.";
      };
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = (!cfg.dispatcher.enable) || cfg.dispatcher.configFile != null;
        message = "services.burrow.forgejoNsc.dispatcher.configFile must be set when the dispatcher is enabled.";
      }
      {
        assertion = (!cfg.autoscaler.enable) || cfg.autoscaler.configFile != null;
        message = "services.burrow.forgejoNsc.autoscaler.configFile must be set when the autoscaler is enabled.";
      }
    ];

    users.groups.${cfg.group} = { };
    users.users.${cfg.user} = {
      uid = mkDefault 2011;
      isSystemUser = true;
      group = cfg.group;
      description = "Forgejo Namespace Cloud runner services";
      home = cfg.stateDir;
      createHome = true;
      shell = pkgs.bashInteractive;
    };

    systemd.tmpfiles.rules = mkAfter [
      "d ${cfg.stateDir} 0750 ${cfg.user} ${cfg.group} - -"
    ];

    systemd.services.forgejo-nsc-dispatcher = mkIf cfg.dispatcher.enable {
      description = "Forgejo Namespace Cloud dispatcher";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      unitConfig.ConditionPathExists =
        optional (cfg.dispatcher.configFile != null) cfg.dispatcher.configFile
        ++ optional (cfg.nscTokenFile != null) cfg.nscTokenFile;
      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = cfg.stateDir;
        ExecStart = "${cfg.dispatcher.package}/bin/forgejo-nsc-dispatcher --config ${dispatcherRuntimeConfig}";
        Restart = "on-failure";
        RestartSec = 5;
      };
      path = lib.optional (cfg.nscPackage != null) cfg.nscPackage;
      environment = dispatcherEnv;
      preStart = lib.concatStringsSep "\n" (lib.filter (s: s != "") [
        (optionalString (!cfg.dispatcher.allowPending) (pendingCheck cfg.dispatcher.configFile))
        dispatcherConfigSync
        tokenSync
      ]);
    };

    systemd.services.forgejo-nsc-autoscaler = mkIf cfg.autoscaler.enable {
      description = "Forgejo Namespace Cloud autoscaler";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" "forgejo-nsc-dispatcher.service" ];
      wants = [ "network-online.target" ];
      unitConfig.ConditionPathExists =
        optional (cfg.autoscaler.configFile != null) cfg.autoscaler.configFile
        ++ optional (cfg.nscTokenFile != null) cfg.nscTokenFile;
      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = cfg.stateDir;
        ExecStart = "${cfg.autoscaler.package}/bin/forgejo-nsc-autoscaler --config ${autoscalerRuntimeConfig}";
        Restart = "on-failure";
        RestartSec = 5;
      };
      path = lib.optional (cfg.nscPackage != null) cfg.nscPackage;
      environment = dispatcherEnv;
      preStart = lib.concatStringsSep "\n" (lib.filter (s: s != "") [
        (optionalString (!cfg.autoscaler.allowPending) (pendingCheck cfg.autoscaler.configFile))
        autoscalerConfigSync
        tokenSync
      ]);
    };
  };
}
