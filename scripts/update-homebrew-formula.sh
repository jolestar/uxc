#!/usr/bin/env bash

set -euo pipefail

VERSION=""
DIST_DIR=""
REPO=""
TAP_REPO="holon-run/homebrew-tap"
TAP_BRANCH="main"

usage() {
  cat <<'USAGE'
Update Homebrew tap formula for uxc.

Usage:
  update-homebrew-formula.sh --version <x.y.z> --dist-dir <path> --repo <owner/repo> [--tap-repo <owner/repo>] [--tap-branch <branch>]

Environment:
  HOMEBREW_TAP_TOKEN  GitHub token with write access to tap repository
USAGE
}

fail() {
  printf '[brew-update] error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --version)
        VERSION="${2:-}"
        shift 2
        ;;
      --dist-dir)
        DIST_DIR="${2:-}"
        shift 2
        ;;
      --repo)
        REPO="${2:-}"
        shift 2
        ;;
      --tap-repo)
        TAP_REPO="${2:-}"
        shift 2
        ;;
      --tap-branch)
        TAP_BRANCH="${2:-}"
        shift 2
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

checksum_for() {
  local file="$1"
  local path="${DIST_DIR}/${file}"
  [[ -f "${path}" ]] || fail "artifact not found: ${path}"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${path}" | awk '{print $1}'
  else
    shasum -a 256 "${path}" | awk '{print $1}'
  fi
}

render_formula() {
  local version="$1"
  local repo="$2"
  local mac_arm_sha="$3"
  local mac_x64_sha="$4"
  local linux_arm_sha="$5"
  local linux_x64_sha="$6"
  local release_base="https://github.com/${repo}/releases/download/v${version}"

  cat <<EOF
class Uxc < Formula
  desc "Universal X-Protocol Call"
  homepage "https://github.com/${repo}"
  license "MIT"
  version "${version}"

  if OS.mac?
    if Hardware::CPU.arm?
      url "${release_base}/uxc-v${version}-aarch64-apple-darwin.tar.gz"
      sha256 "${mac_arm_sha}"
    else
      url "${release_base}/uxc-v${version}-x86_64-apple-darwin.tar.gz"
      sha256 "${mac_x64_sha}"
    end
  elsif OS.linux?
    if Hardware::CPU.arm?
      url "${release_base}/uxc-v${version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "${linux_arm_sha}"
    else
      url "${release_base}/uxc-v${version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "${linux_x64_sha}"
    end
  end

  def install
    bin.install "uxc"
  end

  test do
    output = shell_output("#{bin}/uxc --version")
    assert_match version.to_s, output
  end
end
EOF
}

main() {
  parse_args "$@"

  need_cmd git
  need_cmd mktemp
  if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
    fail "sha256sum or shasum is required"
  fi

  [[ -n "${VERSION}" ]] || fail "--version is required"
  if ! [[ "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    fail "--version must match x.y.z (numeric semantic version, e.g. 1.2.3)"
  fi
  [[ -n "${DIST_DIR}" ]] || fail "--dist-dir is required"
  [[ -n "${REPO}" ]] || fail "--repo is required"
  [[ -d "${DIST_DIR}" ]] || fail "dist dir not found: ${DIST_DIR}"
  local tap_token="${HOMEBREW_TAP_TOKEN:-${HOMEBREW_TAP_GITHUB_TOKEN:-}}"
  [[ -n "${tap_token}" ]] || fail "HOMEBREW_TAP_TOKEN is not set"

  local mac_arm_sha mac_x64_sha linux_arm_sha linux_x64_sha
  mac_arm_sha="$(checksum_for "uxc-v${VERSION}-aarch64-apple-darwin.tar.gz")"
  mac_x64_sha="$(checksum_for "uxc-v${VERSION}-x86_64-apple-darwin.tar.gz")"
  linux_arm_sha="$(checksum_for "uxc-v${VERSION}-aarch64-unknown-linux-gnu.tar.gz")"
  linux_x64_sha="$(checksum_for "uxc-v${VERSION}-x86_64-unknown-linux-musl.tar.gz")"

  local workdir
  workdir="$(mktemp -d)"
  trap 'rm -rf "${workdir}"' EXIT

  local askpass_script clone_url
  askpass_script="${workdir}/git-askpass.sh"
  cat > "${askpass_script}" <<'EOF'
#!/usr/bin/env bash
case "$1" in
  *Username*)
    echo "x-access-token"
    ;;
  *Password*)
    echo "${HOMEBREW_TAP_TOKEN:-${HOMEBREW_TAP_GITHUB_TOKEN:-}}"
    ;;
esac
EOF
  chmod 700 "${askpass_script}"

  clone_url="https://github.com/${TAP_REPO}.git"
  GIT_TERMINAL_PROMPT=0 GIT_ASKPASS="${askpass_script}" \
    git clone --quiet --depth 1 --branch "${TAP_BRANCH}" "${clone_url}" "${workdir}/tap"
  mkdir -p "${workdir}/tap/Formula"
  render_formula "${VERSION}" "${REPO}" "${mac_arm_sha}" "${mac_x64_sha}" "${linux_arm_sha}" "${linux_x64_sha}" > "${workdir}/tap/Formula/uxc.rb"

  pushd "${workdir}/tap" >/dev/null
  git config user.name "uxc-release-bot"
  git config user.email "uxc-release-bot@users.noreply.github.com"

  if git diff --quiet -- Formula/uxc.rb; then
    printf '[brew-update] no formula changes, skipping push\n'
    popd >/dev/null
    exit 0
  fi

  git add Formula/uxc.rb
  git commit -m "uxc ${VERSION}"
  git push origin "${TAP_BRANCH}"
  popd >/dev/null

  printf '[brew-update] updated %s Formula/uxc.rb to %s\n' "${TAP_REPO}" "${VERSION}"
}

main "$@"
