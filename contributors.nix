{
  groups = {
    users = "burrow-users";
    admins = "burrow-admins";
  };

  identities = {
    contact = {
      displayName = "Burrow";
      canonicalEmail = "contact@burrow.net";
      sourceEmail = "net.burrow@gmail.com";
      isAdmin = true;
      forgeAuthorized = true;
      bootstrapAuthentik = true;
      sshPublicKeyPath = ./nixos/keys/contact_at_burrow_net.pub;
      roles = [
        "operator"
        "forge-admin"
      ];
    };

    conrad = {
      displayName = "Conrad Kramer";
      canonicalEmail = "conrad@burrow.net";
      sourceEmail = "ckrames1234@gmail.com";
      isAdmin = true;
      forgeAuthorized = false;
      bootstrapAuthentik = true;
      roles = [
        "operator"
        "founder"
      ];
    };

    agent = {
      displayName = "Burrow Agent";
      canonicalEmail = "agent@burrow.net";
      isAdmin = false;
      forgeAuthorized = true;
      bootstrapAuthentik = false;
      sshPublicKeyPath = ./nixos/keys/agent_at_burrow_net.pub;
      roles = [
        "automation"
      ];
    };
  };
}
