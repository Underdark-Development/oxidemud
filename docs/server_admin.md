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

For automated remote deployments from a local administrator machine, the distribution package includes an Ansible playbook ([deploy.yml](file:///Users/therealklanni/Projects/oxidemud/ansible/deploy.yml)). This playbook targets the precompiled binaries and assets contained within the unpacked package.

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

### 4. CLI Command Options

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

### 5. Deployment & Host Environment Considerations

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
```

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

| Tick          | Interval        | Phase                | Description                                      |
| ------------- | --------------- | -------------------- | ------------------------------------------------ |
| Player State  | 250ms           | `player_state_tick`  | Processes decay of player stun and cast timers   |
| Skill Decay   | 1s              | `skill_decay_tick`   | Decrements cooldowns and temporary buff durations|
| Combat Pulse  | 2s              | `combat_tick`        | Runs combat rounds, stance systems, and AI ticks |
| Maintenance   | 5s              | `maintenance_tick`   | Flushes dirty entities, saves positions, cleans groups |
| Set Bonus     | 10s             | `set_bonus_tick`     | Re-evaluates equipment set bonus thresholds      |
| Big Tick      | 30–90s (random) | `big_tick`           | Restores HP/MP/SP, broadcasts prompts to players |

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

Administrators executing commands directly from the server console or via game client connections with `Admin` credentials can access the following controls:

- `shutdown` — Initiates the graceful shutdown sequence.
- `restart` — Gracefully saves state and reboots the process.
- `wizlock` — Toggles restricting server entry to Immortals only.
- `version` — Displays current engine version and build details.
- `audit` — Reviews the admin action audit log database.
