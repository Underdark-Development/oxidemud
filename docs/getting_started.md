# Getting Started Guide

Welcome to Oxide MUD! This guide will walk you through the process of installing the server, configuring the database, starting the engine, and connecting your first client.

---

## 1. Installation

Oxide MUD compiles into a set of standalone binaries. You can install it on your host machine by downloading the release package matching your platform:

*   **macOS / Linux:** Unpack the `.tar.gz` archive and run the installer:
    ```bash
    tar -xzf oxide-v*.tar.gz
    cd oxide-v*
    ./install.sh
    ```
*   **Windows:** Unpack the `.zip` archive and run the PowerShell installer:
    ```powershell
    Expand-Archive oxide-v*.zip
    cd oxide-v*
    .\install.ps1
    ```

---

## 2. Server Configuration

The installation script places default configurations under `/opt/oxide/content/server.toml` (or your custom directory). You can configure ports, database connections, and system tick rates here:

```toml
# server.toml configuration example
port = 4000
db_path = "data/oxide.db"
content_path = "content"
motd_path = "content/motd.txt"
```

---

## 3. Launching the MUD

Once installed, you can start the game server directly from the command line:

```bash
/opt/oxide/bin/oxide-server --config-path /opt/oxide/content/server.toml --db-path /opt/oxide/data/oxide.db
```

If you installed systemd services during the install setup, you can control the background process using standard system tools:

```bash
sudo systemctl start oxide
```

---

## 4. Connecting a Client

Oxide MUD features a raw Telnet network reader that supports ANSI 256 colors. You can connect to the live port using standard client utilities:

*   **TinTin++ (Recommended):**
    ```bash
    tt++ -r /dev/null localhost 4000
    ```
*   **Telnet Fallback:**
    ```bash
    telnet localhost 4000
    ```

---

## 5. Starting the MCP Co-Builder

To enable AI assistant world building, make sure the Model Context Protocol (MCP) server is running:

```bash
/opt/oxide/bin/oxide-mcp /opt/oxide/content
```
This listens on port `5000` (or systemd socket) allowing compatible AI tools to securely read, write, and validate MUD data models.
