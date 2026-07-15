#!/usr/bin/env bash
set -euo pipefail

# OxideMUD Local Deployment Script
# Packages the server for Linux, uploads to VPS, and triggers remote installation.

# Color helpers
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0;0m' # No Color

usage() {
    echo "Usage: $0 <user@host> [ssh_port] [options]"
    echo ""
    echo "Options:"
    echo "  --native                       Compile natively on the VPS instead of local cross-build"
    echo "  -t, --target <target-triple>   Specify the target architecture (default: x86_64-unknown-linux-musl)"
    echo "  -h, --help                     Show this help message"
    exit 0
}

if [ $# -lt 1 ]; then
    usage
fi

REMOTE_HOST="$1"
SSH_PORT="22"
shift

# Check if second arg is a number (ssh port)
if [[ $# -gt 0 ]] && [[ "$1" =~ ^[0-9]+$ ]]; then
    SSH_PORT="$1"
    shift
fi

NATIVE_BUILD=false
TARGET="x86_64-unknown-linux-musl"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --native)
            NATIVE_BUILD=true
            shift
            ;;
        -t|--target)
            TARGET="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown argument: $1"
            usage
            ;;
    esac
done

cd "$(dirname "$0")/.."

# Resolve Version
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)

# Check if version resolved
if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: Could not resolve workspace version from Cargo.toml.${NC}"
    exit 1
fi

echo -e "${BLUE}=== Deploying OxideMUD v$VERSION to $REMOTE_HOST (Port: $SSH_PORT) ===${NC}"

# If building natively on the remote host
if [ "$NATIVE_BUILD" = "true" ]; then
    echo -e "${YELLOW}Deploy Mode: Native compilation on the VPS (assumes Rust toolchain is set up on VPS).${NC}"
    
    # 1. Archive the source files
    ARCHIVE_NAME="oxide-source.tar.gz"
    echo -e "Archiving source code..."
    tar --exclude='./target' --exclude='./.git' --exclude='./.cargo' -czf "$ARCHIVE_NAME" .

    # 2. Upload source code to remote server
    echo -e "Uploading source to VPS..."
    scp -P "$SSH_PORT" "$ARCHIVE_NAME" "$REMOTE_HOST:/tmp/"

    # 3. Compile and install on VPS
    echo -e "SSH executing build and installer on VPS..."
    ssh -p "$SSH_PORT" -t "$REMOTE_HOST" "bash -c '
        cd /tmp
        rm -rf oxide-src-build
        mkdir oxide-src-build
        tar -xzf $ARCHIVE_NAME -C oxide-src-build
        rm $ARCHIVE_NAME
        cd oxide-src-build
        
        echo -e \"=== Compiling OxideMUD on VPS ===\"
        cargo build --release --workspace
        
        # Structure the release folder manually on VPS
        mkdir -p release-pkg/bin
        cp target/release/oxide-server release-pkg/bin/
        cp target/release/oxide-mcp release-pkg/bin/
        cp target/release/spade release-pkg/bin/
        cp -r content release-pkg/
        echo \"$VERSION\" > release-pkg/.version
        cp scripts/install.sh release-pkg/
        chmod +x release-pkg/install.sh
        
        # Execute installer
        cd release-pkg
        echo -e \"=== Running Installer ===\"
        NON_INTERACTIVE=true sudo ./install.sh
        
        # Cleanup
        cd /tmp
        rm -rf oxide-src-build
    '"
    
    # Clean up local source tarball
    rm "$ARCHIVE_NAME"
else
    # Cross build locally and deploy packaged archive (Default & recommended)
    echo -e "${YELLOW}Deploy Mode: Local Cross-Compilation (targets: $TARGET).${NC}"
    
    # 1. Run local packager
    echo -e "Packaging release archive..."
    ./scripts/package.sh -t "$TARGET"

    # Resolve filename
    ARCHIVE_NAME="oxide-v${VERSION}-${TARGET}.tar.gz"
    LOCAL_ARCHIVE_PATH="target/release/$ARCHIVE_NAME"

    if [ ! -f "$LOCAL_ARCHIVE_PATH" ]; then
        echo -e "${RED}Error: Packaged archive not found at $LOCAL_ARCHIVE_PATH.${NC}"
        exit 1
    fi

    # 2. Upload archive to VPS
    echo -e "Uploading archive to VPS..."
    scp -P "$SSH_PORT" "$LOCAL_ARCHIVE_PATH" "$REMOTE_HOST:/tmp/"

    # 3. SSH in, extract and run installer
    echo -e "SSH executing installer on VPS..."
    ssh -p "$SSH_PORT" -t "$REMOTE_HOST" "bash -c '
        cd /tmp
        rm -rf oxide-pkg-extract
        mkdir oxide-pkg-extract
        tar -xzf $ARCHIVE_NAME -C oxide-pkg-extract
        rm $ARCHIVE_NAME
        cd oxide-pkg-extract
        
        # Run the installer with sudo in non-interactive mode
        NON_INTERACTIVE=true sudo ./install.sh
        
        # Cleanup
        cd /tmp
        rm -rf oxide-pkg-extract
    '"
fi

echo -e "${GREEN}=== Deployment Finished! ===${NC}"
