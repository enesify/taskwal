#!/usr/bin/env bash
# Install the `tw` binary from GitHub Releases (no Rust/Cargo required).
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/OWNER/REPO/BRANCH/scripts/install-tw.sh | bash
# Optional environment:
#   TASKWAL_REPO=owner/repo       (default: enesify/taskwal)
#   TASKWAL_VERSION=v0.1.0        (default: latest GitHub release)
#   TASKWAL_PREFIX=/usr/local     (install to $PREFIX/bin; default: ~/.local)

set -euo pipefail

TASKWAL_REPO="${TASKWAL_REPO:-enesify/taskwal}"
TASKWAL_VERSION="${TASKWAL_VERSION:-}"
TASKWAL_PREFIX="${TASKWAL_PREFIX:-}"

die() {
  echo "install-tw.sh: $*" >&2
  exit 1
}

detect_target_triple() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux)
      case "$arch" in
        x86_64) echo "x86_64-unknown-linux-gnu" ;;
        aarch64 | arm64) die "no prebuilt binary for Linux $arch yet; build from source with Rust" ;;
        *) die "unsupported Linux architecture: $arch" ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64) echo "x86_64-apple-darwin" ;;
        arm64) echo "aarch64-apple-darwin" ;;
        *) die "unsupported macOS architecture: $arch" ;;
      esac
      ;;
    *)
      die "unsupported OS: $os (install the Windows zip from Releases, or build from source)"
      ;;
  esac
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

download_to() {
  local url="$1" out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$out" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$out" "$url"
  else
    die "need curl or wget to download"
  fi
}

fetch_latest_tag() {
  local api_base="$1" json
  json="$(curl -fsSL "${api_base}/releases/latest")"
  if command -v python3 >/dev/null 2>&1; then
    printf '%s' "$json" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])"
    return
  fi
  require_cmd grep
  require_cmd sed
  printf '%s' "$json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
}

main() {
  require_cmd uname
  local triple
  triple="$(detect_target_triple)"

  local api_base="https://api.github.com/repos/${TASKWAL_REPO}"
  local tag asset_name tmpdir install_dir

  if [[ -n "$TASKWAL_VERSION" ]]; then
    tag="$TASKWAL_VERSION"
  else
    tag="$(fetch_latest_tag "$api_base")"
    [[ -n "$tag" ]] || die "could not determine latest release tag"
  fi

  asset_name="taskwal-${tag}-${triple}.tar.gz"

  if [[ -n "$TASKWAL_PREFIX" ]]; then
    install_dir="${TASKWAL_PREFIX%/}/bin"
  else
    install_dir="${HOME}/.local/bin"
  fi

  tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/taskwal-install.XXXXXX")"
  trap 'rm -rf "$tmpdir"' EXIT

  local sums_url="https://github.com/${TASKWAL_REPO}/releases/download/${tag}/SHA256SUMS"
  local asset_url="https://github.com/${TASKWAL_REPO}/releases/download/${tag}/${asset_name}"

  echo "Downloading ${asset_name} ..."
  download_to "$sums_url" "${tmpdir}/SHA256SUMS"
  download_to "$asset_url" "${tmpdir}/${asset_name}"

  (
    cd "$tmpdir"
    grep -F "${asset_name}" SHA256SUMS | sha256sum -c
    tar -xzf "${asset_name}"
    local inner="taskwal-${tag}-${triple}/tw"
    [[ -f "$inner" ]] || die "expected ${inner} inside archive"
    chmod +x "$inner"
    mkdir -p "$install_dir"
    mv "$inner" "${install_dir}/tw"
  )

  echo "Installed tw to ${install_dir}/tw"
  case ":${PATH}:" in
    *":${install_dir}:"*) ;;
    *)
      echo "Add ${install_dir} to your PATH, e.g.:"
      echo "  export PATH=\"${install_dir}:\$PATH\""
      ;;
  esac
}

main "$@"
