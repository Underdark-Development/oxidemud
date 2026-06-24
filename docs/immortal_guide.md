# Immortal Guide

This manual is for the game's immortal staff (Builders, Immortals, Gods, and Admins). It describes their roles, commands, safety mechanics, and logging/auditing requirements.

---

## Permission Tiers

The engine organizes staff into five hierarchical access levels. Commands are gated, and each tier inherits all commands of the levels below it.

| Access Level | Role | Scope & Authority |
| :--- | :--- | :--- |
| `Player` | Player | Standard gameplay commands. |
| `Builder` | Builder | World-building, area maintenance, and template adjustments. |
| `Immortal` | Moderator | Player moderation, teleportation, diagnostic inspection, and chat channels. |
| `God` | Administrator | Character manipulation, item loading, purging, banning, and freezing. |
| `Admin` | System Owner | Server control, configuration overrides, shutdowns, and system diagnostics. |

> [!IMPORTANT]
> Command execution checks permission levels dynamically: `connection.access_level() >= command.access`. If a staff member attempts to run a command above their access level, the system rejects it.

---

## Command Registry

### Builder Commands
- `@award <player> <xp>` — Grants a specific amount of experience points to a player.
- OLC commands (`@area`, `@dig`, `@link`, `@set`, `@desc`, `@portal`, `@mob`, `@item`, `@load`). *See the [Builder Manual](builder_manual.md) for details.*

### Immortal Commands
- `goto <room_id / player>` — Teleports the immortal instantly to the specified room or player.
- `at <room_id / player> <command>` — Executes a command at the location of the specified target without moving the immortal.
- `force <player> <command>` — Forces a player to execute a command.
- `stat <target / item / room>` — Inspects the raw ECS components, attributes, and variables of an entity.
- `owhere` / `olocate <item_id>` — Locates all instances of a specific item template in the world.
- `gecho <text>` — Broadcasts a message to the entire server.
- `gtell <text>` — Sends a message on the private Immortal-only chat channel.
- `wizwho` — Lists all online staff and their current access level/visibility status.
- `wizin` — Toggles Incognito mode (hides your presence from players).
- `holylight` — Toggles Holy Light mode (lets you see hidden exits, invisible targets, and incognito staff).
- `@teleport <entity> <room_id>` — Teleports the specified entity to another room.
- `switch <mob_id>` — Possesses an NPC, taking control of its actions.
- `return` — Releases control of a possessed NPC, returning to the immortal character.

### God Commands
- `@purge [target]` — Deletes transient entities (e.g. mobs, items, corpses) from the current room.
- `@slay <player / mob>` — Instantly reduces a target's health to 0, killing them.
- `@restore <player>` — Fully restores a player's health, mana, stamina, and resource pools.
- `@clone <entity>` — Spawns an exact copy of the specified entity in the current room.
- `ban <account / IP>` / `unban <account / IP>` — Restricts or restores connection access for accounts or IPs.
- `freeze <player>` / `unfreeze <player>` — Locks a player's account to prevent them from executing any commands.
- `load <item_id / mob_id>` — Loads a new instance of an item or mobile into the current room.

### Admin Commands
- `shutdown` — Initiates a graceful shutdown of the server.
- `restart` — Gracefully restarts the server.
- `wizlock` — Locks the server, permitting only staff with `Builder` access or higher to connect.

---

## Core Staff Mechanics

### Incognito Mode (`wizin`)
When an immortal toggles `wizin`, they are hidden from standard gameplay systems:
- They are omitted from the public `who` list.
- Normal players looking at a room containing an incognito immortal will only see `"You sense a presence here."` rather than the immortal's name and description.
- Staff members with `God` access or higher who have `holylight` enabled can see through incognito mode.

### Holy Light (`holylight`)
Holy Light allows staff members to bypass standard visibility checks:
- See hidden exits and invisible characters.
- Detect incognito staff.
- Bypass zone darkness restrictions.

---

## Safety Invariants

To prevent accidental disruption of gameplay or abuse of staff commands, the command dispatcher enforces the following invariants:

1. **Combat Immunity**: Game systems skip damage calculations on entities containing the `Immortal` component. Immortals cannot be attacked or take damage from environmental hazards.
2. **Force Command Level Gate**: The `force` command checks the access level of both the executor and the target. A staff member cannot force an entity whose access level is equal to or higher than their own.
3. **Switch Command Target Restraints**: The `switch` command refuses player targets. Staff can only possess NPC entities.
4. **Purge Command Safeguards**: The `@purge` command rejects entities containing `Immortal` or `Player` components. It only operates on items, corpses, and NPCs.

---

## Auditing and Tracing

All administrative and destructive actions are logged for security and auditing purposes:
- Destructive commands (e.g., `slay`, `purge`, `ban`, `freeze`) emit high-priority trace logs via the `tracing::warn!` macro with a `"audit"` target.
- Logs include the administrator's account ID, the target name/ID, and the exact command executed.
- These events are written to the server's rotating log files and can be queried using the console's `audit` command.
