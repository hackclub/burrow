#!/usr/bin/env python3

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import hmac
import sys
import textwrap
from pathlib import Path
from urllib.parse import urlencode, urlparse

import requests


def read_secret(path: str) -> str:
    value = Path(path).read_text(encoding="utf-8").strip()
    if not value:
        raise SystemExit(f"error: empty secret file: {path}")
    return value


def sign(key: bytes, msg: str) -> bytes:
    return hmac.new(key, msg.encode("utf-8"), hashlib.sha256).digest()


def request(
    *,
    method: str,
    endpoint: str,
    region: str,
    access_key: str,
    secret_key: str,
    bucket: str,
    query: dict[str, str] | None = None,
    body: bytes = b"",
    content_type: str | None = None,
) -> requests.Response:
    parsed = urlparse(endpoint)
    if parsed.scheme != "https":
        raise SystemExit("error: endpoint must use https")

    host = parsed.netloc
    canonical_uri = f"/{bucket}"
    query = query or {}
    canonical_querystring = urlencode(sorted(query.items()), doseq=True, safe="~")

    now = dt.datetime.now(dt.timezone.utc)
    amz_date = now.strftime("%Y%m%dT%H%M%SZ")
    date_stamp = now.strftime("%Y%m%d")
    payload_hash = hashlib.sha256(body).hexdigest()

    headers = {
        "host": host,
        "x-amz-content-sha256": payload_hash,
        "x-amz-date": amz_date,
    }
    if content_type:
        headers["content-type"] = content_type

    signed_headers = ";".join(sorted(headers.keys()))
    canonical_headers = "".join(f"{name}:{headers[name]}\n" for name in sorted(headers.keys()))
    canonical_request = "\n".join(
        [
            method,
            canonical_uri,
            canonical_querystring,
            canonical_headers,
            signed_headers,
            payload_hash,
        ]
    )

    algorithm = "AWS4-HMAC-SHA256"
    credential_scope = f"{date_stamp}/{region}/s3/aws4_request"
    string_to_sign = "\n".join(
        [
            algorithm,
            amz_date,
            credential_scope,
            hashlib.sha256(canonical_request.encode("utf-8")).hexdigest(),
        ]
    )

    k_date = sign(("AWS4" + secret_key).encode("utf-8"), date_stamp)
    k_region = sign(k_date, region)
    k_service = sign(k_region, "s3")
    signing_key = sign(k_service, "aws4_request")
    signature = hmac.new(signing_key, string_to_sign.encode("utf-8"), hashlib.sha256).hexdigest()

    auth_header = (
        f"{algorithm} Credential={access_key}/{credential_scope}, "
        f"SignedHeaders={signed_headers}, Signature={signature}"
    )

    url = f"{parsed.scheme}://{host}{canonical_uri}"
    if canonical_querystring:
      url = f"{url}?{canonical_querystring}"

    response = requests.request(
        method,
        url,
        headers={**headers, "Authorization": auth_header},
        data=body,
        timeout=30,
    )
    return response


def ensure_bucket(args: argparse.Namespace, bucket: str) -> None:
    head = request(
        method="HEAD",
        endpoint=args.endpoint,
        region=args.region,
        access_key=args.access_key,
        secret_key=args.secret_key,
        bucket=bucket,
    )
    if head.status_code == 200:
        print(f"{bucket}: exists")
        return
    if head.status_code != 404:
        raise SystemExit(f"error: HEAD {bucket} returned {head.status_code}: {head.text[:200]}")

    body = textwrap.dedent(
        f"""\
        <?xml version="1.0" encoding="UTF-8"?>
        <CreateBucketConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
          <LocationConstraint>{args.region}</LocationConstraint>
        </CreateBucketConfiguration>
        """
    ).encode("utf-8")
    create = request(
        method="PUT",
        endpoint=args.endpoint,
        region=args.region,
        access_key=args.access_key,
        secret_key=args.secret_key,
        bucket=bucket,
        body=body,
        content_type="application/xml",
    )
    if create.status_code not in (200, 204):
        raise SystemExit(f"error: PUT {bucket} returned {create.status_code}: {create.text[:200]}")
    print(f"{bucket}: created")


def put_lifecycle(args: argparse.Namespace, bucket: str) -> None:
    body = textwrap.dedent(
        f"""\
        <?xml version="1.0" encoding="UTF-8"?>
        <LifecycleConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
          <Rule>
            <ID>expire-forwardemail-backups-after-{args.expire_days}-days</ID>
            <Status>Enabled</Status>
            <Filter>
              <Prefix></Prefix>
            </Filter>
            <Expiration>
              <Days>{args.expire_days}</Days>
            </Expiration>
          </Rule>
        </LifecycleConfiguration>
        """
    ).encode("utf-8")
    response = request(
        method="PUT",
        endpoint=args.endpoint,
        region=args.region,
        access_key=args.access_key,
        secret_key=args.secret_key,
        bucket=bucket,
        query={"lifecycle": ""},
        body=body,
        content_type="application/xml",
    )
    if response.status_code not in (200, 204):
        raise SystemExit(
            f"error: PUT lifecycle for {bucket} returned {response.status_code}: {response.text[:200]}"
        )
    print(f"{bucket}: lifecycle set to {args.expire_days} days")


def get_lifecycle(args: argparse.Namespace, bucket: str) -> None:
    response = request(
        method="GET",
        endpoint=args.endpoint,
        region=args.region,
        access_key=args.access_key,
        secret_key=args.secret_key,
        bucket=bucket,
        query={"lifecycle": ""},
    )
    if response.status_code != 200:
        raise SystemExit(
            f"error: GET lifecycle for {bucket} returned {response.status_code}: {response.text[:200]}"
        )
    print(f"=== {bucket} lifecycle ===")
    print(response.text.strip())


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Provision Hetzner object-storage buckets for Forward Email backups."
    )
    parser.add_argument(
        "--endpoint",
        default="https://hel1.your-objectstorage.com",
        help="Public S3-compatible endpoint URL. For Hetzner, use the regional endpoint, not the account alias.",
    )
    parser.add_argument("--region", default="hel1", help="S3 region.")
    parser.add_argument(
        "--access-key-file",
        default="intake/hetzner-s3-user.txt",
        help="File containing the S3 access key id.",
    )
    parser.add_argument(
        "--secret-key-file",
        default="intake/hetzner-s3-secret.txt",
        help="File containing the S3 secret key.",
    )
    parser.add_argument(
        "--bucket",
        action="append",
        required=True,
        help="Bucket to provision. Repeat for multiple buckets.",
    )
    parser.add_argument(
        "--expire-days",
        type=int,
        default=90,
        help="Lifecycle expiry window in days.",
    )
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="Skip create/update and only read the current lifecycle.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.access_key = read_secret(args.access_key_file)
    args.secret_key = read_secret(args.secret_key_file)

    for bucket in args.bucket:
        if args.verify_only:
            get_lifecycle(args, bucket)
            continue
        ensure_bucket(args, bucket)
        put_lifecycle(args, bucket)
        get_lifecycle(args, bucket)


if __name__ == "__main__":
    try:
        main()
    except requests.RequestException as err:
        raise SystemExit(f"error: request failed: {err}") from err
