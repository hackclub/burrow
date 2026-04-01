# Forward Email Backups

Burrow's mail direction is hosted mail on [Forward Email](https://forwardemail.net/), with domain-owned backup retention in our own S3-compatible object storage.

This is the first mail path to operationalize for `burrow.net` and `burrow.rs`. It keeps SMTP/IMAP hosting off the first forge host while still giving Burrow control over backup retention and object ownership.

## What Forward Email Requires

Forward Email exposes custom backup storage per domain. The documented API shape is:

- `PUT /v1/domains/{domain}` with:
  - `has_custom_s3=true`
  - `s3_endpoint`
  - `s3_access_key_id`
  - `s3_secret_access_key`
  - `s3_region`
  - `s3_bucket`
- `POST /v1/domains/{domain}/test-s3-connection`

Forward Email also documents these operational constraints:

- the bucket must remain private
- credentials are validated with `HeadBucket`
- failed or public-bucket configurations fall back to Forward Email's default storage and notify domain administrators
- custom S3 keeps every backup version, so lifecycle expiration is our responsibility

## Burrow Secret Layout

Present in `intake/` today:

- `intake/forwardemail_api_token.txt`
- `intake/hetzner-s3-user.txt`
- `intake/hetzner-s3-secret.txt`
- Hetzner public S3 endpoint for Forward Email: `https://hel1.your-objectstorage.com`
- Hetzner object storage region: `hel1`
- Hetzner bucket used for Forward Email backups: `burrow`

## Verified Storage State

As of March 15, 2026, Burrow's Forward Email custom S3 configuration is live:

- endpoint: `https://hel1.your-objectstorage.com`
- region: `hel1`
- bucket: `burrow`
- `burrow.net` has `has_custom_s3=true`
- `burrow.rs` has `has_custom_s3=true`
- Forward Email's `/test-s3-connection` succeeded for both domains
- the `burrow` bucket enforces lifecycle expiration after `90` days

Forward Email performs bucket validation with bucket-style addressing. For Hetzner Object Storage, this means the working endpoint is the regional S3 endpoint (`https://hel1.your-objectstorage.com`), not the account alias (`https://burrow.hel1.your-objectstorage.com`). Using the account alias causes TLS hostname mismatches when the vendor prepends the bucket name.

## Helper

Use [`Tools/forwardemail-custom-s3.sh`](../Tools/forwardemail-custom-s3.sh) to configure or retest the domain setting without putting secrets on the process list.

Use [`Tools/forwardemail-hetzner-storage.py`](../Tools/forwardemail-hetzner-storage.py) to ensure the Hetzner backup bucket exists and to apply lifecycle expiry before enabling custom S3 on the Forward Email side.

Bucket bootstrap example:

```sh
Tools/forwardemail-hetzner-storage.py \
  --endpoint https://hel1.your-objectstorage.com \
  --bucket burrow \
  --expire-days 90
```

Example:

```sh
Tools/forwardemail-custom-s3.sh \
  --domain burrow.net \
  --api-token-file intake/forwardemail_api_token.txt \
  --s3-endpoint https://hel1.your-objectstorage.com \
  --s3-region hel1 \
  --s3-bucket burrow \
  --s3-access-key-file intake/hetzner-s3-user.txt \
  --s3-secret-key-file intake/hetzner-s3-secret.txt
```

Retest an existing domain configuration without rewriting it:

```sh
Tools/forwardemail-custom-s3.sh \
  --domain burrow.net \
  --api-token-file intake/forwardemail_api_token.txt \
  --test-only
```

## Retention

Forward Email preserves every backup object when custom S3 is enabled. Configure lifecycle expiration on the bucket itself. A 30-day or 90-day expiry window is the baseline recommendation from the vendor docs; Burrow should choose explicitly per domain instead of letting the bucket grow without bound. The current Burrow bootstrap helper defaults to `90` days.

## Identity Direction

Hosted mail and SaaS identity are separate concerns:

- mail hosting/backups: Forward Email + Burrow-owned S3-compatible storage
- interactive identity: Authentik as the long-term IdP
- future SaaS SSO target: Linear via SAML once the workspace and plan are ready

This means the forge host does not need to become the first mail server just to give Burrow mailboxes or retention control.
