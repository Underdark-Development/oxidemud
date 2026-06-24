# Server Administration Guide

This guide is intended for server administrators, systems engineers, and game owners responsible for configuring, deploying, maintaining, and backing up the MUD game server.

---

## Starting the Server

The server binary is compile-run via Cargo or executed directly from target release builds:

```bash
cargo run --bin mud_server [options]
```

### CLI Command Options

You can customize the server behavior at launch using the following command-line flags:

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--host <IP>` | `-h` | `127.0.0.1` | The bind IP address for the game listener. |
| `--port <port>` | `-p` | `4000` | The bind TCP port. |
| `--db-path <path>`| `-d` | `data/mud.db` | Path to the SQLite persistence database. |
| `--motd-path <path>`| `-m`| `content/motd.txt`| Path to the Message of the Day file. |
| `--config-path <path>`| `-c`| `content/server.toml`| Path to the server configuration file. |

#### Precedence Order
Startup configuration parameters are applied in the following order of precedence (highest to lowest):
1. **Command Line Flags** (e.g., `--port 4001`)
2. **Environment Variables** (e.g., `MUD_CONTENT`)
3. **Configuration File** (`content/server.toml`)
4. **Built-in Defaults**

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
default_prompt = "<%hhp %hmhp> "

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
4. **ContentLoad**: Scans the template files under `content/` and compiles the `TemplateRegistry`.
5. **Validation**: Runs integrity checks on the templates.
6. **DatabaseOpen**: Connects to the SQLite database and executes migrations if needed.
7. **WorldCreate**: Seeds the initial ECS `World`.
8. **ScriptingInit**: Sets up the Rhai scripting runtime and resolver.
9. **ListenerBind**: Binds to the TCP port and initiates the game loop.

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

## Console and Logging

### Server Logs
Server logs are written to both standard output (`stdout`) and rotating files in the system's temporary directory (e.g., `/tmp` or OS-specific equivalent).
- Log file names follow the format: `mud_server_log_YYYYMMDD_HHMMSS.log`.
- Log rotation is triggered based on the `logging.rotation` setting (e.g., daily).
- Expired logs exceeding the `logging.retention_days` threshold are pruned on startup.

### Console Commands
Administrators executing commands directly from the server console or via game client connections with `Admin` credentials can access the following controls:

- `shutdown` — Initiates the graceful shutdown sequence.
- `restart` — Gracefully saves state and reboots the process.
- `wizlock` — Toggles restricting server entry to Immortals only.
- `version` — Displays current engine version and build details.
- `audit` — Reviews the admin action audit log database.
