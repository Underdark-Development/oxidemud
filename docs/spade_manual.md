# Spade Manual

**Spade** is the interactive terminal-based builder tool and MUD client. Named after the `@dig` command, it serves as the unified interface for creating rooms, editing templates, running validations, and testing the game live.

---

## Invocation Modes

| Command | Mode | Status | Description |
| :--- | :--- | :--- | :--- |
| `spade` | Offline Builder | **Implemented** | Opens the local TOML template editor, file browser, and validation suite directly from disk. |
| `spade --mode online` | Online Client | **Planned** | Will connect to a running game server as a player/administrator. |
| `spade --mode split` | Split Mode | **Planned** | Will open builder tools and MUD client side-by-side. |
| `spade connect <host> <port>` | Quick Connect | **Planned** | Will connect to a game server using a saved profile. |

> Only **offline builder** mode is currently implemented. Online and split modes parse from CLI but are not wired.

---

## Editor Panels and Screens (F1–F6)

Use the function keys to switch between primary workspace panels:

### F1: Entities Editor
A split-pane editor:
- **Left**: World tree listing all templates grouped by category (Races, Classes, Items, Mobs, Areas, Skills, Stances, Passives, Affixes, Sets).
- **Right**: Inline field editor with text, number, multiline, and dropdown edit modes. Supports dirty tracking, undo for field edits, and delete with confirmation.
- Context-sensitive **Command Sidebar** (right edge) showing available actions for the selected entity.

### F2: Room Grid
An ASCII-art map displaying the layout of the world:
- Centers on the selected room and runs a Breadth-First Search (BFS) to map exits.
- **Right Pane**: Lists exits and portal keywords.
- **Bottom Bar**: Allows executing quick digging commands (e.g. `@dig north new_room_id`).

### F3: Validation Panel
An interactive diagnostics suite:
- Runs `TemplateRegistry::validate()` and lists errors (broken exits, invalid field values, etc.) and warnings (missing descriptions).
- Errors displayed in a sortable table with columns: Type, ID, Field, Message.

### F4: File Browser
A directory explorer for the `content/` path:
- **Left**: Directory tree with expand/collapse.
- **Right**: Syntax-highlighted text preview of raw `.toml` templates and `.rhai` script files.

### F5: Script Console
An execution environment for Rhai scripting:
- Write and execute multi-line Rhai scripts in a built-in editor with syntax highlighting (keyword colors, string highlighting, comment dimming).
- **Console Output** pane shows script results and error messages.
- Press `F9` to run the script. The engine discovers test functions (any function whose name starts with `test_`) and runs them in a try/catch harness.
- Load script files from the File Browser via double-click / Enter.

### F6: Live Dashboard (Planned)
A diagnostics panel for running servers — **not yet implemented**:
- Will display performance gauges, ticks per second, memory allocations, and database query latencies.
- Will show a real-time tail of the server's warning and audit logs.

---

## Keyboard and Controls Reference

### Navigation and Focus
- `Tab` / `Shift+Tab` — Cycle focus between active panes and the sidebar. The focused pane is highlighted with a bright white or cyan border.
- `Arrows (↑/↓/←/→)` — Navigate lists, forms, and maps.
- `Enter` — Open a folder, select a list item, or confirm an action.
- `Escape` — Go back, dismiss context menus, or close modal dialogs.
- `Ctrl+P` — Open the Command Palette to search and execute any editor action.
- `Ctrl+H` or `?` — Toggle the overlay help screen.
- `Ctrl+D` — Quit Spade.

### Editing Files
- `Ctrl+S` — Save modified templates (persists TOML to disk).
- `Ctrl+Z` — Undo the last field edit.
- `/` — Open a search box to filter the active panel list.

### Script Console
- `F9` — Run the current script and its test functions.
- `Tab` — Switch focus between the script editor (top) and console output (bottom).

### General
- `Ctrl+M` — Toggle mouse support.

---

## Mouse and Scrolling Support

Spade features full mouse support, which can be toggled using `Ctrl+M`:
- **Left Click**: Select items, click sidebar command buttons, or select input fields.
- **Double Left Click**: Open templates, folders, or items.
- **Scroll Wheel**: Scroll up and down through help text, file previews, list boxes, and script editor / console output.
- `Ctrl+Click` on an entity preview in entities-screen table to navigate to that entity's detail view.

---

## Connection Profiles (`~/.config/spade/config.toml`)

Spade stores configuration in the user's home directory. You can pre-configure server parameters for future online mode:

```toml
[connection]
host = "127.0.0.1"
port = 4000
username = "admin"
tls = false

[prefs]
mouse = true
scrollback_size = 5000
sidebar_open = true
```

---

## Feature Status Summary

| Category | Feature | Status |
| :--- | :--- | :--- |
| **Offline Builder** | Entity tree + inline inspector | Implemented |
| **Offline Builder** | Room grid (BFS map) | Implemented |
| **Offline Builder** | Template validation | Implemented |
| **Offline Builder** | File browser with preview | Implemented |
| **Offline Builder** | Script console (editor + runner) | Implemented |
| **Offline Builder** | Command Palette (Ctrl+P) | Implemented |
| **Offline Builder** | Menu bar (File/Edit/View) | Implemented |
| **Offline Builder** | Mouse support (click, scroll) | Implemented |
| **Offline Builder** | TOML save-to-disk | Implemented |
| **Online Client** | Telnet connection | Planned |
| **Online Client** | Macro sidebar | Planned |
| **Online Client** | Scrollback buffer | Planned |
| **Online Client** | Command history | Planned |
| **Split Mode** | Side-by-side builder + client | Planned |
| **F6 Live Dashboard** | Performance gauges, log tail | Planned |
| **Right-click Menus** | Context menus on entities | Planned |
| **Pane Resizing** | Drag panel borders | Planned |
