#!/bin/sh
set -eu

REPO="saltyskip/rift"
BINARY="rift"

# --- Helpers ----------------------------------------------------------------

say() {
  printf "  %s\n" "$@"
}

err() {
  printf "  \033[31merror:\033[0m %s\n" "$@" >&2
  exit 1
}

bold() {
  printf "\033[1m%s\033[0m" "$1"
}

# --- Detect platform --------------------------------------------------------

detect_platform() {
  OS="$(uname -s)"
  ARCH="$(uname -m)"

  case "$OS" in
    Linux)  OS="linux" ;;
    Darwin) OS="darwin" ;;
    *)      err "Unsupported OS: $OS. Download manually from GitHub Releases." ;;
  esac

  case "$ARCH" in
    x86_64|amd64)   ARCH="x86_64" ;;
    arm64|aarch64)   ARCH="arm64" ;;
    *)               err "Unsupported architecture: $ARCH" ;;
  esac

  # macOS Intel: use ARM binary via Rosetta 2
  if [ "$OS" = "darwin" ] && [ "$ARCH" = "x86_64" ]; then
    say "macOS Intel detected — downloading ARM binary (runs via Rosetta 2)"
    ARCH="arm64"
  fi

  # Map to release label
  case "${OS}-${ARCH}" in
    linux-x86_64) LABEL="linux-x86_64" ;;
    linux-arm64)  LABEL="linux-arm64" ;;
    darwin-arm64) LABEL="macos-arm64" ;;
    *)            err "No prebuilt binary for ${OS}-${ARCH}" ;;
  esac
}

# --- Resolve version --------------------------------------------------------

get_version() {
  if [ -n "${RIFT_VERSION:-}" ]; then
    VERSION="$RIFT_VERSION"
    return
  fi

  say "Fetching latest version..."

  VERSION=$(
    curl -fsSL "https://api.github.com/repos/${REPO}/releases" \
      | grep -o '"tag_name": *"rift-cli-v[^"]*"' \
      | head -1 \
      | sed 's/.*"rift-cli-v\([^"]*\)"/\1/'
  ) || true

  if [ -z "$VERSION" ]; then
    err "Could not determine latest version. Set RIFT_VERSION to install a specific version."
  fi
}

# --- Install ----------------------------------------------------------------

install() {
  TAG="rift-cli-v${VERSION}"
  ARCHIVE="rift-${VERSION}-${LABEL}.tar.gz"
  URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"

  TMPDIR="$(mktemp -d)"
  trap 'rm -rf "$TMPDIR"' EXIT

  say "Downloading $(bold "rift ${VERSION}") for ${LABEL}..."
  curl -fsSL "$URL" -o "${TMPDIR}/${ARCHIVE}" \
    || err "Download failed. Check that version ${VERSION} exists at:\n  ${URL}"

  tar -xzf "${TMPDIR}/${ARCHIVE}" -C "$TMPDIR"

  # Find the binary inside the extracted directory
  BIN="${TMPDIR}/rift-${VERSION}-${LABEL}/${BINARY}"
  if [ ! -f "$BIN" ]; then
    err "Binary not found in archive"
  fi

  # Determine install directory
  INSTALL_DIR="${RIFT_INSTALL_DIR:-}"
  if [ -z "$INSTALL_DIR" ]; then
    if [ -w "/usr/local/bin" ]; then
      INSTALL_DIR="/usr/local/bin"
    else
      INSTALL_DIR="${HOME}/.rift/bin"
    fi
  fi

  mkdir -p "$INSTALL_DIR"
  cp "$BIN" "${INSTALL_DIR}/${BINARY}"
  chmod +x "${INSTALL_DIR}/${BINARY}"

  say "Installed to $(bold "${INSTALL_DIR}/${BINARY}")"

  # Check if install dir is on PATH
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      say ""
      say "Add $(bold "$INSTALL_DIR") to your PATH:"
      say ""
      say "  export PATH=\"${INSTALL_DIR}:\$PATH\""
      say ""
      say "Add that line to your ~/.zshrc or ~/.bashrc to make it permanent."
      ;;
  esac

  # Verify
  if "${INSTALL_DIR}/${BINARY}" --help >/dev/null 2>&1; then
    say ""
    say "$(bold "rift ${VERSION}") installed successfully"
    say "Run $(bold "rift init") to get started."
  else
    say ""
    say "\033[33mwarning:\033[0m binary downloaded but could not run. Check your system compatibility."
  fi
}

# --- Main -------------------------------------------------------------------

main() {
  printf "\n"
  say "$(bold "Rift CLI installer")"
  say ""

  detect_platform
  get_version
  install
  printf "\n"
}

main
