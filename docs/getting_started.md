# Getting Started Guide

Welcome to Oxide MUD! This guide will walk you through the process of installing the server, configuring the database, starting the engine, and connecting your first client.

---

## 1. Installation

Oxide MUD compiles into a set of standalone binaries. You can install it on your host machine by downloading the release package matching your platform:

- **macOS / Linux:** Unpack the `.tar.gz` archive and run the installer:
  ```bash
  tar -xzf oxide-v*.tar.gz
  cd oxide-v*
  ./install.sh
  ```
- **Windows:** Unpack the `.zip` archive and run the PowerShell installer:
  ```powershell
  Expand-Archive oxide-v*.zip
  cd oxide-v*
  .\install.ps1
  ```

---

## 2. Server Configuration

> See [Server Administration Guide](server_admin.md) for detailed configuration documentation.

The installation script places default configurations under `/opt/oxide/content/server.toml` (or your custom directory). You can configure server name, client limits, logging, API/WebSocket, time, and the content directory here. Bind host/port and database path are set at launch via CLI flags (see the [Server Administration Guide](server_admin.md)):

```toml
# server.toml configuration example
server_name = "OxideMUD"
max_clients = 256

[content]
path = "content"

[logging]
retention_days = 5
rotation = "daily"
```

---

## 3. Launching the MUD

Once installed, you can start the game server directly from the command line:

```bash
/opt/oxide/bin/oxide-server --content-path /opt/oxide/content --config-path /opt/oxide/content/server.toml --db-path /opt/oxide/data/oxide.db
```

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
