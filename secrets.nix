let
  conradev = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBueQxNbP2246pxr/m7au4zNVm+ShC96xuOcfEcpIjWZ";
  contact = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIO42guJ5QvNMw3k6YKWlQnjcTsc+X4XI9F2GBtl8aHOa";
  agent = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEN0+tRJy7Y2DW0uGYHb86N2t02WyU5lDNX6FaxBF/G8 agent@burrow.net";
  burrowForgeHost = "age1quxf27gnun0xghlnxf3jrmqr3h3a3fzd8qxpallsaztd2u74pdfq9e7w9l";
  burrowForgeRecipients = [
    contact
    agent
    burrowForgeHost
  ];
  uiTestRecipients = burrowForgeRecipients ++ [ conradev ];
in
{
  "secrets/infra/authentik.env.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/authentik-google-client-id.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/authentik-google-client-secret.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/authentik-google-account-map.json.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/authentik-ui-test-password.age".publicKeys = uiTestRecipients;
  "secrets/infra/forgejo-oidc-client-secret.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/forgejo-nsc-autoscaler-config.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/forgejo-nsc-dispatcher-config.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/forgejo-nsc-token.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/headscale-oidc-client-secret.age".publicKeys = burrowForgeRecipients;
  "secrets/infra/tailscale-oidc-client-secret.age".publicKeys = burrowForgeRecipients;
}
