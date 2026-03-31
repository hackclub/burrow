{ config, lib, pkgs, ... }:

let
  cfg = config.services.burrow.forgeRunner;
  runnerPkg = pkgs.forgejo-runner;
  stateDir = cfg.stateDir;
  runnerFile = "${stateDir}/.runner";
  configFile = "${stateDir}/runner.yaml";
  labelsCsv = lib.concatStringsSep "," (map (label: "${label}:host") cfg.labels);
  sshPrivateKeyFile = cfg.sshPrivateKeyFile or "";
in
{
  options.services.burrow.forgeRunner = {
    enable = lib.mkEnableOption "the Burrow Forgejo Actions runner";

    instanceUrl = lib.mkOption {
      type = lib.types.str;
      default = "http://127.0.0.1:3000";
      description = "Forgejo base URL used by the local runner for registration and job polling.";
    };

    labels = lib.mkOption {
      type = with lib.types; listOf str;
      default = [ "burrow-forge" ];
      description = "Runner labels exposed to Forgejo Actions.";
    };

    name = lib.mkOption {
      type = lib.types.str;
      default = "burrow-forge-agent";
      description = "Runner name shown in Forgejo.";
    };

    capacity = lib.mkOption {
      type = lib.types.int;
      default = 1;
      description = "Maximum concurrent jobs on this runner.";
    };

    stateDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/forgejo-runner-agent";
      description = "Persistent runner state directory.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "forgejo-runner-agent";
      description = "System user that runs the Forgejo runner.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "forgejo-runner-agent";
      description = "System group that runs the Forgejo runner.";
    };

    forgejoConfigFile = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/forgejo/custom/conf/app.ini";
      description = "Forgejo app.ini path used to generate runner tokens.";
    };

    gitUserName = lib.mkOption {
      type = lib.types.str;
      default = "agent";
      description = "Git commit author name for automation on the forge host.";
    };

    gitUserEmail = lib.mkOption {
      type = lib.types.str;
      default = "agent@burrow.net";
      description = "Git commit author email for automation on the forge host.";
    };

    sshPrivateKeyFile = lib.mkOption {
      type = with lib.types; nullOr str;
      default = null;
      description = "Optional host-local path to the agent SSH private key copied into the runner home.";
    };
  };

  config = lib.mkIf cfg.enable {
    users.groups.${cfg.group} = { };

    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      description = "Burrow Forgejo Actions runner";
      home = cfg.stateDir;
      createHome = true;
      shell = pkgs.bashInteractive;
    };

    environment.systemPackages = with pkgs; [
      runnerPkg
      bash
      coreutils
      findutils
      git
      git-lfs
      openssh
      python3
      rsync
    ];

    systemd.tmpfiles.rules = [
      "d ${stateDir} 0750 ${cfg.user} ${cfg.group} - -"
    ];

    systemd.services.burrow-forgejo-runner-bootstrap = {
      description = "Bootstrap Burrow Forgejo runner registration";
      after = [ "forgejo.service" "network-online.target" "systemd-tmpfiles-setup.service" ];
      wants = [ "forgejo.service" "network-online.target" "systemd-tmpfiles-setup.service" ];
      before = [ "burrow-forgejo-runner.service" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
      };
      script = ''
        set -euo pipefail
        umask 077

        install -d -m 0750 -o ${cfg.user} -g ${cfg.group} ${stateDir}
        cat > ${configFile} <<EOF
runner:
  file: ${runnerFile}
  capacity: ${toString cfg.capacity}
  name: ${cfg.name}
  labels:
EOF
        for label in ${lib.concatStringsSep " " cfg.labels}; do
          echo "  - ${"$"}label:host" >> ${configFile}
        done
        cat >> ${configFile} <<'EOF'
cache:
  enabled: false
EOF
        chown ${cfg.user}:${cfg.group} ${configFile}
        chmod 0640 ${configFile}

        install -d -m 0700 -o ${cfg.user} -g ${cfg.group} ${stateDir}/.ssh
        ${pkgs.util-linux}/bin/runuser -u ${cfg.user} -- \
          ${pkgs.git}/bin/git config --global user.name ${lib.escapeShellArg cfg.gitUserName}
        ${pkgs.util-linux}/bin/runuser -u ${cfg.user} -- \
          ${pkgs.git}/bin/git config --global user.email ${lib.escapeShellArg cfg.gitUserEmail}

        if [ -n ${lib.escapeShellArg sshPrivateKeyFile} ] && [ -s ${lib.escapeShellArg sshPrivateKeyFile} ]; then
          install -m 0600 -o ${cfg.user} -g ${cfg.group} \
            ${lib.escapeShellArg sshPrivateKeyFile} \
            ${stateDir}/.ssh/id_ed25519
          cat > ${stateDir}/.ssh/config <<EOF
Host *
  IdentityFile ${stateDir}/.ssh/id_ed25519
  IdentitiesOnly yes
  StrictHostKeyChecking accept-new
EOF
          chown ${cfg.user}:${cfg.group} ${stateDir}/.ssh/config
          chmod 0600 ${stateDir}/.ssh/config
        fi

        if [ ! -s ${runnerFile} ]; then
          token="$(${pkgs.util-linux}/bin/runuser -u forgejo -- \
            ${config.services.forgejo.package}/bin/forgejo actions generate-runner-token --config ${cfg.forgejoConfigFile} | tr -d '\r\n')"
          if [ -z "${"$"}token" ]; then
            echo "[burrow-forgejo-runner] failed to generate runner token" >&2
            exit 1
          fi

          ${pkgs.util-linux}/bin/runuser -u ${cfg.user} -- \
            ${runnerPkg}/bin/forgejo-runner register \
              --no-interactive \
              --instance ${lib.escapeShellArg cfg.instanceUrl} \
              --token "${"$"}token" \
              --name ${lib.escapeShellArg cfg.name} \
              --labels ${lib.escapeShellArg labelsCsv} \
              --config ${configFile}
        fi
      '';
    };

    systemd.services.burrow-forgejo-runner = {
      description = "Burrow Forgejo Actions runner";
      after = [ "burrow-forgejo-runner-bootstrap.service" ];
      wants = [ "burrow-forgejo-runner-bootstrap.service" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = stateDir;
        Restart = "on-failure";
        RestartSec = 2;
        ExecStart = pkgs.writeShellScript "burrow-forgejo-runner" ''
          set -euo pipefail
          export PATH="/run/wrappers/bin:/run/current-system/sw/bin:${"$"}{PATH:-}"
          tmp="$(${pkgs.coreutils}/bin/mktemp)"
          set +e
          ${runnerPkg}/bin/forgejo-runner daemon --config ${configFile} 2>&1 | ${pkgs.coreutils}/bin/tee "${"$"}tmp"
          rc="${"$"}{PIPESTATUS[0]}"
          set -e
          if ${pkgs.gnugrep}/bin/grep -qi "unregistered runner" "${"$"}tmp"; then
            rm -f ${runnerFile}
          fi
          rm -f "${"$"}tmp"
          exit "${"$"}rc"
        '';
      };
    };
  };
}
