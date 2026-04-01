{ config, self, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./disko-config.nix
    self.nixosModules.burrow-forge
    self.nixosModules.burrow-forge-runner
    self.nixosModules.burrow-forgejo-nsc
    self.nixosModules.burrow-authentik
    self.nixosModules.burrow-headscale
  ];

  system.stateVersion = "24.11";

  time.timeZone = "America/Los_Angeles";

  nix.settings.experimental-features = [
    "nix-command"
    "flakes"
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

  networking.extraHosts = ''
    127.0.0.1 burrow.net git.burrow.net auth.burrow.net ts.burrow.net nsc-autoscaler.burrow.net
    ::1 burrow.net git.burrow.net auth.burrow.net ts.burrow.net nsc-autoscaler.burrow.net
  '';

  services.burrow.forge = {
    enable = true;
    adminPasswordFile = "/var/lib/burrow/intake/forgejo_pass_contact_at_burrow_net.txt";
    oidcClientSecretFile = config.age.secrets.burrowForgejoOidcClientSecret.path;
    authorizedKeys = [
      (builtins.readFile ../../keys/contact_at_burrow_net.pub)
      (builtins.readFile ../../keys/agent_at_burrow_net.pub)
    ];
  };

  services.burrow.forgeRunner = {
    enable = true;
    sshPrivateKeyFile = "/var/lib/burrow/intake/agent_at_burrow_net_ed25519";
  };

  services.burrow.forgejoNsc = {
    enable = true;
    nscTokenFile = "/var/lib/burrow/intake/forgejo_nsc_token.txt";
    dispatcher = {
      configFile = "/var/lib/burrow/intake/forgejo_nsc_dispatcher.yaml";
    };
    autoscaler = {
      enable = true;
      configFile = "/var/lib/burrow/intake/forgejo_nsc_autoscaler.yaml";
    };
  };

  services.burrow.authentik = {
    enable = true;
    envFile = config.age.secrets.burrowAuthentikEnv.path;
    forgejoClientSecretFile = config.age.secrets.burrowForgejoOidcClientSecret.path;
    headscaleClientSecretFile = config.age.secrets.burrowHeadscaleOidcClientSecret.path;
    googleClientIDFile = config.age.secrets.burrowAuthentikGoogleClientId.path;
    googleClientSecretFile = config.age.secrets.burrowAuthentikGoogleClientSecret.path;
  };

  services.burrow.headscale = {
    enable = true;
    oidcClientSecretFile = config.age.secrets.burrowHeadscaleOidcClientSecret.path;
  };
}
