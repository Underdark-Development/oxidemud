#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Error: Version argument missing."
    exit 1
fi

NEW_VERSION="$1"

# Update version in root Cargo.toml (compatible with macOS sed)
sed -i '' 's/^version = ".*"/version = "'"$NEW_VERSION"'"/g' Cargo.toml

echo "Updated Cargo.toml version to $NEW_VERSION"
