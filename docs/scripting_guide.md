# Scripting Guide — OxideMUD Engine

> **Status: Active.** The Rhai scripting engine and Rust↔Rhai bindings are fully integrated into the game loop. Script files inside `content/scripts/` are compiled, cached, and hot-reloaded at runtime.

---

## 1. Overview & Architecture

OxideMUD follows a strict **driver/content separation** architecture. The core Rust engine provides networking, ECS storage, database persistence, state machines, and system pulses. **All DIKU MUD gameplay content — skills, spells, mob AI, quest triggers, room behaviors, and item procs — is implemented in Rhai scripts.**

Rhai is an embedded, lightweight, sandboxed scripting language for Rust with zero dependencies.

---

## 2. The Security Sandbox

Since scripts can be written by world builders or reloaded live, the Rhai engine runs inside a strict sandboxed execution environment to protect server stability:

| Metric              | Bounded Limit      | Protection                                          |
| :------------------ | :----------------- | :-------------------------------------------------- |
| **Max Operations**  | 50,000 operations  | Prevents infinite loops and CPU hogging.            |
| **Max Call Stack**  | 32 call levels     | Prevents stack overflow crashes from recursion.     |
| **Loaded Modules**  | 8 modules          | Limits file imports.                                |
| **Max String Size** | 10,000 characters  | Prevents out-of-memory errors from buffer bloat.    |
| **Max Arrays**      | 100 dynamic arrays | Prevents heap-allocation exhaustion.                |
| **Max Maps**        | 50 key-value maps  | Prevents heap-allocation exhaustion.                |

Additionally, the scripting sandbox:
- **Filesystem isolation**: Resolves script files exclusively within `content/scripts/`.
- **No Network / OS Access**: Sockets, HTTP requests, shell calls, and database connections are unavailable inside scripts.

---

## 3. Script Lifecycle & Execution Context

### 3.1 Caching & Hot Reloading
1. **Compilation & Caching**: On startup, `ScriptEngine` scans `content/scripts/`, compiles all `.rhai` files into Abstract Syntax Trees (ASTs), and caches them in memory.
2. **Execution**: When a skill, spell, AI tick, or trigger fires, the engine uses thread-local execution context (`CURRENT_SCRIPT_CONTEXT`) to implicitly bind `world`, `actor`, `self`, `target`, and current `room`.
3. **Hot-Reloading**: A background file watcher monitors `content/scripts/`. When a script is edited on disk, its AST is automatically recompiled and updated in memory without restarting the server.

### 3.2 Implicit Thread-Local Context (`CURRENT_SCRIPT_CONTEXT`)
All script functions operate on a zero-parameter / zero-world clean API. Scripts **never** need to receive or pass `world` or `room` pointers for standard game operations:

```rhai
// World and room are inferred automatically from execution context
let rank = get_skill_rank(actor, "parry");
let name = actor.name();

send("You take a defensive stance.");
echo(actor.name() + " readies their weapon.");
```

---

## 4. Messaging API Reference

OxideMUD provides a clean, scoped messaging API. Room-scoped broadcasts automatically target the current room of the executing entity/actor:

| Function | Example Usage | Description |
| :--- | :--- | :--- |
| **`send`** | `send("You focus your energy.");` | Direct line to the current `actor` / `self`. |
| **`send_to` / `entity.send`** | `target.send("You take 15 damage.");` | Direct message to a specific entity handle. |
| **`echo`** | `echo("A rumble shakes the room.");` | Scoped broadcast to current execution room. |
| **`echo_except`** | `echo_except(name + " parries!", [actor, target]);` | Scoped broadcast to current room, excluding specified entities. |
| **`echo_to`** | `echo_to(room_handle, "A bell tolls in town.");` | Remote broadcast to an explicit room entity handle. |
| **`echo_to_except`** | `echo_to_except(room_handle, "Lightning strikes!", [actor]);` | Remote broadcast to an explicit room entity, excluding specified entities. |

---

## 5. Entity Handles, Methods & Properties

In Rhai scripts, player, NPC, item, and room handles expose both function calls and property getters:

```rhai
// Entity properties and getters
let name = actor.name;         // Property getter or actor.name()
let lvl = get_level(actor);    // Level query
let hp = get_hp(actor);        // Current HP
let max_hp = get_max_hp(actor);// Max HP

// Messaging method
target.send("You feel a strange force take hold!");
```

---

## 6. Cooldowns, Script Effects & Data-Driven Expiry

### 6.1 Cooldown Management
```rhai
// Set skill cooldown (duration in seconds)
set_cooldown(actor, "chain_lightning", 10);

// Query active cooldown
if is_on_cooldown(actor, "chain_lightning") {
    send("Chain Lightning is still on cooldown.");
    return;
}
```

### 6.2 Data-Driven Effect Expiry (`EffectExpireCondition`)
Core systems like `combat.rs` are 100% agnostic of specific skill or spell names. Active script effects specify their own expiration conditions via `EffectExpireCondition`:

- **`timer`**: Standard TTL duration countdown.
- **`exit_combat`**: Automatically expires when the entity leaves combat (`CombatState::NotInCombat`).
- **`change_stance`**: Automatically expires when the entity changes fighting stance.
- **`custom`**: Custom condition handled by script triggers.

### 6.3 Applying Effects (`apply_script_effect_full`)
```rhai
apply_script_effect_full(
    actor,
    "parrying",                             // Effect ID
    "Parrying Stance",                     // Display Name
    "Parry",                               // Source
    3600,                                  // Duration in seconds
    "Actively parrying incoming melee attacks", // Description for 'affects' command
    "",                                    // Name prefix override
    "",                                    // Name suffix override
    "",                                    // Short description override
    "",                                    // Visual aura shown on 'look'
    "You lower your guard and stop parrying.", // Expiration message broadcast to actor on removal
    #{
        "expire_conditions": ["exit_combat"] // Automatically deactivates when combat ends!
    }
);

// Query or remove active script effects
if has_script_effect(actor, "parrying") {
    remove_script_effect(actor, "parrying");
}
```

---

## 7. Engine Command Restrictions (`CommandRestrictions`)

The engine provides built-in quantitative restriction evaluation (`CommandRestrictions`). Designers specify usage conditions in `[skill]` or `[command]` definitions, eliminating hardcoded checks inside scripts:

- `allowed_classes`: e.g. `["warrior"]` or `["mage", "wizard"]`.
- `allowed_races`: e.g. `["elf", "human"]`.
- `min_level`: Level gate (e.g. `5`). Engine yields: *"You are not experienced enough to use that ability."*
- `min_skill_ranks`: Skill practice level requirements.
- `allowed_stances`: e.g. `["defensive"]`.
- `in_combat_only`: Skill only usable while in combat.
- `out_of_combat_only`: Skill only usable outside combat.

---

## 8. Comprehensive Example Scripts

### 8.1 Parry Skill (`content/scripts/skills/parry.rhai`)
Deflects incoming melee attacks based on practice rank (max 60% deflect chance at 100% practice). Automatically deactivates when combat ends.

```rhai
fn on_use() {
    let rank = get_skill_rank(actor, "parry");
    if rank == 0 {
        send("You have not practiced the art of parrying.");
        return;
    }

    if has_script_effect(actor, "parrying") {
        remove_script_effect(actor, "parrying");
        send("You lower your guard and stop attempting to parry incoming attacks.");
        echo(actor.name + " lowers their guard.");
        return;
    }

    let chance = (rank * 60) / 100;
    apply_script_effect_full(
        actor,
        "parrying",
        "Parrying Stance",
        "Parry",
        3600,
        "Actively parrying incoming melee attacks (Max 60% deflect chance)",
        "", "", "", "",
        "You lower your guard and stop parrying.",
        #{
            "expire_conditions": ["exit_combat"]
        }
    );

    send("You focus on parrying incoming attacks! (Parry Rank: " + rank + "%, Deflect Chance: " + chance + "%)");
    echo(actor.name + " readies their weapon, watching for incoming strikes.");
}

fn on_combat_hit() {
    if !has_script_effect(hit_ctx.target, "parrying") {
        return hit_ctx;
    }

    let rank = get_skill_rank(hit_ctx.target, "parry");
    if rank == 0 {
        return hit_ctx;
    }

    let chance = (rank * 60) / 100;
    if rand(1, 100) <= chance {
        let defender_name = hit_ctx.target.name;
        let attacker_name = hit_ctx.attacker.name;

        hit_ctx.target.send("You parry " + attacker_name + "'s attack!");
        hit_ctx.attacker.send(defender_name + " parries your attack!");

        echo_except(defender_name + " parries " + attacker_name + "'s attack!", [hit_ctx.target, hit_ctx.attacker]);
        hit_ctx.abort("");
    }

    return hit_ctx;
}
```

---

### 8.2 Chain Lightning Spell (`content/scripts/spells/chain_lightning.rhai`)
Strikes the primary target and arcs to up to 2 secondary targets in the room with decaying damage.

```rhai
fn on_use() {
    if is_on_cooldown(actor, "chain_lightning") {
        send("Chain Lightning is still on cooldown.");
        return;
    }

    set_cooldown(actor, "chain_lightning", 8);

    send("You unleash a crackling bolt of Chain Lightning!");
    echo(actor.name + " unleashes a crackling arc of lightning!");

    // Primary target hit (30-50 damage)
    let main_dmg = rand(30, 50);
    target.send("A bolt of lightning strikes you for " + main_dmg + " damage!");
    echo_except("Lightning strikes " + target.name + " for " + main_dmg + " damage!", [actor, target]);

    // Arcs to up to 2 secondary targets in room with decaying damage
    let chained = 0;
    let room_occupants = entities_in_room();
    for occupant in room_occupants {
        if occupant != actor && occupant != target && chained < 2 {
            let chain_dmg = rand(15, 25);
            occupant.send("A stray arc of lightning leaps from " + target.name + " and zaps you for " + chain_dmg + " damage!");
            echo_except("Lightning arcs to " + occupant.name + " for " + chain_dmg + " damage!", [actor, occupant]);
            chained += 1;
        }
    }
}
```

---

### 8.3 Envenom Blade Command (`content/scripts/items/envenom_blade.rhai`)
Contextual item command granted by holding the Venomous Serpent Dagger.

```rhai
fn on_use() {
    if is_on_cooldown(actor, "envenom_blade") {
        send("You must wait before coating your blade again.");
        return;
    }

    set_cooldown(actor, "envenom_blade", 30);
    send("You coat your blade with lethal serpent venom!");
    echo(actor.name + " coats their dagger blade with a shimmering dark venom.");

    apply_script_effect_full(
        actor,
        "envenomed_weapon",
        "Envenomed Weapon",
        "Serpent Dagger",
        60,
        "Weapon coated in deadly serpent poison",
        "", "", "", "",
        "The venom on your blade evaporates.",
        #{}
    );
}
```

---

### 8.4 Poison Proc (`content/scripts/items/poison_proc.rhai`)
Weapon proc trigger attached to the serpent dagger, applying poison on strike.

```rhai
fn on_hit() {
    if rand(1, 100) <= 25 {
        send("Your serpent blade drips venom into " + target.name + "'s wound!");
        target.send("Deadly poison burns in your veins from " + actor.name + "'s weapon!");
        echo_except(actor.name + "'s blade poisons " + target.name + "!", [actor, target]);

        apply_script_effect_full(
            target,
            "serpent_poison",
            "Poisoned",
            "Serpent Dagger",
            6,
            "Suffering from deadly serpent venom",
            "", " (poisoned)", "", "looks pale and feverish",
            "The poison leaves your system.",
            #{}
        );
    }
}
```

---

### 8.5 Mob AI Trigger (`content/scripts/mobs/goblin.rhai`)
NPC AI tick trigger controlling goblin behavior.

```rhai
fn on_ai_tick() {
    let hp = get_hp(self);
    let max_hp = get_max_hp(self);

    if (hp * 100) / max_hp < 25 {
        echo(self.name + " shriek: 'Retreat! Fall back!'");
        flee();
    } else {
        if rand(1, 100) <= 15 {
            echo(self.name + " cackles maniacally and sharpens their rusted dagger.");
        }
    }
}
```

---

### 8.6 Room Door Trigger (`content/scripts/rooms/open_sesame.rhai`)
Room say hook opening a locked secret exit when a spoken password matches.

```rhai
fn on_say() {
    if message.contains("open sesame") || message.contains("please open") {
        if is_exit_closed("north") {
            set_exit_locked("north", false);
            set_exit_closed("north", false);
            echo("With a loud rumble, the heavy stone door to the north grinds open!");
        } else {
            send("The stone door is already open.");
        }
    }
}
```
