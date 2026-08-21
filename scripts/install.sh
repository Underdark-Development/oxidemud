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
    echo -e "${YELLOW}For a local install, consider running as a regular user with: ./install.sh --install-dir ~/.oxidemud${NC}"
fi

# Defaults
INSTALL_DIR="$HOME/.oxidemud"
BIN_DIR=""
SYMLINK_DIR=""
CREATE_SYMLINKS=""  # unset by default, prompting in interactive mode if unset
MCP_PORT=5000
SYSTEMD_DIR="/etc/systemd/system"
RUN_AS_USER="${SUDO_USER:-$(id -un)}"
INSTALL_GAME_SERVICE=false  # OPT-IN
INSTALL_MCP_SERVICE=false   # OPT-IN
INSTALL_SPADE=true          # Default to install spade
NON_INTERACTIVE=false
ASSUME_YES=false
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
            BIN_DIR="$2"
            shift 2
            ;;
        --symlink-dir)
            SYMLINK_DIR="$2"
            shift 2
            ;;
        --create-symlinks)
            CREATE_SYMLINKS=true
            shift
            ;;
        --no-symlinks)
            CREATE_SYMLINKS=false
            shift
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
        -y|--yes)
            ASSUME_YES=true
            shift
            ;;
        --non-interactive)
            NON_INTERACTIVE=true
            ASSUME_YES=true
            shift
            ;;
        -h|--help)
            echo "Usage: ./install.sh [options]"
            echo ""
            echo "Options:"
            echo "  --install-dir <path>       Install path. Default: ~/.oxidemud"
            echo "  --bin-dir <path>           Path for binary executables. Default: <install-dir>/bin"
            echo "  --symlink-dir <path>       Path for public symlinks in PATH. Default: auto-detected"
            echo "  --create-symlinks          Opt-in: Symlink binaries to system PATH"
            echo "  --no-symlinks              Opt-out: Skip symlinking binaries to system PATH"
            echo "  --mcp-port <port>          Port for on-demand MCP socket. Default: 5000"
            echo "  --user <username>          Unix user to run services as. Default: current user"
            echo "  --api-url <url>            REST API URL of the game server. Default: http://127.0.0.1:8080"
            echo "  --api-key <key>            API key for the game server (immortal key)"
            echo "  --install-service          Opt-in: Install game server systemd service"
            echo "  --install-mcp              Opt-in: Install MCP server on-demand systemd socket service"
            echo "  --no-spade                 Skip installing 'spade' TUI editor"
            echo "  -y, --yes                  Automatic yes to prompts (skip confirmation)"
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

# If BIN_DIR was not explicitly passed via --bin-dir, set default based on finalized INSTALL_DIR
if [ -z "$BIN_DIR" ]; then
    BIN_DIR="$INSTALL_DIR/bin"
fi

# Auto-detect symlink dir in PATH if not provided
detect_symlink_dir() {
    local candidate
    local candidates=()

    if [ "$IS_ROOT" = "true" ]; then
        candidates=("/usr/local/bin" "/usr/bin" "$HOME/.local/bin")
    else
        candidates=("/usr/local/bin" "$HOME/.local/bin" "$HOME/bin")
    fi

    for candidate in "${candidates[@]}"; do
        # Check if directory is in PATH
        if [[ ":$PATH:" == *":$candidate:"* ]]; then
            if [ -w "$candidate" ] || { [ ! -d "$candidate" ] && [ -w "$(dirname "$candidate")" ]; }; then
                echo "$candidate"
                return 0
            fi
        fi
    done

    # Fallback to ~/.local/bin if none in PATH are writable
    echo "$HOME/.local/bin"
}

if [ -z "$SYMLINK_DIR" ]; then
    SYMLINK_DIR=$(detect_symlink_dir)
fi

SYMLINK_IN_PATH=false
if [[ ":$PATH:" == *":$SYMLINK_DIR:"* ]]; then
    SYMLINK_IN_PATH=true
fi

if [ "$NON_INTERACTIVE" = "false" ]; then
    # Interactive Prompts
    echo -n "Enter installation path [$INSTALL_DIR]: "
    read -r user_install_dir
    if [ -n "$user_install_dir" ]; then
        # Only update BIN_DIR if user hadn't passed a custom --bin-dir
        if [ "$BIN_DIR" = "$INSTALL_DIR/bin" ]; then
            INSTALL_DIR="$user_install_dir"
            BIN_DIR="$INSTALL_DIR/bin"
        else
            INSTALL_DIR="$user_install_dir"
        fi
    fi

    echo -n "Enter binary installation path [$BIN_DIR]: "
    read -r user_bin_dir
    if [ -n "$user_bin_dir" ]; then
        BIN_DIR="$user_bin_dir"
    fi

    if [ -z "$CREATE_SYMLINKS" ]; then
        echo -n "Symlink binaries to your path automatically? [y/N]: "
        read -r user_symlink_opt
        if [[ "$user_symlink_opt" =~ ^[yY](es)?$ ]]; then
            CREATE_SYMLINKS=true
        else
            CREATE_SYMLINKS=false
        fi
    fi

    if [ "$CREATE_SYMLINKS" = "true" ]; then
        echo -n "Enter public symlink directory (in PATH) [$SYMLINK_DIR]: "
        read -r user_symlink_dir
        if [ -n "$user_symlink_dir" ]; then
            SYMLINK_DIR="$user_symlink_dir"
            SYMLINK_IN_PATH=false
            if [[ ":$PATH:" == *":$SYMLINK_DIR:"* ]]; then
                SYMLINK_IN_PATH=true
            fi
        fi
    fi

    # Detect upgrade after install dir is finalized
    IS_UPGRADE=false
    if [ -d "$INSTALL_DIR/content" ]; then
        IS_UPGRADE=true
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
    # Non-interactive: default CREATE_SYMLINKS and detect upgrade silently
    if [ -z "$CREATE_SYMLINKS" ]; then
        CREATE_SYMLINKS=true
    fi
    IS_UPGRADE=false
    if [ -d "$INSTALL_DIR/content" ]; then
        IS_UPGRADE=true
    fi
fi


# Check permissions for root paths
if [ "$IS_ROOT" = "false" ] && { [ "$INSTALL_GAME_SERVICE" = "true" ] || [ "$INSTALL_MCP_SERVICE" = "true" ]; }; then
    echo -e "${RED}Error: Installing systemd services requires root privileges. Please run with sudo or remove service options.${NC}"
    exit 1
fi

# 2. Line-by-line planned operations preview
echo -e "\n${BLUE}=== Installation & Symlink Plan ===${NC}"
echo -e "Installation Directory:      ${YELLOW}$INSTALL_DIR${NC}"
echo -e "Binary Directory:            ${YELLOW}$BIN_DIR${NC}"
echo -e "Create Symlinks in PATH:     ${YELLOW}$CREATE_SYMLINKS${NC}"
if [ "$CREATE_SYMLINKS" = "true" ]; then
    echo -e "Public Symlink Directory:    ${YELLOW}$SYMLINK_DIR${NC}"
    if [ "$SYMLINK_IN_PATH" = "false" ]; then
        echo -e "                       ${YELLOW}(Warning: $SYMLINK_DIR is currently NOT in your PATH)${NC}"
    fi
fi
echo -e "Run as User:                 ${YELLOW}$RUN_AS_USER${NC}"
echo -e "Install Type:                ${YELLOW}${IS_UPGRADE:+upgrade}${IS_UPGRADE:-fresh install}${NC}"
echo -e "MCP Server Port:             ${YELLOW}$MCP_PORT${NC}"
if [ "$IS_ROOT" = "true" ]; then
    echo -e "Install Game Service:        ${YELLOW}$INSTALL_GAME_SERVICE${NC}"
    echo -e "Install MCP Socket:          ${YELLOW}$INSTALL_MCP_SERVICE${NC}"
fi
echo ""
echo -e "${BLUE}Planned File Operations:${NC}"
echo -e "  [Create Directory] ${GREEN}$INSTALL_DIR${NC}"
echo -e "  [Create Directory] ${GREEN}$BIN_DIR${NC}"
echo -e "  [Create Directory] ${GREEN}$INSTALL_DIR/data${NC}"
echo -e "  [Create Directory] ${GREEN}$INSTALL_DIR/logs${NC}"
echo -e "  [Copy Binary]      bin/oxide-server -> ${GREEN}$BIN_DIR/oxide-server${NC}"
echo -e "  [Copy Binary]      bin/oxide-mcp    -> ${GREEN}$BIN_DIR/oxide-mcp${NC}"
if [ "$INSTALL_SPADE" = "true" ]; then
    echo -e "  [Copy Binary]      bin/spade        -> ${GREEN}$BIN_DIR/spade${NC}"
fi
if [ "$CREATE_SYMLINKS" = "true" ]; then
    echo -e "  [Symlink]          ${GREEN}$SYMLINK_DIR/oxide-server${NC} -> $BIN_DIR/oxide-server"
    echo -e "  [Symlink]          ${GREEN}$SYMLINK_DIR/oxide-mcp${NC}    -> $BIN_DIR/oxide-mcp"
    if [ "$INSTALL_SPADE" = "true" ]; then
        echo -e "  [Symlink]          ${GREEN}$SYMLINK_DIR/spade${NC}        -> $BIN_DIR/spade"
    fi
fi
echo ""

# Confirm before taking action
if [ "$ASSUME_YES" = "false" ]; then
    echo -n "Proceed with installation? [y/N]: "
    read -r user_confirm
    if [[ ! "$user_confirm" =~ ^[yY](es)?$ ]]; then
        echo -e "${YELLOW}Installation aborted by user.${NC}"
        exit 0
    fi
fi

# 3. Create target directory structures
echo -e "\nSetting up directory structure..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$BIN_DIR"
mkdir -p "$INSTALL_DIR/data"
mkdir -p "$INSTALL_DIR/logs"
if [ "$CREATE_SYMLINKS" = "true" ]; then
    mkdir -p "$SYMLINK_DIR" 2>/dev/null || true
fi

# 4. Copy Binaries
echo -e "Installing binaries to $BIN_DIR..."
cp bin/oxide-server "$BIN_DIR/"
cp bin/oxide-mcp "$BIN_DIR/"
chmod +x "$BIN_DIR/oxide-server"
chmod +x "$BIN_DIR/oxide-mcp"

if [ "$INSTALL_SPADE" = "true" ]; then
    cp bin/spade "$BIN_DIR/"
    chmod +x "$BIN_DIR/spade"
    echo -e "  Installed spade binary to ${GREEN}$BIN_DIR/spade${NC}"
fi

# Symlink binaries into PATH directory
if [ "$CREATE_SYMLINKS" = "true" ]; then
    echo -e "Creating symlinks in $SYMLINK_DIR..."
    if mkdir -p "$SYMLINK_DIR" 2>/dev/null || [ -w "$SYMLINK_DIR" ]; then
        ln -sf "$BIN_DIR/oxide-server" "$SYMLINK_DIR/oxide-server"
        ln -sf "$BIN_DIR/oxide-mcp" "$SYMLINK_DIR/oxide-mcp"
        echo -e "  Symlinked: ${GREEN}$SYMLINK_DIR/oxide-server${NC}"
        echo -e "  Symlinked: ${GREEN}$SYMLINK_DIR/oxide-mcp${NC}"
        if [ "$INSTALL_SPADE" = "true" ]; then
            ln -sf "$BIN_DIR/spade" "$SYMLINK_DIR/spade"
            echo -e "  Symlinked: ${GREEN}$SYMLINK_DIR/spade${NC}"
        fi
    else
        echo -e "${YELLOW}Could not write symlinks to $SYMLINK_DIR (permission denied).${NC}"
        echo -e "${YELLOW}You can manually create symlinks with:${NC}"
        echo -e "${YELLOW}  sudo ln -sf $BIN_DIR/oxide-server $SYMLINK_DIR/oxide-server${NC}"
        echo -e "${YELLOW}  sudo ln -sf $BIN_DIR/oxide-mcp $SYMLINK_DIR/oxide-mcp${NC}"
        if [ "$INSTALL_SPADE" = "true" ]; then
            echo -e "${YELLOW}  sudo ln -sf $BIN_DIR/spade $SYMLINK_DIR/spade${NC}"
        fi
    fi
fi


# Copy Docker files
if [ -f "Dockerfile" ]; then
    cp "Dockerfile" "$INSTALL_DIR/Dockerfile"
    echo -e "  Added Dockerfile"
fi
if [ -f "docker-compose.yml" ]; then
    cp "docker-compose.yml" "$INSTALL_DIR/docker-compose.yml"
    echo -e "  Added docker-compose.yml"
fi

# 5. Handle Content Upgrades
if [ -d "$INSTALL_DIR/content" ]; then
    # Upgrade scenario
    echo -e "Upgrade detected. Preserving existing content folder."

    # Backup active SQLite database before upgrade schema migrations trigger
    if [ -f "$INSTALL_DIR/data/mud.db" ]; then
        BACKUP_TIME=$(date +%Y%m%d_%H%M%S)
        BACKUP_DIR="$INSTALL_DIR/data/backups"
        mkdir -p "$BACKUP_DIR"
        cp "$INSTALL_DIR/data/mud.db" "$BACKUP_DIR/mud.db.pre-upgrade-$BACKUP_TIME"
        chmod 600 "$BACKUP_DIR/mud.db.pre-upgrade-$BACKUP_TIME"
        echo -e "  Backed up active database to: ${GREEN}$BACKUP_DIR/mud.db.pre-upgrade-$BACKUP_TIME${NC}"
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

    if [ -f "server.toml" ] && [ ! -f "$INSTALL_DIR/server.toml" ]; then
        cp server.toml "$INSTALL_DIR/server.toml"
    fi
else
    # Fresh Install scenario
    echo -e "Fresh install detected. Copying default example templates..."
    cp -r content "$INSTALL_DIR/content"
    echo -e "  Installed example templates to: ${GREEN}$INSTALL_DIR/content/${NC}"

    if [ -f "server.toml" ]; then
        cp server.toml "$INSTALL_DIR/server.toml"
        echo -e "  Installed server config to: ${GREEN}$INSTALL_DIR/server.toml${NC}"
    fi
fi

# Ensure content is writable
chmod 775 "$INSTALL_DIR/content"
find "$INSTALL_DIR/content" -type d -exec chmod 775 {} +
find "$INSTALL_DIR/content" -type f -exec chmod 664 {} +

# Ensure data dir is writable
chmod 775 "$INSTALL_DIR/data"

# Write MCP config on upgrade
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

    # Set ownership of installation directory
    chown -R "$RUN_AS_USER":"$RUN_AS_USER" "$INSTALL_DIR"

    # If running via sudo and RUN_AS_USER is different from SUDO_USER, ensure SUDO_USER has group access
    if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "$RUN_AS_USER" ] && id "$SUDO_USER" &>/dev/null; then
        SUDO_GROUP=$(id -gn "$SUDO_USER" 2>/dev/null || echo "$SUDO_USER")
        chgrp -R "$SUDO_GROUP" "$INSTALL_DIR"
        chmod -R g+w "$INSTALL_DIR"
        echo -e "  Granted group write permissions to calling user: ${GREEN}$SUDO_USER${NC} ($SUDO_GROUP)"
    fi
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
ExecStart=$BIN_DIR/oxide-server --base-dir $INSTALL_DIR
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

        MCP_EXEC_ARGS="$INSTALL_DIR/content"
        if [ -n "$API_KEY" ]; then
            MCP_EXEC_ARGS="$MCP_EXEC_ARGS --url $API_URL --key $API_KEY"
        fi

        cat <<EOF > "$SYSTEMD_DIR/oxide-mcp@.service"
[Unit]
Description=OxideMUD MCP Instance
Requires=oxide-mcp.socket

[Service]
Type=simple
User=$RUN_AS_USER
Group=$RUN_AS_USER
ExecStart=$BIN_DIR/oxide-mcp $MCP_EXEC_ARGS
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
        echo -e "    ${GREEN}$BIN_DIR/oxide-server --base-dir $INSTALL_DIR${NC}"
    fi
    if [ "$INSTALL_MCP_SERVICE" = "false" ]; then
        MCP_CMD="$BIN_DIR/oxide-mcp $INSTALL_DIR/content"
        if [ -n "$API_KEY" ]; then
            MCP_CMD="$MCP_CMD --url $API_URL --key $API_KEY"
        fi
        echo -e "  To start the MCP server manually (stdio):"
        echo -e "    ${GREEN}$MCP_CMD${NC}"
    fi
fi

if [ "$CREATE_SYMLINKS" = "true" ] && [ "$SYMLINK_IN_PATH" = "false" ]; then
    echo -e "\n${YELLOW}Notice: $SYMLINK_DIR is not in your shell PATH environment variable.${NC}"
    echo -e "To run Oxide commands from anywhere, add it to your PATH by adding this line to your shell profile (~/.bashrc or ~/.zshrc):"
    echo -e "  ${GREEN}export PATH=\"$SYMLINK_DIR:\$PATH\"${NC}"
fi


echo -e "\n${GREEN}Installation Complete!${NC}"
