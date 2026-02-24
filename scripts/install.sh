#!/usr/bin/env bash

set -euo pipefail

REPO="jolestar/uxc"
INSTALL_DIR="/usr/local/bin"
VERIFY_CHECKSUMS=1
VERSION=""

usage() {
  cat <<'USAGE'
Install uxc from GitHub Releases.

Usage:
  install.sh [-v VERSION] [-d INSTALL_DIR] [--no-verify] [-h]

Options:
  -v, --version VERSION   Version to install (for example: 0.2.0 or v0.2.0)
  -d, --dir PATH          Install directory (default: /usr/local/bin)
      --no-verify         Skip SHA256 checksum verification
  -h, --help              Show this help
USAGE
}

log() {
  printf '[install] %s\n' "$*"
}

fail() {
  printf '[install] error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

resolve_latest_tag() {
  local api_url="https://api.github.com/repos/${REPO}/releases/latest"
  local tag

  tag="$(curl -fsSL "${api_url}" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
  [[ -n "${tag}" ]] || fail "failed to resolve latest release tag from ${api_url}"
  printf '%s' "${tag}"
}

normalize_tag() {
  local v="$1"
  if [[ "${v}" == v* ]]; then
    printf '%s' "${v}"
  else
    printf 'v%s' "${v}"
  fi
}

resolve_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Darwin)
      case "${arch}" in
        arm64|aarch64) printf 'aarch64-apple-darwin' ;;
        x86_64|amd64) printf 'x86_64-apple-darwin' ;;
        *) fail "unsupported macOS architecture: ${arch}" ;;
      esac
      ;;
    Linux)
      case "${arch}" in
        x86_64|amd64) printf 'x86_64-unknown-linux-musl' ;;
        arm64|aarch64) printf 'aarch64-unknown-linux-gnu' ;;
        *) fail "unsupported Linux architecture: ${arch}" ;;
      esac
      ;;
    *)
      fail "unsupported operating system: ${os} (supported: Linux, macOS)"
      ;;
  esac
}

sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${file}" | awk '{print $1}'
  else
    shasum -a 256 "${file}" | awk '{print $1}'
  fi
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -v|--version)
        [[ $# -ge 2 ]] || fail "missing value for $1"
        VERSION="$2"
        shift 2
        ;;
      -d|--dir)
        [[ $# -ge 2 ]] || fail "missing value for $1"
        INSTALL_DIR="$2"
        shift 2
        ;;
      --no-verify)
        VERIFY_CHECKSUMS=0
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        fail "unknown argument: $1"
        ;;
    esac
  done
}

main() {
  parse_args "$@"

  need_cmd curl
  need_cmd tar
  need_cmd install
  if [[ "${VERIFY_CHECKSUMS}" -eq 1 ]]; then
    if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
      fail "sha256sum or shasum is required for checksum verification"
    fi
  fi

  local tag version target base_url package_name checksums_name tmpdir package_path checksums_path

  if [[ -n "${VERSION}" ]]; then
    tag="$(normalize_tag "${VERSION}")"
  else
    tag="$(resolve_latest_tag)"
  fi

  version="${tag#v}"
  target="$(resolve_target)"
  base_url="https://github.com/${REPO}/releases/download/${tag}"
  package_name="uxc-v${version}-${target}.tar.gz"
  checksums_name="uxc-v${version}-checksums.txt"

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "${tmpdir}"' EXIT

  package_path="${tmpdir}/${package_name}"
  checksums_path="${tmpdir}/${checksums_name}"

  log "installing ${tag} for target ${target}"
  log "downloading ${package_name}"
  curl -fL "${base_url}/${package_name}" -o "${package_path}"

  if [[ "${VERIFY_CHECKSUMS}" -eq 1 ]]; then
    log "downloading ${checksums_name}"
    curl -fL "${base_url}/${checksums_name}" -o "${checksums_path}"

    local expected actual
    expected="$(grep " ${package_name}\$" "${checksums_path}" | awk '{print $1}')"
    [[ -n "${expected}" ]] || fail "failed to find checksum for ${package_name}"
    actual="$(sha256_file "${package_path}")"
    [[ "${actual}" == "${expected}" ]] || fail "checksum mismatch for ${package_name}"
    log "checksum verification passed"
  fi

  tar -xzf "${package_path}" -C "${tmpdir}"

  local extracted_dir binary_path
  extracted_dir="${tmpdir}/uxc-v${version}-${target}"
  binary_path="${extracted_dir}/uxc"

  [[ -x "${binary_path}" ]] || fail "binary not found in archive: ${binary_path}"
  mkdir -p "${INSTALL_DIR}"

  if install -m 0755 "${binary_path}" "${INSTALL_DIR}/uxc" 2>/dev/null; then
    log "installed to ${INSTALL_DIR}/uxc"
  else
    fail "cannot write to ${INSTALL_DIR}. re-run with sudo or use -d <writable_dir>"
  fi

  "${INSTALL_DIR}/uxc" --version
}

main "$@"
