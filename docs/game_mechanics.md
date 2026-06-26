# Game Mechanics

This document provides a reference for admins and world builders to understand the underlying mechanics, formulas, and systems that drive combat, equipment, and looting in the game engine.

---

## Combat System

Combat runs on a periodic **Combat Pulse (every 2 seconds)**. Each pulse, all engaged entities perform their attacks.

### 1. Attack Flow
For each combat round:
1. **Target Verification**: The system checks if the attacker and target exist, are alive, and are in the same room.
2. **Hit Check**: The attacker rolls a `d20` to determine if the attack lands (see hit formula below).
3. **Damage Calculation**: If the attack is a hit, the damage is calculated based on weapon/unarmed values, attributes, and styles (see damage formula below).
4. **Resistances and Buffs**: The target's resistances modify the incoming damage amount.
5. **Apply Damage**: The target's health is decremented. If health falls to 0 or below, death is triggered.
6. **Death and Rewards**: On death, XP is awarded to the attacker, a corpse is spawned containing the victim's inventory/equipment, and the victim is despawned.

### 2. To-Hit Formula
To determine if an attack hits:
\[ \text{Roll} + \text{Attacker Level} + \text{Strength Modifier} + \text{Dual Wield Penalty} \ge \text{AC} \]

- **Roll**: A random integer from 1 to 20 (`fastrand::i32(1..=20)`).
- **Attacker Level**: The level component of the attacking entity.
- **Strength Modifier**: Derived from the attacker's strength attribute (using standard D&D-style modifier: `(Strength - 10) / 2` integer division).
- **Dual Wield Penalty**: Applied only when dual-wielding (see Weapon Styles below).
- **Armor Class (AC)**: The defender's Armor Class.

#### Critical Hits & Automatic Misses
- **Natural 1**: An attack roll of 1 automatically misses, regardless of modifiers.
- **Natural 20 (Critical Hit)**: An attack roll of 20 automatically hits and deals **double damage** (total calculated damage is multiplied by 2).

### 3. Defense (Armor Class) Formula
An entity's Armor Class (AC) is calculated as:
\[ \text{AC} = 10 + \text{Level} + \text{Dexterity Modifier} + \text{Armor Rating} + \text{Shield Bonus} \]

- **Level**: The entity's current level.
- **Dexterity Modifier**: Derived from the entity's dexterity attribute (`(Dexterity - 10) / 2`).
- **Armor Rating**: The sum of base and bonus armor from all equipped items.
- **Shield Bonus**: A flat `+2` bonus to AC if a shield is equipped.

### 4. Damage Formula
The base damage of a successful hit depends on whether a weapon is equipped:

- **Armed (Melee/Ranged)**:
  \[ \text{Damage} = \text{Weapon Damage Roll} + \text{Strength Bonus} \]
  - **Weapon Damage Roll**: A roll from the weapon template's damage dice (e.g. `1d6` rolls a random value from 1 to 6).
  - **Strength Bonus**:
    - **One-Handed**: Full Strength Modifier.
    - **Two-Handed**: 1.5× Strength Modifier (rounded to nearest integer).
    - **Off-Hand (Dual-Wield)**: 0.5× Strength Modifier (rounded to nearest integer).
- **Unarmed**:
  \[ \text{Damage} = \text{1d4 Roll} + \text{Strength Modifier} + \text{Level} / 5 \]
  - **Level / 5**: Attacker's level divided by 5 (using integer division).

*Note: Final calculated damage is always clamped to a minimum of 1.*

---

## Damage Types & Resistances

The game supports 10 distinct damage types. Every source of damage (weapons, spells, environment) is typed, and entities can have individual resistance multipliers per damage type.

### Damage Types
1. **Slash**: Typical edge weapon damage (swords, axes).
2. **Pierce**: Typical pointed weapon damage (daggers, arrows, spears).
3. **Bludgeon**: Typical blunt weapon damage (maces, clubs, unarmed).
4. **Fire**: Elemental fire damage.
5. **Cold**: Elemental cold damage.
6. **Lightning**: Elemental electrical damage.
7. **Acid**: Corrosive chemical damage.
8. **Poison**: Toxic biological damage.
9. **Magic**: Non-elemental spell/arcane damage.
10. **True**: Pure damage that bypasses all standard resistances.

### Resistance Multipliers
Damage calculations are multiplied by the target's resistance profile for that type:

| Multiplier | State | Effect on Incoming Damage |
|---|---|---|
| `2.0` | Vulnerable | Double damage |
| `1.0` | Normal | Standard damage |
| `0.5` | Resistant | Halved damage (rounded) |
| `0.0` | Immune | Zero damage |
| `-1.0` | Absorbed | Reverses damage to heal the target |

---

## Weapon Styles

How a character chooses to wield their weapons changes their hit and damage output:

### 1. Two-Handed Style
- Wielding a single weapon designated as two-handed.
- Grants **1.5× Strength Modifier** bonus to damage calculations.
- Prevents equipping a shield or off-hand item.
- *Note: Weapon speed modifications (planned 1.2x) are not yet implemented.*

### 2. Dual-Wield Style
- Wielding a weapon in both the primary hand and the off-hand (shield slot).
- **Hit Penalties**:
  - Primary hand attack has a `-2` penalty to hit.
  - Off-hand attack has a `-4` penalty to hit.
- **Damage Modifiers**:
  - Primary hand weapon uses full Strength Modifier.
  - Off-hand weapon gets only **0.5× Strength Modifier** bonus.
- *Note: Weapon speed modifications, dual-attack timing, and the Ambidexterity skill mitigation are not yet implemented; only the raw damage and hit penalties are currently wired.*

---

## Corpse & Looting

When a character or NPC dies, their body is represented as a transient `Corpse` entity in the room.

### 1. Corpse Spawn & Decay
- The corpse is populated with the victim's inventory items and equipped items.
- Corpses are transient and are not saved to the SQLite database.
- **Decay Timers**:
  - **Player Corpses**: Decay after **10 minutes** (600 seconds) of game time.
  - **NPC Corpses**: Decay after **5 minutes** (300 seconds) of game time.
- **Expiration**: When a corpse's decay timer expires, a pulse sweep transfers all remaining items in the corpse directly onto the room floor and despawns the corpse entity.

### 2. Loot Rules
Loot rules determine who has the right to search or take items from a corpse:
- **Public**: Anyone can loot the corpse. (Default for NPC corpses).
- **GroupOnly**: Only the owner or members of the owner's party/group can loot. (Default for player corpses).
- **OwnerOnly**: Only the owner of the character can loot.
- **Faction**: Only players matching a specific faction standing can loot.

### 3. Looting Commands
In-game characters interact with corpses using standard command syntax:
- `loot <corpse>`: Transfers all items from the corpse into the player's inventory (bypasses loot rules).
- `get <item> <corpse>`: Transfers a specific item from the corpse's inventory into the player's inventory (not yet wired — use `loot`).
- `get all <corpse>`: Transfers all items from the corpse into the player's inventory.

---

## Death & Ghost System

When a player's health reaches 0 (or they choose to die while unconscious), they enter **ghost mode**.

### Death Flow

1. Health drops to 0 or below → `PlayerState` transitions to `Dead`
2. A `Ghost` marker component is added — restricts commands and interactions
3. A corpse entity spawns in the room containing the player's inventory and equipment
4. The player's inventory and equipment are cleared
5. The player's health is reset to full (ghost form)
6. A death broadcast is sent to the room
7. The player receives the ghost prompt

### Ghost State Restrictions

While in ghost form (`PlayerState::Dead`):

| Allowed | Blocked |
|---|---|
| `look`, `say`, `tell`, `reply` | `kill`, `flee`, `get`, `drop`, `wear`, `wield`, `remove` |
| `die`, `reclaim`, `revive` | All combat, equipment, and inventory commands |
| Movement (rooms) | Rest state changes (`sit`, `rest`, `sleep`, `wake`, `stand`) |
| `quit` | |

Ghost speech renders in alternating cyan/bright-blue colors. Ghosts appear as a cold shiver to normal players (unless they have `detect_invisible` or `detect_undead` effects, in which case they see the ghost).

### Reclaiming Your Body

**`reclaim`** — If the player's corpse is in the same room, they can reclaim it:

1. Equipment from the corpse is transferred to the player
2. Inventory from the corpse is transferred to the player
3. Health is restored to full
4. `PlayerState` returns to `Alive { rest: Standing }`
5. The corpse entity is despawned
6. A room broadcast announces the return

**`revive`** — If the corpse is not present, the player must be in a room whose name contains "temple" or "altar". Reviving restores health and returns to alive state, but equipment and inventory remain in the corpse (must be retrieved separately).

**`toggle resurrect`** — Players can set `no_resurrect = true` to prevent unwanted resurrection attempts.

### Death Penalty

XP penalty on death is tracked via `DeathPenalty` resource (`xp_penalty_pct`, `corpse_retention_secs`). The system marks affected entities as dirty so the penalty persists to the database.

---

## Rest States

Players can adopt different rest states that gate activity and modify regen rates.

### Rest State Machine

```
Standing ↔ Sitting ↔ Resting ↔ Sleeping
```

| State | Movement | Combat | Regen Multiplier |
|---|---|---|---|
| Standing | Yes | Yes | 1.0× |
| Sitting | No | No | 1.5× |
| Resting | No | No | 2.0× |
| Sleeping | No | No | 3.0× |
| Unconscious | No | No | 0.0× (no regen) |
| Dead (ghost) | Yes | No | 0.0× |

### Commands

| Command | Action | Transition |
|---|---|---|
| `sit` | Sit down | Standing/Resting → Sitting |
| `rest` | Lean back and rest | Any → Resting |
| `sleep` | Go to sleep | Any → Sleeping |
| `wake` | Wake from sleep | Sleeping → Resting |
| `stand` | Stand up | Any → Standing |

All rest commands broadcast to the room ("Player sits down.", "Player rests.", etc.). Combat, stun, and casting states block rest transitions with appropriate messages.

### Stun & Casting

- `PlayerState::Stunned { remaining_ms }` — blocks all actions until timer expires
- `PlayerState::Casting { .. }` — blocks movement and combat while casting

---

## Skills & Practice System

Skills represent trained abilities that gate equipment usage and provide combat modifiers.

### Skill Levels (Ranks)

Each skill has a numerical rank. Maximum rank at a given level:

```
max_rank = level * 5 + 5
```

| Level | Max Skill Rank |
|---|---|
| 1 | 10 |
| 10 | 55 |
| 50 | 255 |

### Practice Points

Players receive practice points through leveling (amount determined by class template). Points are spent to increase skill ranks.

**`practice`** — List known skills and current ranks. Shows available practice points.

**`practice <skill>`** — Increase a skill rank by 1, costing 1 practice point. Requires a trainer NPC in the same room that can train the skill's category (combat, weapon, magic, etc.).

### Attribute Training

**`train`** — Lists attributes and training costs. Each attribute costs practice points to increase: 3 for the class's prime attribute, 5 for others. Maximum attribute value is defined by `Attributes::MAX`.

**`train <attribute>`** — Increase the named attribute by 1 point. Requires a trainer with `"attributes"` in their `trainer_types`.

### Skill Gates on Items

Items can have `requires_skill = { id = "two_handed", level = 3 }`. The system checks the wielder's `LearnedSkills` rank against the requirement each time the item is equipped (on `wear`/`wield`). If the requirement is not met, equipping is rejected.

---

## Stances

Combat stances modify hit chance, damage, armor, and skill effectiveness. Players set their stance with the **`stance`** command.

### Built-in Stances

**`stance`** (no args) — Shows current stance and available options.

**`stance <name>`** — Adopts the named stance.

| Stance | Effect |
|---|---|
| `normal` | No modifiers (default). |
| `defensive` | +hit defense, −damage dealt. |
| `aggressive` | +hit chance, −defense. |
| `berserk` | +3 damage, −2 armor, +stun resist, −5% hit, +25% damage. |

The stance is stored in the `ActiveStance` component. The combat system reads the stance to apply modifiers during attack rolls and damage calculations. Custom stances can be defined in `content/stances/*.toml`.

---

## Doors

Exits can be configured as doors with open/close/lock/unlock states.

### Door State

Exit templates support:
- `is_door = true` — marks the exit as a door
- `is_closed = true` — door starts closed (blocks movement)
- `is_locked = true` — door starts locked
- `key_id = "iron_key"` — item template ID of the required key

### Commands

| Command | Action |
|---|---|
| `open <direction>` | Opens a closed, unlocked door |
| `close <direction>` | Closes an open door |
| `lock <direction>` | Locks a closed door (requires key) |
| `unlock <direction>` | Unlocks a locked door (requires key) |

Door operations broadcast to both sides of the exit ("Player opens the door north." in current room, "The door opens." in destination room). The key item must be in the player's inventory.

---

## Regen & Game Loop Ticks

The game loop runs multiple tick intervals rather than a fixed tick rate.

### Tick Intervals

| Tick | Interval | Description |
|---|---|---|
| Combat | 2s | Processes all active combat engagements |
| Big Tick | 30–90s (randomized) | HP/MP/SP regen + prompt broadcast |
| Maintenance | 5s | Dirty entity flush to SQLite |
| Set Bonus | 10s | Re-evaluates set item bonuses |
| Position Save | 60s | Saves player positions to DB |

### Regen (Big Tick)

The big tick fires every 30–90 seconds (uniform random). Each entity with `Health`, `Mana`, `Stamina`, `Energy`, or `Psi` components regens a portion of their max value:

```
regen_amount = max_value / 10 * rest_multiplier
```

| Rest State | Multiplier |
|---|---|
| Standing | 1.0× |
| Sitting | 1.5× |
| Resting | 2.0× |
| Sleeping | 3.0× |
| Unconscious/Dead | 0.0× |

After regen, `send_player_prompt` is called for all connected players, broadcasting their updated prompt line.

---

## Prompt System

The prompt is the status line displayed after command output. It's configurable per-player.

### Prompt Variables

| Variable | Description |
|---|---|
| `%hhp` | Current HP |
| `%hmhp` | Max HP |
| `%hsp` | Current Stamina |
| `%hmsp` | Max Stamina |
| `%hmp` | Current Mana |
| `%hmmp` | Max Mana |
| `%hep` | Current Energy |
| `%hmep` | Max Energy |
| `%hpp` | Current Psi |
| `%hmpp` | Max Psi |
| `%hxp` | Current XP |
| `%hmaxxp` | XP needed for next level |
| `%hnextl` | XP remaining to next level |
| `%hxl` | Current level |
| `%hcredits` | Credits |
| `%hqpoints` | Quest points |

### Commands

**`prompt`** — Shows current prompt template. `prompt <template>` — Sets a custom prompt. `prompt reset` — Reverts to server default.

Default prompt is configurable via `server.toml` (`default_prompt` field).

**`width <columns>`** — Sets screen width for text wrapping. `0` = unlimited. Persisted to the player's DB record.

**`colors`** — Toggles ANSI color output (persisted to player DB).

**`toggle`** — Toggles player settings. Currently only `toggle resurrect` is wired (prevents resurrection from temples).

---

## Communication Channels

| Command | Scope | Description |
|---|---|---|
| `say <message>` | Room | Speaks aloud in the current room |
| `tell <player> <message>` | Global | Sends a private message to any online player |
| `reply <message>` | Global | Replies to the last player who messaged you (`r` alias) |
| `shout <message>` | Area | Shouts to all players in the same area |
| `whisper <player> <message>` | Room | Whispers to a player in the same room |

Ghost speech renders in a distinctive alternating color pattern. `tell` and `reply` track the last messenger via the `LastMessenger` component.

---

## Inventory & Equipment Commands

| Command | Description |
|---|---|
| `inventory` / `inv` / `i` | List items you are carrying |
| `equipment` / `eq` | Show what you are wearing and wielding |
| `get <item>` | Pick up an item from the floor |
| `drop <item>` | Drop a carried item to the floor |
| `put <item> <container>` | Place item in a container (not yet wired) |
| `give <item> <player>` | Give item to another player (not yet wired) |
| `loot <corpse>` | Take all items from a corpse |
| `wear <item>` | Wear a piece of armor |
| `wield <item>` | Wield a weapon |
| `remove <slot>` | Unequip an item (removes to inventory) |
| `examine` / `exa` | Inspect an item's detailed properties |

### Equipment Slots

Available equipment slots: `head`, `neck`, `torso`, `arms`, `hands`, `finger`, `legs`, `feet`, `weapon`, `shield`, `back`, `waist`.

Items with `requires_skill` are checked on `wear`/`wield`. If the player lacks the required skill rank, equipping is rejected with "You lack the skill to use that."

---

## Time & Weather

| Command | Description |
|---|---|
| `time` | Shows current in-game time (stub: "It is currently 10:00 AM in the morning.") |
| `weather` | Shows current weather conditions (stub: "The sky is clear...") |

Both are placeholder implementations. Future phases will integrate a full day/night cycle and weather system.
