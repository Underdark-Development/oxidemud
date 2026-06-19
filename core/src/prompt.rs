use crate::{
    Alignment, Attributes, CombatState, Entity, Experience, Health, Level, Mana, Name, PlayerState,
    Position, Room, RoomExits, Stamina, Wallet, World,
};

pub struct PromptVars {
    pub hp: String,
    pub max_hp: String,
    pub mana: String,
    pub max_mana: String,
    pub stamina: String,
    pub max_stamina: String,
    pub level: String,
    pub xp: String,
    pub xp_next: String,
    pub name: String,
    pub gold: String,
    pub alignment: String,
    pub room_name: String,
    pub exits: String,
    pub strength: String,
    pub dexterity: String,
    pub intelligence: String,
    pub wisdom: String,
    pub constitution: String,
    pub charisma: String,
    pub rest_state: String,
    pub combat_state: String,
}

pub fn build_vars(world: &World, entity: Entity) -> PromptVars {
    let hp = world
        .query_one::<&Health>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let mana = world
        .query_one::<&Mana>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let stamina = world
        .query_one::<&Stamina>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let level = world
        .query_one::<&Level>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());
    let xp = world
        .query_one::<&Experience>(entity)
        .ok()
        .and_then(|mut q| q.get().copied());
    let name = world
        .query_one::<&Name>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let wallet = world
        .query_one::<&Wallet>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let alignment = world
        .query_one::<&Alignment>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let attrs = world
        .query_one::<&Attributes>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let player_state = world
        .query_one::<&PlayerState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let combat_state = world
        .query_one::<&CombatState>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());
    let pos = world
        .query_one::<&Position>(entity)
        .ok()
        .and_then(|mut q| q.get().cloned());

    let (room_name, exits_str) = if let Some(ref p) = pos {
        let room_name = world
            .query_one::<&Room>(p.room)
            .ok()
            .and_then(|mut q| q.get().map(|r| r.name.clone()))
            .unwrap_or_default();
        let exits_str = world
            .query_one::<&RoomExits>(p.room)
            .ok()
            .and_then(|mut q| {
                q.get().map(|exits| {
                    exits
                        .0
                        .iter()
                        .map(|e| e.direction.short_name())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
            })
            .unwrap_or_default();
        (room_name, exits_str)
    } else {
        (String::new(), String::new())
    };

    let lvl = level.map(|l| l.0).unwrap_or(1);
    let xp_val = xp.map(|x| x.0).unwrap_or(0);

    PromptVars {
        hp: hp
            .as_ref()
            .map(|h| h.current.to_string())
            .unwrap_or_else(|| "?".to_string()),
        max_hp: hp
            .as_ref()
            .map(|h| h.max.to_string())
            .unwrap_or_else(|| "?".to_string()),
        mana: mana
            .as_ref()
            .map(|m| m.current.to_string())
            .unwrap_or_else(|| "?".to_string()),
        max_mana: mana
            .as_ref()
            .map(|m| m.max.to_string())
            .unwrap_or_else(|| "?".to_string()),
        stamina: stamina
            .as_ref()
            .map(|s| s.current.to_string())
            .unwrap_or_else(|| "?".to_string()),
        max_stamina: stamina
            .as_ref()
            .map(|s| s.max.to_string())
            .unwrap_or_else(|| "?".to_string()),
        level: lvl.to_string(),
        xp: xp_val.to_string(),
        xp_next: Experience::for_level(lvl + 1)
            .saturating_sub(xp_val)
            .to_string(),
        name: name
            .as_ref()
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "?".to_string()),
        gold: wallet
            .as_ref()
            .map(|w| w.total_copper().to_string())
            .unwrap_or_else(|| "0".to_string()),
        alignment: alignment.as_ref().map(|a| a.0.clone()).unwrap_or_default(),
        room_name,
        exits: exits_str,
        strength: attrs
            .as_ref()
            .map(|a| a.strength.to_string())
            .unwrap_or_else(|| "?".to_string()),
        dexterity: attrs
            .as_ref()
            .map(|a| a.dexterity.to_string())
            .unwrap_or_else(|| "?".to_string()),
        intelligence: attrs
            .as_ref()
            .map(|a| a.intelligence.to_string())
            .unwrap_or_else(|| "?".to_string()),
        wisdom: attrs
            .as_ref()
            .map(|a| a.wisdom.to_string())
            .unwrap_or_else(|| "?".to_string()),
        constitution: attrs
            .as_ref()
            .map(|a| a.constitution.to_string())
            .unwrap_or_else(|| "?".to_string()),
        charisma: attrs
            .as_ref()
            .map(|a| a.charisma.to_string())
            .unwrap_or_else(|| "?".to_string()),
        rest_state: player_state
            .as_ref()
            .map(|ps| format!("{:?}", ps.rest()))
            .unwrap_or_else(|| "Standing".to_string()),
        combat_state: combat_state
            .as_ref()
            .map(|cs| {
                if cs.is_in_combat() {
                    "In Combat".to_string()
                } else {
                    "Not In Combat".to_string()
                }
            })
            .unwrap_or_else(|| "Not In Combat".to_string()),
    }
}

pub fn render_prompt(template: &str, vars: &PromptVars) -> String {
    let mut result = String::with_capacity(template.len() + 32);
    let mut chars = template.chars();

    while let Some(ch) = chars.next() {
        if ch != '%' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('h') => result.push_str(&vars.hp),
            Some('H') => result.push_str(&vars.max_hp),
            Some('m') => result.push_str(&vars.mana),
            Some('M') => result.push_str(&vars.max_mana),
            Some('v') => result.push_str(&vars.stamina),
            Some('V') => result.push_str(&vars.max_stamina),
            Some('l') => result.push_str(&vars.level),
            Some('x') => result.push_str(&vars.xp),
            Some('X') => result.push_str(&vars.xp_next),
            Some('n') => result.push_str(&vars.name),
            Some('g') => result.push_str(&vars.gold),
            Some('a') => result.push_str(&vars.alignment),
            Some('r') => result.push_str(&vars.room_name),
            Some('e') => result.push_str(&vars.exits),
            Some('s') => result.push_str(&vars.strength),
            Some('d') => result.push_str(&vars.dexterity),
            Some('i') => result.push_str(&vars.intelligence),
            Some('w') => result.push_str(&vars.wisdom),
            Some('o') => result.push_str(&vars.constitution),
            Some('u') => result.push_str(&vars.charisma),
            Some('R') => result.push_str(&vars.rest_state),
            Some('C') => result.push_str(&vars.combat_state),
            Some('c') => result.push_str("\r\n"),
            Some('%') => result.push('%'),
            Some(c) => {
                result.push('%');
                result.push(c);
            }
            None => result.push('%'),
        }
    }

    result
}
