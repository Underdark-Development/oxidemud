# Spade Manual

**Spade** is the interactive terminal-based builder tool and MUD client. Named after the `@dig` command, it serves as the unified interface for creating rooms, editing templates, running validations, and testing the game live.

---

## Invocation Modes

Spade can be started in different modes depending on whether you are editing offline files or interacting with a running server.

| Command | Mode | Description |
| :--- | :--- | :--- |
| `spade` | Offline Builder | Default mode. Opens the local TOML template editor, file browser, and validation suite directly from disk. |
| `spade --mode online` | Online Client | Connects to a running game server as a player/administrator with scrollable output, collapsible macro sidebars, and right-click target interactions. |
| `spade --mode split` | Split Mode | Opens the builder tools on one half of the screen and the online MUD client on the other half. Toggle with `F9`. |
| `spade connect <host> <port>` | Quick Connect | Instantly connects to the specified game server using a saved profile. |

---

## Editor Panels and Screens (F1–F6)

Use the function keys to switch between primary workspace panels:

### F1: Entities Editor
A split-pane editor:
- **Left**: World tree listing all templates grouped by category (Races, Classes, Items, Mobs).
- **Center**: Visual form builder to edit properties without raw text manipulation.
- **Right**: Inspector panel displaying derived stats and runtime values.
- Press `F10` to preview the generated TOML.

### F2: Room Grid
An ASCII-art map displaying the layout of the world:
- Centers on the selected room and runs a Breadth-First Search (BFS) to map exits.
- **Right Pane**: Lists exits and portal keywords.
- **Bottom Bar**: Allows executing quick digging commands (e.g. `@dig north new_room_id`).

### F3: Validation Panel
An interactive diagnostics suite:
- Scans all files and lists errors (e.g., broken exits, invalid item level requirements) and warnings (e.g., missing descriptions).
- Double-clicking or pressing `Enter` on an error automatically jumps to the file and line containing the issue in the Entities Editor.

### F4: File Browser
A directory explorer for the `content/` path:
- **Left**: Directory tree.
- **Right**: Syntax-highlighted text preview of raw `.toml` templates and `.rhai` script files.

### F5: Script Console
An execution environment for Rhai scripting:
- Write and execute multi-line Rhai scripts.
- Contains a test runner that executes code enclosed in `//#test` and `//#end` blocks to run unit tests on scripts.

### F6: Live Dashboard
A diagnostics panel for running servers:
- Displays performance gauges, ticks per second, memory allocations, and database query latencies.
- Shows a real-time tail of the server's warning and audit logs.

---

## Keyboard and Controls Reference

### Navigation and Focus
- `Tab` / `Shift+Tab` — Cycle focus between active panes. The focused pane is highlighted with a bright white or cyan border.
- `Arrows (↑/↓/←/→)` — Navigate lists, forms, and maps.
- `Enter` — Open a folder, select a list item, or confirm an action.
- `Escape` — Go back, dismiss context menus, or close modal dialogs.
- `Ctrl+P` — Open the Command Palette to search and execute any editor action.
- `Ctrl+H` or `?` — Toggle the overlay help screen.
- `Ctrl+D` — Quit Spade.

### Editing Files
- `Ctrl+S` — Save modified templates (in offline mode).
- `Ctrl+Z` — Undo the last text edit.
- `Ctrl+C` / `Ctrl+V` — Copy and paste text.
- `/` — Open a search box to filter the active panel list.

### Online MUD Client Controls
- `Ctrl+B` — Toggle the macro buttons sidebar.
- `Ctrl+K` — Clear the text scrollback buffer.
- `Ctrl+L` — Toggle line numbers in the output window.
- `Ctrl+T` — Toggle timestamps on incoming text lines.
- `Ctrl+R` — Search command history.
- `Ctrl+U` — Clear the current input line.
- `Ctrl+A` / `Ctrl+E` — Jump to the beginning or end of the input line.

---

## Mouse and Scrolling Support

Spade features full mouse support, which can be toggled using `Ctrl+M`:
- **Left Click**: Select items, click sidebar command buttons, or select input fields.
- **Double Left Click**: Open templates, folders, or items.
- **Right Click**: Open context menus (e.g., right-clicking a player's name in MUD client mode to display actions like Stat, Tell, Goto, or Kick).
- **Scroll Wheel**: Scroll up and down through help text, file previews, list boxes, and the MUD client output buffer.
- **Pane Resizing**: Drag panel borders to resize splits.

---

## Connection Profiles (`~/.config/spade/profiles.toml`)

Spade stores host profiles in the user's home directory. You can pre-configure server parameters to quickly jump online:

```toml
# profiles.toml
[profiles.local]
host = "127.0.0.1"
port = 4000
mode = "telnet"
username = "admin"
tls = false

[profiles.production]
host = "play.oxidemud.org"
port = 443
mode = "websocket"
username = "staff_member"
tls = true
```
