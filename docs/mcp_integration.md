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

Start the MCP server using the `mud_mcp` binary:

```bash
cargo run --bin mud_mcp [options] [content_path]
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
- `list_mobs` / `get_mob` / `create_mob` / `update_mob` / `delete_mob` — Manages NPC templates.
- `list_items` / `get_item` / `create_item` / `update_item` / `delete_item` — Manages item templates.
- `list_quests` / `get_quest` / `create_quest` / `update_quest` / `delete_quest` — Manages quest lines.
- `list_recipes` / `get_recipe` / `create_recipe` — Manages crafting recipes.
- `list_factions` / `get_faction` / `create_faction` — Manages faction definitions.
- `list_shops` / `get_shop` / `create_shop` — Manages shop vendors.

### Validation & Search
- `validate` — Runs the game's compilation validator. Accepts a `scope` parameter (`"all"`, `"area"`, or `"type"`) to narrow validation targets and returns a list of errors and warnings.
- `search` — Performs a fuzzy search across all template IDs, names, and descriptions.
- `get_stats` — Returns count summaries of races, classes, mobs, items, rooms, and areas.

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

To connect the MUD MCP server to Claude Desktop, add the following configuration to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "mud-builder": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "/absolute/path/to/mud/mcp/Cargo.toml",
        "--bin",
        "mud_mcp",
        "--",
        "/absolute/path/to/mud/content"
      ]
    }
  }
}
```
