# MCP Integration Guide

This guide is for developers and administrators setting up the **Model Context Protocol (MCP)** server. The MCP server allows AI agents (e.g., Claude Desktop, Cursor, or custom IDE plugins) to read, write, edit, and validate the MUD game world database and TOML templates.

---

## Architecture Overview

The MCP server connects AI workflows directly to the MUD content files. It runs as a separate process communicating via standard input/output (stdio) streams or server-sent events (SSE).

```
[ AI Agent Client ] <--- stdio (JSON-RPC) ---> [ MCP Server Crate ] ---> [ Content TOMLs ]
                                                                     ---> [ SQLite Database ]
```

---

## Invocation and Configuration

Start the MCP server using the `oxide-mcp` binary:

```bash
cargo run --bin oxide-mcp [options] [content_path]
```

### Server Modes

The MCP server supports two runtime modes:

| Mode | Trigger | Data Source | Write Operation |
| :--- | :--- | :--- | :--- |
| **Offline Mode** | Default | Local TOML files under `content/` | Direct atomic write: writes content to a temporary file, validates, and then renames to target. |
| **Online Mode** | `mcp --db <path>` | Game SQLite Database | Executes commands using a REST HTTP/JSON bridge to the running game server. |

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
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "/absolute/path/to/mud/mcp/Cargo.toml",
        "--bin",
        "oxide-mcp",
        "--",
        "/absolute/path/to/mud/content"
      ]
    }
  }
}
```
