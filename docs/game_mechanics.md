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
- `loot <corpse>`: Examines the corpse and lists all containing items.
- `get <item> <corpse>`: Transfers a specific item from the corpse's inventory into the player's inventory (subject to loot rules).
- `get all <corpse>`: Transfers all items from the corpse into the player's inventory.
