# `BEP-0004` - Hosted Mail Backups and SaaS Identity

```text
Status: Draft
Proposal: BEP-0004
Authors: gpt-5.4
Coordinator: gpt-5.4
Reviewers: Pending
Constitution Sections: II, III, V
Implementation PRs: Pending
Decision Date: Pending
```

## Summary

Burrow should start with hosted mail on Forward Email instead of self-hosting SMTP and IMAP on the first forge machine. Backup retention should still be controlled by Burrow through custom S3-compatible storage backed by Burrow-owned object storage. In parallel, Burrow should treat SaaS identity as a separate track and converge on Authentik as the long-term IdP, with Linear SAML SSO as a planned downstream integration rather than an immediate bootstrap dependency.

## Motivation

- The first forge host already carries source control, CI, and deployment bootstrap risk. Adding a self-hosted mail stack increases operational scope before the forge is stable.
- Forward Email already exposes SMTP and IMAP while allowing per-domain custom S3 backup storage, which preserves Burrow's data ownership over mailbox backups.
- The repository needs a durable decision record that separates hosted mail operations from future SaaS SSO work.

## Detailed Design

- Use Forward Email as the operational mail provider for `burrow.net` and `burrow.rs`.
- Configure custom S3-compatible storage per domain using Burrow-controlled object storage credentials.
- Keep one backup bucket per domain and enforce lifecycle expiration at the bucket layer.
- Add repository-owned tooling and documentation for applying and testing the Forward Email custom S3 configuration.
- Treat Authentik as the future identity authority for SaaS applications, but keep Linear SAML as a later rollout once the workspace and vendor prerequisites are available. Linear's current docs place SAML and SCIM behind higher-tier workspace security settings, so Burrow should treat plan availability as an explicit precondition.

## Security and Operational Considerations

- Forward Email API tokens and S3 credentials must stay in secret files and must not be passed directly on the shell command line.
- Buckets must remain private. Public bucket detection by the vendor should be treated as a hard failure, not a warning.
- Backup growth is unbounded without lifecycle rules. Retention policy is part of the rollout, not optional cleanup.
- Hosted mail reduces MTA attack surface on the forge host, but it adds third-party dependency risk; keeping backups in Burrow-owned storage limits that blast radius.

## Contributor Playbook

- Put the Forward Email API token in `intake/forwardemail_api_token.txt`.
- Use `Tools/forwardemail-custom-s3.sh` to configure `burrow.net` and `burrow.rs`.
- Run the helper again with `--test-only` after any credential rotation.
- Record the chosen endpoint, region, bucket names, and lifecycle policy alongside rollout evidence.
- Do not claim Linear SAML is live until the Authentik app, Linear workspace settings, workspace plan prerequisites, and end-to-end login flow are verified.

## Alternatives Considered

- Self-host Stalwart on the forge host immediately. Rejected for the first rollout because it expands host scope before source control and CI are stable.
- Rely on Forward Email default backup storage only. Rejected because it gives Burrow less control over retention and data location.
- Delay all SaaS identity planning until after forge cutover. Rejected because Linear and other SaaS integrations will otherwise accrete without an agreed authority.

## Impact on Other Work

- Narrows the first forge host scope.
- Creates a clean mail path for `contact@burrow.net` without requiring self-hosted SMTP and IMAP.
- Leaves Authentik and Linear SAML as explicit follow-up work instead of hidden assumptions.

## Decision

Pending.

## References

- `docs/FORWARDEMAIL.md`
- `Tools/forwardemail-custom-s3.sh`
- Forward Email FAQ: custom S3-compatible storage for backups
- Linear docs: SAML SSO
