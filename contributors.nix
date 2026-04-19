{
  groups = {
    users = "burrow-users";
    admins = "burrow-admins";
    linear = {
      owners = "linear-owners";
      admins = "linear-admins";
      guests = "linear-guests";
    };
  };

  identities = {
    contact = {
      displayName = "Burrow";
      canonicalEmail = "contact@burrow.net";
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
      isAdmin = true;
      forgeAuthorized = false;
      bootstrapAuthentik = true;
      roles = [
        "operator"
        "founder"
      ];
    };

    jett = {
      displayName = "Jett";
      canonicalEmail = "jett@burrow.net";
      isAdmin = true;
      forgeAuthorized = false;
      forgeUnixUser = true;
      bootstrapAuthentik = true;
      sshPublicKeyPath = ./nixos/keys/jett_at_burrow_net.pub;
      roles = [
        "member"
        "operator"
        "forge-admin"
      ];
    };

    davnotdev = {
      displayName = "David";
      canonicalEmail = "davnotdev@burrow.net";
      isAdmin = true;
      forgeAuthorized = false;
      bootstrapAuthentik = true;
      roles = [
        "member"
        "operator"
        "forge-admin"
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

    ui-test = {
      displayName = "Burrow UI Test";
      canonicalEmail = "ui-test@burrow.net";
      isAdmin = false;
      forgeAuthorized = false;
      bootstrapAuthentik = true;
      authentikPasswordSecret = "burrowAuthentikUiTestPassword";
      roles = [
        "testing"
        "apple-ui"
      ];
    };
  };
}
