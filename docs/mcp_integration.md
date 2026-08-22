# MCP Integration Guide

This guide is for developers and administrators setting up the **Model Context Protocol (MCP)** server. The MCP server allows AI agents (e.g., Claude Desktop, Cursor, or custom IDE plugins) to read, write, edit, and validate the MUD game world database and TOML templates.

---

## Architecture Overview

The MCP server connects AI workflows directly to the MUD content files. It runs as a separate process communicating via standard input/output (stdio) streams or server-sent events (SSE). When connected to a running game server, it can also operate over a WebSocket connection to the server's MCP endpoint.

```
[ AI Agent Client ] <--- stdio (JSON-RPC) ---> [ MCP Server Crate ] ---> [ Content TOMLs ]
                                                                     ---> [ SQLite Database ]
```

For live server administration, the running `oxide-server` exposes its own MCP endpoint over WebSocket at `/ws/mcp`. A remote `oxide-mcp` client (or any compliant MCP client) connects to that endpoint with an API key to operate on the live world in real time.

```
[ Remote MCP Client ] <--- WebSocket (JSON-RPC) ---> [ oxide-server /ws/mcp ] ---> [ Live World ]
```

---

## Invocation and Configuration

Start the MCP server using the `oxide-mcp` binary:

```bash
cargo run --bin oxide-mcp [options] [content_path]
```

### Server Modes & Low-Friction Execution

The MCP server supports two runtime modes:

| Mode             | Trigger / CLI Flags            | Transport / Data Source                   | Description                                                                                             |
| :--------------- | :----------------------------- | :---------------------------------------- | :------------------------------------------------------------------------------------------------------ |
| **Offline Mode** | Default                        | Stdio / Local `content/` TOML files       | Direct atomic TOML file editing and local gameplay simulation without requiring a running server.       |
| **Online Mode**  | `oxide-mcp --online` or `--ws` | WebSocket (`wss://.../ws/mcp`) / REST API | Connects to a live OxideMUD server over WebSockets for real-time agent execution and streaming updates. |

#### Low-Friction Online Mode Execution

AI Agents and developer tools can connect to a running server effortlessly. The `oxide-server` exposes a live MCP endpoint at `/ws/mcp`, authenticated by an API key (bearer token). The `oxide-mcp-client` binary connects to it and lists the live server's tools:

```bash
# 1. Quick online connect to default local server (ws://127.0.0.1:8080/ws/mcp):
oxide-mcp-client --ws ws://127.0.0.1:8080/ws/mcp --key <API_KEY>

# 2. Connect to a custom WebSocket URL:
oxide-mcp-client --ws wss://mud.example.com/ws/mcp --key <API_KEY>
```

The server-side MCP endpoint exposes live-world tools (connected players, player state, and a focused set of immortal operations) that operate directly on the running server's in-memory state — no separate process or REST hop required.

---

## Exposed MCP Tools

AI agents can execute the following tools via JSON-RPC calls:

### World Editing (CRUD)

- `list_areas` / `get_area` — Lists all registered areas or retrieves metadata for a single area.
- `create_area` / `update_area` / `delete_area` — Creates, updates, or deletes area configurations.
- `list_rooms` / `get_room` — Retrieves room structures.
- `create_room` / `update_room` / `delete_room` — Manages room instances.
- `link_rooms` — Connects two rooms via directional exits.
- `add_portal` / `remove_portal` — Manages keyword-based portals.
- `list_mobs` / `get_mob` / `create_mob` / `delete_mob` — Manages NPC templates.
- `list_items` / `get_item` / `create_item` / `delete_item` — Manages item templates.
- `list_shops` / `get_shop` — Lists shops or retrieves a single shop definition.

### Template Lists & Inspect

- `list_classes` — Lists all class templates in the registry.
- `list_races` — Lists all race templates.
- `list_skills` — Lists all skill templates.
- `list_stances` — Lists all stance templates.
- `list_passives` — Lists all passive trait definitions.
- `list_triggers` — Lists available item trigger event types.
- `get_template_raw` — Returns the raw TOML content for any template category + ID.
- `preview_room <area_id> <room_id>` — Renders a room as a player would see it (name, description, exits, contents).
- `preview_mob <mob_id>` — Renders a mob template's description and stats.
- `preview_item <item_id>` — Renders an item template's description and properties.
- `search` — Performs a fuzzy search across all template IDs, names, and descriptions.
- `get_stats` — Returns count summaries of races, classes, mobs, items, rooms, and areas.

### Validation

- `validate` — Runs full cross-reference validation on all templates, checking broken links, missing templates, attribute bounds, skill gates, and deity policies.
- `validate_area <area_id>` — Validates a single area file for structural integrity.
- `validate_content_dag` — Validates skill prerequisite trees for circular dependency loops.

### Simulation Tools

- `simulate_combat` — Simulates `N` combat rounds between two mob templates (or a mob and a player-level character). Returns round-by-round hit/miss/damage results and aggregate stats.
- `simulate_loot <mob_id> <iterations>` — Rolls loot drops from a mob template across multiple iterations and returns drop rate percentages.
- `simulate_ai_wander <mob_id> <start_room> <ticks>` — Simulates an NPC's AI wander path across a given number of ticks, reporting room visit frequency.
- `simulate_progression <race_id> <class_id> <start_level> <end_level>` — Simulates character level-by-level stat progression and returns the stat table.
- `simulate_gear_loadout <race_id> <class_id> <level> [items...]` — Simulates a character's final stats with a given set of equipped items.
- `simulate_shop_transaction <shop_id> <item_id>` — Simulates buy/sell pricing across reputation levels.
- `simulate_character_creation` — Simulates starting stats, pool calculations, and auto-learned skills for race and class combinations.
- `simulate_crafting` — Simulates crafting success probabilities, required stations, and recipe quality outputs.
- `simulate_skill_use` — Simulates skill usage checks, resource costs, and success rates.
- `simulate_prayer` — Simulates prayer buff effects, duration, cooldowns, and deity alignment checks.
- `simulate_prestige_eligibility` — Evaluates character stats/skills against prestige class prerequisites.
- `simulate_group_formation` — Simulates group formation positioning and tactical stat bonuses.
- `simulate_death_penalty` — Simulates XP loss, corpse creation, and ghost state transitions upon death.

### Online Immortal Tools (REST API Required)

When configured with `--url` and `--key`, the MCP server connects to a live server via the REST API bridge (`/api/imm/*`):

- `imm_put_item` — Spawns an item template directly into an online player's inventory.
- `imm_teleport` — Teleports an online player to a target room key.
- `imm_force_command` — Forces an online player entity to execute a command string (requires `confirm: true`).
- `imm_set_stat` — Sets character attributes, HP/mana/stamina pools, level, or XP.
- `imm_load_mob` — Spawns an NPC template directly into a specified room.
- `imm_load_item` — Spawns an item template directly into a specified room.
- `imm_gecho` — Broadcasts a global echo message to all online players.
- `imm_advance` — Advances a player to a target level.
- `imm_stat` — Inspects ECS components and stats of a player or NPC entity.
- `imm_heal` — Fully restores a target's HP, mana, and stamina.
- `imm_damage` — Deals direct damage to a target entity.
- `imm_kill` — Instantly kills a target entity (requires `confirm: true`).
- `imm_revive` — Revives a dead or ghost target entity.
- `imm_set_alignment` — Modifies a player's alignment.
- `imm_set_faction` — Adjusts a player's standing with a faction.
- `imm_purge_room` — Purges all NPCs and items from a room (requires `confirm: true`).
- `imm_reboot` — Initiates a graceful server reboot (requires `confirm: true`).

---

## Resources Schema URIs

The MCP server exposes the game templates as read-only resources using custom URI schemas:

- `content://areas/` — Index of all areas.
- `content://areas/{area_id}` — Raw area template content.
- `content://areas/{area_id}/rooms/{room_id}` — Raw room template content.
- `content://mobs/{mob_id}` — Mob template content.
- `content://items/{item_id}` — Item template content.
- `content://skills/{skill_id}` — Skill template content.
- `content://races/{race_id}` — Race template content.
- `content://classes/{class_id}` — Class template content.
- `content://validation/` — Current validation diagnostics list.
- `content://stats/` — Summary count metrics.

---

## Guided Prompts (Workflows)

Prompts provide structured, interactive templates for AI assistants:

- `create_area_flow` — Guided area design prompting for name, level ranges, theme, and room structures.
- `review_content` — Evaluates template files and provides balancing feedback.
- `balance_encounter` — Analyzes a room's spawned mob attributes against the target area level and suggests stat adjustments.
- `design_quest_chain` — Generates TOML structures for quest lines, ensuring prerequisites and objective rewards are linked correctly.

---

## Claude Desktop Configuration

To connect the OxideMUD MCP server to Claude Desktop, add the following configuration to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "oxide-builder": {
      "command": "oxide-mcp",
      "args": [
        "/path/to/oxidemud/content",
        "--url", "http://localhost:8080",
        "--key", "your-mcp-api-key"
      ]
    }
  }
}
```

The `--url` and `--key` flags connect the MCP server to a running OxideMUD
instance for online-only tools (player list, teleport, force, give item). When
omitted, the server runs in offline mode using only the local TOML content
files.

---

## API Key Authentication

MCP access requires an API key with the `mcp` scope. Generate one from the MUD server console:

```
apikey generate myagent --scope mcp
```

Keys can be scoped to `mcp` (REST API access for AI agents), `spade` (builder access via Spade TUI), or both. Keys support optional expiration (`--expires 30d`) and can be revoked at any time.
