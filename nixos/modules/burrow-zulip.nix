{ config, lib, pkgs, ... }:

let
  cfg = config.services.burrow.zulip;
  yamlFormat = pkgs.formats.yaml { };
  composeFile = yamlFormat.generate "burrow-zulip-compose.yaml" {
    services = {
      database = {
        image = "zulip/zulip-postgresql:14";
        restart = "unless-stopped";
        secrets = [ "zulip__postgres_password" ];
        environment = {
          POSTGRES_DB = "zulip";
          POSTGRES_USER = "zulip";
          POSTGRES_PASSWORD_FILE = "/run/secrets/zulip__postgres_password";
        };
        volumes = [ "postgresql-14:/var/lib/postgresql/data:rw" ];
        attach = false;
      };
      memcached = {
        image = "memcached:alpine";
        restart = "unless-stopped";
        command = [
          "sh"
          "-euc"
          ''
            echo 'mech_list: plain' > "$SASL_CONF_PATH"
            echo "zulip@$HOSTNAME:$(cat $MEMCACHED_PASSWORD_FILE)" > "$MEMCACHED_SASL_PWDB"
            echo "zulip@localhost:$(cat $MEMCACHED_PASSWORD_FILE)" >> "$MEMCACHED_SASL_PWDB"
            exec memcached -S
          ''
        ];
        secrets = [ "zulip__memcached_password" ];
        environment = {
          SASL_CONF_PATH = "/home/memcache/memcached.conf";
          MEMCACHED_SASL_PWDB = "/home/memcache/memcached-sasl-db";
          MEMCACHED_PASSWORD_FILE = "/run/secrets/zulip__memcached_password";
        };
        attach = false;
      };
      rabbitmq = {
        image = "rabbitmq:4.2";
        restart = "unless-stopped";
        secrets = [ "zulip__rabbitmq_password" ];
        environment = {
          RABBITMQ_DEFAULT_USER = "zulip";
          RABBITMQ_DEFAULT_PASS_FILE = "/run/secrets/zulip__rabbitmq_password";
        };
        volumes = [ "rabbitmq:/var/lib/rabbitmq:rw" ];
        attach = false;
      };
      redis = {
        image = "redis:alpine";
        restart = "unless-stopped";
        command = [
          "sh"
          "-euc"
          "/usr/local/bin/docker-entrypoint.sh --requirepass \"$(cat \"$REDIS_PASSWORD_FILE\")\""
        ];
        secrets = [ "zulip__redis_password" ];
        environment = {
          REDIS_PASSWORD_FILE = "/run/secrets/zulip__redis_password";
        };
        volumes = [ "redis:/data:rw" ];
        attach = false;
      };
      zulip = {
        image = "ghcr.io/zulip/zulip-server:11.6-1";
        restart = "unless-stopped";
        secrets = [
          "zulip__postgres_password"
          "zulip__memcached_password"
          "zulip__rabbitmq_password"
          "zulip__redis_password"
          "zulip__secret_key"
          "zulip__email_password"
        ];
        environment = {
          SETTING_REMOTE_POSTGRES_HOST = "database";
          SETTING_MEMCACHED_LOCATION = "memcached:11211";
          SETTING_RABBITMQ_HOST = "rabbitmq";
          SETTING_REDIS_HOST = "redis";
        };
        volumes = [ "zulip:/data:rw" ];
        ulimits.nofile = {
          soft = 1000000;
          hard = 1048576;
        };
        depends_on = [
          "database"
          "memcached"
          "rabbitmq"
          "redis"
        ];
      };
    };

    volumes = {
      zulip = { };
      postgresql-14 = { };
      rabbitmq = { };
      redis = { };
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

    memcachedPasswordFile = lib.mkOption {
      type = lib.types.str;
      description = "File containing the Zulip memcached password.";
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

    services.caddy.virtualHosts."${cfg.domain}".extraConfig = ''
      encode gzip zstd
      reverse_proxy 127.0.0.1:${toString cfg.port}
    '';

    systemd.tmpfiles.rules = [
      "d ${cfg.dataDir} 0755 root root - -"
      "d ${cfg.dataDir}/secrets 0700 root root - -"
      "d ${cfg.dataDir}/logs 0755 root root - -"
    ];

    systemd.services.burrow-zulip-runtime = {
      description = "Prepare Burrow Zulip compose and SAML runtime files";
      after = [
        "burrow-authentik-ready.service"
        "burrow-authentik-zulip-saml.service"
        "network-online.target"
      ];
      wants = [
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
        cfg.memcachedPasswordFile
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
        install -d -m 0700 ${lib.escapeShellArg "${cfg.dataDir}/secrets"}
        install -d -m 0755 ${lib.escapeShellArg "${cfg.dataDir}/logs"}
        install -m 0644 ${composeFile} ${lib.escapeShellArg "${cfg.dataDir}/compose.yaml"}
        : > ${lib.escapeShellArg "${cfg.dataDir}/secrets/email-password"}
        chmod 0600 ${lib.escapeShellArg "${cfg.dataDir}/secrets/email-password"}

        metadata_xml="$(${pkgs.curl}/bin/curl -fsSL https://${cfg.authentikDomain}/application/saml/${cfg.authentikProviderSlug}/metadata/)"
        saml_cert="$(printf '%s' "$metadata_xml" | ${pkgs.python3}/bin/python3 -c '
import re, sys, xml.etree.ElementTree as ET
xml = sys.stdin.read()
root = ET.fromstring(xml)
ns = {"md": "urn:oasis:names:tc:SAML:2.0:metadata", "ds": "http://www.w3.org/2000/09/xmldsig#"}
node = root.find(".//ds:X509Certificate", ns)
if node is None or not (node.text or "").strip():
    raise SystemExit("missing X509 certificate in Authentik metadata")
print((node.text or "").strip())
')"

        cat > ${lib.escapeShellArg "${cfg.dataDir}/compose.override.yaml"} <<EOF
secrets:
  zulip__postgres_password:
    file: ${cfg.postgresPasswordFile}
  zulip__memcached_password:
    file: ${cfg.memcachedPasswordFile}
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
    ports:
      - "127.0.0.1:${toString cfg.port}:80"
    environment:
      SETTING_EXTERNAL_HOST: "${cfg.domain}"
      SETTING_ZULIP_ADMINISTRATOR: "${cfg.administratorEmail}"
      TRUST_GATEWAY_IP: "True"
      SETTING_SEND_LOGIN_EMAILS: "False"
      ZULIP_AUTH_BACKENDS: "EmailAuthBackend,SAMLAuthBackend"
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
      CONFIG_application_server__queue_workers_multiprocess: false
EOF
      '';
    };

    systemd.services.burrow-zulip = {
      description = "Run Burrow Zulip via the official compose topology";
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
        pkgs.podman
        pkgs.podman-compose
      ];
      restartTriggers = [
        composeFile
        cfg.postgresPasswordFile
        cfg.memcachedPasswordFile
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
        ExecStop = "${pkgs.bash}/bin/bash -lc 'cd ${lib.escapeShellArg cfg.dataDir} && ${pkgs.podman-compose}/bin/podman-compose -p burrow-zulip down'";
      };
      script = ''
        set -euo pipefail
        cd ${lib.escapeShellArg cfg.dataDir}

        if [ ! -e .initialized ]; then
          ${pkgs.podman-compose}/bin/podman-compose -p burrow-zulip pull
          ${pkgs.podman-compose}/bin/podman-compose -p burrow-zulip run --rm zulip app:init
          touch .initialized
        fi

        ${pkgs.podman-compose}/bin/podman-compose -p burrow-zulip up -d
      '';
    };
  };
}
