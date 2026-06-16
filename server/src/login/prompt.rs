use tokio::sync::Mutex;

use mud_core::templates::TemplateRegistry;
use mud_core::{Alignment, Name, World};

use crate::registry::ConnectionRegistry;

use super::state::LoginState;
use super::LoginFlow;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STAT_NAMES: [&str; 6] = [
    "Strength",
    "Dexterity",
    "Intelligence",
    "Wisdom",
    "Constitution",
    "Charisma",
];

/// Retrieve the list of characters for the account stored in the flow and
/// return the character-selection screen as a vector of output lines.
pub async fn go_to_character_select(
    flow: &mut LoginFlow,
    db: Option<&Mutex<mud_data::Database>>,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(String::new());

    let world_ready = match templates {
        Some(t) => !t.races.is_empty(),
        None => false,
    };
    if !world_ready {
        lines.push("World building is still in progress — please check back later.".to_string());
        flow.disconnect_requested = true;
        return lines;
    }

    let account_id = match flow.account_id {
        Some(id) => id,
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::Username;
            return lines;
        }
    };

    let db = match db {
        Some(d) => d,
        None => {
            lines.push("Server error: database unavailable.".to_string());
            flow.state = LoginState::CharacterSelect;
            return lines;
        }
    };

    let db_guard = match db.try_lock() {
        Ok(g) => g,
        Err(_) => {
            lines.push("Server error: database unavailable.".to_string());
            flow.state = LoginState::CharacterSelect;
            return lines;
        }
    };

    let chars = match mud_data::get_characters_by_account(db_guard.conn(), account_id) {
        Ok(c) => c,
        Err(e) => {
            lines.push(format!("DB error: {e}"));
            flow.state = LoginState::CharacterSelect;
            return lines;
        }
    };

    lines.push("--- Character Selection ---".to_string());

    if chars.is_empty() {
        if flow.create_dismissed {
            lines.push("Type 'c' to create a character, or 'who' to see who's online.".to_string());
        } else {
            lines.push("You have no characters yet. Create one now? (y/n)".to_string());
        }
        flow.state = LoginState::CharacterSelect;
        return lines;
    }

    for (i, ch) in chars.iter().enumerate() {
        lines.push(format!(
            "{}. {} — {} {} (Level {})",
            i + 1,
            ch.name,
            ch.race,
            ch.class,
            ch.level
        ));
    }
    lines.push(String::new());
    lines.push(
        "Type a number to pick a character, 'c' to create one, or 'who' to see who's online."
            .to_string(),
    );
    flow.state = LoginState::CharacterSelect;
    lines
}

/// Return the race-selection prompt as a vector of output lines.
pub fn show_character_race_prompt(
    flow: &mut LoginFlow,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let templates = match templates {
        Some(t) => t,
        None => {
            lines.push("No race templates available. Cannot create character.".to_string());
            return lines;
        }
    };

    lines.push(String::new());
    lines.push("--- Choose a Race ---".to_string());
    let mut races: Vec<(&str, &mud_core::templates::RaceTemplate)> = templates
        .races
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    races.sort_by(|a, b| a.0.cmp(b.0));

    let ids: Vec<String> = races.iter().map(|(id, _)| id.to_string()).collect();
    flow.state = LoginState::CharacterCreateRace(ids);

    for (i, (_id, race)) in races.iter().enumerate() {
        lines.push(format!("{}. {} — {}", i + 1, race.name, race.description));
    }
    lines.push(format!("Pick a race by number (1-{}):", races.len()));
    lines
}

/// Return the class-selection prompt as a vector of output lines.
pub fn show_character_class_prompt(
    flow: &mut LoginFlow,
    templates: &TemplateRegistry,
) -> Vec<String> {
    let mut lines = Vec::new();
    let race_id = flow.create_buffer.race.as_deref().unwrap_or("");
    let available = templates.available_classes_for_race(race_id);

    let ids: Vec<String> = available.iter().map(|c| c.id.clone()).collect();
    flow.state = LoginState::CharacterCreateClass(ids);

    lines.push(String::new());
    lines.push("--- Choose a Class ---".to_string());

    for (i, class) in available.iter().enumerate() {
        lines.push(format!("{}. {} — {}", i + 1, class.name, class.description));
    }
    lines.push(format!("Pick a class by number (1-{}):", available.len()));
    lines
}

// ---------------------------------------------------------------------------
// Attribute Method Selection
// ---------------------------------------------------------------------------

pub fn show_attribute_method_prompt() -> Vec<String> {
    vec![
        String::new(),
        "--- Choose Your Approach ---".to_string(),
        "1. Distribute carefully".to_string(),
        "2. Choose from a set".to_string(),
        "3. Let fate decide".to_string(),
        String::new(),
        "Pick an approach (1-3):".to_string(),
    ]
}

// ---------------------------------------------------------------------------
// Point-Buy
// ---------------------------------------------------------------------------

pub fn show_point_buy_prompt(flow: &LoginFlow) -> Vec<String> {
    let mut lines = Vec::new();

    let (remaining, attrs) = match &flow.state {
        LoginState::CharacterCreateAttributesPointBuy { remaining, attrs } => (*remaining, *attrs),
        _ => return lines,
    };

    let costs: Vec<String> = attrs
        .iter()
        .map(|&v| {
            if !(8..18).contains(&v) {
                String::new()
            } else {
                format!("({} pt)", point_buy_cost_str(v))
            }
        })
        .collect();

    lines.push(String::new());
    lines.push(format!(
        "--- Distribute Attributes ({} points remaining) ---",
        remaining
    ));
    lines.push(String::new());
    for i in 0..6 {
        let label = STAT_NAMES[i];
        let val = attrs[i];
        lines.push(format!("  {:<14} {:>2}  {}", label, val, costs[i]));
    }
    lines.push(String::new());
    lines.push("Commands: str+1, dex-1, str=12, or 'done' to finish.".to_string());
    lines.push("Type 'reset' to start over.".to_string());
    lines
}

fn point_buy_cost_str(current: u8) -> u8 {
    if !(8..18).contains(&current) {
        return 0;
    }
    let costs: [u8; 11] = [1, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4];
    costs[(current - 8) as usize]
}

// ---------------------------------------------------------------------------
// Standard Array
// ---------------------------------------------------------------------------

pub fn show_standard_array_prompt(flow: &LoginFlow) -> Vec<String> {
    let mut lines = Vec::new();

    let (values, assign_idx, attrs) = match &flow.state {
        LoginState::CharacterCreateAttributesArray {
            values,
            assign_idx,
            attrs,
        } => (*values, *assign_idx, *attrs),
        _ => return lines,
    };

    if assign_idx >= 6 {
        return lines;
    }

    let value_to_assign = values[assign_idx];

    // Show already-assigned stats
    lines.push(String::new());
    lines.push("--- Choose From a Set ---".to_string());
    lines.push(String::new());
    for i in 0..6 {
        let label = STAT_NAMES[i];
        if attrs[i] != 0 {
            lines.push(format!("  {:<14} {:>2}", label, attrs[i]));
        } else {
            lines.push(format!("  {:<14}   —", label));
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "Assign {} to which stat? (str/dex/int/wis/con/cha)",
        value_to_assign
    ));
    lines.push("Type 'reset' to start over.".to_string());
    lines
}

// ---------------------------------------------------------------------------
// Roll
// ---------------------------------------------------------------------------

pub fn show_roll_prompt(flow: &LoginFlow) -> Vec<String> {
    let mut lines = Vec::new();

    let (rolls, assign_idx, attrs, rerolls) = match &flow.state {
        LoginState::CharacterCreateAttributesRoll {
            rolls,
            assign_idx,
            attrs,
            rerolls,
        } => (*rolls, *assign_idx, *attrs, *rerolls),
        _ => return lines,
    };

    // Show all rolled values
    lines.push(String::new());
    lines.push("--- Rolled Attributes ---".to_string());
    lines.push(String::new());
    let mut display_n = 1;
    for (i, roll) in rolls.iter().enumerate() {
        if i < assign_idx {
            continue;
        }
        lines.push(format!("  Roll {}: {:>2}", display_n, roll));
        display_n += 1;
    }

    // Show already-assigned stats
    lines.push(String::new());
    lines.push("Assigned so far:".to_string());
    for i in 0..6 {
        let label = STAT_NAMES[i];
        if attrs[i] != 0 {
            lines.push(format!("  {:<14} {:>2}", label, attrs[i]));
        } else {
            lines.push(format!("  {:<14}   —", label));
        }
    }

    lines.push(String::new());
    if assign_idx >= 6 {
        return lines;
    }

    let value_to_assign = rolls[assign_idx];
    if rerolls > 0 {
        lines.push(format!(
            "Assign {} to which stat? (str/dex/int/wis/con/cha), or 'reroll' to discard and re-roll all ({} left)",
            value_to_assign, rerolls
        ));
    } else {
        lines.push(format!(
            "Assign {} to which stat? (str/dex/int/wis/con/cha)",
            value_to_assign
        ));
    }
    lines.push("Type 'reset' to start over.".to_string());
    lines
}

// ---------------------------------------------------------------------------
// Alignment
// ---------------------------------------------------------------------------

pub fn show_alignment_prompt(
    flow: &LoginFlow,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();

    // Determine which alignments are valid for the chosen class and race
    let class_allowed: Vec<&str> = flow
        .create_buffer
        .class
        .as_deref()
        .and_then(|class_id| templates.and_then(|t| t.get_class(class_id)))
        .map(|class| {
            if class.allowed_alignments.is_empty() {
                Alignment::ALL.to_vec()
            } else {
                class
                    .allowed_alignments
                    .iter()
                    .map(|s| s.as_str())
                    .collect()
            }
        })
        .unwrap_or_else(|| Alignment::ALL.to_vec());

    let race_allowed: Vec<&str> = flow
        .create_buffer
        .race
        .as_deref()
        .and_then(|race_id| templates.and_then(|t| t.get_race(race_id)))
        .map(|race| {
            if race.allowed_alignments.is_empty() {
                Alignment::ALL.to_vec()
            } else {
                race.allowed_alignments.iter().map(|s| s.as_str()).collect()
            }
        })
        .unwrap_or_else(|| Alignment::ALL.to_vec());

    // Intersection: alignments must satisfy both race and class restrictions
    let allowed: Vec<&str> = class_allowed
        .into_iter()
        .filter(|a| race_allowed.contains(a))
        .collect();

    lines.push(String::new());
    lines.push("--- Choose Alignment ---".to_string());
    lines.push(String::new());

    // Display as a 3x3 grid: Lawful — Neutral — Chaotic rows
    lines.push(format!(
        "{:>8} {:20} {:20} {:20}",
        "", "Lawful", "Neutral", "Chaotic"
    ));
    lines.push(String::new());

    let grid = [
        ("lawful_good", "neutral_good", "chaotic_good"),
        ("lawful_neutral", "true_neutral", "chaotic_neutral"),
        ("lawful_evil", "neutral_evil", "chaotic_evil"),
    ];
    let row_labels = ["Good", "Neutral", "Evil"];
    let mut idx = 1;
    for (row_idx, (left, center, right)) in grid.iter().enumerate() {
        let left_str = if allowed.contains(left) {
            format!("{}. {}", idx, left.replace('_', " "))
        } else {
            format!("{}. {}", idx, "(unavailable)")
        };
        idx += 1;
        let center_str = if allowed.contains(center) {
            format!("{}. {}", idx, center.replace('_', " "))
        } else {
            format!("{}. {}", idx, "(unavailable)")
        };
        idx += 1;
        let right_str = if allowed.contains(right) {
            format!("{}. {}", idx, right.replace('_', " "))
        } else {
            format!("{}. {}", idx, "(unavailable)")
        };
        idx += 1;

        lines.push(format!(
            "{:<8} {:20} {:20} {:20}",
            row_labels[row_idx], left_str, center_str, right_str
        ));
    }

    lines.push(String::new());
    lines.push("Pick an alignment by number (1-9):".to_string());
    lines
}

// ---------------------------------------------------------------------------
// Skill Selection
// ---------------------------------------------------------------------------

pub fn show_skill_selection_prompt(
    flow: &LoginFlow,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();

    let (pool, selected, slots) = match &flow.state {
        LoginState::CharacterCreateSkillSelection {
            pool,
            selected,
            slots,
        } => (pool.clone(), selected.clone(), *slots),
        _ => return lines,
    };

    lines.push(String::new());
    lines.push(format!("--- Choose Your Skills (pick {slots}) ---"));
    lines.push(String::new());

    for skill_id in &pool {
        let desc = templates
            .and_then(|t| t.get_skill(skill_id))
            .map(|s| s.description.as_str())
            .unwrap_or("");
        let marker = if selected.contains(skill_id) {
            "[x]"
        } else {
            "[ ]"
        };
        lines.push(format!("  {marker} {skill_id}: {desc}"));
    }

    let remaining = (slots as usize).saturating_sub(selected.len());
    lines.push(String::new());
    lines.push(format!("Remaining picks: {remaining}"));
    lines.push(String::new());
    lines.push("Commands: add <skill>, remove <skill>, list, done".to_string());
    lines
}

// ---------------------------------------------------------------------------
// Confirm
// ---------------------------------------------------------------------------

/// Return the character-confirmation screen as a vector of output lines.
pub fn show_character_confirm(flow: &mut LoginFlow, templates: &TemplateRegistry) -> Vec<String> {
    let mut lines = Vec::new();

    let name = flow
        .create_buffer
        .name
        .clone()
        .unwrap_or_else(|| "?".to_string());
    let race_id = flow
        .create_buffer
        .race
        .clone()
        .unwrap_or_else(|| "?".to_string());
    let class_id = flow
        .create_buffer
        .class
        .clone()
        .unwrap_or_else(|| "?".to_string());
    let alignment = flow
        .create_buffer
        .alignment
        .clone()
        .unwrap_or_else(|| "?".to_string());
    let description = flow.create_buffer.description.clone().unwrap_or_default();
    let player_attrs = flow.create_buffer.attributes.clone().unwrap_or_default();

    let race = templates.get_race(&race_id);
    let class = templates.get_class(&class_id);

    let race_name = race.as_ref().map(|r| r.name.as_str()).unwrap_or("?");
    let class_name = class.as_ref().map(|c| c.name.as_str()).unwrap_or("?");

    // Compute final attributes
    let default_attrs = mud_core::templates::RaceAttributes::default();
    let default_mods = mud_core::templates::ClassAttributeMods::default();
    let race_attrs = race
        .as_ref()
        .map(|r| &r.attributes)
        .unwrap_or(&default_attrs);
    let class_mods = class
        .as_ref()
        .map(|c| &c.attribute_mods)
        .unwrap_or(&default_mods);

    let str = ((race_attrs.strength as i16 + class_mods.strength as i16)
        + player_attrs.strength as i16
        - 8)
    .clamp(3, 50) as u8;
    let dex = ((race_attrs.dexterity as i16 + class_mods.dexterity as i16)
        + player_attrs.dexterity as i16
        - 8)
    .clamp(3, 50) as u8;
    let int = ((race_attrs.intelligence as i16 + class_mods.intelligence as i16)
        + player_attrs.intelligence as i16
        - 8)
    .clamp(3, 50) as u8;
    let wis = ((race_attrs.wisdom as i16 + class_mods.wisdom as i16) + player_attrs.wisdom as i16
        - 8)
    .clamp(3, 50) as u8;
    let con = ((race_attrs.constitution as i16 + class_mods.constitution as i16)
        + player_attrs.constitution as i16
        - 8)
    .clamp(3, 50) as u8;
    let cha = ((race_attrs.charisma as i16 + class_mods.charisma as i16)
        + player_attrs.charisma as i16
        - 8)
    .clamp(3, 50) as u8;

    let default_wallet = mud_core::templates::WalletAmount::default();
    let wallet = class
        .as_ref()
        .map(|c| &c.starting_gold)
        .unwrap_or(&default_wallet);

    lines.push(String::new());
    lines.push("--- Character Summary ---".to_string());
    lines.push(format!("  Name:       {name}"));
    lines.push(format!("  Race:       {race_name}"));
    lines.push(format!("  Class:      {class_name}"));
    lines.push(format!("  Alignment:  {}", alignment.replace('_', " ")));
    lines.push(String::new());
    lines.push(format!("  STR: {str}   DEX: {dex}   INT: {int}"));
    lines.push(format!("  WIS: {wis}   CON: {con}   CHA: {cha}"));
    lines.push(String::new());
    lines.push(format!(
        "  Gold:  {}g {}s {}c {}p",
        wallet.gold, wallet.silver, wallet.copper, wallet.platinum
    ));
    if !description.is_empty() {
        lines.push(format!(
            "  Desc: {}",
            description.lines().next().unwrap_or("")
        ));
        if description.lines().count() > 1 {
            lines.push("         ...".to_string());
        }
    }
    if let Some(r) = race {
        if !r.racial_abilities.is_empty() {
            lines.push(format!("  Racial:     {}", r.racial_abilities.join(", ")));
        }
    }
    if let Some(c) = class {
        let mut skill_list: Vec<String> = Vec::new();
        for auto in &c.auto_skills {
            let name = templates
                .get_skill(auto)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| auto.clone());
            skill_list.push(name);
        }
        for selected in &flow.create_buffer.selected_skills {
            if !c.auto_skills.contains(selected) {
                let name = templates
                    .get_skill(selected)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| selected.clone());
                skill_list.push(name);
            }
        }
        if !skill_list.is_empty() {
            lines.push(format!("  Skills:     {}", skill_list.join(", ")));
        }
        if !c.starting_items.is_empty() {
            lines.push(format!("  Equipment:  {}", c.starting_items.join(", ")));
        }
    }
    lines.push(String::new());
    lines.push("Accept this character? (y/n)".to_string());
    lines
}

// ---------------------------------------------------------------------------
// Spawn Selection
// ---------------------------------------------------------------------------

/// Return the spawn-selection prompt as a vector of output lines.
pub fn show_spawn_prompt(flow: &LoginFlow, templates: &TemplateRegistry) -> Vec<String> {
    let mut lines = Vec::new();

    let race_id = flow.create_buffer.race.as_deref().unwrap_or("unknown");
    let class_id = flow.create_buffer.class.as_deref().unwrap_or("unknown");
    let alignment = flow
        .create_buffer
        .alignment
        .as_deref()
        .unwrap_or("true_neutral");

    let available = templates.available_spawns(race_id, class_id, alignment);

    lines.push(String::new());
    lines.push("--- Choose Your Starting Location ---".to_string());

    if available.is_empty() {
        lines.push("No spawn points available. Contact an administrator.".to_string());
        return lines;
    }

    for (i, (area_id, spawn)) in available.iter().enumerate() {
        let area_name = templates
            .get_area(area_id)
            .map(|a| a.name.as_str())
            .unwrap_or(area_id);
        lines.push(format!(
            "{}. {} — {} ({})",
            i + 1,
            spawn.label,
            spawn.description,
            area_name
        ));
    }

    lines.push(format!(
        "Pick a starting location by number (1-{}):",
        available.len()
    ));
    lines
}

// ---------------------------------------------------------------------------
// WHO list
// ---------------------------------------------------------------------------

/// Return a list of online players as output lines.
pub fn list_who(world: &World, registry: &ConnectionRegistry) -> Vec<String> {
    let mut lines = Vec::new();
    let entities = registry.connected_entities();
    let count = entities.len();

    lines.push(String::new());
    if count == 0 {
        lines.push("No one else is online.".to_string());
        return lines;
    }

    lines.push(format!("Players online ({count}):"));
    for entity in entities {
        if let Ok(mut q) = world.query_one::<&Name>(entity) {
            if let Some(name) = q.get() {
                lines.push(format!("  {}", name.as_str()));
            }
        }
    }
    lines.push(String::new());
    lines
}
