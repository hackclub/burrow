{ config, lib, pkgs, self, ... }:

let
  contributors = import ../../../contributors.nix;
  identities = contributors.identities;
  linearGroups = contributors.groups.linear;
  stripNewline = value: lib.replaceStrings [ "\n" ] [ "" ] value;
  authentikPasswordSecretPath = identity:
    if identity ? authentikPasswordSecret
    then config.age.secrets.${identity.authentikPasswordSecret}.path
    else null;
  bootstrapUsers = lib.mapAttrsToList
    (
      username: identity: {
        inherit username;
        name = identity.displayName;
        email = identity.canonicalEmail;
        isAdmin = identity.isAdmin or false;
        groups = lib.optionals (identity.isAdmin or false) [ linearGroups.owners ];
        passwordFile = authentikPasswordSecretPath identity;
      }
    )
    (lib.filterAttrs (_: identity: identity.bootstrapAuthentik or false) identities);
  headscaleBootstrapUsers = lib.mapAttrsToList
    (
      username: identity: {
        name = username;
        displayName = identity.displayName;
        email = identity.canonicalEmail;
      }
    )
    (lib.filterAttrs (_: identity: identity.bootstrapAuthentik or false) identities);
  forgeUnixUsernames =
    builtins.attrNames (lib.filterAttrs (_: identity: identity.forgeUnixUser or false) identities);
  forgeUnixUsers = lib.genAttrs forgeUnixUsernames (username:
    let
      identity = identities.${username};
      sshKeys = lib.optional (identity ? sshPublicKeyPath) (stripNewline (builtins.readFile identity.sshPublicKeyPath));
    in
    {
      isNormalUser = true;
      createHome = true;
      home = "/home/${username}";
      shell = pkgs.bashInteractive;
      extraGroups = lib.optional (identity.isAdmin or false) "wheel";
      openssh.authorizedKeys.keys = sshKeys;
    });
  forgeUnixAdminUsernames =
    builtins.attrNames (lib.filterAttrs (_: identity: (identity.forgeUnixUser or false) && (identity.isAdmin or false)) identities);
  forgeAuthorizedKeys = map
    (username: builtins.readFile identities.${username}.sshPublicKeyPath)
    (builtins.attrNames (lib.filterAttrs (_: identity: identity.forgeAuthorized or false) identities));
in

{
  imports = [
    ./hardware-configuration.nix
    ./disko-config.nix
    self.nixosModules.burrow-forge
    self.nixosModules.burrow-forge-runner
    self.nixosModules.burrow-forgejo-nsc
    self.nixosModules.burrow-authentik
    self.nixosModules.burrow-headscale
    self.nixosModules.burrow-zulip
  ];

  system.stateVersion = "24.11";

  time.timeZone = "America/Los_Angeles";

  nix.settings.experimental-features = [
    "nix-command"
    "flakes"
  ];

  users.users = forgeUnixUsers;

  security.sudo.extraRules = lib.map (username: {
    users = [ username ];
    commands = [
      {
        command = "ALL";
        options = [ "NOPASSWD" ];
      }
    ];
  }) forgeUnixAdminUsernames;

  environment.systemPackages = lib.optionals config.services.forgejo-nsc.enable [
    self.packages.${pkgs.stdenv.hostPlatform.system}.nsc
  ];

  age.identityPaths = [ "/var/lib/agenix/agenix.key" ];
  age.secrets.burrowAuthentikEnv = {
    file = ../../../secrets/infra/authentik.env.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };
  age.secrets.burrowHeadscaleOidcClientSecret = {
    file = ../../../secrets/infra/headscale-oidc-client-secret.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };
  age.secrets.burrowForgejoOidcClientSecret = {
    file = ../../../secrets/infra/forgejo-oidc-client-secret.age;
    owner = "forgejo";
    group = "forgejo";
    mode = "0440";
  };
  age.secrets.burrowTailscaleOidcClientSecret = {
    file = ../../../secrets/infra/tailscale-oidc-client-secret.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };
  age.secrets.burrowLinearScimToken = {
    file = ../../../secrets/infra/linear-scim-token.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };
  age.secrets.burrowAuthentikGoogleClientId = {
    file = ../../../secrets/infra/authentik-google-client-id.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };
  age.secrets.burrowAuthentikGoogleClientSecret = {
    file = ../../../secrets/infra/authentik-google-client-secret.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };
  age.secrets.burrowAuthentikGoogleAccountMap = {
    file = ../../../secrets/infra/authentik-google-account-map.json.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };
  age.secrets.burrowAuthentikUiTestPassword = {
    file = ../../../secrets/infra/authentik-ui-test-password.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };
  age.secrets.burrowForgejoNscToken = {
    file = ../../../secrets/infra/forgejo-nsc-token.age;
    owner = "forgejo-nsc";
    group = "forgejo-nsc";
    mode = "0400";
  };
  age.secrets.burrowForgejoNscDispatcherConfig = {
    file = ../../../secrets/infra/forgejo-nsc-dispatcher-config.age;
    owner = "forgejo-nsc";
    group = "forgejo-nsc";
    mode = "0400";
  };
  age.secrets.burrowForgejoNscAutoscalerConfig = {
    file = ../../../secrets/infra/forgejo-nsc-autoscaler-config.age;
    owner = "forgejo-nsc";
    group = "forgejo-nsc";
    mode = "0400";
  };

  age.secrets.burrowZulipPostgresPassword = {
    file = ../../../secrets/infra/zulip-postgres-password.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };

  age.secrets.burrowZulipRabbitmqPassword = {
    file = ../../../secrets/infra/zulip-rabbitmq-password.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };

  age.secrets.burrowZulipRedisPassword = {
    file = ../../../secrets/infra/zulip-redis-password.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };

  age.secrets.burrowZulipSecretKey = {
    file = ../../../secrets/infra/zulip-secret-key.age;
    owner = "root";
    group = "root";
    mode = "0400";
  };

  networking.extraHosts = ''
    127.0.0.1 burrow.net git.burrow.net auth.burrow.net ts.burrow.net chat.burrow.net nsc-autoscaler.burrow.net
    ::1 burrow.net git.burrow.net auth.burrow.net ts.burrow.net chat.burrow.net nsc-autoscaler.burrow.net
  '';

  services.burrow.forge = {
    enable = true;
    contactEmail = identities.contact.canonicalEmail;
    adminUsername = "contact";
    adminEmail = identities.contact.canonicalEmail;
    adminPasswordFile = "/var/lib/burrow/intake/forgejo_pass_contact_at_burrow_net.txt";
    oidcAdminGroup = contributors.groups.admins;
    oidcRestrictedGroup = contributors.groups.users;
    oidcClientSecretFile = config.age.secrets.burrowForgejoOidcClientSecret.path;
    authorizedKeys = forgeAuthorizedKeys;
  };

  services.burrow.forgeRunner = {
    enable = true;
    sshPrivateKeyFile = "/var/lib/burrow/intake/agent_at_burrow_net_ed25519";
    labels = [
      "self-hosted"
      "linux"
      "x86_64"
      "burrow-forge"
    ];
  };

  services.forgejo-nsc = {
    enable = true;
    nscTokenFile = config.age.secrets.burrowForgejoNscToken.path;
    dispatcher = {
      configFile = config.age.secrets.burrowForgejoNscDispatcherConfig.path;
    };
    autoscaler = {
      enable = true;
      configFile = config.age.secrets.burrowForgejoNscAutoscalerConfig.path;
    };
  };

  services.burrow.authentik = {
    enable = true;
    envFile = config.age.secrets.burrowAuthentikEnv.path;
    forgejoClientSecretFile = config.age.secrets.burrowForgejoOidcClientSecret.path;
    headscaleClientSecretFile = config.age.secrets.burrowHeadscaleOidcClientSecret.path;
    tailscaleClientSecretFile = config.age.secrets.burrowTailscaleOidcClientSecret.path;
    defaultExternalApplicationSlug = "tailscale";
    googleClientIDFile = config.age.secrets.burrowAuthentikGoogleClientId.path;
    googleClientSecretFile = config.age.secrets.burrowAuthentikGoogleClientSecret.path;
    googleAccountMapFile = config.age.secrets.burrowAuthentikGoogleAccountMap.path;
    googleLoginMode = "redirect";
    userGroupName = contributors.groups.users;
    adminGroupName = contributors.groups.admins;
    tailscaleAccessGroupName = contributors.groups.users;
    bootstrapUsers = bootstrapUsers;
    linearAcsUrl = "https://api.linear.app/auth/sso/d0ca13dc-ac41-4824-8aab-e0ca352fc3de/acs";
    linearAudience = "https://auth.linear.app/sso/d0ca13dc-ac41-4824-8aab-e0ca352fc3de";
    linearDefaultRelayState = "https://linear.app/auth/sso/d0ca13dc-ac41-4824-8aab-e0ca352fc3de";
    linearScimUrl = "https://api.linear.app/auth/scim/d0ca13dc-ac41-4824-8aab-e0ca352fc3de";
    linearScimTokenFile = config.age.secrets.burrowLinearScimToken.path;
    linearScimUserIdentifier = "email";
    linearOwnerGroupName = linearGroups.owners;
    linearAdminGroupName = linearGroups.admins;
    linearGuestGroupName = linearGroups.guests;
    zulipAccessGroupName = contributors.groups.users;
  };

  services.burrow.headscale = {
    enable = true;
    oidcClientSecretFile = config.age.secrets.burrowHeadscaleOidcClientSecret.path;
    bootstrapUsers = headscaleBootstrapUsers;
  };

  services.burrow.zulip = {
    enable = true;
    administratorEmail = identities.contact.canonicalEmail;
    postgresPasswordFile = config.age.secrets.burrowZulipPostgresPassword.path;
    rabbitmqPasswordFile = config.age.secrets.burrowZulipRabbitmqPassword.path;
    redisPasswordFile = config.age.secrets.burrowZulipRedisPassword.path;
    secretKeyFile = config.age.secrets.burrowZulipSecretKey.path;
  };
}
