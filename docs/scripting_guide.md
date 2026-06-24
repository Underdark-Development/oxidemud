# Scripting Guide

The game engine separates the driver (core engine features) from the game content. While TOML templates cover static data structures, all dynamic gameplay logic is written in **Rhai** scripts located in the `content/scripts/` directory.

Rhai scripts drive NPC AI, item spells/procs, quest triggers, room behaviors, and custom player commands.

---

## The Security Sandbox

Since scripts can be loaded dynamically or created by builders, the Rhai engine runs inside a sandboxed execution environment with strict resource limits:

| Metric | Bounded Limit | Prevention |
| :--- | :--- | :--- |
| **Max Operations** | 50,000 operations | Prevents infinite loops and CPU hogging. |
| **Max Call Stack** | 32 call levels | Prevents stack overflow crashes from deep recursion. |
| **Loaded Modules** | 8 modules | Limits file imports. |
| **Max String Size**| 10,000 characters | Prevents out-of-memory errors from buffer bloat. |
| **Max Arrays** | 100 dynamic arrays | Prevents heap-allocation exhaustion. |
| **Max Maps** | 50 key-value maps | Prevents heap-allocation exhaustion. |

Additionally, the scripting sandbox:
- Restricts file access: The Rhai script resolver only reads files inside `content/scripts/`.
- Disables network I/O: Socket, HTTP, and database bindings are unavailable.

---

## Script Lifecycle

1. **Caching**: During startup, the engine scans the `content/scripts/` directory, compiles all `.rhai` files into Abstract Syntax Trees (ASTs), and caches them in memory.
2. **Event Binding**: Script triggers are linked via TOML template definitions (e.g. attaching a trigger to an item's `on_use` event).
3. **Execution**: When an event fires, the `ScriptTriggerSystem` fetches the cached AST, instantiates a fresh sandboxed engine, injects the context variables, and executes the script.
4. **Hot-Reloading**: The engine runs a file watcher on the `scripts/` directory. When a `.rhai` file is modified, the engine compiles it and updates the cache. If compilation fails, the error is logged and the last known good AST remains active.

---

## Script Execution Context (`ScriptCtx`)

When a script is triggered, the engine injects three global handles into the script's scope:

- `self` — The entity the script is running on (e.g., the item, room, or mob).
- `actor` — The entity that triggered the event (usually a player).
- `target` — The recipient of the action (if applicable).

---

## Scripting API Reference

### `EntityHandle`
Represents an active player, NPC, or item in the game world.

```rust
// Get character properties
let name = actor.name();
let lvl = actor.level();
let hp = actor.health();
let is_pc = actor.is_player();

// Send output message to the player connection
actor.send("You feel a sudden chill run down your spine.");

// Force target to speak
self.say("Intruders will be incinerated!");

// Retrieve the room the entity resides in
let room = actor.room();
```

### `RoomHandle`
Represents a room in the game world.

```rust
// Print message to all occupants in the room
self.echo("A low rumble shakes the cavern walls.");

// Get list of entities inside the room
let entities = self.entities();

// Inspect exit destinations
let dest = self.exit_destination("north");
```

### `WorldHandle`
Provides system-level control to spawn or manipulate world state.

```rust
// Spawn a mobile template into the world
let mob = world.spawn_mob("goblin_scout", actor.room());

// Despawn an entity
world.remove_entity(target);

// Grant experience points
world.grant_xp(actor, 500);

// Learn a crafting recipe
world.grant_recipe(actor, "iron_shortsword");
```

---

## Example Script Trigger

Here is an example script for a scroll of teleportation (`content/scripts/scroll_teleport.rhai`):

```rust
// Scroll of Teleportation
if actor.is_player() {
    actor.send("You read the ancient runes on the scroll...");
    
    // Check if the area allows summoning/teleportation
    let room = actor.room();
    if room.has_flag("no_teleport_out") {
        actor.send("A magical ward blocks your teleportation spell!");
        return;
    }
    
    // Choose destination and teleport
    let dest_room = "midgaard.temple_square";
    actor.send("Your body dissolves into shimmering dust!");
    room.echo(actor.name() + " vanishes in a flash of light.");
    
    world.teleport_entity(actor, dest_room);
    
    let target_room = world.get_room(dest_room);
    target_room.echo(actor.name() + " appears in a flash of light.");
    
    // Consume scroll item
    world.remove_entity(self);
}
```
