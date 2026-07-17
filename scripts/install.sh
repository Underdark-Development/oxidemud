#!/usr/bin/env bash
set -euo pipefail

# OxideMUD Host Installer Script
# Installs binaries, configures paths, handles content upgrades, and configures systemd.

# Color helpers
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0;0m' # No Color

echo -e "${BLUE}=== OxideMUD Installer ===${NC}"

# 1. Verify environment and read version
if [ ! -f .version ]; then
    echo -e "${RED}Error: .version file not found. Run this installer from the unpacked archive directory.${NC}"
    exit 1
fi
VERSION=$(cat .version)
echo -e "Installing OxideMUD version: ${GREEN}v$VERSION${NC}"

# Check if running as root
IS_ROOT=false
if [ "$EUID" -eq 0 ]; then
    IS_ROOT=true
    echo -e "${YELLOW}Running as root. This is only needed for systemd service installation or system-wide paths (e.g. /opt/oxide).${NC}"
    echo -e "${YELLOW}For a local install, consider running as a regular user with: ./install.sh --install-dir ~/.oxide${NC}"
fi

# Defaults
INSTALL_DIR="$HOME/.oxide"
BIN_INSTALL_PATH="$HOME/.local/bin"
MCP_PORT=5000
SYSTEMD_DIR="/etc/systemd/system"
RUN_AS_USER="$(id -un)"
INSTALL_GAME_SERVICE=false  # OPT-IN
INSTALL_MCP_SERVICE=false   # OPT-IN
INSTALL_SPADE=true          # Default to install spade
NON_INTERACTIVE=false
API_URL="${OXIDE_API_URL:-http://127.0.0.1:8080}"
API_KEY="${OXIDE_API_KEY:-}"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --bin-dir)
            BIN_INSTALL_PATH="$2"
            shift 2
            ;;
        --mcp-port)
            MCP_PORT="$2"
            shift 2
            ;;
        --user)
            RUN_AS_USER="$2"
            shift 2
            ;;
        --api-url)
            API_URL="$2"
            shift 2
            ;;
        --api-key)
            API_KEY="$2"
            shift 2
            ;;
        --install-service)
            INSTALL_GAME_SERVICE=true
            shift
            ;;
        --install-mcp)
            INSTALL_MCP_SERVICE=true
            shift
            ;;
        --no-spade)
            INSTALL_SPADE=false
            shift
            ;;
        --non-interactive)
            NON_INTERACTIVE=true
            shift
            ;;
        -h|--help)
            echo "Usage: ./install.sh [options]"
            echo ""
            echo "Options:"
            echo "  --install-dir <path>       Install path. Default: ~/.oxide"
            echo "  --bin-dir <path>           Path for public executables (spade). Default: ~/.local/bin"
            echo "  --mcp-port <port>          Port for on-demand MCP socket. Default: 5000"
            echo "  --user <username>          Unix user to run services as. Default: current user"
            echo "  --api-url <url>            REST API URL of the game server. Default: http://127.0.0.1:8080"
            echo "  --api-key <key>            API key for the game server (immortal key)"
            echo "  --install-service          Opt-in: Install game server systemd service"
            echo "  --install-mcp              Opt-in: Install MCP server on-demand systemd socket service"
            echo "  --no-spade                 Skip copying 'spade' TUI editor to executable path"
            echo "  --non-interactive          Do not prompt for interactive inputs"
            exit 0
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Type ./install.sh --help for options."
            exit 1
            ;;
    esac
done

if [ "$NON_INTERACTIVE" = "false" ]; then
    # Interactive Prompts
    echo -n "Enter installation path [$INSTALL_DIR]: "
    read -r user_install_dir
    if [ -n "$user_install_dir" ]; then
        INSTALL_DIR="$user_install_dir"
    fi

    # Detect upgrade after install dir is finalized
    IS_UPGRADE=false
    if [ -d "$INSTALL_DIR/content" ]; then
        IS_UPGRADE=true
    fi

    if [ "$INSTALL_SPADE" = "true" ]; then
        echo -n "Enter global executable path (for spade) [$BIN_INSTALL_PATH]: "
        read -r user_bin_path
        if [ -n "$user_bin_path" ]; then
            BIN_INSTALL_PATH="$user_bin_path"
        fi
    fi

    echo -n "Enter MCP server listen port [$MCP_PORT]: "
    read -r user_mcp_port
    if [ -n "$user_mcp_port" ] && [[ "$user_mcp_port" =~ ^[0-9]+$ ]]; then
        MCP_PORT="$user_mcp_port"
    fi

    # API config: only prompt on upgrade with existing config
    if [ "$IS_UPGRADE" = "true" ] && [ -f "$INSTALL_DIR/mcp_config.toml" ]; then
        EXISTING_URL=$(grep '^url' "$INSTALL_DIR/mcp_config.toml" | sed 's/.*"\(.*\)"/\1/')
        EXISTING_KEY=$(grep '^key' "$INSTALL_DIR/mcp_config.toml" | sed 's/.*"\(.*\)"/\1/')
        [ -n "$EXISTING_URL" ] && API_URL="$EXISTING_URL"
        [ -n "$EXISTING_KEY" ] && API_KEY="$EXISTING_KEY"

        echo -n "Game server API URL [$API_URL]: "
        read -r user_api_url
        if [ -n "$user_api_url" ]; then
            API_URL="$user_api_url"
        fi

        echo -n "API key (blank for offline mode) [$API_KEY]: "
        read -r user_api_key
        if [ -n "$user_api_key" ]; then
            API_KEY="$user_api_key"
        fi
    fi

    # Services prompt (root only)
    if [ "$IS_ROOT" = "true" ]; then
        echo -n "System user to run systemd services as [$RUN_AS_USER]: "
        read -r user_run_user
        if [ -n "$user_run_user" ]; then
            RUN_AS_USER="$user_run_user"
        fi

        echo -n "Do you want to configure the game server as a background systemd service? [y/N]: "
        read -r opt_game_srv
        if [[ "$opt_game_srv" =~ ^[yY](es)?$ ]]; then
            INSTALL_GAME_SERVICE=true
        fi

        echo -n "Do you want to configure the MCP server as an on-demand systemd socket service? [y/N]: "
        read -r opt_mcp_srv
        if [[ "$opt_mcp_srv" =~ ^[yY](es)?$ ]]; then
            INSTALL_MCP_SERVICE=true
        fi
    fi
else
    # Non-interactive: detect upgrade silently
    IS_UPGRADE=false
    if [ -d "$INSTALL_DIR/content" ]; then
        IS_UPGRADE=true
    fi
fi

# Confirming parameters
echo -e "\nParameters:"
echo -e "  Installation Directory:  ${YELLOW}$INSTALL_DIR${NC}"
echo -e "  Run as User:             ${YELLOW}$RUN_AS_USER${NC}"
echo -e "  Install Type:            ${YELLOW}${IS_UPGRADE:+upgrade}${IS_UPGRADE:-fresh install}${NC}"
echo -e "  Install Spade Editor:    ${YELLOW}$INSTALL_SPADE${NC}"
if [ "$INSTALL_SPADE" = "true" ]; then
    echo -e "  Executable Binary Path:  ${YELLOW}$BIN_INSTALL_PATH${NC}"
fi
echo -e "  MCP Server Port:         ${YELLOW}$MCP_PORT${NC}"
if [ "$IS_UPGRADE" = "true" ] && [ -n "$API_KEY" ]; then
    echo -e "  Game Server API URL:     ${YELLOW}$API_URL${NC}"
    echo -e "  API Key:                 ${YELLOW}${API_KEY:-(not set)}${NC}"
fi
if [ "$IS_ROOT" = "true" ]; then
    echo -e "  Install Game Service:    ${YELLOW}$INSTALL_GAME_SERVICE${NC}"
    echo -e "  Install MCP Socket:      ${YELLOW}$INSTALL_MCP_SERVICE${NC}"
fi
echo ""

# 2. Check permissions for root paths
if [ "$IS_ROOT" = "false" ] && { [ "$INSTALL_GAME_SERVICE" = "true" ] || [ "$INSTALL_MCP_SERVICE" = "true" ]; }; then
    echo -e "${RED}Error: Installing systemd services requires root privileges. Please run with sudo or remove service options.${NC}"
    exit 1
fi

# 3. Create target directory structures
echo -e "Setting up directory structure..."
mkdir -p "$INSTALL_DIR/bin"
mkdir -p "$INSTALL_DIR/data"

# 4. Copy Binaries
echo -e "Installing binaries..."
cp bin/oxide-server "$INSTALL_DIR/bin/"
cp bin/oxide-mcp "$INSTALL_DIR/bin/"
chmod +x "$INSTALL_DIR/bin/oxide-server"
chmod +x "$INSTALL_DIR/bin/oxide-mcp"

# Install Spade
if [ "$INSTALL_SPADE" = "true" ]; then
    if mkdir -p "$BIN_INSTALL_PATH" 2>/dev/null && cp bin/spade "$BIN_INSTALL_PATH/" 2>/dev/null; then
        chmod +x "$BIN_INSTALL_PATH/spade"
        echo -e "  Installed: ${GREEN}$BIN_INSTALL_PATH/spade${NC}"
    else
        echo -e "${YELLOW}Could not write to $BIN_INSTALL_PATH/spade. Skipping spade install.${NC}"
        echo -e "${YELLOW}You can manually copy bin/spade to a directory in your PATH.${NC}"
    fi
fi

# 5. Handle Content Upgrades
if [ -d "$INSTALL_DIR/content" ]; then
    # Upgrade scenario
    echo -e "Upgrade detected. Preserving existing content folder."

    # Backup active SQLite database before upgrade schema migrations trigger
    if [ -f "$INSTALL_DIR/data/oxide.db" ]; then
        BACKUP_TIME=$(date +%Y%m%d_%H%M%S)
        BACKUP_DIR="$INSTALL_DIR/data/backups"
        mkdir -p "$BACKUP_DIR"
        cp "$INSTALL_DIR/data/oxide.db" "$BACKUP_DIR/oxide.db.pre-upgrade-$BACKUP_TIME"
        chmod 600 "$BACKUP_DIR/oxide.db.pre-upgrade-$BACKUP_TIME"
        echo -e "  Backed up active database to: ${GREEN}$BACKUP_DIR/oxide.db.pre-upgrade-$BACKUP_TIME${NC}"
    fi

    # Store old version if readable
    OLD_VERSION="unknown"
    if [ -f "$INSTALL_DIR/.version" ]; then
        OLD_VERSION=$(cat "$INSTALL_DIR/.version")
    fi
    echo -e "  Upgrading from ${YELLOW}v$OLD_VERSION${NC} to ${GREEN}v$VERSION${NC}"

    # Place new baseline templates in content.default/
    rm -rf "$INSTALL_DIR/content.default"
    cp -r content "$INSTALL_DIR/content.default"
    echo -e "  Placed new default templates in ${GREEN}$INSTALL_DIR/content.default/${NC} for reference."
else
    # Fresh Install scenario
    echo -e "Fresh install detected. Copying default example templates..."
    cp -r content "$INSTALL_DIR/content"
    echo -e "  Installed example templates to: ${GREEN}$INSTALL_DIR/content/${NC}"
fi

# Ensure content is writable (MCP, OLC, and Spade write TOML files at runtime)
chmod 775 "$INSTALL_DIR/content"
find "$INSTALL_DIR/content" -type d -exec chmod 775 {} +
find "$INSTALL_DIR/content" -type f -exec chmod 664 {} +

# Ensure data dir is writable
chmod 775 "$INSTALL_DIR/data"

# Write MCP config on upgrade (preserves API connection settings)
if [ "$IS_UPGRADE" = "true" ] && [ -n "$API_KEY" ]; then
    cat > "$INSTALL_DIR/mcp_config.toml" <<EOF
url = "$API_URL"
key = "$API_KEY"
EOF
    echo -e "  Updated MCP config: ${GREEN}$INSTALL_DIR/mcp_config.toml${NC}"
fi

# Write target version metadata
echo "$VERSION" > "$INSTALL_DIR/.version"

# 6. Configure ownership and permissions
if [ "$IS_ROOT" = "true" ]; then
    if ! id "$RUN_AS_USER" &>/dev/null; then
        echo -e "Creating system user: ${YELLOW}$RUN_AS_USER${NC}..."
        useradd -r -s /bin/false "$RUN_AS_USER"
    fi
    chown -R "$RUN_AS_USER":"$RUN_AS_USER" "$INSTALL_DIR"
fi

# 7. Setup systemd Services
if [ "$IS_ROOT" = "true" ] && command -v systemctl &>/dev/null; then
    SERVICES_MODIFIED=false

    # Write oxide.service (game server)
    if [ "$INSTALL_GAME_SERVICE" = "true" ]; then
        echo -e "Configuring oxide.service..."
        cat <<EOF > "$SYSTEMD_DIR/oxide.service"
[Unit]
Description=OxideMUD Game Server
After=network.target

[Service]
Type=simple
User=$RUN_AS_USER
Group=$RUN_AS_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/bin/oxide-server --config-path $INSTALL_DIR/content/server.toml --db-path $INSTALL_DIR/data/oxide.db
Restart=always
RestartSec=5
LimitNOFILE=2048

# Sandboxing
ProtectSystem=full
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF
        systemctl enable oxide.service
        systemctl restart oxide.service
        echo -e "  ${GREEN}Started oxide.service${NC}"
        SERVICES_MODIFIED=true
    fi

    # Write oxide-mcp.socket (MCP Server port)
    if [ "$INSTALL_MCP_SERVICE" = "true" ]; then
        echo -e "Configuring oxide-mcp.socket..."
        cat <<EOF > "$SYSTEMD_DIR/oxide-mcp.socket"
[Unit]
Description=OxideMUD MCP Server Socket

[Socket]
ListenStream=127.0.0.1:$MCP_PORT
Accept=yes

[Install]
WantedBy=sockets.target
EOF

        # Build MCP args for ExecStart
        MCP_EXEC_ARGS="$INSTALL_DIR/content"
        if [ -n "$API_KEY" ]; then
            MCP_EXEC_ARGS="$MCP_EXEC_ARGS --url $API_URL --key $API_KEY"
        fi

        # Write oxide-mcp@.service (MCP server instances)
        cat <<EOF > "$SYSTEMD_DIR/oxide-mcp@.service"
[Unit]
Description=OxideMUD MCP Instance
Requires=oxide-mcp.socket

[Service]
Type=simple
User=$RUN_AS_USER
Group=$RUN_AS_USER
ExecStart=$INSTALL_DIR/bin/oxide-mcp $MCP_EXEC_ARGS
StandardInput=socket
StandardOutput=socket
StandardError=journal
LimitNOFILE=1024
EOF
        systemctl enable oxide-mcp.socket
        systemctl restart oxide-mcp.socket
        echo -e "  ${GREEN}Started oxide-mcp.socket (listen port: $MCP_PORT)${NC}"
        SERVICES_MODIFIED=true
    fi

    if [ "$SERVICES_MODIFIED" = "true" ]; then
        systemctl daemon-reload
    fi
fi

# Print instructions for manual launch
if [ "$INSTALL_GAME_SERVICE" = "false" ] || [ "$INSTALL_MCP_SERVICE" = "false" ]; then
    echo -e "\nManual Launch Commands (services not installed):"
    if [ "$INSTALL_GAME_SERVICE" = "false" ]; then
        echo -e "  To start the game server manually:"
        echo -e "    ${GREEN}$INSTALL_DIR/bin/oxide-server --config-path $INSTALL_DIR/content/server.toml --db-path $INSTALL_DIR/data/oxide.db${NC}"
    fi
    if [ "$INSTALL_MCP_SERVICE" = "false" ]; then
        MCP_CMD="$INSTALL_DIR/bin/oxide-mcp $INSTALL_DIR/content"
        if [ -n "$API_KEY" ]; then
            MCP_CMD="$MCP_CMD --url $API_URL --key $API_KEY"
        fi
        echo -e "  To start the MCP server manually (stdio):"
        echo -e "    ${GREEN}$MCP_CMD${NC}"
    fi
fi

echo -e "\n${GREEN}Installation Complete!${NC}"
