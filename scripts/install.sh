#!/usr/bin/env bash
# install.sh — One-command Karoowa installer.
#
# Usage:
#   curl -fsSL https://install.karoowa.io | bash
#   # or:
#   curl -fsSL https://raw.githubusercontent.com/mmxxdynamics/karoowa/main/scripts/install.sh | bash
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
    local tmpdir archive_name sha
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    # The archive must be saved under the exact name recorded in
    # checksums-sha256.txt (release.yml runs
    # `sha256sum karoowa-*.tar.gz karoowa-*.zip`), because `sha256sum -c`
    # resolves the filename from the checksum line, not from whatever we
    # happened to call the download.
    archive_name="karoowa-${version}-${platform}.${archive_ext}"

    # macOS ships `shasum`, not `sha256sum`.
    if command -v sha256sum >/dev/null 2>&1; then
        sha="sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        sha="shasum -a 256"
    else
        echo "ERROR: neither sha256sum nor shasum is available; cannot verify the download." >&2
        exit 1
    fi

    echo "Downloading..."
    curl -fsSL "$url" -o "$tmpdir/${archive_name}"

    echo "Verifying checksum..."
    local checksums_url="https://github.com/${REPO}/releases/download/${version}/checksums-sha256.txt"
    if ! curl -fsSL "$checksums_url" -o "$tmpdir/checksums.txt"; then
        echo "ERROR: could not download checksums file from $checksums_url" >&2
        if [ "${KAROOWA_SKIP_CHECKSUM:-0}" = "1" ]; then
            echo "KAROOWA_SKIP_CHECKSUM=1 — continuing without verification (NOT recommended)." >&2
        else
            echo "Refusing to install an unverified binary. Set KAROOWA_SKIP_CHECKSUM=1 to bypass (NOT recommended)." >&2
            exit 1
        fi
        # Note: this escape hatch covers a *missing* checksums file only. A
        # checksum *mismatch* below is always fatal.
    else
        if ! (cd "$tmpdir" && grep -F "$archive_name" checksums.txt | $sha -c --quiet); then
            echo "ERROR: checksum mismatch — refusing to install." >&2
            echo "If this is unexpected, please file a security report — see SECURITY.md." >&2
            exit 1
        fi
        echo "Checksum OK."
    fi

    local binary="karoowa"
    case "$archive_ext" in
        tar.gz)
            tar xzf "$tmpdir/${archive_name}" -C "$INSTALL_DIR"
            chmod +x "$INSTALL_DIR/karoowa"
            ;;
        zip)
            unzip -o "$tmpdir/${archive_name}" -d "$INSTALL_DIR"
            binary="karoowa.exe"
            ;;
    esac

    echo ""
    echo "Installed: $INSTALL_DIR/${binary}"
    "$INSTALL_DIR/${binary}" --version

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
