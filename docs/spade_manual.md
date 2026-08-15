# Spade Manual

**Spade** is the interactive terminal-based builder tool and MUD client. Named after the `@dig` command, it serves as the unified interface for creating rooms, editing templates, running validations, and testing the game live.

---

## Invocation Modes & WebSocket Connectivity

| Command                               | Mode              | Status      | Description                                                                    |
| :------------------------------------ | :---------------- | :---------- | :----------------------------------------------------------------------------- |
| `spade` / `spade --mode offline`      | Offline Builder   | Implemented | Full-screen local TOML template editor, room grid, file browser, & validation. |
| `spade --mode online --url wss://...` | Online Builder    | Implemented | Full-screen online builder syncing template edits to server via WebSockets.    |
| `spade connect wss://<host>:<port>`   | Standalone Client | Implemented | Full-screen MUD player client (telnet/WebSocket stream, macros, scrollback).   |
| `spade --mode split`                  | Split Mode        | Planned     | Dual-pane view: horizontal split with builder tools top & MUD client bottom.   |

### Runtime Mode Switching

You can switch execution modes dynamically while Spade is running without restarting the app:

1. Open the Command Palette using `Ctrl + P`.
2. Type `Switch Mode` to select:
   - `Switch Mode to Offline`
   - `Switch Mode to Online (WSS)`
   - `Switch Mode to Client`
   - `Switch Mode to Split`
3. The bottom status bar immediately updates to reflect the active mode (e.g. `[OFFLINE]`, `[ONLINE]`, `[CLIENT]`, `[SPLIT]`).

---

## Workspace Layout & Execution Modes

Spade supports four primary operational modes designed for different workflows:

1. **Offline Builder Mode (`spade` / `spade --mode offline`)**:
   - Dedicated full-screen workspace for offline content creation.
   - Access to the Entity Inspector (`F1`), Room Grid (`F2`), Template Validator (`F3`), File Browser (`F4`), and Rhai Script Console (`F5`).
2. **Online Builder Mode (`spade --mode online`)**:
   - Full-screen builder environment connected to a live server via WebSockets.
   - Synchronizes template changes, area edits, and live validation results directly with the server runtime.
3. **Standalone Client Mode (`spade connect` / `spade --mode client`)**:
   - Full-screen player terminal interface focused purely on playing the game.
   - Provides clean ANSI output stream, command input history, macros, and scrollback navigation.
4. **Split Mode (`spade --mode split`)**:
   - Hybrid workspace combining world builder tools and the live game client into a single interface.
   - **Horizontal Split Layout**: The screen is divided vertically into top and bottom regions (builder tools occupying the top pane, live game client stream occupying the bottom pane).
   - _Design Rationale_: A vertical (side-by-side) split is intentionally avoided because narrow column bounds make builder tree views, form inspectors, and client text log outputs unreadably cramped in standard terminal widths.

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

| Category              | Feature                                       | Status      |
| :-------------------- | :-------------------------------------------- | :---------- |
| **Offline Builder**   | Entity tree + inline inspector                | Implemented |
| **Offline Builder**   | Room grid (BFS map)                           | Implemented |
| **Offline Builder**   | Template validation                           | Implemented |
| **Offline Builder**   | File browser with preview                     | Implemented |
| **Offline Builder**   | Script console (editor + runner)              | Implemented |
| **Offline Builder**   | Command Palette (Ctrl+P)                      | Implemented |
| **Offline Builder**   | Menu bar (File/Edit/View)                     | Implemented |
| **Offline Builder**   | Mouse support (click, scroll)                 | Implemented |
| **Offline Builder**   | TOML save-to-disk                             | Implemented |
| **Online Client**     | Telnet connection                             | Planned     |
| **Online Client**     | Macro sidebar                                 | Planned     |
| **Online Client**     | Scrollback buffer                             | Planned     |
| **Online Client**     | Command history                               | Planned     |
| **Split Mode**        | Top/bottom split (builder top, client bottom) | Planned     |
| **F6 Live Dashboard** | Performance gauges, log tail                  | Planned     |
| **Right-click Menus** | Context menus on entities                     | Planned     |
| **Pane Resizing**     | Drag panel borders                            | Planned     |
