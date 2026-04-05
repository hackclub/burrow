#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

release_ref="${RELEASE_REF:-manual-${GITHUB_SHA:-unknown}}"
target="x86_64-unknown-linux-gnu"
out_dir="${repo_root}/dist"
staging="${out_dir}/burrow-${release_ref}-${target}"

mkdir -p "${staging}"

cargo build --locked --release -p burrow --bin burrow
install -m 0755 target/release/burrow "${staging}/burrow"
cp README.md "${staging}/README.md"

tarball="${out_dir}/burrow-${release_ref}-${target}.tar.gz"
tar -C "${out_dir}" -czf "${tarball}" "$(basename "${staging}")"
shasum -a 256 "${tarball}" > "${tarball}.sha256"
