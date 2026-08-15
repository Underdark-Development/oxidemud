# Server Administration Guide

This guide is intended for server administrators, systems engineers, and game owners responsible for configuring, deploying, maintaining, and backing up the MUD game server.

---

## Starting the Server

The server is distributed as a precompiled executable (`oxide-server`).

### 1. Direct Execution

Execute the server binary directly from the installation path:

```bash
./bin/oxide-server [options]
```

### 2. Docker Compose Deployment (Recommended)

OxideMUD can be run in containerized environments using Docker and Docker Compose. The distribution package includes both a `Dockerfile` and a `docker-compose.yml` preconfigured for this setup.

#### File Layout & Volumes

The container configuration maps files on the host filesystem directly to the container to ensure persistence of your database, logs, and custom configurations:

- `./content/` (mapped to `/app/content`) — Game configuration files, example templates, and scripts.
- `./data/` (mapped to `/app/data`) — The folder where the SQLite database (`oxide.db`) and backups are saved.
- `./logs/` (mapped to `/app/logs`) — Exposes rotating log files (e.g. `oxide_server_log_*.log`) directly onto the host for easy monitoring and backup.

#### Build & Launch

To build the Docker image and start the server in the background:

```bash
docker-compose up -d --build
```

This automatically builds the container using the precompiled binary, binds the MUD telnet port `4000` to the host, mounts the host's `./content`, `./data`, and `./logs` folders, and configures the container to auto-restart on crashes or VPS reboots.

#### Stopping the Server

To stop the server:

```bash
docker-compose down
```

The game engine gracefully handles the `SIGTERM` signal sent by Docker to execute a graceful shutdown, flushing all dirty in-memory database states to the SQLite database, and checkpointing the WAL journal.

#### Viewing Logs

You can view the logs in two ways:

1. **Directly on Host**: Access the rotating log files locally under the `./logs/` directory.
2. **Via Docker**: To stream the stdout logs of the running container:
   ```bash
   docker-compose logs -f oxide-server
   ```

#### Connecting to the Server Console

Since the container is configured to keep stdin open with a TTY (`stdin_open: true` and `tty: true` in `docker-compose.yml`), you can attach directly to the server's live interactive console (to run console commands like `broadcast` or `save`):

```bash
docker attach oxide-server
```

> [!CAUTION]
> **Detaching Safely**: To disconnect from the attached console without shutting down the server, press the escape sequence:
> `Ctrl + P` followed by `Ctrl + Q`.
> If you press `Ctrl + C` while attached, it sends a `SIGINT` and will terminate the game server.

#### Running Temporary Commands in Docker

To execute temporary commands or inspect the environment of a running container:

```bash
docker exec -it oxide-server /app/bin/oxide-server --version
```

### 3. Ansible Automation Deployment

For automated remote deployments from a local administrator machine, the distribution package includes an Ansible playbook ([deploy.yml](../ansible/deploy.yml)). This playbook targets the precompiled binaries and assets contained within the unpacked package.

> [!NOTE]
> **Direct Host Deployments**: The Ansible playbook is designed to push deployment files from a local machine to a remote VPS. If the distribution archive is already uploaded and extracted directly on the target host VPS, you should **not** use the Ansible playbook. Instead, directly invoke `docker-compose up -d --build` (for Docker) or execute `./install.sh` (for host systemd) directly on the host VPS.

#### Setup

1. Copy the example configuration to `.env` in the root of the unpacked distribution directory, or in the `ansible/` subdirectory:
   ```bash
   cp ansible/.env.example .env
   ```
2. Open `.env` and fill in your VPS connection details:
   - `VPS_HOST` — Remote host IP or domain.
   - `VPS_USER` — Remote SSH username (e.g. `root`).
   - `VPS_PORT` — SSH port (default: `22`).
   - `VPS_KEY_PATH` — Path to your SSH private key.
   - `INSTALL_DIR` / `RUN_AS_USER` — Destination directory and service owner.

#### Run Deployment

Execute the deployment directly via `ansible-playbook`:

```bash
ansible-playbook ansible/deploy.yml
```

_(Note: If executing within the development workspace root, you can also run `just deploy-ansible`)._

This playbook:

1. Prompts you to confirm whether you want to deploy containerized via Docker (default: `yes`) or via host systemd services (`no`).
2. Natively parses your connection settings from the `.env` file.
3. Automatically copies the local precompiled binaries, templates, scripts, and container definitions to the remote VPS temporary folder `/tmp/oxide_deploy`.
4. Executes remote installation steps and starts the server based on your selection.

### 4. GitHub Actions CI/CD

The repository ships two GitHub Actions workflows under `.github/workflows/`.

**CI** (`ci.yml`) runs on every push to `main` and every pull request. It enforces the same gate as the local pre-commit hook: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and the full workspace test suite.

**Release & Deploy** (`release.yml`) runs when a version tag (`v*`) is pushed, e.g. after `just release` / `cog bump --auto`. It always cross-compiles the Linux release tarball (`x86_64-unknown-linux-musl`) and attaches it to a GitHub Release. If the deploy secrets below are configured, it also uploads the tarball to your VPS over SSH and installs it.

Two deploy modes are supported, selected with the `VPS_DEPLOY_MODE` variable:

- **`docker`** (default) — runs the installer to stage binaries, content, `Dockerfile`, and `docker-compose.yml` under the install dir, stops any legacy host systemd services, and starts the stack with `docker compose up -d --build`. If the `TUNNEL_TOKEN` secret is set, it is written to `<install-dir>/.env` and the `cloudflared` service is started via the compose `tunnel` profile for HTTPS/WSS ingress.
- **`systemd`** — installs host systemd services directly instead of using Docker.

To enable automatic VPS deployment, create a dedicated SSH keypair on your admin machine (`ssh-keygen -t ed25519 -f oxide-deploy`), add the public key to the VPS user's `~/.ssh/authorized_keys`, then configure these on GitHub under **Settings → Secrets and variables → Actions**:

| Kind     | Name                 | Value                                                               |
| :------- | :------------------- | :------------------------------------------------------------------ |
| Secret   | `VPS_HOST`           | VPS IP or hostname                                                  |
| Secret   | `VPS_USER`           | SSH user with sudo rights (e.g. `root`)                             |
| Secret   | `VPS_PORT`           | SSH port (e.g. `22`)                                                |
| Secret   | `VPS_SSH_KEY`        | Full private key contents (`oxide-deploy` file)                     |
| Secret   | `TUNNEL_TOKEN`       | (Docker mode) Cloudflare Tunnel token from the Zero Trust dashboard |
| Variable | `VPS_DEPLOY_ENABLED` | Set to `true` to turn the deploy job on                             |
| Variable | `VPS_DEPLOY_MODE`    | `docker` (default) or `systemd`                                     |
| Variable | `VPS_INSTALL_DIR`    | (Optional) Remote install dir, default `/opt/oxide`                 |
| Variable | `VPS_RUN_AS_USER`    | (Optional) Service owner on the VPS, default `oxide`                |

The VPS needs Docker Engine with the Compose plugin installed for `docker` mode. The deploy job binds to a `production` GitHub environment, so you can optionally require manual approval on deployments via **Settings → Environments → production → Required reviewers**.

### 5. CLI Command Options

You can customize the server behavior at launch using the following command-line flags:

| Flag                   | Short | Default               | Description                                |
| :--------------------- | :---- | :-------------------- | :----------------------------------------- |
| `--host <IP>`          | `-h`  | `127.0.0.1`           | The bind IP address for the game listener. |
| `--port <port>`        | `-p`  | `4000`                | The bind TCP port.                         |
| `--db-path <path>`     | `-d`  | `data/oxide.db`       | Path to the SQLite persistence database.   |
| `--motd-path <path>`   | `-m`  | `content/motd.txt`    | Path to the Message of the Day file.       |
| `--banner-path <path>` | `-b`  | `content/banner.txt`  | Path to the welcome ASCII banner file.     |
| `--config-path <path>` | `-c`  | `content/server.toml` | Path to the server configuration file.     |

#### Precedence Order

Startup configuration parameters are applied in the following order of precedence (highest to lowest):

1. **Command Line Flags** (e.g., `--port 4001`)
2. **Environment Variables**:
   - `OXIDE_CONTENT` — Override the content directory path (default: `content/`)
3. **Configuration File** (`content/server.toml`)
4. **Built-in Defaults**

### 6. Deployment & Host Environment Considerations

#### Host Firewall Configuration

By default, the server listens on TCP port `4000`. You must configure your host VPS or server firewall to accept incoming connections on this port:

- **Ubuntu/Debian (UFW)**:
  ```bash
  sudo ufw allow 4000/tcp
  ```
- **RHEL/CentOS (Firewalld)**:
  ```bash
  sudo firewall-cmd --add-port=4000/tcp --permanent
  sudo firewall-cmd --reload
  ```

#### SQLite WAL Storage Restrictions (Docker & VPS)

The persistence layer operates in SQLite **Write-Ahead Logging (WAL)** mode. WAL requires robust support for shared memory (`mmap`) and file locking (`fcntl`).

- [!WARNING]
  > **Do NOT use Network Volumes**: Do not place the database file or bind-mount the `./data` directory over network-mounted filesystems (e.g., NFS, AWS EFS, Samba/CIFS, or VM shared folders like VirtualBox/Vagrant folders). Doing so will prevent locking and lead to database corruption or engine crashes.
- **Local Storage only**: Ensure the host `./data` folder is mounted on standard local filesystems (e.g., ext4, xfs, APFS, or NTFS).

#### Windows PowerShell Installation Policy

When deploying on Windows hosts, execution security policies will block the execution of the unsigned installer script by default. Run the installer by explicitly bypassing the execution policy for that session:

```powershell
PowerShell -ExecutionPolicy Bypass -File .\install.ps1
```

---

## Server Configuration (`content/server.toml`)

The server configuration file controls global settings. A sample configuration file is structured as follows:

```toml
# server.toml

# The display name of the MUD server
server_name = "OxideMUD"

# Maximum number of concurrent connections
max_clients = 256

# The default prompt template assigned to new characters
default_prompt = "<%h/%Hhp %m/%Mmp> "

[logging]
# Number of days to retain server logs before pruning
retention_days = 5

# Rotation policy: "daily", "hourly", or "never"
rotation = "daily"

[api]
enabled = true
bind_addr = "0.0.0.0:8080"

[api.tls]
# Primary Option A: Automatic Let's Encrypt (Recommended for public production servers)
# OxideMUD will automatically request and renew TLS certificates for your domain.
acme_domain = "mud.example.com"
acme_email = "admin@example.com"

# Option B: Custom Certificate Files (Uncomment if using existing cert files)
# cert_path = "certs/server.crt"
# key_path = "certs/server.key"

# Option C: Automatic Self-Signed Dev Certs (For local testing of spade/mcp)
# auto_dev_cert = true

# WARNING: Advanced setting — Not recommended for direct public deployments.
# Disables TLS encryption on the server. Use ONLY if OxideMUD is bound strictly to loopback
# (127.0.0.1) behind a reverse proxy (e.g., Caddy, Nginx, Cloudflare Tunnel) that handles TLS upstream,
# or for local browser testing. Never expose unencrypted HTTP/WS directly to public interfaces.
# allow_insecure_http = false

[websocket]
enabled = true
ping_interval_secs = 30
max_message_size_bytes = 65536
```

#### TLS Precedence Order

When starting `oxide-server`, the engine evaluates `[api.tls]` settings in the following strict order of precedence (highest to lowest):

1. **Explicit Certificate Files (`cert_path` & `key_path`):** Custom or organization-managed SSL certificates.
2. **Automatic ACME / Let's Encrypt (`acme_domain` & `acme_email`):** Automatically requests and renews certificates from Let's Encrypt. Primary recommendation for public production deployments.
3. **Auto-Generated Self-Signed Dev Certs (`auto_dev_cert = true`):** Ephemeral in-memory TLS certificates for local testing with native clients (`spade`, `oxide-mcp`).
4. **Reverse Proxy / Plain Loopback (`allow_insecure_http = true`):** Plain HTTP/WS when bound to `127.0.0.1` behind a reverse proxy (Caddy, Nginx, Cloudflare Tunnel).

---

### Production Docker Deployment & Cloudflare Tunnel Setup (Default)

The recommended production deployment for OxideMUD uses **Docker Compose** with **Cloudflare Tunnel (`cloudflared`)** for zero-configuration TLS encryption, DDOS protection, and hidden VPS web ports.

#### Step-by-Step Cloudflare Tunnel Setup

1. **Create Cloudflare Account & Add Domain:**
   Ensure your domain DNS is managed by Cloudflare.
2. **Create Tunnel in Cloudflare Zero Trust Dashboard:**
   - Log into [dash.teams.cloudflare.com](https://dash.teams.cloudflare.com/) (free for up to 50 users).
   - Go to **Networks** -> **Tunnels** -> Click **Create a Tunnel**.
   - Select **Cloudflared** and enter a tunnel name (e.g. `oxide-mud`).
3. **Copy Tunnel Token:**
   - Copy the generated tunnel token string (`eyJh...`).
   - Create or edit `.env` in your deployment path and set:
     ```env
     TUNNEL_TOKEN=eyJh...
     ```
4. **Configure Public Hostname Route in Cloudflare Dashboard:**
   - On the **Public Hostnames** step:
     - **Subdomain / Domain:** e.g., `mud.example.com`
     - **Type:** `HTTP`
     - **URL:** `oxide-server:8080` _(uses internal Docker container service name)_
5. **Launch Docker Containers:**
   ```bash
   docker-compose up -d --build
   ```

#### Telnet vs. WebSockets Traffic Flow

- **Telnet (Port 4000):** Exposed directly on the host VPS. Desktop clients (TinTin++, Mudlet) connect to `mud.example.com:4000`.
- **WebSockets & REST API (Port 8080):** Bound internally to loopback (`127.0.0.1:8080`) and encrypted at Cloudflare Edge. Web clients, Spade, and MCP connect to `wss://mud.example.com/ws/*`.

#### Alternate Setup 1: Caddy Reverse Proxy (Non-Cloudflare)

If you prefer using Caddy for automatic Let's Encrypt certificates instead of Cloudflare Tunnel:

```yaml
services:
  caddy:
    image: caddy:latest
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - caddy_data:/data
  oxide-server:
    image: oxide-server:latest
    ports:
      - "4000:4000"
      - "127.0.0.1:8080:8080"
```

And a 3-line `Caddyfile`:

```caddy
mud.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

#### Alternate Setup 2: Built-in Native Let's Encrypt (Single Container)

Set `acme_domain = "mud.example.com"` and `acme_email = "admin@example.com"` in `content/server.toml`, and map port `443:8080` and `80:80` in `docker-compose.yml`.

> [!NOTE]
> Changes made directly to the `server.toml` file require a server restart to take effect. If you modify settings via the in-game `config` command, changes are written to the database and take effect immediately.

---

## Persistence and SQLite WAL Mode

The engine utilizes a **two-tier persistence model**:

1. **In-Memory ECS World**: Fast, active game state powered by the `hecs` ECS.
2. **SQLite On-Disk Database**: Persistent storage for characters, accounts, and server state.

### Write-Ahead Logging (WAL)

To support high concurrency and avoid blocking game loop executions during disk writes, the database is configured in **WAL mode** with the following parameters:

- `journal_mode = WAL` — Enables concurrent reads while writing.
- `synchronous = NORMAL` — Balance of safety and speed; transaction writes checkpoint sequentially.
- `busy_timeout = 5000` — Prevents locking failures by waiting up to 5 seconds for write access.

### Database Flush System

A background system (`DirtyFlushSystem`) ticks every **5 seconds** during the `DirtyFlush` phase:

- Identifies entities containing the `Dirty` marker component.
- Batches and persists modified component data to SQLite.
- Removes the `Dirty` component upon successful write.
- Performs a full database flush and WAL checkpoint on server shutdown.

### Automated Backups

The `BackupSystem` runs hourly during the `DirtyFlush` phase:

- Creates hot backups of the SQLite database using the online backup API (no locking).
- Backups are stored in `data/backups/`.
- **Retention Policy**: Keeps 7 daily backups and 4 weekly backups, pruning older backups automatically.

---

## Server Lifecycle

### Startup Flow

When launched, the server executes the following startup sequence:

1. **CliParse**: Evaluates command-line options.
2. **ConfigLoad**: Loads the configuration from the TOML file.
3. **LoggingInit**: Sets up the log file writers.
4. **ContentLoad**: Scans the template files under `content/` and compiles the `TemplateRegistry`. The content directory includes subdirectories for areas, mobs, items, races, classes, skills, shops, deities, affixes, sets, **passives**, **stances**, scripts, and top-level files for languages, socials, treasure classes, and server config.
5. **Validation**: Runs integrity checks on the templates.
6. **DatabaseOpen**: Connects to the SQLite database and executes migrations if needed.
7. **WorldCreate**: Seeds the initial ECS `World`.
8. **ScriptingInit**: Sets up the Rhai scripting runtime and resolver.
9. **ListenerBind**: Binds to the TCP port and initiates the game loop.

### Game Loop Ticks

The server runs a multi-tick background game loop using an asynchronous event-driven scheduler. Each tick interval fires independently:

| Tick         | Interval        | Phase               | Description                                            |
| ------------ | --------------- | ------------------- | ------------------------------------------------------ |
| Player State | 250ms           | `player_state_tick` | Processes decay of player stun and cast timers         |
| Skill Decay  | 1s              | `skill_decay_tick`  | Decrements cooldowns and temporary buff durations      |
| Combat Pulse | 2s              | `combat_tick`       | Runs combat rounds, stance systems, and AI ticks       |
| Maintenance  | 5s              | `maintenance_tick`  | Flushes dirty entities, saves positions, cleans groups |
| Set Bonus    | 10s             | `set_bonus_tick`    | Re-evaluates equipment set bonus thresholds            |
| Big Tick     | 30–90s (random) | `big_tick`          | Restores HP/MP/SP, broadcasts prompts to players       |

The tick intervals are not configurable at runtime. See `game_mechanics.md` for regen formulas and rest state multipliers.

### Graceful Shutdown

To trigger a graceful shutdown, administrators can send a `SIGINT` (Ctrl+C), a `SIGTERM` signal, or execute the in-game `shutdown` command.
The server will:

1. Close the TCP port listener.
2. Send a warning message to all connected players.
3. Wait **200ms** to drain any in-flight commands.
4. Mark all loaded entities as `Dirty` and perform a final full flush.
5. Checkpoint the WAL journal and close the SQLite database.
6. Disconnect all players and terminate.

---

## Welcome Banner and MOTD

The server displays an optional welcome ASCII banner and a Message of the Day (MOTD) to connections upon connecting.

### Banner File (`content/banner.txt`)

- Displayed first during the connection handshake.
- If the file is missing, empty, or fails to load, nothing is displayed.
- Defaults to `content/banner.txt`, but can be customized with `--banner-path <path>`.

### MOTD File (`content/motd.txt`)

- Displayed right after the server name, uptime, and game stats line.
- Available to players in-game via the `motd` command.
- If the file is missing, empty, or fails to load, nothing is displayed (and the `motd` command remains silent).
- Defaults to `content/motd.txt`, but can be customized with `--motd-path <path>`.

### Styling and Markup Format

Both the banner and the MOTD support OxideMUD's rich text markup tags for inline colors and styling. The parser formats text dynamically for ANSI-capable terminals and strips tags for plain-text connections.

#### Colors

- Colors: `{red}`, `{green}`, `{yellow}`, `{blue}`, `{magenta}`, `{cyan}`, `{white}`, `{black}`
- Bright/Vibrant colors: `{brightRed}`, `{brightGreen}`, `{brightYellow}`, `{brightBlue}`, `{brightMagenta}`, `{brightCyan}`, `{brightWhite}`, `{brightBlack}`
- Background colors: `{bg:red}`, `{bg:green}`, `{bg:blue}`, etc.

#### Modifiers

- Text modifiers: `{bold}`, `{italic}`, `{underline}`, `{blink}`, `{reverse}`

#### Tag Usage

- Close active styles with `{/}` or a specific closing tag (e.g. `{/bold}`).
- Escape braces by doubling them: `{{` renders as `{`, `}}` renders as `}`.

**Example MOTD:**

```text
{brightYellow bold}Welcome to OxideMUD!{/}
Type {cyan}help{/} to get started.
```

---

## Console and Logging

### Server Logs

Server logs are written to both standard output (`stdout`) and rotating files in the system's temporary directory (e.g., `/tmp` or OS-specific equivalent).

- Log file names follow the format: `oxide_server_log_YYYYMMDD_HHMMSS.log`.
- Log rotation is triggered based on the `logging.rotation` setting (e.g., daily).
- Expired logs exceeding the `logging.retention_days` threshold are pruned on startup.

### Console Commands

Administrators executing commands directly from the server console can access the following controls:

- **General Commands**
  - `help` — Show help listing all available console commands.
  - `save` — Force flush dirty entities to the SQLite database.
  - `broadcast <message>` — Send an administrative broadcast message to all connected players.
  - `online` (or `who`) — List all currently connected players in a tabular format showing their Entity ID, Username, Character Name, Access Level, and Location.
  - `kick <username_or_character>` — Disconnect an active player by their account username or character name.
  - `shutdown` — Gracefully stop the server.
  - `restart` — Gracefully stop (restart behavior is MUD-client handled).

- **Account Management**
  - `account list` — List all registered accounts in the SQLite database.
  - `account create <username> <password> [access_level]` — Create a new account with a hashed password and optional access level (`player`, `builder`, `immortal` / `imm`, `god`, `admin` / `adm`).
  - `account info <username>` — Show detailed account credentials, access levels, and login timestamps.
  - `account set-access <username> <level>` — Set the access tier of an account. Accepts short aliases: `imm` (immortal), `build` (builder), `play` (player), `adm` (admin). If the player is online, their active session access level is updated instantly.
  - `account set-password <username>` — Reset the password for the specified account.

- **Character & API Key Management**
  - `character set <character_name> <field> <value>` — Directly modify character fields (`level`, `xp`, `name`, `race`, `class`) on the database (and live session if connected).
  - `apikey generate <username> [description] [--scope mcp|spade] [--expires <duration>]` — Generate a new API key. Use `--scope mcp` for REST API access (AI agents), `--scope spade` for Spade online mode builder access, or both. Duration formats: `30d`, `2w`, `6m`, `1y`.
  - `apikey list` — List all active API keys with scopes, expiry, and descriptions.
  - `apikey revoke <key>` — Revoke and delete a specific API key.
  - `apikey scope <key> add <scope>` — Add a scope (`mcp` or `spade`) to an existing key.
  - `apikey scope <key> remove <scope>` — Remove a scope from a key.
