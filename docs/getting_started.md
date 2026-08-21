# Getting Started Guide

Welcome to Oxide MUD! This guide will walk you through the process of installing the server, configuring the database, starting the engine, and connecting your first client.

---

## 1. Installation

The server ships as a single `x86_64-unknown-linux-musl` tarball. Unpack it and run the installer:

```bash
tar -xzf oxide-v*.tar.gz
cd oxide-v*
./install.sh
```

> For hosts that can't run the native binary (e.g. Windows, macOS), run the server with the bundled `Dockerfile` / `docker-compose.yml` instead.

---

## 2. Server Configuration

> See [Server Administration Guide](server_admin.md) for detailed configuration documentation.

The installation script places the default configuration at `/opt/oxide/server.toml` (or your custom directory). You can configure server name, client limits, logging, API/WebSocket, and time here. Content, database, and log locations are fixed conventions under the server's base directory (see the [Server Administration Guide](server_admin.md)):

```toml
# server.toml configuration example
server_name = "OxideMUD"
max_clients = 256

[logging]
retention_days = 5
rotation = "daily"
```

---

## 3. Launching the MUD

Once installed, you can start the game server directly from the command line:

```bash
/opt/oxide/bin/oxide-server --base-dir /opt/oxide
```

All server paths (content, config, motd, banner, database, logs) resolve under the base directory.

If you installed systemd services during the install setup, you can control the background process using standard system tools:

```bash
sudo systemctl start oxide
```

---

## 4. Connecting a Client

OxideMUD supports both standard Telnet connections and modern WebSockets (WSS/WS):

- **WebSocket Web Client / Browser (Recommended):**
  Connect to `ws://localhost:8080/ws/play` or `wss://mud.example.com/ws/play` using any browser web client or WebSocket tool.
- **Spade TUI Client & World Builder:**
  ```bash
  spade connect wss://localhost:8080/ws/spade
  ```
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

To enable AI assistant world building, run the Model Context Protocol (MCP) server in offline or online mode:

- **Offline Mode (Local TOML Files):**
  ```bash
  oxide-mcp /opt/oxide/content
  ```
- **Online Mode (Live WebSocket Connection):**
  ```bash
  oxide-mcp --online
  ```
  or connect to a custom WSS server:
  ```bash
  oxide-mcp --ws wss://mud.example.com/ws/mcp --key <API_KEY>
  ```
