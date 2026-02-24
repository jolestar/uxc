#!/usr/bin/env bash

set -euo pipefail

EXPECTED_TAG="${1:-}"

usage() {
  cat <<'USAGE'
Run pre-release checks.

Usage:
  release-check.sh [vX.Y.Z]
  release-check.sh refs/tags/vX.Y.Z
USAGE
}

fail() {
  printf '[release-check] error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

main() {
  if [[ "${EXPECTED_TAG}" == "-h" || "${EXPECTED_TAG}" == "--help" ]]; then
    usage
    exit 0
  fi

  need_cmd cargo
  need_cmd git

  if ! git diff --quiet || ! git diff --cached --quiet; then
    fail "working tree is not clean"
  fi

  local version
  version="$(awk -F\" '
    /^\[package\]/ { in_pkg = 1; next }
    /^\[/ && !/^\[package\]/ { if (in_pkg) exit }
    in_pkg && /^version[[:space:]]*=/ { print $2; exit }
  ' Cargo.toml)"
  [[ -n "${version}" ]] || fail "failed to read version from Cargo.toml"

  if [[ -n "${EXPECTED_TAG}" ]]; then
    local normalized
    normalized="${EXPECTED_TAG#refs/tags/}"
    [[ "${normalized}" == "v${version}" ]] || fail "tag ${normalized} does not match Cargo.toml version v${version}"
  fi

  grep -Eq "^## \\[${version}\\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$" CHANGELOG.md \
    || fail "CHANGELOG.md does not contain dated release section for ${version}"

  cargo clippy -- -D warnings -A non_camel_case_types -A unused_variables -A unused_imports -A dead_code -A clippy::upper_case_acronyms -A clippy::enum_variant_names -A clippy::vec_init_then_push -A clippy::type_complexity
  cargo test --locked -- --test-threads=1

  printf '[release-check] OK: version=%s\n' "${version}"
}

main
