{ config, lib, pkgs, ... }:

let
  cfg = config.services.burrow.zulip;
  yamlFormat = pkgs.formats.yaml { };
  composeFile = yamlFormat.generate "burrow-zulip-compose.yaml" {
    services = {
      zulip = {
        image = "ghcr.io/zulip/zulip-server:11.6-1";
        restart = "unless-stopped";
        network_mode = "host";
        secrets = [
          "zulip__postgres_password"
          "zulip__rabbitmq_password"
          "zulip__redis_password"
          "zulip__secret_key"
          "zulip__email_password"
        ];
        environment = {
          SETTING_REMOTE_POSTGRES_HOST = "127.0.0.1";
          SETTING_MEMCACHED_LOCATION = "127.0.0.1:11211";
          SETTING_RABBITMQ_HOST = "127.0.0.1";
          SETTING_REDIS_HOST = "127.0.0.1";
        };
        volumes = [ "${cfg.dataDir}/data:/data:rw" ];
        ulimits.nofile = {
          soft = 1000000;
          hard = 1048576;
        };
      };
    };
  };
in
{
  options.services.burrow.zulip = {
    enable = lib.mkEnableOption "the Burrow Zulip deployment";

    domain = lib.mkOption {
      type = lib.types.str;
      default = "chat.burrow.net";
      description = "Public Zulip domain.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 18090;
      description = "Local loopback port Caddy should proxy to.";
    };

    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/burrow/zulip";
      description = "Host directory storing Zulip compose state and generated runtime files.";
    };

    administratorEmail = lib.mkOption {
      type = lib.types.str;
      default = "contact@burrow.net";
      description = "Operational Zulip administrator email.";
    };

    realmName = lib.mkOption {
      type = lib.types.str;
      default = "Burrow";
      description = "Initial Zulip organization name for single-tenant bootstrap.";
    };

    realmOwnerName = lib.mkOption {
      type = lib.types.str;
      default = "Burrow";
      description = "Display name used for the initial Zulip organization owner.";
    };

    authentikDomain = lib.mkOption {
      type = lib.types.str;
      default = config.services.burrow.authentik.domain;
      description = "Authentik domain Zulip should trust as its SAML IdP.";
    };

    authentikProviderSlug = lib.mkOption {
      type = lib.types.str;
      default = config.services.burrow.authentik.zulipProviderSlug;
      description = "Authentik SAML application slug used for Zulip.";
    };

    postgresPasswordFile = lib.mkOption {
      type = lib.types.str;
      description = "File containing the Zulip PostgreSQL password.";
    };

    rabbitmqPasswordFile = lib.mkOption {
      type = lib.types.str;
      description = "File containing the Zulip RabbitMQ password.";
    };

    redisPasswordFile = lib.mkOption {
      type = lib.types.str;
      description = "File containing the Zulip Redis password.";
    };

    secretKeyFile = lib.mkOption {
      type = lib.types.str;
      description = "File containing the Zulip Django secret key.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [
      pkgs.podman
      pkgs.podman-compose
    ];

    services.postgresql = {
      ensureDatabases = [ "zulip" ];
      ensureUsers = [
        {
          name = "zulip";
          ensureDBOwnership = true;
        }
      ];
      settings = {
        listen_addresses = lib.mkDefault "127.0.0.1";
        password_encryption = lib.mkDefault "scram-sha-256";
      };
      authentication = lib.mkAfter ''
        host zulip zulip 127.0.0.1/32 scram-sha-256
      '';
    };

    services.postgresqlBackup = {
      enable = true;
      backupAll = false;
      databases = [ "zulip" ];
    };

    services.memcached = {
      enable = true;
      listen = "127.0.0.1";
      port = 11211;
      extraOptions = [ "-U 0" ];
    };

    services.redis.servers.zulip = {
      enable = true;
      bind = "127.0.0.1";
      port = 6379;
      requirePassFile = cfg.redisPasswordFile;
    };

    services.rabbitmq = {
      enable = true;
      listenAddress = "127.0.0.1";
      port = 5672;
    };

    services.caddy.virtualHosts."${cfg.domain}".extraConfig = ''
      encode gzip zstd
      reverse_proxy 127.0.0.1:${toString cfg.port}
    '';

    systemd.tmpfiles.rules = [
      "d ${cfg.dataDir} 0755 root root - -"
      "d ${cfg.dataDir}/data 0755 root root - -"
      "d ${cfg.dataDir}/data/logs 0755 root root - -"
      "d ${cfg.dataDir}/data/logs/emails 0755 root root - -"
      "d ${cfg.dataDir}/data/secrets 0700 root root - -"
      "d ${cfg.dataDir}/secrets 0700 root root - -"
      "d ${cfg.dataDir}/logs 0755 root root - -"
    ];

    systemd.services.burrow-zulip-postgres-bootstrap = {
      description = "Bootstrap PostgreSQL role for Burrow Zulip";
      after = [ "postgresql.service" ];
      wants = [ "postgresql.service" ];
      requiredBy = [ "burrow-zulip.service" ];
      before = [ "burrow-zulip.service" ];
      path = [
        config.services.postgresql.package
        pkgs.bash
        pkgs.coreutils
        pkgs.python3
        pkgs.util-linux
      ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
      };
      script = ''
        set -euo pipefail

        db_password="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.postgresPasswordFile})"
        db_password_sql="$(printf '%s' "$db_password" | python3 -c "import sys; print(sys.stdin.read().replace(chr(39), chr(39) * 2), end=\"\")")"
        setup_sql="$(mktemp)"
        trap 'rm -f "$setup_sql"' EXIT

        cat > "$setup_sql" <<SQL
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'zulip') THEN
    CREATE ROLE zulip LOGIN;
  END IF;
END
\$\$;
ALTER ROLE zulip WITH LOGIN PASSWORD '$db_password_sql';
SQL
        chmod 0644 "$setup_sql"

        ${pkgs.util-linux}/bin/runuser -u postgres -- psql -v ON_ERROR_STOP=1 -f "$setup_sql"
      '';
    };

    systemd.services.burrow-zulip-rabbitmq-bootstrap = {
      description = "Bootstrap RabbitMQ user for Burrow Zulip";
      after = [ "rabbitmq.service" ];
      wants = [ "rabbitmq.service" ];
      requiredBy = [ "burrow-zulip.service" ];
      before = [ "burrow-zulip.service" ];
      path = [
        config.services.rabbitmq.package
        pkgs.bash
        pkgs.coreutils
        pkgs.gawk
        pkgs.gnugrep
      ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
      };
      script = ''
        set -euo pipefail

        rabbit_password="$(tr -d '\r\n' < ${lib.escapeShellArg cfg.rabbitmqPasswordFile})"
        export HOME=${config.services.rabbitmq.dataDir}

        rabbitmqctl await_startup

        if rabbitmqctl list_users -q | awk '{ print $1 }' | grep -qx zulip; then
          rabbitmqctl change_password zulip "$rabbit_password"
        else
          rabbitmqctl add_user zulip "$rabbit_password"
        fi

        rabbitmqctl set_permissions -p / zulip '.*' '.*' '.*'

        if rabbitmqctl list_users -q | awk '{ print $1 }' | grep -qx guest; then
          rabbitmqctl delete_user guest
        fi
      '';
    };

    systemd.services.burrow-zulip-runtime = {
      description = "Prepare Burrow Zulip compose and SAML runtime files";
      after = [
        "postgresql.service"
        "redis-zulip.service"
        "memcached.service"
        "rabbitmq.service"
        "burrow-zulip-postgres-bootstrap.service"
        "burrow-zulip-rabbitmq-bootstrap.service"
        "burrow-authentik-ready.service"
        "burrow-authentik-zulip-saml.service"
        "network-online.target"
      ];
      wants = [
        "postgresql.service"
        "redis-zulip.service"
        "memcached.service"
        "rabbitmq.service"
        "burrow-zulip-postgres-bootstrap.service"
        "burrow-zulip-rabbitmq-bootstrap.service"
        "burrow-authentik-ready.service"
        "burrow-authentik-zulip-saml.service"
        "network-online.target"
      ];
      requiredBy = [ "burrow-zulip.service" ];
      before = [ "burrow-zulip.service" ];
      path = [
        pkgs.bash
        pkgs.coreutils
        pkgs.curl
        pkgs.python3
      ];
      restartTriggers = [
        composeFile
        cfg.postgresPasswordFile
        cfg.rabbitmqPasswordFile
        cfg.redisPasswordFile
        cfg.secretKeyFile
      ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
      };
      script = ''
        set -euo pipefail

        install -d -m 0755 ${lib.escapeShellArg cfg.dataDir}
        install -d -m 0755 ${lib.escapeShellArg "${cfg.dataDir}/data"}
        install -d -m 0755 ${lib.escapeShellArg "${cfg.dataDir}/data/logs"}
        install -d -m 0755 ${lib.escapeShellArg "${cfg.dataDir}/data/logs/emails"}
        install -d -m 0700 ${lib.escapeShellArg "${cfg.dataDir}/data/secrets"}
        install -d -m 0700 ${lib.escapeShellArg "${cfg.dataDir}/secrets"}
        install -d -m 0755 ${lib.escapeShellArg "${cfg.dataDir}/logs"}
        install -m 0644 ${composeFile} ${lib.escapeShellArg "${cfg.dataDir}/compose.yaml"}
        : > ${lib.escapeShellArg "${cfg.dataDir}/secrets/email-password"}
        chmod 0600 ${lib.escapeShellArg "${cfg.dataDir}/secrets/email-password"}

        metadata_xml="$(${pkgs.curl}/bin/curl -fsSL https://${cfg.authentikDomain}/application/saml/${cfg.authentikProviderSlug}/metadata/)"
        saml_cert="$(printf '%s' "$metadata_xml" | ${pkgs.python3}/bin/python3 -c '
import xml.etree.ElementTree as ET, sys
xml = sys.stdin.read()
root = ET.fromstring(xml)
ns = {"ds": "http://www.w3.org/2000/09/xmldsig#"}
node = root.find(".//ds:X509Certificate", ns)
if node is None or not (node.text or "").strip():
    raise SystemExit("missing X509 certificate in Authentik metadata")
print((node.text or "").strip())
')"

        cat > ${lib.escapeShellArg "${cfg.dataDir}/compose.override.yaml"} <<EOF
secrets:
  zulip__postgres_password:
    file: ${cfg.postgresPasswordFile}
  zulip__rabbitmq_password:
    file: ${cfg.rabbitmqPasswordFile}
  zulip__redis_password:
    file: ${cfg.redisPasswordFile}
  zulip__secret_key:
    file: ${cfg.secretKeyFile}
  zulip__email_password:
    file: ${cfg.dataDir}/secrets/email-password

services:
  zulip:
    environment:
      SETTING_EXTERNAL_HOST: "${cfg.domain}"
      SETTING_ZULIP_ADMINISTRATOR: "${cfg.administratorEmail}"
      TRUST_GATEWAY_IP: "True"
      SETTING_SEND_LOGIN_EMAILS: "False"
      ZULIP_AUTH_BACKENDS: "EmailAuthBackend,SAMLAuthBackend"
      CONFIG_application_server__http_only: true
      CONFIG_application_server__nginx_listen_port: ${toString cfg.port}
      CONFIG_application_server__queue_workers_multiprocess: false
      ZULIP_CUSTOM_SETTINGS: |
        EMAIL_BACKEND = "django.core.mail.backends.filebased.EmailBackend"
        EMAIL_FILE_PATH = "/data/logs/emails"
        SOCIAL_AUTH_SAML_ORG_INFO = {
            "en-US": {
                "displayname": "Burrow Zulip",
                "name": "zulip",
                "url": "https://${cfg.domain}",
            },
        }
        SOCIAL_AUTH_SAML_ENABLED_IDPS = {
            "authentik": {
                "entity_id": "https://${cfg.authentikDomain}",
                "url": "https://${cfg.authentikDomain}/application/saml/${cfg.authentikProviderSlug}/sso/binding/redirect/",
                "display_name": "burrow.net",
                "x509cert": """$saml_cert""",
                "attr_user_permanent_id": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
                "attr_username": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
                "attr_email": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
                "attr_first_name": "firstName",
                "attr_last_name": "lastName",
            },
        }
EOF
      '';
    };

    systemd.services.burrow-zulip = {
      description = "Run Burrow Zulip with host-managed dependencies";
      after = [
        "burrow-zulip-runtime.service"
        "network-online.target"
      ];
      wants = [
        "burrow-zulip-runtime.service"
        "network-online.target"
      ];
      wantedBy = [ "multi-user.target" ];
      path = [
        pkgs.bash
        pkgs.coreutils
        pkgs.gawk
        pkgs.gnugrep
        pkgs.openssl
        pkgs.podman
        pkgs.podman-compose
      ];
      restartTriggers = [
        composeFile
        cfg.postgresPasswordFile
        cfg.rabbitmqPasswordFile
        cfg.redisPasswordFile
        cfg.secretKeyFile
      ];
      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
        WorkingDirectory = cfg.dataDir;
        RemainAfterExit = true;
        TimeoutStopSec = "20s";
        ExecStop = "${pkgs.bash}/bin/bash -lc 'set -euo pipefail; if ${pkgs.podman}/bin/podman container exists burrow-zulip_zulip_1; then ${pkgs.podman}/bin/podman stop --ignore --time 10 burrow-zulip_zulip_1 >/dev/null || true; ${pkgs.podman}/bin/podman rm -f --ignore burrow-zulip_zulip_1 >/dev/null || true; fi'";
      };
      script = ''
        set -euo pipefail
        cd ${lib.escapeShellArg cfg.dataDir}

        compose() {
          ${pkgs.podman-compose}/bin/podman-compose -p burrow-zulip "$@"
        }

        ensure_zulip_data_layout() {
          local zulip_data_dir=${lib.escapeShellArg "${cfg.dataDir}/data"}

          install -d -m 0755 "$zulip_data_dir/logs"
          install -d -m 0755 "$zulip_data_dir/logs/emails"
          install -d -m 0700 "$zulip_data_dir/secrets"
          chown 1000:1000 "$zulip_data_dir/logs" "$zulip_data_dir/logs/emails" "$zulip_data_dir/secrets"

          if [ ! -s "$zulip_data_dir/secrets/bootstrap-owner-password" ]; then
            umask 077
            openssl rand -base64 24 > "$zulip_data_dir/secrets/bootstrap-owner-password"
          fi
          chown 1000:1000 "$zulip_data_dir/secrets/bootstrap-owner-password"
          chmod 0600 "$zulip_data_dir/secrets/bootstrap-owner-password"
        }

        bootstrap_realm_if_needed() {
          local realm_exists
          local attempts=0
          while ! podman exec burrow-zulip_zulip_1 test -r /etc/zulip/zulip-secrets.conf >/dev/null 2>&1; do
            attempts=$((attempts + 1))
            if [ "$attempts" -ge 90 ]; then
              echo "error: Zulip did not finish generating production secrets" >&2
              exit 1
            fi
            sleep 2
          done

          realm_exists="$(
            podman exec burrow-zulip_zulip_1 bash -lc \
              "su zulip -c '/home/zulip/deployments/current/manage.py list_realms'" \
              | awk '$NF == "https://${cfg.domain}" { print "yes" }'
          )"

          if [ -n "$realm_exists" ]; then
            return 0
          fi

          local realm_name=${lib.escapeShellArg cfg.realmName}
          local admin_email=${lib.escapeShellArg cfg.administratorEmail}
          local owner_name=${lib.escapeShellArg cfg.realmOwnerName}
          local create_realm_cmd

          printf -v create_realm_cmd '%q ' \
            /home/zulip/deployments/current/manage.py \
            create_realm \
            --string-id= \
            --password-file /data/secrets/bootstrap-owner-password \
            --automated \
            "$realm_name" \
            "$admin_email" \
            "$owner_name"

          podman exec burrow-zulip_zulip_1 su zulip -c "$create_realm_cmd"
        }

        if [ ! -e .initialized ]; then
          compose pull
          compose run --rm -T zulip app:init
          touch .initialized
        fi

        ensure_zulip_data_layout
        compose up -d zulip
        bootstrap_realm_if_needed
      '';
    };
  };
}
