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

> **Status note:** Most staff-specific OLC and moderation commands are planned. Currently only `@award` is implemented (builder level). See the [Builder Manual](builder_manual.md) for planned OLC command reference.

### Player Commands (available to all access levels)

#### General
- `look` / `l` — Look at the room, a direction, or a target
- `help` / `h` / `?` — Show available commands or help for a specific command
- `who` — List connected players
- `quit` / `exit` — Disconnect from the game
- `motd` — Show message of the day
- `commands` — List all commands you can use
- `width <columns>` — Set screen width for text wrapping (0 = unlimited)
- `time` — Show current in-game time
- `weather` — Show current weather conditions

#### Character
- `score` / `stats` — Display character stats, attributes, and resources
- `train` — List/train attributes using practice points (requires trainer)
- `practice` — List/practice skills (requires trainer)
- `sit` — Sit down (increases regen)
- `rest` — Rest (faster regen)
- `sleep` — Go to sleep (maximum regen)
- `wake` — Wake from sleep
- `stand` — Stand up (enables movement and combat)
- `die` — Submit to death when unconscious (become a ghost)
- `reclaim` — Reclaim your corpse to return to life
- `revive` — Pray at a temple altar to return to life
- `toggle` — Toggle player settings (`toggle resurrect`)
- `prompt` — View/set custom prompt template
- `pray` — Pray to your deity or at a deity's shrine

#### Communication
- `say <message>` — Speak aloud in the room
- `tell <player> <message>` — Send a private message to any online player
- `reply` / `r` — Reply to the last player who messaged you
- `shout <message>` — Shout to all players in the same area
- `whisper <player> <message>` — Whisper to a player in the same room

#### Movement
- `north` / `n`, `south` / `s`, `east` / `e`, `west` / `w`, `up` / `u`, `down` / `d`
- `northeast` / `ne`, `northwest` / `nw`, `southeast` / `se`, `southwest` / `sw`
- `open <direction>` — Open a closed door
- `close <direction>` — Close an open door
- `lock <direction>` — Lock a door (requires key)
- `unlock <direction>` — Unlock a door (requires key)

#### Combat
- `kill <target>` — Attack a target
- `flee` — Attempt to flee from combat
- `stance` — View/set combat stance (normal, defensive, aggressive, berserk)

#### Items
- `inventory` / `inv` / `i` — List carried items
- `equipment` / `eq` — Show worn/wielded equipment
- `get` / `take <item>` — Pick up an item
- `drop <item>` — Drop a carried item
- `put <item> <container>` — Place item in container (not yet wired)
- `give <item> <player>` — Give item to another (not yet wired)
- `loot <corpse>` — Take all items from a corpse
- `wear <item>` — Wear a piece of armor
- `wield <item>` — Wield a weapon
- `remove <slot>` — Unequip an item to inventory
- `examine` / `exa` — Inspect an item's detailed properties

### Builder Commands
- `@award <xp>` — Grants XP to yourself (for testing). Currently the only implemented staff command.
- (Planned: `@area`, `@dig`, `@link`, `@set`, `@desc`, `@portal`, `@mob`, `@item`, `@load`, `@validate` — see [Builder Manual](builder_manual.md))

### Immortal Commands (planned)
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

### God Commands (planned)
- `@purge [target]` — Deletes transient entities (e.g. mobs, items, corpses) from the current room.
- `@slay <player / mob>` — Instantly reduces a target's health to 0, killing them.
- `@restore <player>` — Fully restores a player's health, mana, stamina, and resource pools.
- `@clone <entity>` — Spawns an exact copy of the specified entity in the current room.
- `ban <account / IP>` / `unban <account / IP>` — Restricts or restores connection access for accounts or IPs.
- `freeze <player>` / `unfreeze <player>` — Locks a player's account to prevent them from executing any commands.
- `load <item_id / mob_id>` — Loads a new instance of an item or mobile into the current room.

### Admin Commands (planned)
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
