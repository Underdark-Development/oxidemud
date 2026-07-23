# Scripting Guide

> **Status: Active.** The Rhai engine and Rust↔Rhai bindings are fully integrated and wired into the game loop. Script files inside `content/scripts/` are compiled, cached, and hot-reloaded at runtime.

---

The game engine separates the driver (core engine features) from the game content. While TOML templates cover static data structures, all dynamic gameplay logic is written in **Rhai** scripts located in the `content/scripts/` directory.

Rhai scripts drive NPC AI, item spells/procs, quest triggers, room behaviors, and custom player interactions.

---

## The Security Sandbox

Since scripts can be loaded dynamically or created by builders, the Rhai engine runs inside a sandboxed execution environment with strict resource limits:

| Metric              | Bounded Limit      | Prevention                                           |
| :------------------ | :----------------- | :--------------------------------------------------- |
| **Max Operations**  | 50,000 operations  | Prevents infinite loops and CPU hogging.             |
| **Max Call Stack**  | 32 call levels     | Prevents stack overflow crashes from deep recursion. |
| **Loaded Modules**  | 8 modules          | Limits file imports.                                 |
| **Max String Size** | 10,000 characters  | Prevents out-of-memory errors from buffer bloat.     |
| **Max Arrays**      | 100 dynamic arrays | Prevents heap-allocation exhaustion.                 |
| **Max Maps**        | 50 key-value maps  | Prevents heap-allocation exhaustion.                 |

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

When a script is triggered, the engine injects global handles into the script's scope:

- `self` — The entity the script is running on (e.g., the item, room, or mob).
- `actor` — The entity that triggered the event (usually a player).
- `target` — The recipient of the action (if applicable).
- `world` — Access to the game world.

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

### Messaging API Reference

| Function | Example Usage | Description |
| :--- | :--- | :--- |
| **`send`** | `send("You feel a surge of energy.");` | Direct line to the current `actor`. |
| **`entity.send` / `send_to`** | `target.send("You take 5 damage.");` | Direct line to a specific entity. |
| **`echo`** | `echo("A low rumble shakes the cavern.");` | Scoped broadcast to the current execution room (actor or self's room). |
| **`echo_except`** | `echo_except(def + " parries " + atk + "'s attack!", [actor, target]);` | Scoped broadcast to current room, excluding listed entities. |
| **`echo_to`** | `echo_to(town_square, "The cathedral bell tolls.");` | Remote broadcast to any explicit room entity. |
| **`echo_to_except`** | `echo_to_except(room, "A lightning strike shakes the area!", [actor]);` | Remote broadcast to any explicit room entity, excluding specified entities. |


### Clean Execution Context (Zero `world` & Zero `room` Parameters)

All script functions automatically resolve `world`, `actor`, `self`, and current `room` from the execution context. **Scripts never need to receive or pass `world` or `room` pointers for standard game operations.**

```rust
// Character & Room Querying (World inferred implicitly)
let rank = get_skill_rank(actor, "parry");
let name = actor.name();
let room = actor.room();

// Cooldown & Effect Management (World inferred implicitly)
set_cooldown(actor, "chain_lightning", 10);
if is_on_cooldown(actor, "chain_lightning") { ... }

apply_script_effect_full(
    target,
    "serpent_poison",
    "Poisoned",
    "Serpent Venom",
    2,
    "Suffering from deadly serpent poison",
    "", " (poisoned)", "", "looks pale and shivering",
    "The poison leaves your veins.",
    #{}
);

// Messaging (Current Room & Actor inferred implicitly)
send("You focus on parrying incoming attacks!");
echo(actor.name() + " readies their weapon.");
echo_except(def + " parries " + atk + "'s attack!", [actor, target]);
```

### Dynamic Skills, Spells, and Entity Commands

Scripts can dynamically register global skills/spells, as well as contextual commands bound to specific rooms, items, or NPCs:

```rust
// Register a global player skill (e.g. direct command "parry")
register_skill("parry", "Parry", "parry", "skills/parry.rhai", "Parry incoming melee attacks.");

// Register a spell accessible via the `cast` command (e.g. `cast fireball`)
register_spell("fireball", "Fireball", "fireball", "spells/fireball.rhai", "Hurl a ball of fire at your target.");

// Register a contextual entity command (on an item, room, or mob)
register_entity_command(world, item_entity, "ignite", "scripts/ignite_sword.rhai", "Ignite your sword with elemental fire.");
```

### Script Effects & Visual Auras

Scripts can apply temporary status effects, buffs, debuffs, visual auras, and short description overrides:

```rust
// Apply a temporary script effect with full inspection & short description overrides
apply_script_effect_full(
    world,
    actor,
    "ignited_weapon",                     // Effect ID
    "Ignited",                             // Display Name
    "Ignite Sword",                        // Source
    120,                                   // Duration in seconds
    "Your blade burns with holy flame.",   // Custom 'affects' line
    "a flaming ",                          // Name prefix override (replaces 'a ')
    "",                                    // Name suffix
    "a steel longsword wreathed in fire",  // Short desc override
    "is blazing with holy fire.",          // Visual aura shown on 'look' / 'examine'
    "Your sword stops burning."            // Expiration message broadcast when TTL reaches 0
);

// Check or remove active script effects
if has_script_effect(world, actor, "ignited_weapon") {
    remove_script_effect(world, actor, "ignited_weapon");
}
```

---

## Example Script Trigger (on_say / Dialogue Trigger)

Here is an example say hook script for a lost sheep guard (`content/scripts/mobs/sheep_guard.rhai`):

```rust
// Attached to the town guard's on_say trigger
if message.contains("help") || message.contains("sheep") {
    if !world.is_on_quest(speaker, "save_the_sheep") && !world.has_completed_quest(speaker, "save_the_sheep") {
        world.accept_quest(speaker, "save_the_sheep");
        send_to(speaker, "The Guard says: 'Thank you! Please find my lost sheep.'");
    }
}
```

---

## Example Script Trigger (on_pickup / Item Trigger)

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

---

## Quest Scripts (on_accept, on_complete, on_update)

Quest script hooks are executed inside a specialized environment where they automatically receive variables related to the quest state:

- `quest_id`: The ID of the quest triggering the script.
- `rewards`: A map containing the rewards defined in the quest template:
  - `rewards.xp`: (integer) Experience rewarded.
  - `rewards.gold`: (integer) Gold rewarded.
  - `rewards.items`: (array of maps) Items rewarded, e.g. `[{ "item_template_id": "iron_ring", "count": 1 }]`.
  - `rewards.faction`: (array of maps) Faction standing modifications, e.g. `[{ "faction_id": "town_guard", "amount": 15 }]`.

### Example Quest Chain script (`content/scripts/quests/chain_quest.rhai`)

You can use `rewards` to custom-log or modify details, and automatically accept the next quest in a chain when the current quest is completed:

```rust
// Runs inside quest_a's on_complete hook script
send_to(player, "You completed " + quest_id + "! You earned " + rewards.xp + " XP and " + rewards.gold + " copper.");

// Accept Quest B automatically to form a quest chain
accept_quest(world, player, "quest_b");
send_to(player, "A new quest has started: Quest B!");
```
