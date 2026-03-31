{ self, ... }:

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

  services.burrow.forge = {
    enable = true;
    adminPasswordFile = "/var/lib/burrow/intake/forgejo_pass_contact_at_burrow_net.txt";
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
    envFile = "/var/lib/burrow/intake/authentik.env";
    headscaleClientSecretFile = "/var/lib/burrow/intake/authentik_headscale_client_secret.txt";
  };

  services.burrow.headscale = {
    enable = true;
  };
}
