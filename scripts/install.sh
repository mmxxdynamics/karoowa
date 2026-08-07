#!/usr/bin/env bash
# install.sh — One-command Karoowa installer.
#
# Usage:
#   curl -fsSL https://install.karoowa.io | sh
#   # or:
#   curl -fsSL https://raw.githubusercontent.com/mmxxdynamics/karoowa/main/scripts/install.sh | sh
#
# Installs the latest karoowa binary to ~/.karoowa/bin/karoowa.

set -euo pipefail

REPO="mmxxdynamics/karoowa"
INSTALL_DIR="${KAROOWA_HOME:-$HOME/.karoowa}/bin"

# Detect OS and architecture.
detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="pc-windows-msvc" ;;
        *) echo "Unsupported OS: $os" >&2; exit 1 ;;
    esac

    case "$arch" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
    esac

    echo "${arch}-${os}"
}

# Get the latest release tag from GitHub.
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | head -1 \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
}

main() {
    echo "Karoowa Installer"
    echo "================="
    echo ""

    local platform version url archive_ext

    platform="$(detect_platform)"
    echo "Platform: $platform"

    version="$(get_latest_version 2>/dev/null || echo "")"
    if [ -z "$version" ]; then
        echo ""
        echo "Could not determine the latest release."
        echo "No releases have been published yet."
        echo ""
        echo "To build from source:"
        echo "  git clone https://github.com/${REPO}.git"
        echo "  cd karoowa && cargo build --release"
        echo "  ./target/release/karoowa --version"
        exit 1
    fi

    echo "Version:  $version"

    case "$platform" in
        *windows*) archive_ext="zip" ;;
        *)         archive_ext="tar.gz" ;;
    esac

    url="https://github.com/${REPO}/releases/download/${version}/karoowa-${version}-${platform}.${archive_ext}"
    echo "URL:      $url"
    echo ""

    # Create install directory.
    mkdir -p "$INSTALL_DIR"

    # Download and extract.
    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    echo "Downloading..."
    curl -fsSL "$url" -o "$tmpdir/karoowa.${archive_ext}"

    echo "Verifying checksum..."
    local checksums_url="https://github.com/${REPO}/releases/download/${version}/checksums-sha256.txt"
    if ! curl -fsSL "$checksums_url" -o "$tmpdir/checksums.txt"; then
        echo "ERROR: could not download checksums file from $checksums_url" >&2
        echo "Refusing to install an unverified binary. Set KAROOWA_SKIP_CHECKSUM=1 to bypass (NOT recommended)." >&2
        [ "${KAROOWA_SKIP_CHECKSUM:-0}" = "1" ] || exit 1
    else
        if ! (cd "$tmpdir" && grep "karoowa-${version}-${platform}" checksums.txt | sha256sum -c --quiet); then
            echo "ERROR: checksum mismatch — refusing to install." >&2
            echo "If this is unexpected, please file a security report — see SECURITY.md." >&2
            exit 1
        fi
        echo "Checksum OK."
    fi

    echo "Installing to $INSTALL_DIR/karoowa..."
    case "$archive_ext" in
        tar.gz) tar xzf "$tmpdir/karoowa.tar.gz" -C "$INSTALL_DIR" ;;
        zip)    unzip -o "$tmpdir/karoowa.zip" -d "$INSTALL_DIR" ;;
    esac

    chmod +x "$INSTALL_DIR/karoowa"

    echo ""
    echo "Installed: $INSTALL_DIR/karoowa"
    "$INSTALL_DIR/karoowa" --version

    # Check PATH.
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        echo ""
        echo "Add Karoowa to your PATH by adding this to your shell profile:"
        echo ""
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
    fi

    echo ""
    echo "Get started:"
    echo "  karoowa wallet new"
    echo "  karoowa node --validator-key validator.key --consensus poa"
    echo ""
}

main "$@"
