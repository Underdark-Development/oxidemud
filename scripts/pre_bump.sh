#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Error: Version argument missing."
    exit 1
fi

NEW_VERSION="$1"

# Update version in root Cargo.toml (compatible with macOS sed)
sed -i '' 's/^version = ".*"/version = "'"$NEW_VERSION"'"/g' Cargo.toml

# Update Cargo.lock to match the new version
cargo check --workspace

# Stage both files for the bump commit
git add Cargo.toml Cargo.lock

echo "Updated Cargo.toml and Cargo.lock version to $NEW_VERSION"
