#!/usr/bin/env bash
set -euo pipefail

# OxideMUD Packager Script
# Build binaries, collect example templates, write metadata, and create release archive.

# Print helper usage
usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -t, --target <target-triple>   Cross-compile to target using cargo-zigbuild"
    echo "                                 (e.g., x86_64-unknown-linux-musl)"
    echo "  -h, --help                     Show this help message"
    exit 0
}

TARGET=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
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

# Ensure we are in the workspace root
cd "$(dirname "$0")/.."

# 1. Resolve workspace version
if [ -f Cargo.toml ]; then
    VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
    if [ -z "$VERSION" ]; then
        # Fallback to cargo metadata if simple parse fails
        VERSION=$(cargo metadata --no-deps --format-version 1 | grep -o '"version":"[^"]*"' | head -n1 | cut -d'"' -f4)
    fi
else
    echo "Error: Cargo.toml not found in current directory. Run from workspace root."
    exit 1
fi

if [ -z "$VERSION" ]; then
    echo "Error: Could not resolve workspace version."
    exit 1
fi

echo "Packaging OxideMUD v$VERSION"

# 2. Determine target info and build command
BUILD_CMD="cargo build --release"
TARGET_DIR="target/release"
ARCHIVE_TARGET=""

if [ -n "$TARGET" ]; then
    echo "Cross-compiling for target: $TARGET"
    if ! command -v cargo-zigbuild &> /dev/null; then
        echo "Error: cargo-zigbuild is required for cross-compilation."
        echo "Install it via: cargo install cargo-zigbuild"
        exit 1
    fi
    BUILD_CMD="cargo zigbuild --target $TARGET --release"
    TARGET_DIR="target/$TARGET/release"
    ARCHIVE_TARGET="$TARGET"
else
    # Detect local target OS/Arch
    OS_NAME=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH_NAME=$(uname -m)
    ARCHIVE_TARGET="${OS_NAME}-${ARCH_NAME}"
    echo "Building for host: $ARCHIVE_TARGET"
fi

# 3. Compile workspace binaries
echo "Running: $BUILD_CMD"
$BUILD_CMD

# 4. Draft packaging directory
STAGE_DIR="target/release/oxide-pkg-temp"
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/bin"
mkdir -p "$STAGE_DIR/data"

# Copy compiled binaries
BINS=("oxide-server" "oxide-mcp" "spade")
for bin in "${BINS[@]}"; do
    SRC_BIN="$TARGET_DIR/$bin"
    if [ ! -f "$SRC_BIN" ]; then
        # Try checking for executable with .exe for windows targets just in case
        if [ -f "${SRC_BIN}.exe" ]; then
            SRC_BIN="${SRC_BIN}.exe"
        else
            echo "Error: Binary not found at $SRC_BIN"
            exit 1
        fi
    fi
    cp "$SRC_BIN" "$STAGE_DIR/bin/"
    echo "  Added binary: $(basename "$SRC_BIN")"
done

# Copy example game content
if [ -d "content" ]; then
    cp -r "content" "$STAGE_DIR/"
    echo "  Added example content directory"
else
    echo "Warning: content directory not found."
fi

# Write version metadata file
echo "$VERSION" > "$STAGE_DIR/.version"
echo "  Written .version metadata file"

# Copy installer scripts (Unix shell and Windows PowerShell)
if [ -f "scripts/install.sh" ]; then
    cp "scripts/install.sh" "$STAGE_DIR/install.sh"
    chmod +x "$STAGE_DIR/install.sh"
    echo "  Added install.sh script"
fi

if [ -f "scripts/install.ps1" ]; then
    cp "scripts/install.ps1" "$STAGE_DIR/install.ps1"
    echo "  Added install.ps1 script"
fi

# Copy Docker configurations for distribution
if [ -f "Dockerfile" ]; then
    cp "Dockerfile" "$STAGE_DIR/Dockerfile"
    echo "  Added Dockerfile"
fi

if [ -f "docker-compose.yml" ]; then
    cp "docker-compose.yml" "$STAGE_DIR/docker-compose.yml"
    echo "  Added docker-compose.yml"
fi

if [ -f ".dockerignore" ]; then
    cp ".dockerignore" "$STAGE_DIR/.dockerignore"
    echo "  Added .dockerignore"
fi

# Copy Ansible playbooks for distribution
if [ -d "ansible" ]; then
    cp -r "ansible" "$STAGE_DIR/ansible"
    echo "  Added ansible deployment files"
fi

# 5. Archive package (ZIP for Windows, TAR.GZ for Unix)
IS_WINDOWS=false
if [[ "$ARCHIVE_TARGET" == *"windows"* ]]; then
    IS_WINDOWS=true
fi

if [ "$IS_WINDOWS" = "true" ]; then
    ARCHIVE_NAME="oxide-v${VERSION}-${ARCHIVE_TARGET}.zip"
    ARCHIVE_PATH="target/release/$ARCHIVE_NAME"
    echo "Creating ZIP archive $ARCHIVE_PATH..."
    
    if command -v zip &> /dev/null; then
        rm -f "$ARCHIVE_PATH"
        # Navigate to temp folder and zip contents recursively
        (cd "$STAGE_DIR" && zip -q -r "../$ARCHIVE_NAME" .)
        echo "Package created successfully: $ARCHIVE_PATH"
    else
        echo "Warning: zip command not found. Falling back to TAR.GZ format for Windows package."
        ARCHIVE_NAME="oxide-v${VERSION}-${ARCHIVE_TARGET}.tar.gz"
        ARCHIVE_PATH="target/release/$ARCHIVE_NAME"
        echo "Creating TAR.GZ archive $ARCHIVE_PATH..."
        tar -C "$STAGE_DIR" -czf "$ARCHIVE_PATH" .
        echo "Package created successfully: $ARCHIVE_PATH"
    fi
else
    ARCHIVE_NAME="oxide-v${VERSION}-${ARCHIVE_TARGET}.tar.gz"
    ARCHIVE_PATH="target/release/$ARCHIVE_NAME"
    echo "Creating TAR.GZ archive $ARCHIVE_PATH..."
    tar -C "$STAGE_DIR" -czf "$ARCHIVE_PATH" .
    echo "Package created successfully: $ARCHIVE_PATH"
fi

# Cleanup temp build directory
rm -rf "$STAGE_DIR"
