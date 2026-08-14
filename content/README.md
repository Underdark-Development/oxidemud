# OxideMUD Content Directory (`content/`)

This directory contains all game data templates, server configuration files, text banners, and dynamic Rhai scripts. OxideMUD follows a strict **driver/content separation design**: the engine binary (`oxide-server`) provides networking, persistence, and ECS execution, while all game content, balance formulas, and scripting logic live in this directory.

---

## Server Configuration & Data Files

The following server-level configuration and text asset files live in the root of `content/`:

- **`server.toml`** — Master engine server configuration. Controls server display name, bind IP addresses, ports, default player prompt templates, log retention/rotation policies, REST API & WebSocket settings, TLS/ACME security options, and in-game clock time scaling.
- **`motd.txt`** — Message of the Day banner displayed to players immediately after logging into the game.
- **`banner.txt`** — Welcome ASCII art banner displayed on initial socket connection before authentication.
- **`socials.toml`** — Social emote verb definitions (`bounce`, `nod`, `smile`, `giggle`, etc.) supporting target and untargeted messaging.
- **`weather.toml`** — World weather system data definitions, seasonal climate weight matrices, base weather types, and room/area weather modifiers.

---

## Content Category Subdirectories

Game content is organized into subdirectories by template category. All content templates use TOML data format, while dynamic behaviors use Rhai scripts (`.rhai`):

- **`affixes/`** — Item prefix and suffix stat modifier templates.
- **`areas/`** — Nested world zone and room layout definitions. Each area lives in its own subdirectory (`content/areas/<area_id>/`):
  - **`area.toml`** — Master zone metadata file containing the area's display name, recommended level range, climate, author details, reset intervals, and zone-wide flags.
  - **`rooms/`** — Subdirectory containing individual room definition files (`.toml`) specifying room descriptions, coordinates, directional exits, and keyword portals within that zone.
- **`classes/`** — Character class templates (attribute progression, saving throws, hit dice, auto-granted skills).
- **`deities/`** — Deity templates and prayer blessing buff definitions.
- **`factions/`** — Faction definitions, initial standing matrices, and reputation decay/boost settings.
- **`items/`** — Equipment, weapon, armor, container, and consumable item templates.
- **`mobs/`** — Non-Player Character (NPC) templates (monsters, merchants, trainers, bosses).
- **`passives/`** — Passive trait definitions and racial/class passive skill behaviors.
- **`races/`** — Playable race templates (attribute modifiers, traits, sizes, base languages).
- **`scripts/`** — Rhai dynamic script files (`.rhai`) for item triggers, room events, quest logic, and custom spell/ability logic.
- **`sets/`** — Item set bonus threshold specifications.
- **`shops/`** — Vendor shop definitions (inventory lists, counter-offer haggling modifiers, restock policies).
- **`skills/`** — Unified skill and spell templates (resource costs, cooldowns, trainer prerequisites, gates).
- **`stances/`** — Martial and combat stance definitions.
