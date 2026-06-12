# ─── Build ──────────────────────────────────────────────────────────

# Build all workspace crates
build:
    cargo build --workspace

# Build with release optimizations
release:
    cargo build --workspace --release

# ─── Run ────────────────────────────────────────────────────────────

# Run the MUD server (default port 4000)
run port="4000":
    cargo run -p mud-bin -- {{ port }}

# ─── Connect ────────────────────────────────────────────────────────

# Connect via tintin++ (auto-loads tinytin.tin if present)
connect addr="127.0.0.1" port="4000":
    test -f tinytin.tin && tt++ -r tinytin.tin || tt++ -r /dev/null {{ addr }} {{ port }}

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

# Format all Rust source files
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all --check

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

# Install development tooling (cargo-watch, lefthook, cocogitto)
install-tools:
    cargo install cargo-watch
    brew install lefthook cocogitto
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
