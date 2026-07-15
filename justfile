# ─── Build ──────────────────────────────────────────────────────────

# Build all workspace crates
build:
    cargo build --workspace

# Build with release optimizations
build-release:
    cargo build --workspace --release

# ─── Package & Deploy ────────────────────────────────────────────────

# Bump version, generate changelog, tag, and package the release for macOS, Linux, and Windows
release:
    cog bump --auto
    just package
    just package x86_64-unknown-linux-musl
    just package x86_64-pc-windows-gnu

# Build and package all binaries + templates into a release tarball
package target="":
    chmod +x scripts/package.sh scripts/install.sh
    ./scripts/package.sh {{ if target == "" { "" } else { "-t " + target } }}

# Deploy the packaged release to a remote VPS
deploy host port="22" *args="":
    chmod +x scripts/deploy.sh scripts/package.sh scripts/install.sh
    ./scripts/deploy.sh {{ host }} {{ port }} {{ args }}

# Deploy the packaged release to a remote VPS using Ansible (loads connection details from .env)
deploy-ansible *args="":
    ansible-playbook ansible/deploy.yml {{ args }}


# ─── Server ─────────────────────────────────────────────────────────

# Run the MUD server (default port 4000)
server port="4000":
    cargo run -p oxide-bin -- {{ port }}

# Run spade (offline builder mode)
spade *args="":
    cargo run -p spade -- {{ args }}

# Run the MCP world-building server (stdio transport, pre-built binary)
mcp content_path="content":
    cargo build -p oxide-mcp -q
    ./target/debug/oxide-mcp "{{ content_path }}"

# ─── Connect ────────────────────────────────────────────────────────

# Connect via tintin++ (auto-loads oxide.tin if present)
connect addr="127.0.0.1" port="4000":
    test -f oxide.tin && tt++ -r oxide.tin || tt++ -r /dev/null {{ addr }} {{ port }}

# Connect via raw telnet (fallback)
connect-raw addr="127.0.0.1" port="4000":
    telnet {{ addr }} {{ port }}

# Install tintin++ (macOS)
install-tintin:
    @if command -v brew > /dev/null 2>&1; then \
        brew install tintin; \
    elif command -v apt > /dev/null 2>&1; then \
        sudo apt install -y tintin++; \
    else \
        echo "Unsupported platform. Install tintin++ manually: https://tintin.mudhalla.net/"; \
        exit 1; \
    fi

# ─── Lint & Format ──────────────────────────────────────────────────

# Run clippy on the entire workspace
lint:
    cargo clippy --workspace -- -D warnings

# Format all Rust source and Markdown files
fmt:
    cargo fmt --all
    dprint fmt

# Check formatting without modifying files
fmt-check:
    cargo fmt --all --check
    dprint check

# ─── Test ───────────────────────────────────────────────────────────

# Run all workspace tests
test:
    cargo test --workspace

# ─── DB ─────────────────────────────────────────────────────────────

# Remove local database file(s)
db-clean:
    rm -f *.db *.db-shm *.db-wal

# ─── Clean ──────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# ─── Tools ──────────────────────────────────────────────────────────

# Install development tooling (cargo-watch, lefthook, cocogitto, dprint)
install-tools:
    cargo install cargo-watch
    brew install lefthook cocogitto dprint
    lefthook install

# ─── Watch ──────────────────────────────────────────────────────────

# Auto-rebuild on source changes (requires cargo-watch)
watch:
    cargo watch -x check -x clippy

# ─── CI Check (runs everything CI would) ────────────────────────────

ci-check:
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings
    cargo test --workspace

# ─── Conventional Commits ─────────────────────────────────────────────

# Lint a commit message against conventional commits spec
lint-commit file:
    cog verify --file {{file}}

# Generate CHANGELOG.md from git tags
changelog:
    cog changelog

# Auto-bump version based on commit history + generate changelog
bump:
    cog bump --auto

# ─── Default ────────────────────────────────────────────────────────

default: build
