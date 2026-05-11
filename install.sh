#!/usr/bin/env bash
# noterm install / upgrade script
# Usage: curl -sSf https://raw.githubusercontent.com/cyberslacks/Noterm/main/install.sh | sh
# Or:    sh install.sh [--force]
set -euo pipefail

REPO="cyberslacks/Noterm"
BINARY="noterm"
INSTALL_DIR="${INSTALL_DIR:-}"   # override with INSTALL_DIR=/my/path sh install.sh
FORCE="${1:-}"

# ── Colours ───────────────────────────────────────────────────────────────────
if [ -t 1 ]; then
  RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
  CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'
else
  RED=''; GREEN=''; YELLOW=''; CYAN=''; BOLD=''; RESET=''
fi

info()    { printf "${CYAN}info${RESET}  %s\n" "$*"; }
success() { printf "${GREEN}ok${RESET}    %s\n" "$*"; }
warn()    { printf "${YELLOW}warn${RESET}  %s\n" "$*"; }
die()     { printf "${RED}error${RESET} %s\n" "$*" >&2; exit 1; }

# ── Platform detection ────────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux*)
    case "$ARCH" in
      x86_64) ARTIFACT="noterm-linux-x86_64.tar.gz" ;;
      *)      die "Unsupported Linux architecture: $ARCH. Build from source with: cargo build --release" ;;
    esac
    ;;
  Darwin*)
    # Prefer the universal binary when available; fall back to arch-specific.
    if   [ "$ARCH" = "arm64" ];  then ARTIFACT="noterm-macos-universal.tar.gz"
    elif [ "$ARCH" = "x86_64" ]; then ARTIFACT="noterm-macos-universal.tar.gz"
    else die "Unsupported macOS architecture: $ARCH"
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows*)
    die "Windows detected. Download the .exe from: https://github.com/${REPO}/releases/latest"
    ;;
  *)
    die "Unsupported OS: $OS. Build from source with: cargo build --release"
    ;;
esac

# ── Resolve install directory ─────────────────────────────────────────────────
resolve_install_dir() {
  if [ -n "$INSTALL_DIR" ]; then
    echo "$INSTALL_DIR"
    return
  fi

  # Prefer /usr/local/bin if writable, else ~/.local/bin
  if [ -w "/usr/local/bin" ]; then
    echo "/usr/local/bin"
  elif [ -d "$HOME/.local/bin" ] && echo "$PATH" | grep -q "$HOME/.local/bin"; then
    echo "$HOME/.local/bin"
  elif [ -d "$HOME/bin" ] && echo "$PATH" | grep -q "$HOME/bin"; then
    echo "$HOME/bin"
  else
    echo "/usr/local/bin"   # will use sudo below
  fi
}

DEST_DIR="$(resolve_install_dir)"
DEST="$DEST_DIR/$BINARY"

# ── Fetch latest release metadata ────────────────────────────────────────────
info "Checking latest release from github.com/${REPO}…"

if command -v curl &>/dev/null; then
  FETCH="curl -sSfL"
elif command -v wget &>/dev/null; then
  FETCH="wget -qO-"
else
  die "Neither curl nor wget found. Install one and retry."
fi

RELEASE_JSON=$($FETCH "https://api.github.com/repos/${REPO}/releases/latest")

# Parse without jq — works on plain sh
LATEST_TAG=$(echo "$RELEASE_JSON" | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
LATEST_VER="${LATEST_TAG#v}"   # strip leading 'v'

DOWNLOAD_URL=$(echo "$RELEASE_JSON" | grep "browser_download_url" \
  | grep "$ARTIFACT" \
  | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')

[ -n "$LATEST_TAG" ]    || die "Could not parse release tag from GitHub API response."
[ -n "$DOWNLOAD_URL" ]  || die "No download URL found for ${ARTIFACT} in release ${LATEST_TAG}."

# ── Version comparison ────────────────────────────────────────────────────────
CURRENT_VER=""
if command -v "$BINARY" &>/dev/null; then
  CURRENT_VER=$(noterm --version 2>/dev/null | awk '{print $2}' || true)
fi

printf "${BOLD}Latest :${RESET} %s\n" "$LATEST_TAG"
if [ -n "$CURRENT_VER" ]; then
  printf "${BOLD}Current:${RESET} %s\n" "$CURRENT_VER"
fi

if [ "$CURRENT_VER" = "$LATEST_VER" ] && [ "$FORCE" != "--force" ]; then
  success "Already up to date ($LATEST_TAG). Use --force to reinstall."
  exit 0
fi

# ── Download ──────────────────────────────────────────────────────────────────
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

info "Downloading ${ARTIFACT}…"
$FETCH "$DOWNLOAD_URL" > "$TMP_DIR/$ARTIFACT"

# ── Extract ───────────────────────────────────────────────────────────────────
info "Extracting…"
tar -xzf "$TMP_DIR/$ARTIFACT" -C "$TMP_DIR"

# The archive contains a single binary named e.g. noterm-linux-x86_64
EXTRACTED=$(find "$TMP_DIR" -maxdepth 1 -type f -name "noterm-*" | head -1)
[ -n "$EXTRACTED" ] || die "Could not find binary after extraction."
chmod +x "$EXTRACTED"

# ── Install ───────────────────────────────────────────────────────────────────
mkdir -p "$DEST_DIR"

if [ -w "$DEST_DIR" ]; then
  cp "$EXTRACTED" "$DEST"
  success "Installed → $DEST"
else
  info "Installing to $DEST_DIR requires elevated permissions."
  sudo cp "$EXTRACTED" "$DEST"
  success "Installed → $DEST  (via sudo)"
fi

# ── Post-install hint ─────────────────────────────────────────────────────────
echo ""
printf "${GREEN}${BOLD}noterm ${LATEST_TAG} ready.${RESET}\n"

# Warn if install dir is not in PATH
if ! echo "$PATH" | grep -q "$DEST_DIR"; then
  warn "$DEST_DIR is not in your PATH."
  warn "Add this to your shell rc:  export PATH=\"\$PATH:$DEST_DIR\""
fi

echo ""
echo "  Run:  noterm"
echo "  Docs: https://github.com/${REPO}#readme"
