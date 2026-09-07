# Getting Started Guide

Welcome to Oxide MUD! This guide will walk you through the process of installing the server, configuring the database, starting the engine, and connecting your first client.

---

## 1. Quick Start with Docker (Recommended)

Oxide MUD distribution packages include a preconfigured `Dockerfile` and `docker-compose.yml`, making Docker the fastest and most portable way to get up and running on Linux, macOS, or Windows.

### Step 1: Extract the Release Package

Unpack the release tarball and enter the directory:

```bash
tar -xzf oxide-v*.tar.gz
cd oxide-v*
```

### Step 2: Start the Server

Run Docker Compose from within the extracted folder:

```bash
docker compose up -d --build
```

_(If you are using Docker Compose v1, use `docker-compose up -d --build` instead.)_

This automatically builds a lightweight runtime image using the bundled precompiled server binary and mounts your local directory for persistent data:

- `server.toml` &rarr; `/app/server.toml` (Server configuration)
- `content/` &rarr; `/app/content/` (Game world templates, zones, and rooms)
- `data/` &rarr; `/app/data/` (SQLite game database and player data)
- `logs/` &rarr; `/app/logs/` (Server logs)

The container exposes port **4000** for direct MUD Telnet traffic and port **8080** for the REST API and WebSocket connections.

### Useful Docker Commands

- **View Live Logs:**
  ```bash
  docker compose logs -f oxide-server
  ```
- **Attach to Interactive Game Console:**
  ```bash
  docker attach oxide-server
  ```
  _(To detach without stopping the server, press `Ctrl-P` followed by `Ctrl-Q`.)_
- **Restart After Config Changes:**
  ```bash
  docker compose restart oxide-server
  ```
- **Stop Server:**
  ```bash
  docker compose down
  ```

---

## 2. Alternative: Native Host Installation

For Linux hosts where you prefer running directly on bare metal or managing services via systemd, you can use the bundled installer script.

### Step 1: Run the Interactive Installer

From the unpacked archive folder, run:

```bash
./install.sh
```

By default, the installer places files into `~/.oxidemud` (or `/opt/oxide` when run as root). It handles path setup, binary symlinks, and optional systemd service creation.

### Step 2: Launching the Native Binary

Start the game server directly from the command line:

```bash
/opt/oxide/bin/oxide-server --base-dir /opt/oxide
```

Or if you installed systemd services:

```bash
sudo systemctl start oxide
sudo systemctl status oxide
```

---

## 3. Server Configuration

> See the [Server Administration Guide](server_admin.md) for detailed configuration documentation.

The server configuration resides in `server.toml` (in your unpacked directory for Docker, or in `/opt/oxide/server.toml` / `~/.oxidemud/server.toml` for native host installs). You can configure server name, client limits, logging, API/WebSocket settings, and game time:

```toml
# server.toml configuration example
server_name = "OxideMUD"
max_clients = 256

[logging]
retention_days = 5
rotation = "daily"
log_level = "info"
```

When running with Docker, edits to `server.toml` take effect with a container restart:

```bash
docker compose restart oxide-server
```

---

## 4. Connecting a Client

OxideMUD supports both modern WebSockets (WSS/WS) and standard Telnet connections:

- **WebSocket Web Client / Browser (Recommended):**
  Connect to `ws://localhost:8080/ws/play` (or `wss://mud.example.com/ws/play`) using any browser client or WebSocket tool.
- **Spade TUI Client & World Builder:**
  ```bash
  ./bin/spade connect ws://localhost:8080/ws/spade
  ```
  _(If installed to your PATH, run `spade connect ws://localhost:8080/ws/spade`)_
- **TinTin++ (Telnet):**
  ```bash
  tt++ -r /dev/null localhost 4000
  ```
- **Telnet Fallback:**
  ```bash
  telnet localhost 4000
  ```

---

## 5. Starting the MCP Server for AI Agents

To enable AI assistant world building via the Model Context Protocol (MCP), run the bundled `oxide-mcp` service:

- **Online Mode (Live WebSocket Connection to Running Server):**
  ```bash
  ./bin/oxide-mcp --online
  ```
  or connect to a custom WebSocket URL with an API key:
  ```bash
  ./bin/oxide-mcp --ws ws://localhost:8080/ws/rpc --key <API_KEY>
  ```
- **Offline Mode (Direct Local Filesystem):**
  ```bash
  ./bin/oxide-mcp ./content
  ```
