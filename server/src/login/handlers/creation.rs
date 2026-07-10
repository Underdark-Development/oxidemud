use tokio::sync::Mutex;

use oxide_core::templates::{SkillResolveError, TemplateRegistry};
use oxide_core::{
    Alignment, Class, CombatStats, DbId, Description, Equipment, Experience, Gender, Health,
    Inventory, Level, Mana, Name, Player, PlayerState, Position, PracticePoints, Race, RecallRoom,
    Stamina, Wallet, World,
};

use crate::registry::ConnectionRegistry;

use super::super::state::LoginState;
use super::super::LoginFlow;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const POINT_BUY_COST: [u8; 11] = [1, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4];

const STANDARD_ARRAY: [u8; 6] = [15, 14, 13, 12, 10, 8];

const MAX_POINT_BUY_POINTS: u8 = 27;
const MAX_REROLLS: u8 = 3;

// ---------------------------------------------------------------------------
// Handler helpers
// ---------------------------------------------------------------------------

/// Validates a character name: 3-16 chars, letters, hyphens, apostrophes.
fn is_valid_character_name(s: &str) -> bool {
    if !(3..=16).contains(&s.len()) {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    let last = chars.last().unwrap_or(first);
    if !first.is_ascii_alphabetic() || !last.is_ascii_alphabetic() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '\'')
}

/// Roll 4d6 drop lowest, return the result.
fn roll_4d6_drop_lowest() -> u8 {
    let mut rolls: [u8; 4] = [
        fastrand::u8(1..=6),
        fastrand::u8(1..=6),
        fastrand::u8(1..=6),
        fastrand::u8(1..=6),
    ];
    rolls.sort_unstable();
    rolls[1] + rolls[2] + rolls[3]
}

/// Roll 6 sets of 4d6 drop lowest, return sorted descending.
fn roll_all_stats() -> [u8; 6] {
    let mut stats = [0u8; 6];
    for s in &mut stats {
        *s = roll_4d6_drop_lowest();
    }
    stats.sort_unstable_by(|a, b| b.cmp(a));
    stats
}

/// Match an option by 1-based index or by name (case-insensitive).
fn match_option_index_or_name<'a>(input_lower: &str, options: &'a [String]) -> Option<&'a String> {
    if let Ok(idx) = input_lower.parse::<usize>() {
        if idx > 0 && idx <= options.len() {
            return Some(&options[idx - 1]);
        }
    }
    options.iter().find(|o| o.to_lowercase() == input_lower)
}

/// Parse a stat name abbreviation to index (0-5).
fn stat_index(s: &str) -> Option<usize> {
    match s {
        "str" | "strength" => Some(0),
        "dex" | "dexterity" => Some(1),
        "int" | "intelligence" => Some(2),
        "wis" | "wisdom" => Some(3),
        "con" | "constitution" => Some(4),
        "cha" | "charisma" => Some(5),
        _ => None,
    }
}

/// Cost to raise a stat from its current value by 1.
fn point_buy_cost(current: u8) -> Option<u8> {
    if !(8..18).contains(&current) {
        return None;
    }
    Some(POINT_BUY_COST[(current - 8) as usize])
}

/// Compute final attributes from race base + class mod + player-chosen base.
pub fn compute_final_attributes(
    templates: Option<&TemplateRegistry>,
    race_id: &str,
    class_id: &str,
    player_base: &oxide_core::Attributes,
) -> (oxide_core::Attributes, i32, oxide_core::LearnedSkills) {
    let mut skills = oxide_core::LearnedSkills::new();

    let (base_str, base_dex, base_int, base_wis, base_con, base_cha) = templates
        .and_then(|t| t.get_race(race_id))
        .map(|r| {
            for ability in &r.racial_abilities {
                skills.grant(ability);
            }
            (
                r.attributes.strength as i16,
                r.attributes.dexterity as i16,
                r.attributes.intelligence as i16,
                r.attributes.wisdom as i16,
                r.attributes.constitution as i16,
                r.attributes.charisma as i16,
            )
        })
        .unwrap_or((10, 10, 10, 10, 10, 10));

    let (mod_str, mod_dex, mod_int, mod_wis, mod_con, mod_cha, hit_die) = templates
        .and_then(|t| t.get_class(class_id))
        .map(|c| {
            for skill_id in &c.auto_skills {
                skills.grant(skill_id);
            }
            (
                c.attribute_mods.strength,
                c.attribute_mods.dexterity,
                c.attribute_mods.intelligence,
                c.attribute_mods.wisdom,
                c.attribute_mods.constitution,
                c.attribute_mods.charisma,
                c.hit_die,
            )
        })
        .unwrap_or((0, 0, 0, 0, 0, 0, 8));

    let attrs = oxide_core::Attributes::new(
        (base_str + mod_str as i16 + player_base.strength as i16 - 8).clamp(3, 50) as u8,
        (base_dex + mod_dex as i16 + player_base.dexterity as i16 - 8).clamp(3, 50) as u8,
        (base_int + mod_int as i16 + player_base.intelligence as i16 - 8).clamp(3, 50) as u8,
        (base_wis + mod_wis as i16 + player_base.wisdom as i16 - 8).clamp(3, 50) as u8,
        (base_con + mod_con as i16 + player_base.constitution as i16 - 8).clamp(3, 50) as u8,
        (base_cha + mod_cha as i16 + player_base.charisma as i16 - 8).clamp(3, 50) as u8,
    );

    let hp = hit_die as i32 + (attrs.constitution as i32 - 10) / 2;

    (attrs, hp.max(1), skills)
}

/// Retrieve starting gold from class template.
pub fn class_starting_gold(templates: Option<&TemplateRegistry>, class_id: &str) -> Wallet {
    templates
        .and_then(|t| t.get_class(class_id))
        .map(|c| {
            Wallet::new(
                c.starting_gold.copper,
                c.starting_gold.silver,
                c.starting_gold.gold,
                c.starting_gold.platinum,
            )
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Character Select
// ---------------------------------------------------------------------------

pub async fn handle_character_select_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<oxide_data::Database>>,
    world: &mut World,
    registry: &ConnectionRegistry,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();

    let db = match db {
        Some(d) => d,
        None => return lines,
    };

    let account_id = match flow.account_id {
        Some(id) => id,
        None => {
            lines.push("Session error. Please log in again.".to_string());
            flow.state = LoginState::Username;
            return lines;
        }
    };

    let db_guard = db.lock().await;
    let chars = match oxide_data::get_characters_by_account(db_guard.conn(), account_id) {
        Ok(c) => c,
        Err(e) => {
            lines.push(format!("DB error: {e}"));
            return lines;
        }
    };

    if chars.is_empty() {
        if flow.create_dismissed {
            drop(db_guard);
            match input.to_lowercase().as_str() {
                "c" => {
                    clear_create_buffer(flow);
                    lines.push(String::new());
                    lines.push("--- Create a New Character ---".to_string());
                    flow.state = LoginState::CharacterCreateName;
                }
                "who" => {
                    lines.extend(super::super::prompt::list_who(world, registry));
                }
                _ => {
                    lines.push(
                        "Type 'c' to create a character, or 'who' to see who's online.".to_string(),
                    );
                }
            }
            return lines;
        }

        drop(db_guard);
        match input.to_lowercase().as_str() {
            "y" | "yes" | "c" => {
                clear_create_buffer(flow);
                lines.push(String::new());
                lines.push("--- Create a New Character ---".to_string());
                flow.state = LoginState::CharacterCreateName;
            }
            "n" | "no" => {
                flow.create_dismissed = true;
            }
            "who" => {
                lines.extend(super::super::prompt::list_who(world, registry));
            }
            _ => {
                lines.push("You have no characters yet. Create one now? (y/n)".to_string());
            }
        }
        return lines;
    }

    match input.to_lowercase().as_str() {
        "c" => {
            drop(db_guard);
            clear_create_buffer(flow);
            lines.push(String::new());
            lines.push("--- Create a New Character ---".to_string());
            flow.state = LoginState::CharacterCreateName;
        }
        "who" => {
            drop(db_guard);
            lines.extend(super::super::prompt::list_who(world, registry));
        }
        _ => {
            if let Ok(idx) = input.parse::<usize>() {
                if idx == 0 || idx > chars.len() {
                    drop(db_guard);
                    lines.push("Invalid selection. Pick a number from the list, or type 'c' to create a new character.".to_string());
                } else {
                    let char_row = &chars[idx - 1];
                    drop(db_guard);
                    lines.extend(load_character(flow, world, char_row, db).await);
                }
            } else {
                drop(db_guard);
                lines.push(
                    "Type a number to pick a character, 'c' to create one, or 'who' to see who's online.".to_string(),
                );
            }
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Character Creation: Name, Race, Class
// ---------------------------------------------------------------------------

pub async fn handle_character_create_name_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<oxide_data::Database>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let name = input.trim();
    if !is_valid_character_name(name) {
        lines.push("Invalid name. Use 3-16 letters, hyphens, or apostrophes.".to_string());
        return lines;
    }

    let db = match db {
        Some(d) => d,
        None => return lines,
    };

    let db_guard = db.lock().await;
    let existing = match oxide_data::get_character_by_name(db_guard.conn(), name) {
        Ok(e) => e,
        Err(e) => {
            lines.push(format!("DB error: {e}"));
            return lines;
        }
    };
    drop(db_guard);

    if existing.is_some() {
        lines.push("That name is already taken. Please choose another.".to_string());
        return lines;
    }

    flow.create_buffer.name = Some(name.to_string());
    flow.state = LoginState::CharacterCreateRace(Vec::new());
    lines
}

pub fn handle_character_create_race_state(
    flow: &mut LoginFlow,
    input: &str,
    _templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();

    let ordered_races = match &flow.state {
        LoginState::CharacterCreateRace(ids) => ids.clone(),
        _ => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    match input.parse::<usize>() {
        Ok(idx) if idx > 0 && idx <= ordered_races.len() => {
            let race_id = ordered_races[idx - 1].clone();
            flow.create_buffer.race = Some(race_id);
            flow.state = LoginState::CharacterCreateClass(Vec::new());
        }
        _ => {
            lines.push("Invalid selection.".to_string());
        }
    }
    lines
}

pub fn handle_character_create_class_state(
    flow: &mut LoginFlow,
    input: &str,
    _templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();

    let ordered_classes = match &flow.state {
        LoginState::CharacterCreateClass(ids) => ids.clone(),
        _ => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    match input.parse::<usize>() {
        Ok(idx) if idx > 0 && idx <= ordered_classes.len() => {
            let class_id = ordered_classes[idx - 1].clone();
            flow.create_buffer.class = Some(class_id);
            flow.state = LoginState::CharacterCreateGender;
        }
        _ => {
            lines.push("Invalid selection.".to_string());
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Character Creation: Gender
// ---------------------------------------------------------------------------

pub fn handle_character_create_gender_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim().to_lowercase();

    let race_id = match flow.create_buffer.race.as_deref() {
        Some(r) => r,
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let allowed_genders = templates
        .and_then(|t| t.get_race(race_id))
        .map(|r| &r.allowed_genders)
        .cloned()
        .unwrap_or_default();

    // Parse input: number, or "other:<subject>/<object>/<possessive>"
    let mut gender_ids: Vec<String> = allowed_genders.keys().cloned().collect();
    gender_ids.sort();

    // Handle "other" input — requires pronoun specification
    if input.starts_with("other:") || input.starts_with("custom:") {
        let pronoun_str = input.split(':').nth(1).unwrap_or("");
        let parts: Vec<&str> = pronoun_str.split('/').collect();
        if parts.len() != 3 {
            lines.push("Invalid format. Use: other:subject/object/possessive".to_string());
            return lines;
        }
        let subject = parts[0].trim();
        let object = parts[1].trim();
        let possessive = parts[2].trim();
        if subject.is_empty() || object.is_empty() || possessive.is_empty() {
            lines.push("Pronouns must not be empty.".to_string());
            return lines;
        }
        flow.create_buffer.gender = Some("other".into());
        flow.create_buffer.pronoun_subject = Some(subject.into());
        flow.create_buffer.pronoun_object = Some(object.into());
        flow.create_buffer.pronoun_possessive = Some(possessive.into());
        flow.state = LoginState::CharacterCreateAttributesPickMethod;
        return lines;
    }

    // Number selection for standard genders
    if let Ok(idx) = input.parse::<usize>() {
        if idx > 0 && idx <= gender_ids.len() {
            let gender_id = gender_ids[idx - 1].clone();
            if gender_id == "other" {
                lines.push("Enter pronouns using: other:subject/object/possessive".to_string());
                return lines;
            }
            flow.create_buffer.gender = Some(gender_id.clone());
            // Look up pronouns from template, or use defaults
            if let Some(pronouns) = allowed_genders.get(&gender_id) {
                flow.create_buffer.pronoun_subject = Some(pronouns.subject.clone());
                flow.create_buffer.pronoun_object = Some(pronouns.object.clone());
                flow.create_buffer.pronoun_possessive = Some(pronouns.possessive.clone());
            }
            flow.state = LoginState::CharacterCreateAttributesPickMethod;
            return lines;
        }
    }

    lines.push("Invalid selection.".to_string());
    lines
}

// ---------------------------------------------------------------------------
// Attribute Method Selection
// ---------------------------------------------------------------------------

pub fn handle_attributes_pick_method_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    match input.trim() {
        "1" => {
            let attrs = [8u8; 6];
            flow.state = LoginState::CharacterCreateAttributesPointBuy {
                remaining: MAX_POINT_BUY_POINTS,
                attrs,
            };
        }
        "2" => {
            flow.state = LoginState::CharacterCreateAttributesArray {
                values: STANDARD_ARRAY,
                assign_idx: 0,
                attrs: [0u8; 6],
            };
        }
        "3" => {
            let rolls = roll_all_stats();
            flow.state = LoginState::CharacterCreateAttributesRoll {
                rolls,
                assign_idx: 0,
                attrs: [0u8; 6],
                rerolls: MAX_REROLLS,
            };
        }
        _ => {
            lines.push("Invalid selection. Pick 1, 2, or 3.".to_string());
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Point-Buy
// ---------------------------------------------------------------------------

pub fn handle_point_buy_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim().to_lowercase();

    if input == "done" || input == "d" || input == "c" || input == "confirm" {
        let state = match &flow.state {
            LoginState::CharacterCreateAttributesPointBuy { remaining, attrs } => {
                (*remaining, *attrs)
            }
            _ => return lines,
        };
        if state.0 > 0 {
            lines.push(format!(
                "You still have {} point(s) remaining. Use them or type 'reset' to start over.",
                state.0
            ));
            return lines;
        }
        flow.create_buffer.attributes = Some(oxide_core::Attributes::new(
            state.1[0], state.1[1], state.1[2], state.1[3], state.1[4], state.1[5],
        ));
        flow.state = LoginState::CharacterCreateAlignment;
        return lines;
    }

    if input == "reset" {
        flow.state = LoginState::CharacterCreateAttributesPointBuy {
            remaining: MAX_POINT_BUY_POINTS,
            attrs: [8u8; 6],
        };
        return lines;
    }

    // Parse "stat+", "stat-", "stat=N", "stat N", or "stat+N"/"stat-N"
    if let Some((prefix, rest)) = input.split_once(['+', '-', '=', ' ']) {
        let idx = match stat_index(prefix) {
            Some(i) => i,
            None => {
                lines.push(
                    "Unknown stat. Use str/dex/int/wis/con/cha, or 'done' to finish.".to_string(),
                );
                return lines;
            }
        };

        // Determine separator character to choose mode
        let sep = input.as_bytes()[prefix.len()] as char;

        match sep {
            '+' | '-' => {
                // Increment/decrement mode: str+, str+1, str-2, etc.
                let rest = rest.trim();
                let amount: u8 = if rest.is_empty() {
                    1
                } else {
                    match rest.parse() {
                        Ok(n) if n >= 1 => n,
                        _ => {
                            lines.push("Use +1 or -1 to adjust.".to_string());
                            return lines;
                        }
                    }
                };

                let state = match &flow.state {
                    LoginState::CharacterCreateAttributesPointBuy { remaining, attrs } => {
                        (*remaining, *attrs)
                    }
                    _ => return lines,
                };
                let old_val = state.1[idx];

                if sep == '+' {
                    if old_val + amount > 18 {
                        lines.push(format!("Maximum stat value is 18 (currently {old_val})."));
                        return lines;
                    }
                    let mut cost = 0u8;
                    for v in old_val..old_val + amount {
                        cost += point_buy_cost(v).unwrap_or(0);
                    }
                    if cost > state.0 {
                        lines.push(format!(
                            "Not enough points. You have {} remaining, need {cost}.",
                            state.0
                        ));
                        return lines;
                    }
                    let mut new_attrs = state.1;
                    new_attrs[idx] = old_val + amount;
                    flow.state = LoginState::CharacterCreateAttributesPointBuy {
                        remaining: state.0 - cost,
                        attrs: new_attrs,
                    };
                } else {
                    if old_val < 8 + amount {
                        lines.push(format!("Minimum stat value is 8 (currently {old_val})."));
                        return lines;
                    }
                    let mut refund = 0u8;
                    for v in old_val - amount..old_val {
                        refund += point_buy_cost(v).unwrap_or(0);
                    }
                    let mut new_attrs = state.1;
                    new_attrs[idx] = old_val - amount;
                    flow.state = LoginState::CharacterCreateAttributesPointBuy {
                        remaining: state.0 + refund,
                        attrs: new_attrs,
                    };
                }
                return lines;
            }
            '=' | ' ' => {
                // Absolute set mode: str=12, str 12
                let val: u8 = match rest.trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        lines.push("Usage: str=12, str+1, str-1, or 'done' to finish.".to_string());
                        return lines;
                    }
                };
                if !(8..=18).contains(&val) {
                    lines.push("Stats must be between 8 and 18.".to_string());
                    return lines;
                }
                let state = match &flow.state {
                    LoginState::CharacterCreateAttributesPointBuy { remaining, attrs } => {
                        (*remaining, *attrs)
                    }
                    _ => return lines,
                };
                let old_val = state.1[idx];
                if val == old_val {
                    return lines;
                }
                let cost_change = if val > old_val {
                    let mut cost = 0u8;
                    for v in old_val..val {
                        cost += point_buy_cost(v).unwrap_or(0);
                    }
                    if cost > state.0 {
                        lines.push(format!(
                            "Not enough points. You have {} remaining, need {cost}.",
                            state.0
                        ));
                        return lines;
                    }
                    cost
                } else {
                    let mut refund = 0u8;
                    for v in val..old_val {
                        refund += point_buy_cost(v).unwrap_or(0);
                    }
                    refund
                };
                let mut new_attrs = state.1;
                new_attrs[idx] = val;
                let new_remaining = if val > old_val {
                    state.0 - cost_change
                } else {
                    state.0 + cost_change
                };
                flow.state = LoginState::CharacterCreateAttributesPointBuy {
                    remaining: new_remaining,
                    attrs: new_attrs,
                };
                return lines;
            }
            _ => {}
        }
    }

    lines.push("Usage: str+1, dex-1, str=12, or 'done' to finish.".to_string());
    lines
}

// ---------------------------------------------------------------------------
// Standard Array
// ---------------------------------------------------------------------------

pub fn handle_standard_array_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();

    if input == "reset" {
        flow.state = LoginState::CharacterCreateAttributesArray {
            values: STANDARD_ARRAY,
            assign_idx: 0,
            attrs: [0u8; 6],
        };
        return lines;
    }

    let state = match &flow.state {
        LoginState::CharacterCreateAttributesArray {
            values,
            assign_idx,
            attrs,
        } => (*values, *assign_idx, *attrs),
        _ => return lines,
    };

    if state.1 >= 6 {
        flow.create_buffer.attributes = Some(oxide_core::Attributes::new(
            state.2[0], state.2[1], state.2[2], state.2[3], state.2[4], state.2[5],
        ));
        flow.state = LoginState::CharacterCreateAlignment;
        return lines;
    }

    let value_to_assign = state.0[state.1];
    match stat_index(input) {
        Some(idx) => {
            if state.2[idx] != 0 {
                lines.push("That stat already has a value assigned. Pick another.".to_string());
                return lines;
            }
            let mut new_attrs = state.2;
            new_attrs[idx] = value_to_assign;
            let new_idx = state.1 + 1;
            if new_idx >= 6 {
                flow.create_buffer.attributes = Some(oxide_core::Attributes::new(
                    new_attrs[0],
                    new_attrs[1],
                    new_attrs[2],
                    new_attrs[3],
                    new_attrs[4],
                    new_attrs[5],
                ));
                flow.state = LoginState::CharacterCreateAlignment;
            } else {
                flow.state = LoginState::CharacterCreateAttributesArray {
                    values: state.0,
                    assign_idx: new_idx,
                    attrs: new_attrs,
                };
            }
        }
        None => {
            lines.push(
                "Pick a stat name (str, dex, int, wis, con, cha) to assign the value to."
                    .to_string(),
            );
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Roll
// ---------------------------------------------------------------------------

pub fn handle_roll_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim().to_lowercase();

    if input == "reset" {
        let rolls = roll_all_stats();
        flow.state = LoginState::CharacterCreateAttributesRoll {
            rolls,
            assign_idx: 0,
            attrs: [0u8; 6],
            rerolls: MAX_REROLLS,
        };
        return lines;
    }

    let state = match &flow.state {
        LoginState::CharacterCreateAttributesRoll {
            rolls,
            assign_idx,
            attrs,
            rerolls,
        } => (*rolls, *assign_idx, *attrs, *rerolls),
        _ => return lines,
    };

    if state.3 > 0 && (input == "reroll" || input == "r") {
        let new_rolls = roll_all_stats();
        flow.state = LoginState::CharacterCreateAttributesRoll {
            rolls: new_rolls,
            assign_idx: 0,
            attrs: [0u8; 6],
            rerolls: state.3 - 1,
        };
        return lines;
    }

    if state.1 >= 6 {
        flow.create_buffer.attributes = Some(oxide_core::Attributes::new(
            state.2[0], state.2[1], state.2[2], state.2[3], state.2[4], state.2[5],
        ));
        flow.state = LoginState::CharacterCreateAlignment;
        return lines;
    }

    let value_to_assign = state.0[state.1];
    match stat_index(&input) {
        Some(idx) => {
            if state.2[idx] != 0 {
                lines.push("That stat already has a value assigned. Pick another.".to_string());
                return lines;
            }
            let mut new_attrs = state.2;
            new_attrs[idx] = value_to_assign;
            let new_idx = state.1 + 1;
            if new_idx >= 6 {
                flow.create_buffer.attributes = Some(oxide_core::Attributes::new(
                    new_attrs[0],
                    new_attrs[1],
                    new_attrs[2],
                    new_attrs[3],
                    new_attrs[4],
                    new_attrs[5],
                ));
                flow.state = LoginState::CharacterCreateAlignment;
            } else {
                flow.state = LoginState::CharacterCreateAttributesRoll {
                    rolls: state.0,
                    assign_idx: new_idx,
                    attrs: new_attrs,
                    rerolls: state.3,
                };
            }
        }
        None => {
            if state.1 == 0 && state.3 > 0 {
                lines.push("Pick a stat name (str, dex, int, wis, con, cha) to assign, or type 'reroll' to discard all assignments and re-roll." .to_string());
            } else {
                lines.push(
                    "Pick a stat name (str, dex, int, wis, con, cha) to assign the rolled value."
                        .to_string(),
                );
            }
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Alignment Selection
// ---------------------------------------------------------------------------

pub fn handle_alignment_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();

    let idx: usize = match input.parse() {
        Ok(i) if (1..=9).contains(&i) => i,
        _ => {
            lines.push("Invalid selection. Pick a number 1-9.".to_string());
            return lines;
        }
    };

    let alignment = Alignment::ALL[idx - 1];

    if let Some(templates) = templates {
        // Check class alignment restrictions
        if let Some(class_id) = flow.create_buffer.class.as_deref() {
            if let Some(class) = templates.get_class(class_id) {
                if !class.allowed_alignments.is_empty()
                    && !class.allowed_alignments.iter().any(|a| a == alignment)
                {
                    lines.push(format!(
                        "{} is not allowed for {}. Pick a valid alignment.",
                        alignment, class.name
                    ));
                    return lines;
                }
            }
        }

        // Check race alignment restrictions
        if let Some(race_id) = flow.create_buffer.race.as_deref() {
            if let Some(race) = templates.get_race(race_id) {
                if !race.allowed_alignments.is_empty()
                    && !race.allowed_alignments.iter().any(|a| a == alignment)
                {
                    lines.push(format!(
                        "{} is not allowed for {}. Pick a valid alignment.",
                        alignment, race.name
                    ));
                    return lines;
                }
            }
        }
    }

    flow.create_buffer.alignment = Some(alignment.to_string());
    lines.extend(transition_to_deity(flow, templates));
    lines
}

fn get_allowed_deities(flow: &LoginFlow, templates: Option<&TemplateRegistry>) -> Vec<String> {
    let mut allowed = Vec::new();
    let templates = match templates {
        Some(t) => t,
        None => return allowed,
    };

    let race_id = flow.create_buffer.race.as_deref().unwrap_or("");
    let class_id = flow.create_buffer.class.as_deref().unwrap_or("");
    let alignment_str = flow.create_buffer.alignment.as_deref().unwrap_or("");

    let class = templates.get_class(class_id);
    let policy = class
        .map(|c| &c.deity_policy)
        .unwrap_or(&oxide_core::templates::DeityPolicy::Any);

    if matches!(policy, oxide_core::templates::DeityPolicy::None) {
        return allowed;
    }

    for (id, deity) in &templates.deities {
        if let oxide_core::templates::DeityPolicy::Subset(subset) = policy {
            if !subset.contains(id) {
                continue;
            }
        }
        if !deity.allowed_classes.is_empty()
            && !deity.allowed_classes.contains(&class_id.to_string())
        {
            continue;
        }
        if !deity.allowed_races.is_empty() && !deity.allowed_races.contains(&race_id.to_string()) {
            continue;
        }
        if !deity.allowed_alignments.is_empty()
            && !deity
                .allowed_alignments
                .contains(&alignment_str.to_string())
        {
            continue;
        }
        allowed.push(id.clone());
    }

    allowed.sort();
    allowed
}

fn transition_to_deity(flow: &mut LoginFlow, templates: Option<&TemplateRegistry>) -> Vec<String> {
    let lines = Vec::new();
    let class_id = flow.create_buffer.class.as_deref().unwrap_or("");
    let class_policy = templates
        .and_then(|t| t.get_class(class_id))
        .map(|c| &c.deity_policy)
        .unwrap_or(&oxide_core::templates::DeityPolicy::Any);

    if matches!(class_policy, oxide_core::templates::DeityPolicy::None) {
        flow.create_buffer.deity = None;
        transition_from_deity(flow, templates);
    } else {
        let options = get_allowed_deities(flow, templates);
        if options.is_empty() {
            flow.create_buffer.deity = None;
            transition_from_deity(flow, templates);
        } else {
            flow.state = LoginState::CharacterCreateDeity(options);
        }
    }
    lines
}

fn transition_from_deity(flow: &mut LoginFlow, templates: Option<&TemplateRegistry>) {
    let class = flow
        .create_buffer
        .class
        .as_deref()
        .and_then(|class_id| templates.and_then(|t| t.get_class(class_id)));
    let has_pool = class.map(|c| !c.skill_pool.is_empty()).unwrap_or(false);

    if let Some(c) = class.filter(|_| has_pool) {
        flow.state = LoginState::CharacterCreateSkillSelection {
            pool: c.skill_pool.clone(),
            selected: Vec::new(),
            slots: c.starting_skill_slots,
        };
    } else {
        flow.state = LoginState::CharacterCreateAppearanceHeight;
    }
}

fn get_race_appearance_bounds(
    flow: &LoginFlow,
    templates: Option<&TemplateRegistry>,
) -> oxide_core::templates::AppearanceBounds {
    let race_id = flow.create_buffer.race.as_deref().unwrap_or("");
    templates
        .and_then(|t| t.get_race(race_id))
        .map(|r| r.appearance_bounds.clone())
        .unwrap_or_default()
}

pub fn handle_character_create_deity_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();
    let input_lower = input.to_lowercase();

    let options = match &flow.state {
        LoginState::CharacterCreateDeity(opts) => opts.clone(),
        _ => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let class_id = flow.create_buffer.class.as_deref().unwrap_or("");
    let class_policy = templates
        .and_then(|t| t.get_class(class_id))
        .map(|c| &c.deity_policy)
        .unwrap_or(&oxide_core::templates::DeityPolicy::Any);

    if input_lower == "none" {
        if matches!(
            class_policy,
            oxide_core::templates::DeityPolicy::Any | oxide_core::templates::DeityPolicy::None
        ) {
            flow.create_buffer.deity = None;
            transition_from_deity(flow, templates);
            return lines;
        } else {
            lines.push(
                "Your class requires you to choose a deity. 'none' is not allowed.".to_string(),
            );
            return lines;
        }
    }

    let matched_deity = if let Ok(idx) = input_lower.parse::<usize>() {
        if idx > 0 && idx <= options.len() {
            Some(&options[idx - 1])
        } else {
            None
        }
    } else {
        options.iter().find(|d| {
            d.to_lowercase() == input_lower
                || templates
                    .and_then(|t| t.deities.get(*d))
                    .map(|deity| deity.name.to_lowercase() == input_lower)
                    .unwrap_or(false)
        })
    };

    if let Some(deity_id) = matched_deity {
        flow.create_buffer.deity = Some(deity_id.clone());
        transition_from_deity(flow, templates);
    } else {
        lines.push(format!("'{}' is not a valid deity option.", input));
    }
    lines
}

pub fn handle_appearance_height_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();
    let bounds = get_race_appearance_bounds(flow, templates);

    match input.parse::<u8>() {
        Ok(height) if height >= bounds.height_min && height <= bounds.height_max => {
            flow.create_buffer.appearance_height = Some(height);
            flow.state = LoginState::CharacterCreateAppearanceWeight;
        }
        _ => {
            lines.push(format!(
                "Invalid height. Must be a number between {} and {}.",
                bounds.height_min, bounds.height_max
            ));
        }
    }
    lines
}

pub fn handle_appearance_weight_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();
    let bounds = get_race_appearance_bounds(flow, templates);

    match input.parse::<u16>() {
        Ok(weight) if weight >= bounds.weight_min && weight <= bounds.weight_max => {
            flow.create_buffer.appearance_weight = Some(weight);
            flow.state = LoginState::CharacterCreateAppearanceBuild(bounds.allowed_builds.clone());
        }
        _ => {
            lines.push(format!(
                "Invalid weight. Must be a number between {} and {}.",
                bounds.weight_min, bounds.weight_max
            ));
        }
    }
    lines
}

pub fn handle_appearance_build_state(
    flow: &mut LoginFlow,
    input: &str,
    _templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();
    let input_lower = input.to_lowercase();

    let options = match &flow.state {
        LoginState::CharacterCreateAppearanceBuild(opts) => opts.clone(),
        _ => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let matched = match_option_index_or_name(&input_lower, &options);
    if let Some(build) = matched {
        flow.create_buffer.appearance_build = Some(build.clone());
        flow.state = LoginState::CharacterCreateAppearanceHairStyle;
    } else {
        lines.push(format!("'{}' is not a valid build option.", input));
    }
    lines
}

pub fn handle_appearance_hair_style_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();
    if input.is_empty() {
        lines.push("Hair style cannot be empty.".to_string());
        return lines;
    }
    flow.create_buffer.appearance_hair_style = Some(input.to_string());
    let bounds = get_race_appearance_bounds(flow, templates);
    flow.state = LoginState::CharacterCreateAppearanceHairColor(bounds.allowed_hair_colors.clone());
    lines
}

pub fn handle_appearance_hair_color_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();
    let input_lower = input.to_lowercase();

    let options = match &flow.state {
        LoginState::CharacterCreateAppearanceHairColor(opts) => opts.clone(),
        _ => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let matched = match_option_index_or_name(&input_lower, &options);
    if let Some(color) = matched {
        flow.create_buffer.appearance_hair_color = Some(color.clone());
        let bounds = get_race_appearance_bounds(flow, templates);
        flow.state =
            LoginState::CharacterCreateAppearanceEyeColor(bounds.allowed_eye_colors.clone());
    } else {
        lines.push(format!("'{}' is not a valid hair color option.", input));
    }
    lines
}

pub fn handle_appearance_eye_color_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();
    let input_lower = input.to_lowercase();

    let options = match &flow.state {
        LoginState::CharacterCreateAppearanceEyeColor(opts) => opts.clone(),
        _ => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let matched = match_option_index_or_name(&input_lower, &options);
    if let Some(color) = matched {
        flow.create_buffer.appearance_eye_color = Some(color.clone());
        let bounds = get_race_appearance_bounds(flow, templates);
        flow.state =
            LoginState::CharacterCreateAppearanceSkinTone(bounds.allowed_skin_tones.clone());
    } else {
        lines.push(format!("'{}' is not a valid eye color option.", input));
    }
    lines
}

pub fn handle_appearance_skin_tone_state(
    flow: &mut LoginFlow,
    input: &str,
    _templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();
    let input_lower = input.to_lowercase();

    let options = match &flow.state {
        LoginState::CharacterCreateAppearanceSkinTone(opts) => opts.clone(),
        _ => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let matched = match_option_index_or_name(&input_lower, &options);
    if let Some(tone) = matched {
        flow.create_buffer.appearance_skin_tone = Some(tone.clone());
        flow.state = LoginState::CharacterCreateAge;
    } else {
        lines.push(format!("'{}' is not a valid skin tone option.", input));
    }
    lines
}

pub fn handle_age_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim();

    let race = flow
        .create_buffer
        .race
        .as_deref()
        .and_then(|r_id| templates.and_then(|t| t.get_race(r_id)));

    let (age_default, age_max) = match race {
        Some(r) => (r.age_default, r.age_max),
        None => (20, 100),
    };

    if input.is_empty() {
        flow.create_buffer.age = Some(age_default);
        flow.state = LoginState::CharacterCreateDescription { lines: Vec::new() };
        return lines;
    }

    match input.parse::<u16>() {
        Ok(age) if age >= age_default && age <= age_max => {
            flow.create_buffer.age = Some(age);
            flow.state = LoginState::CharacterCreateDescription { lines: Vec::new() };
        }
        _ => {
            lines.push(format!(
                "Invalid age. Must be a number between {} and {}.",
                age_default, age_max
            ));
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Skill Selection
// ---------------------------------------------------------------------------

pub fn handle_skill_selection_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let input = input.trim().to_lowercase();

    let (pool, selected, slots) = match &flow.state {
        LoginState::CharacterCreateSkillSelection {
            pool,
            selected,
            slots,
        } => (pool.clone(), selected.clone(), *slots),
        _ => return lines,
    };

    if input == "done" || input == "d" {
        flow.create_buffer.selected_skills = selected;
        flow.state = LoginState::CharacterCreateAppearanceHeight;
        return lines;
    }

    if input == "list" || input == "l" {
        for skill_id in &pool {
            let display = templates
                .and_then(|t| t.get_skill(skill_id))
                .map(|s| format!("{} — {}", s.id, s.name))
                .unwrap_or_else(|| skill_id.clone());
            let chosen = if selected.contains(skill_id) {
                " [selected]"
            } else {
                ""
            };
            lines.push(format!("  {display}{chosen}"));
        }
        return lines;
    }

    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let (action, raw_target) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        ("add", parts[0])
    };

    let target_id = match resolve_skill_input(raw_target, templates, Some(&pool)) {
        Ok(id) => id,
        Err(msg) => {
            lines.push(msg);
            return lines;
        }
    };

    match action {
        "add" | "a" | "take" | "t" => {
            if templates.is_none() && !pool.contains(&target_id) {
                lines.push(format!("'{target_id}' is not a valid skill."));
                return lines;
            }
            if selected.contains(&target_id) {
                lines.push(format!("You already selected '{target_id}'."));
                return lines;
            }
            if selected.len() >= slots as usize {
                lines.push(format!(
                    "You can only select {slots} skills. Use 'remove' first."
                ));
                return lines;
            }
            let mut new_selected = selected;
            new_selected.push(target_id);
            flow.state = LoginState::CharacterCreateSkillSelection {
                pool,
                selected: new_selected,
                slots,
            };
        }
        "remove" | "r" | "rm" => {
            if !selected.contains(&target_id) {
                lines.push(format!("'{target_id}' is not selected."));
                return lines;
            }
            let mut new_selected = selected;
            new_selected.retain(|s| s != &target_id);
            flow.state = LoginState::CharacterCreateSkillSelection {
                pool,
                selected: new_selected,
                slots,
            };
        }
        _ => {
            lines.push("Commands: add <skill>, remove <skill>, list, done".to_string());
        }
    }

    lines
}

/// Helper: resolve a skill name (exact or partial) to a skill ID, or return an
/// error message suitable for display to the user.
fn resolve_skill_input(
    input: &str,
    templates: Option<&TemplateRegistry>,
    pool: Option<&[String]>,
) -> Result<String, String> {
    let templates = match templates {
        Some(t) => t,
        None => return Ok(input.to_string()),
    };

    match templates.resolve_skill(input, pool) {
        Ok(id) => Ok(id),
        Err(SkillResolveError::NotFound) => Err(format!("'{input}' is not a valid skill.")),
        Err(SkillResolveError::Multiple(candidates)) => {
            let names: Vec<String> = candidates
                .into_iter()
                .map(|(id, name)| format!("{id} ({name})"))
                .collect();
            Err(format!("Which skill did you mean? {}", names.join(", ")))
        }
    }
}

// ---------------------------------------------------------------------------
// Description
// ---------------------------------------------------------------------------

pub fn handle_description_state(flow: &mut LoginFlow, input: &str) -> Vec<String> {
    let lines = Vec::new();

    let state_lines = match &flow.state {
        LoginState::CharacterCreateDescription { lines } => lines.clone(),
        _ => return lines,
    };

    let trimmed = input.trim_end();

    if trimmed == "." && state_lines.is_empty() {
        flow.create_buffer.description = Some(String::new());
        flow.state = LoginState::CharacterCreateSpawn;
        return lines;
    }

    if trimmed == "." {
        let desc = state_lines.join("\n");
        flow.create_buffer.description = Some(desc);
        flow.state = LoginState::CharacterCreateSpawn;
        return lines;
    }

    let mut new_lines = state_lines;
    new_lines.push(input.to_string());
    flow.state = LoginState::CharacterCreateDescription { lines: new_lines };
    lines
}

// ---------------------------------------------------------------------------
// Spawn Selection
// ---------------------------------------------------------------------------

pub fn handle_spawn_select_state(
    flow: &mut LoginFlow,
    input: &str,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let templates = match templates {
        Some(t) => t,
        None => {
            lines.push("No spawn points available.".to_string());
            return lines;
        }
    };

    let race_id = match flow.create_buffer.race.as_deref() {
        Some(r) => r,
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let class_id = match flow.create_buffer.class.as_deref() {
        Some(c) => c,
        None => {
            lines.push("Session error. Starting over.".to_string());
            flow.state = LoginState::CharacterCreateName;
            return lines;
        }
    };

    let alignment = flow
        .create_buffer
        .alignment
        .as_deref()
        .unwrap_or("true_neutral");
    let available = templates.available_spawns(race_id, class_id, alignment);
    let input = input.trim();

    match input.parse::<usize>() {
        Ok(idx) if idx > 0 && idx <= available.len() => {
            let (area_id, spawn) = available[idx - 1];
            let spawn_key = format!("{}:{}", area_id, spawn.room);
            flow.create_buffer.spawn_key = Some(spawn_key);
            flow.state = LoginState::CharacterCreateConfirm;
        }
        _ => {
            lines.push("Invalid selection.".to_string());
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Confirm
// ---------------------------------------------------------------------------

pub async fn handle_character_create_confirm_state(
    flow: &mut LoginFlow,
    input: &str,
    db: Option<&Mutex<oxide_data::Database>>,
    world: &mut World,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => {
            lines.extend(finalize_character(flow, db, world, templates).await);
        }
        "n" | "no" => {
            clear_create_buffer(flow);
            lines.push("Character creation cancelled.".to_string());
            flow.state = LoginState::CharacterSelect;
        }
        _ => {
            lines.push("Type 'y' or 'n'.".to_string());
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Character creation finalisation
// ---------------------------------------------------------------------------

fn check_session<T>(
    value: Option<T>,
    label: &str,
    lines: &mut Vec<String>,
    flow: &mut LoginFlow,
) -> Option<T> {
    match value {
        Some(v) => Some(v),
        None => {
            lines.push(format!("Session error: no {label}. Starting over."));
            flow.state = LoginState::CharacterSelect;
            None
        }
    }
}

async fn finalize_character(
    flow: &mut LoginFlow,
    db: Option<&Mutex<oxide_data::Database>>,
    world: &mut World,
    templates: Option<&TemplateRegistry>,
) -> Vec<String> {
    let mut lines = Vec::new();

    let name = match check_session(flow.create_buffer.name.clone(), "name", &mut lines, flow) {
        Some(n) => n,
        None => return lines,
    };
    let race_id = match check_session(flow.create_buffer.race.clone(), "race", &mut lines, flow) {
        Some(r) => r,
        None => return lines,
    };
    let class_id = match check_session(flow.create_buffer.class.clone(), "class", &mut lines, flow)
    {
        Some(c) => c,
        None => return lines,
    };
    let player_base_attrs = match check_session(
        flow.create_buffer.attributes.clone(),
        "attributes",
        &mut lines,
        flow,
    ) {
        Some(a) => a,
        None => return lines,
    };
    let alignment = match check_session(
        flow.create_buffer.alignment.clone(),
        "alignment",
        &mut lines,
        flow,
    ) {
        Some(a) => a,
        None => return lines,
    };
    let description = flow.create_buffer.description.clone().unwrap_or_default();
    let spawn_key = match check_session(
        flow.create_buffer.spawn_key.clone(),
        "spawn location",
        &mut lines,
        flow,
    ) {
        Some(s) => s,
        None => return lines,
    };
    let account_id = match check_session(flow.account_id, "account", &mut lines, flow) {
        Some(id) => id,
        None => return lines,
    };

    let db_con = match db {
        Some(d) => d,
        None => {
            lines.push("Server error: database unavailable.".to_string());
            return lines;
        }
    };

    let (attrs, hp, mut skills) =
        compute_final_attributes(templates, &race_id, &class_id, &player_base_attrs);
    let mana = Mana::from_formula(1, attrs.intelligence as u16, attrs.wisdom as u16);
    let stamina = Stamina::from_formula(1, attrs.strength as u16, attrs.dexterity as u16);
    // Grant player-chosen skills
    for skill_id in &flow.create_buffer.selected_skills {
        skills.grant(skill_id);
    }
    let starting_gold = class_starting_gold(templates, &class_id);
    let class = templates.and_then(|t| t.get_class(&class_id));
    let combat_stats = if let Some(c) = class {
        c.calculate_combat_stats(1)
    } else {
        CombatStats::default()
    };

    let room_entity = match templates.and_then(|t| t.find_room_by_key(world, &spawn_key)) {
        Some(r) => r,
        None => {
            lines.push("Error: The selected starting room could not be found. Please select a starting location again.".to_string());
            flow.state = LoginState::CharacterCreateSpawn;
            return lines;
        }
    };

    let db_guard = db_con.lock().await;
    let conn_db = db_guard.conn();

    let entity_id = match oxide_data::insert_entity(conn_db, "player") {
        Ok(id) => id,
        Err(e) => {
            lines.push(format!("Error creating character: {e}"));
            return lines;
        }
    };

    if let Err(e) = oxide_data::save_player_component(conn_db, entity_id, account_id, None, 80) {
        lines.push(format!("Error saving character: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_practice_points(conn_db, entity_id, 0) {
        lines.push(format!("Error saving practice points: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_attributes_component(
        conn_db,
        entity_id,
        &oxide_data::AttributesRow {
            strength: attrs.strength,
            dexterity: attrs.dexterity,
            intelligence: attrs.intelligence,
            wisdom: attrs.wisdom,
            constitution: attrs.constitution,
            charisma: attrs.charisma,
        },
    ) {
        lines.push(format!("Error saving attributes: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_health_component(conn_db, entity_id, hp, hp) {
        lines.push(format!("Error saving health: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_mana_component(conn_db, entity_id, mana.current as i32) {
        lines.push(format!("Error saving mana: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_stamina_component(conn_db, entity_id, stamina.current as i32) {
        lines.push(format!("Error saving stamina: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_level_component(conn_db, entity_id, 1) {
        lines.push(format!("Error saving level: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_experience_component(conn_db, entity_id, 0) {
        lines.push(format!("Error saving experience: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_combat_stats_component(
        conn_db,
        entity_id,
        combat_stats.base_attack_bonus,
        combat_stats.fort_save,
        combat_stats.ref_save,
        combat_stats.will_save,
    ) {
        lines.push(format!("Error saving combat stats: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_alignment_component(conn_db, entity_id, &alignment) {
        lines.push(format!("Error saving alignment: {e}"));
        return lines;
    }

    if !description.is_empty() {
        if let Err(e) = oxide_data::save_description_component(conn_db, entity_id, &description) {
            lines.push(format!("Error saving description: {e}"));
            return lines;
        }
    }

    if let Err(e) = oxide_data::save_golds_component(
        conn_db,
        entity_id,
        starting_gold.copper as i64,
        starting_gold.silver as i64,
        starting_gold.gold as i64,
        starting_gold.platinum as i64,
    ) {
        lines.push(format!("Error saving gold: {e}"));
        return lines;
    }

    if let Err(e) = oxide_data::save_skills(conn_db, entity_id, &skills.skills) {
        lines.push(format!("Error saving skills: {e}"));
        return lines;
    }

    let height = flow.create_buffer.appearance_height.unwrap_or(66) as i32;
    let weight = flow.create_buffer.appearance_weight.unwrap_or(160) as i32;
    let build = flow
        .create_buffer
        .appearance_build
        .clone()
        .unwrap_or_else(|| "average".into());
    let hair_style = flow
        .create_buffer
        .appearance_hair_style
        .clone()
        .unwrap_or_else(|| "straight".into());
    let hair_color = flow
        .create_buffer
        .appearance_hair_color
        .clone()
        .unwrap_or_else(|| "brown".into());
    let eye_color = flow
        .create_buffer
        .appearance_eye_color
        .clone()
        .unwrap_or_else(|| "brown".into());
    let skin_tone = flow
        .create_buffer
        .appearance_skin_tone
        .clone()
        .unwrap_or_else(|| "fair".into());

    if let Err(e) = oxide_data::save_appearance_component(
        conn_db,
        entity_id,
        height,
        weight,
        &build,
        &hair_color,
        &hair_style,
        &eye_color,
        &skin_tone,
    ) {
        lines.push(format!("Error saving appearance: {e}"));
        return lines;
    }

    let age = flow.create_buffer.age.unwrap_or(20) as i32;
    if let Err(e) = oxide_data::save_age_component(conn_db, entity_id, age) {
        lines.push(format!("Error saving age: {e}"));
        return lines;
    }

    if let Some(ref deity_id) = flow.create_buffer.deity {
        if let Err(e) = oxide_data::save_deity_component(conn_db, entity_id, deity_id) {
            lines.push(format!("Error saving deity: {e}"));
            return lines;
        }
    }

    let char_id = match oxide_data::create_character(
        conn_db,
        account_id,
        &name,
        &race_id,
        &class_id,
        entity_id,
        Some(&spawn_key),
        None, // new character hasn't saved a position yet
    ) {
        Ok(id) => id,
        Err(e) => {
            lines.push(format!("Error saving character: {e}"));
            return lines;
        }
    };

    // Persist initial recall room (same as spawn point)
    if let Err(e) = oxide_data::update_character_recall_room_key(conn_db, entity_id, &spawn_key) {
        lines.push(format!("Error saving recall room: {e}"));
        return lines;
    }

    let gender_id = flow
        .create_buffer
        .gender
        .clone()
        .unwrap_or_else(|| "neutral".into());
    let pronoun_s = flow
        .create_buffer
        .pronoun_subject
        .clone()
        .unwrap_or_else(|| "they".into());
    let pronoun_o = flow
        .create_buffer
        .pronoun_object
        .clone()
        .unwrap_or_else(|| "them".into());
    let pronoun_p = flow
        .create_buffer
        .pronoun_possessive
        .clone()
        .unwrap_or_else(|| "their".into());

    if let Err(e) = oxide_data::update_character_gender(
        conn_db, char_id, &gender_id, &pronoun_s, &pronoun_o, &pronoun_p,
    ) {
        lines.push(format!("Error saving gender: {e}"));
        return lines;
    }

    let access_level = oxide_data::get_account_by_id(conn_db, account_id)
        .ok()
        .flatten()
        .map(|a| a.access_level)
        .unwrap_or_else(|| "player".to_string());

    let level_enum = match access_level.to_lowercase().as_str() {
        "builder" => oxide_core::AccessLevel::Builder,
        "immortal" => oxide_core::AccessLevel::Immortal,
        "god" => oxide_core::AccessLevel::God,
        "admin" => oxide_core::AccessLevel::Admin,
        _ => oxide_core::AccessLevel::Player,
    };

    drop(db_guard);

    let player = world.spawn((
        Position::new(room_entity),
        Name::new(name.clone()),
        Player::new(account_id),
        Race(race_id.clone()),
        Class(class_id.clone()),
        Gender::new(
            gender_id.clone(),
            pronoun_s.clone(),
            pronoun_o.clone(),
            pronoun_p.clone(),
        ),
        attrs,
        Health::new(hp),
        mana,
        stamina,
        Level::default(),
        Experience::default(),
        RecallRoom(room_entity),
        PlayerState::default(),
        level_enum,
    ));

    if level_enum > oxide_core::AccessLevel::Player {
        let _ = world.insert(player, (oxide_core::Immortal,));
    }

    let appearance = oxide_core::Appearance {
        height: height as u8,
        weight: weight as u16,
        build,
        hair_color,
        hair_style,
        eye_color,
        skin_tone,
    };
    let age_comp = oxide_core::Age(age as u16);
    let deity_comp = oxide_core::Deity(flow.create_buffer.deity.clone());
    let mut multiclass_info = oxide_core::MultiClassInfo::default();
    multiclass_info.add_class(class_id.clone(), 1, true);

    let _ = world.insert(
        player,
        (
            skills,
            DbId::new(entity_id),
            Alignment(alignment),
            Description(description),
            starting_gold,
            Inventory::new(),
            Equipment::new(),
            appearance,
            age_comp,
            deity_comp,
            multiclass_info,
        ),
    );

    if let Some(templates) = templates {
        oxide_core::systems::passive::apply_all_passives(world, player, templates);
    }

    if let Some(class) = class {
        for item_id in &class.starting_items {
            if let Some(t) = templates {
                spawn_starting_item(world, player, t, item_id);
            }
        }
    }

    flow.entity = Some(player);
    flow.entity_just_spawned = true;

    lines.push(String::new());
    lines.push(format!("Welcome, {name}! Your adventure begins."));
    flow.state = LoginState::Playing;
    lines
}

/// Spawn a starting item from the template registry tied to the player.
fn spawn_starting_item(
    world: &mut World,
    player: oxide_core::Entity,
    templates: &TemplateRegistry,
    item_id: &str,
) {
    let spawn = oxide_core::systems::loot::ItemSpawn {
        template_id: item_id.to_string(),
        count: 1,
        quality: oxide_core::systems::loot::QualityTier::Common,
        prefix_ids: vec![],
        suffix_ids: vec![],
    };

    let Some(item_entity) = oxide_core::systems::loot::spawn_loot_item(world, &spawn, templates)
    else {
        return;
    };

    if let Ok(mut q) = world.query_one::<&mut Inventory>(player) {
        if let Some(inv) = q.get() {
            inv.0.push(item_entity);
        }
    }
}

// ---------------------------------------------------------------------------
// Load an existing character
// ---------------------------------------------------------------------------

async fn load_character(
    flow: &mut LoginFlow,
    world: &mut World,
    char_row: &oxide_data::CharacterRow,
    db: &Mutex<oxide_data::Database>,
) -> Vec<String> {
    let mut lines = Vec::new();

    let db_guard = db.lock().await;
    let conn_db = db_guard.conn();

    let entity_id = char_row.entity_id;

    let attrs = oxide_data::load_attributes_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|a| {
            oxide_core::Attributes::new(
                a.strength,
                a.dexterity,
                a.intelligence,
                a.wisdom,
                a.constitution,
                a.charisma,
            )
        })
        .unwrap_or_default();

    let hp = oxide_data::load_health_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|(current, max)| Health { current, max })
        .unwrap_or_else(|| Health::new(20));

    let mana = {
        let level = oxide_data::load_level_component(conn_db, entity_id)
            .ok()
            .flatten()
            .unwrap_or(1);
        let max =
            Mana::from_formula(level as u16, attrs.intelligence as u16, attrs.wisdom as u16).max;
        let current = oxide_data::load_mana_component(conn_db, entity_id)
            .ok()
            .flatten()
            .unwrap_or(max as i32)
            .min(max as i32)
            .max(0) as u16;
        Mana { current, max }
    };

    let stamina = {
        let level = oxide_data::load_level_component(conn_db, entity_id)
            .ok()
            .flatten()
            .unwrap_or(1);
        let max =
            Stamina::from_formula(level as u16, attrs.strength as u16, attrs.dexterity as u16).max;
        let current = oxide_data::load_stamina_component(conn_db, entity_id)
            .ok()
            .flatten()
            .unwrap_or(max as i32)
            .min(max as i32)
            .max(0) as u16;
        Stamina { current, max }
    };

    // Save back if either was missing (migration: fill components_mana/stamina)
    if oxide_data::load_mana_component(conn_db, entity_id)
        .ok()
        .flatten()
        .is_none()
    {
        let _ = oxide_data::save_mana_component(conn_db, entity_id, mana.current as i32);
    }
    if oxide_data::load_stamina_component(conn_db, entity_id)
        .ok()
        .flatten()
        .is_none()
    {
        let _ = oxide_data::save_stamina_component(conn_db, entity_id, stamina.current as i32);
    }

    let level = oxide_data::load_level_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|l| Level(l as u8))
        .unwrap_or_default();

    let xp = oxide_data::load_experience_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|x| Experience(x as u64))
        .unwrap_or_default();

    let alignment = oxide_data::load_alignment_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(Alignment)
        .unwrap_or_default();

    let description = oxide_data::load_description_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(Description)
        .unwrap_or_default();

    let (prompt, screen_width) = oxide_data::load_player_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|(_, prompt, width)| (prompt, width))
        .unwrap_or_else(|| (None, 80));

    let practice_points = oxide_data::load_practice_points(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|p| p as u32)
        .unwrap_or(0);

    let combat_stats = oxide_data::load_combat_stats_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|(bab, fort, ref_save, will)| CombatStats {
            base_attack_bonus: bab,
            fort_save: fort,
            ref_save,
            will_save: will,
        })
        .unwrap_or_else(|| {
            let c_id = &char_row.class;
            if let Some(t) = crate::get_templates() {
                if let Some(class_template) = t.get_class(c_id) {
                    return class_template.calculate_combat_stats(level.0);
                }
            }
            CombatStats::default()
        });

    tracing::debug!(
        entity_id,
        prompt = ?prompt,
        "load_character: loaded prompt from DB"
    );

    let mut player_comp = Player::new(char_row.account_id);
    player_comp.prompt = prompt;
    player_comp.screen_width = screen_width;

    let gold = oxide_data::load_golds_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|(copper, silver, gold, platinum)| {
            Wallet::new(copper as u64, silver as u64, gold as u64, platinum as u64)
        })
        .unwrap_or_default();

    let skills_map = oxide_data::load_skills(conn_db, entity_id)
        .ok()
        .unwrap_or_default();
    let mut skills = oxide_core::LearnedSkills::new();
    for (skill_id, rank) in skills_map {
        skills.set_rank(&skill_id, rank);
    }

    let quest_log = oxide_data::load_quest_log_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|json| serde_json::from_str::<oxide_core::QuestLog>(&json).unwrap_or_default())
        .unwrap_or_default();

    let faction_standing = oxide_data::load_faction_standing_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|json| serde_json::from_str::<oxide_core::FactionStanding>(&json).unwrap_or_default())
        .unwrap_or_default();

    let learned_recipes = oxide_data::load_learned_recipes_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|json| serde_json::from_str::<oxide_core::LearnedRecipes>(&json).unwrap_or_default())
        .unwrap_or_default();

    let multiclass_json = oxide_data::load_multiclass_component(conn_db, entity_id)
        .ok()
        .flatten();
    let multiclass_info = multiclass_json
        .map(|json| serde_json::from_str::<oxide_core::MultiClassInfo>(&json).unwrap_or_default())
        .unwrap_or_else(|| {
            let mut mc = oxide_core::MultiClassInfo::default();
            mc.add_class(char_row.class.clone(), char_row.level as u8, true);
            mc
        });

    // Load inventory
    let inv_rows = oxide_data::load_inventory(conn_db, entity_id).unwrap_or_default();
    let mut inventory = oxide_core::Inventory::new();
    if let Some(ref templates) = crate::get_templates() {
        for (item_db_id, _) in inv_rows {
            if let Ok(Some(template_id)) = oxide_data::load_item_component(conn_db, item_db_id) {
                if let Some(item_tmpl) = templates.get_item(&template_id) {
                    let item_entity = world.spawn((
                        oxide_core::Name::new(item_tmpl.name.clone()),
                        oxide_core::Item::new(template_id),
                        oxide_core::DbId::new(item_db_id),
                    ));
                    inventory.0.push(item_entity);
                }
            }
        }
    }

    // Load equipment
    let eq_rows = oxide_data::load_equipment(conn_db, entity_id).unwrap_or_default();
    let mut equipment = oxide_core::Equipment::new();
    if let Some(ref templates) = crate::get_templates() {
        for (slot_str, item_db_id) in eq_rows {
            if let Ok(slot) = std::str::FromStr::from_str(&slot_str) {
                if let Ok(Some(template_id)) = oxide_data::load_item_component(conn_db, item_db_id)
                {
                    if let Some(item_tmpl) = templates.get_item(&template_id) {
                        let item_entity = world.spawn((
                            oxide_core::Name::new(item_tmpl.name.clone()),
                            oxide_core::Item::new(template_id),
                            oxide_core::DbId::new(item_db_id),
                        ));
                        equipment.equip(slot, item_entity);
                    }
                }
            }
        }
    }

    let appearance = oxide_data::load_appearance_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(
            |(height, weight, build, hair_color, hair_style, eye_color, skin_tone)| {
                oxide_core::Appearance {
                    height: height as u8,
                    weight: weight as u16,
                    build,
                    hair_color,
                    hair_style,
                    eye_color,
                    skin_tone,
                }
            },
        )
        .unwrap_or_default();

    let age = oxide_data::load_age_component(conn_db, entity_id)
        .ok()
        .flatten()
        .map(|a| oxide_core::Age(a as u16))
        .unwrap_or_default();

    let deity = oxide_core::Deity(
        oxide_data::load_deity_component(conn_db, entity_id)
            .ok()
            .flatten(),
    );

    let valid_spawn_key = char_row
        .spawn_key
        .as_deref()
        .and_then(|key| {
            let templates = crate::get_templates()?;
            let race = &char_row.race;
            let class = &char_row.class;
            let alignment_str = alignment.0.as_str();
            let available = templates.available_spawns(race, class, alignment_str);
            if available
                .iter()
                .any(|(area_id, spawn)| format!("{}:{}", area_id, spawn.room) == key)
            {
                Some(key.to_string())
            } else {
                available
                    .first()
                    .map(|(area_id, spawn)| format!("{}:{}", area_id, spawn.room))
            }
        })
        .or_else(|| {
            let templates = crate::get_templates()?;
            let race = &char_row.race;
            let class = &char_row.class;
            let alignment_str = alignment.0.as_str();
            templates
                .available_spawns(race, class, alignment_str)
                .first()
                .map(|(area_id, spawn)| format!("{}:{}", area_id, spawn.room))
        });

    if let Some(ref healed_key) = valid_spawn_key {
        if char_row.spawn_key.as_ref() != Some(healed_key) {
            tracing::info!(
                entity_id,
                old_spawn_key = ?char_row.spawn_key,
                new_spawn_key = healed_key,
                "Healing corrupted/missing character spawn_key in database"
            );
            let _ = oxide_data::update_character_spawn_key(conn_db, entity_id, healed_key);
        }
    }

    let access_level = if let Some(account_id) = flow.account_id {
        oxide_data::get_account_by_id(conn_db, account_id)
            .ok()
            .flatten()
            .map(|a| a.access_level)
            .unwrap_or_else(|| "player".to_string())
    } else {
        "player".to_string()
    };

    let level_enum = match access_level.to_lowercase().as_str() {
        "builder" => oxide_core::AccessLevel::Builder,
        "immortal" => oxide_core::AccessLevel::Immortal,
        "god" => oxide_core::AccessLevel::God,
        "admin" => oxide_core::AccessLevel::Admin,
        _ => oxide_core::AccessLevel::Player,
    };

    drop(db_guard);

    // Resolve current room: saved room key → spawn_key
    let room = char_row
        .current_room_key
        .as_deref()
        .and_then(|key| crate::get_templates().and_then(|t| t.find_room_by_key(world, key)))
        .or_else(|| {
            valid_spawn_key
                .as_deref()
                .and_then(|key| crate::get_templates().and_then(|t| t.find_room_by_key(world, key)))
        })
        .or_else(|| {
            use oxide_core::{Entity, RoomKey};
            let mut query = world.query::<(&RoomKey,)>();
            query.iter().next().map(|(e, _)| Entity::from(e))
        })
        .expect("Must find at least one room in the world");

    // Resolve recall room: saved recall key → spawn_key
    let recall_room = char_row
        .recall_room_key
        .as_deref()
        .and_then(|key| crate::get_templates().and_then(|t| t.find_room_by_key(world, key)))
        .or_else(|| {
            valid_spawn_key
                .as_deref()
                .and_then(|key| crate::get_templates().and_then(|t| t.find_room_by_key(world, key)))
        })
        .or_else(|| {
            use oxide_core::{Entity, RoomKey};
            let mut query = world.query::<(&RoomKey,)>();
            query.iter().next().map(|(e, _)| Entity::from(e))
        })
        .expect("Must find at least one room in the world");

    let player = world.spawn((
        Position::new(room),
        Name::new(char_row.name.clone()),
        player_comp,
        Race(char_row.race.clone()),
        Class(char_row.class.clone()),
        Gender::new(
            char_row.gender.clone(),
            char_row.pronoun_subject.clone(),
            char_row.pronoun_object.clone(),
            char_row.pronoun_possessive.clone(),
        ),
        attrs,
        hp,
        mana,
        stamina,
        level,
        xp,
        RecallRoom(recall_room),
        PlayerState::default(),
        level_enum,
    ));

    if level_enum > oxide_core::AccessLevel::Player {
        let _ = world.insert(player, (oxide_core::Immortal,));
    }

    let _ = world.insert(
        player,
        (
            skills,
            DbId::new(entity_id),
            alignment,
            description,
            gold,
            inventory,
            equipment,
            PracticePoints(practice_points),
            combat_stats,
            appearance,
        ),
    );

    let _ = world.insert(
        player,
        (
            age,
            deity,
            quest_log,
            faction_standing,
            learned_recipes,
            multiclass_info,
        ),
    );

    if let Some(templates) = crate::get_templates() {
        oxide_core::systems::passive::apply_all_passives(world, player, &templates);
    }

    flow.entity = Some(player);
    flow.entity_just_spawned = true;

    lines.push(String::new());
    lines.push(format!("Welcome back, {}!", char_row.name));
    flow.state = LoginState::Playing;
    lines
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn clear_create_buffer(flow: &mut LoginFlow) {
    flow.create_buffer.name = None;
    flow.create_buffer.race = None;
    flow.create_buffer.class = None;
    flow.create_buffer.gender = None;
    flow.create_buffer.pronoun_subject = None;
    flow.create_buffer.pronoun_object = None;
    flow.create_buffer.pronoun_possessive = None;
    flow.create_buffer.spawn_key = None;
    flow.create_buffer.attributes = None;
    flow.create_buffer.alignment = None;
    flow.create_buffer.deity = None;
    flow.create_buffer.appearance_height = None;
    flow.create_buffer.appearance_weight = None;
    flow.create_buffer.appearance_build = None;
    flow.create_buffer.appearance_hair_style = None;
    flow.create_buffer.appearance_hair_color = None;
    flow.create_buffer.appearance_eye_color = None;
    flow.create_buffer.appearance_skin_tone = None;
    flow.create_buffer.age = None;
    flow.create_buffer.description = None;
    flow.create_buffer.selected_skills.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::LoginFlow;
    use oxide_core::templates::{
        ClassAttributeMods, ClassTemplate, DeityPolicy, DeityTemplate, TemplateRegistry,
        WalletAmount,
    };
    use std::collections::HashMap;

    #[test]
    fn test_deity_filtering() {
        let mut registry = TemplateRegistry::new();

        // Register a mock warrior class with DeityPolicy::Any
        let warrior = ClassTemplate {
            id: "warrior".to_string(),
            name: "Warrior".to_string(),
            description: "A warrior".to_string(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods::default(),
            bab: "full".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            skill_pool: vec![],
            starting_skill_slots: 3,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
            prestige: false,
            prestige_gate: None,
        };
        registry.classes.insert(warrior.id.clone(), warrior);

        // Register Astra (good deity, good/neutral can choose)
        let astra = DeityTemplate {
            id: "astra".to_string(),
            name: "Astra".to_string(),
            description: "Good deity".to_string(),
            alignment: Some("neutral_good".to_string()),
            symbol: "Stars".to_string(),
            favored_weapon: None,
            tenets: vec![],
            domains: vec![],
            allowed_races: vec![],
            allowed_classes: vec![],
            allowed_alignments: vec![
                "lawful_good".to_string(),
                "neutral_good".to_string(),
                "chaotic_good".to_string(),
                "lawful_neutral".to_string(),
                "true_neutral".to_string(),
                "chaotic_neutral".to_string(),
            ],
            prayer_effect: None,
            params: HashMap::new(),
        };
        registry.deities.insert(astra.id.clone(), astra);

        // Register Kronos (neutral deity, any alignment can choose)
        let kronos = DeityTemplate {
            id: "kronos".to_string(),
            name: "Kronos".to_string(),
            description: "Neutral deity".to_string(),
            alignment: Some("true_neutral".to_string()),
            symbol: "Hourglass".to_string(),
            favored_weapon: None,
            tenets: vec![],
            domains: vec![],
            allowed_races: vec![],
            allowed_classes: vec![],
            allowed_alignments: vec![
                "lawful_good".to_string(),
                "neutral_good".to_string(),
                "chaotic_good".to_string(),
                "lawful_neutral".to_string(),
                "true_neutral".to_string(),
                "chaotic_neutral".to_string(),
                "lawful_evil".to_string(),
                "neutral_evil".to_string(),
                "chaotic_evil".to_string(),
            ],
            prayer_effect: None,
            params: HashMap::new(),
        };
        registry.deities.insert(kronos.id.clone(), kronos);

        // Register Vulgath (evil deity, neutral/evil can choose)
        let vulgath = DeityTemplate {
            id: "vulgath".to_string(),
            name: "Vulgath".to_string(),
            description: "Evil deity".to_string(),
            alignment: Some("neutral_evil".to_string()),
            symbol: "Skull".to_string(),
            favored_weapon: None,
            tenets: vec![],
            domains: vec![],
            allowed_races: vec![],
            allowed_classes: vec![],
            allowed_alignments: vec![
                "lawful_neutral".to_string(),
                "true_neutral".to_string(),
                "chaotic_neutral".to_string(),
                "lawful_evil".to_string(),
                "neutral_evil".to_string(),
                "chaotic_evil".to_string(),
            ],
            prayer_effect: None,
            params: HashMap::new(),
        };
        registry.deities.insert(vulgath.id.clone(), vulgath);

        // Register Karrgath (orc deity, war focused, any alignment, Orc only)
        let karrgath = DeityTemplate {
            id: "karrgath".to_string(),
            name: "Karrgath".to_string(),
            description: "Orc war deity".to_string(),
            alignment: Some("chaotic_neutral".to_string()),
            symbol: "Axe".to_string(),
            favored_weapon: None,
            tenets: vec![],
            domains: vec![],
            allowed_races: vec!["orc".to_string()],
            allowed_classes: vec![],
            allowed_alignments: vec![
                "lawful_good".to_string(),
                "neutral_good".to_string(),
                "chaotic_good".to_string(),
                "lawful_neutral".to_string(),
                "true_neutral".to_string(),
                "chaotic_neutral".to_string(),
                "lawful_evil".to_string(),
                "neutral_evil".to_string(),
                "chaotic_evil".to_string(),
            ],
            prayer_effect: None,
            params: HashMap::new(),
        };
        registry.deities.insert(karrgath.id.clone(), karrgath);

        // Test Case A: Human Warrior, Lawful Good
        let mut flow = LoginFlow::new();
        flow.create_buffer.race = Some("human".to_string());
        flow.create_buffer.class = Some("warrior".to_string());
        flow.create_buffer.alignment = Some("lawful_good".to_string());
        let allowed = get_allowed_deities(&flow, Some(&registry));
        assert_eq!(allowed, vec!["astra".to_string(), "kronos".to_string()]);

        // Test Case B: Human Warrior, True Neutral
        let mut flow = LoginFlow::new();
        flow.create_buffer.race = Some("human".to_string());
        flow.create_buffer.class = Some("warrior".to_string());
        flow.create_buffer.alignment = Some("true_neutral".to_string());
        let allowed = get_allowed_deities(&flow, Some(&registry));
        assert_eq!(
            allowed,
            vec![
                "astra".to_string(),
                "kronos".to_string(),
                "vulgath".to_string()
            ]
        );

        // Test Case C: Human Warrior, Chaotic Evil
        let mut flow = LoginFlow::new();
        flow.create_buffer.race = Some("human".to_string());
        flow.create_buffer.class = Some("warrior".to_string());
        flow.create_buffer.alignment = Some("chaotic_evil".to_string());
        let allowed = get_allowed_deities(&flow, Some(&registry));
        assert_eq!(allowed, vec!["kronos".to_string(), "vulgath".to_string()]);

        // Test Case D: Orc Warrior, Chaotic Evil
        let mut flow = LoginFlow::new();
        flow.create_buffer.race = Some("orc".to_string());
        flow.create_buffer.class = Some("warrior".to_string());
        flow.create_buffer.alignment = Some("chaotic_evil".to_string());
        let allowed = get_allowed_deities(&flow, Some(&registry));
        assert_eq!(
            allowed,
            vec![
                "karrgath".to_string(),
                "kronos".to_string(),
                "vulgath".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn test_spawn_key_healing() {
        use oxide_core::templates::{AreaTemplate, SpawnEntry};
        use std::collections::HashMap;
        use std::sync::Arc;

        let mut registry = TemplateRegistry::new();

        let warrior = ClassTemplate {
            id: "warrior".to_string(),
            name: "Warrior".to_string(),
            description: "A warrior".to_string(),
            hit_die: 10,
            attribute_mods: ClassAttributeMods::default(),
            bab: "full".to_string(),
            fort_save: "good".to_string(),
            ref_save: "poor".to_string(),
            will_save: "poor".to_string(),
            allowed_races: vec![],
            allowed_alignments: vec![],
            auto_skills: vec![],
            skill_pool: vec![],
            starting_skill_slots: 3,
            starting_items: vec![],
            starting_gold: WalletAmount::default(),
            deity_policy: DeityPolicy::Any,
            params: HashMap::new(),
            prestige: false,
            prestige_gate: None,
        };
        registry.classes.insert("warrior".to_string(), warrior);

        let area = AreaTemplate {
            id: "starting_vale".to_string(),
            name: "Starting Vale".to_string(),
            description: "A start area".to_string(),
            level_range: None,
            flags: vec![],
            weather_zone: None,
            reset_interval: None,
            credits: None,
            spawns: vec![SpawnEntry {
                room: "town_square".to_string(),
                label: "Town Square".to_string(),
                description: "The main town square".to_string(),
                allowed_races: vec![],
                allowed_classes: vec![],
                allowed_alignments: vec![],
            }],
            rooms: HashMap::new(),
        };
        registry.areas.insert("starting_vale".to_string(), area);

        let _ = crate::server::TEMPLATES.set(Arc::new(registry));

        let db = Mutex::new(oxide_data::Database::open_in_memory().unwrap());
        let account_id = {
            let db_guard = db.lock().await;
            oxide_data::create_account(db_guard.conn(), "testuser", "hash").unwrap()
        };

        let mut world = World::new();
        world.spawn((
            oxide_core::Name::new("Town Square"),
            oxide_core::RoomKey("starting_vale:town_square".to_string()),
        ));

        let _char_id = {
            let db_guard = db.lock().await;
            let entity_id = oxide_data::insert_entity(db_guard.conn(), "player").unwrap();
            oxide_data::create_character(
                db_guard.conn(),
                account_id,
                "TestChar",
                "human",
                "warrior",
                entity_id,
                Some("starting_vale:town_square"),
                Some("starting_vale:town_square"),
            )
            .unwrap()
        };

        let char_row = {
            let db_guard = db.lock().await;
            oxide_data::get_characters_by_account(db_guard.conn(), account_id)
                .unwrap()
                .remove(0)
        };

        let mut flow = LoginFlow::new();
        let _lines = load_character(&mut flow, &mut world, &char_row, &db).await;

        let player_entity = flow.entity.unwrap();
        let recall_comp = world
            .query_one::<&RecallRoom>(player_entity)
            .unwrap()
            .get()
            .map(|r| r.0)
            .unwrap();
        assert_eq!(
            world
                .query_one::<&oxide_core::RoomKey>(recall_comp)
                .unwrap()
                .get()
                .map(|k| &k.0),
            Some(&"starting_vale:town_square".to_string())
        );

        let healed_char_row = {
            let db_guard = db.lock().await;
            oxide_data::get_characters_by_account(db_guard.conn(), account_id)
                .unwrap()
                .remove(0)
        };
        assert_eq!(
            healed_char_row.spawn_key,
            Some("starting_vale:town_square".to_string())
        );
    }

    #[test]
    fn test_gender_selection_sorting() {
        use oxide_core::templates::{
            AppearanceBounds, GenderPronouns, RaceAttributes, RaceTemplate,
        };
        let mut registry = TemplateRegistry::new();
        let mut race = RaceTemplate {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: "A human".to_string(),
            attributes: RaceAttributes::default(),
            allowed_classes: vec![],
            allowed_alignments: vec![],
            racial_abilities: vec![],
            allowed_genders: HashMap::new(),
            appearance_bounds: AppearanceBounds::default(),
            age_default: 20,
            age_max: 100,
            params: HashMap::new(),
        };
        // Insert out of alphabetical order
        race.allowed_genders.insert(
            "neutral".to_string(),
            GenderPronouns {
                subject: "they".into(),
                object: "them".into(),
                possessive: "their".into(),
            },
        );
        race.allowed_genders.insert(
            "male".to_string(),
            GenderPronouns {
                subject: "he".into(),
                object: "him".into(),
                possessive: "his".into(),
            },
        );
        race.allowed_genders.insert(
            "female".to_string(),
            GenderPronouns {
                subject: "she".into(),
                object: "her".into(),
                possessive: "hers".into(),
            },
        );
        registry.races.insert("human".to_string(), race);

        let mut flow = LoginFlow::new();
        flow.create_buffer.race = Some("human".to_string());
        flow.state = LoginState::CharacterCreateGender;

        // In both show_character_gender_prompt and handle_character_create_gender_state,
        // options will be sorted alphabetically: ["female", "male", "neutral"]
        // Select index 1 (which should be "female")
        let _ = handle_character_create_gender_state(&mut flow, "1", Some(&registry));
        assert_eq!(flow.create_buffer.gender.as_deref(), Some("female"));
    }

    #[test]
    fn test_deity_selection_index_and_names() {
        let mut registry = TemplateRegistry::new();
        let astra = DeityTemplate {
            id: "astra".to_string(),
            name: "Astra Goddess".to_string(),
            description: "A deity".to_string(),
            alignment: None,
            symbol: "".to_string(),
            favored_weapon: None,
            tenets: vec![],
            domains: vec![],
            allowed_races: vec![],
            allowed_classes: vec![],
            allowed_alignments: vec![],
            prayer_effect: None,
            params: HashMap::new(),
        };
        registry.deities.insert("astra".to_string(), astra);

        // Select by index
        let mut flow = LoginFlow::new();
        flow.state = LoginState::CharacterCreateDeity(vec!["astra".to_string()]);
        flow.create_buffer.class = Some("warrior".to_string());
        let _ = handle_character_create_deity_state(&mut flow, "1", Some(&registry));
        assert_eq!(flow.create_buffer.deity.as_deref(), Some("astra"));

        // Select by ID case-insensitive
        let mut flow = LoginFlow::new();
        flow.state = LoginState::CharacterCreateDeity(vec!["astra".to_string()]);
        flow.create_buffer.class = Some("warrior".to_string());
        let _ = handle_character_create_deity_state(&mut flow, "AsTrA", Some(&registry));
        assert_eq!(flow.create_buffer.deity.as_deref(), Some("astra"));

        // Select by name case-insensitive
        let mut flow = LoginFlow::new();
        flow.state = LoginState::CharacterCreateDeity(vec!["astra".to_string()]);
        flow.create_buffer.class = Some("warrior".to_string());
        let _ = handle_character_create_deity_state(&mut flow, "astra goddess", Some(&registry));
        assert_eq!(flow.create_buffer.deity.as_deref(), Some("astra"));
    }

    #[test]
    fn test_appearance_build_selection() {
        let mut flow = LoginFlow::new();
        flow.state = LoginState::CharacterCreateAppearanceBuild(vec![
            "slim".to_string(),
            "heavy".to_string(),
        ]);

        // Select by index
        let _ = handle_appearance_build_state(&mut flow, "2", None);
        assert_eq!(
            flow.create_buffer.appearance_build.as_deref(),
            Some("heavy")
        );

        // Select by name
        flow.state = LoginState::CharacterCreateAppearanceBuild(vec![
            "slim".to_string(),
            "heavy".to_string(),
        ]);
        let _ = handle_appearance_build_state(&mut flow, "sLiM", None);
        assert_eq!(flow.create_buffer.appearance_build.as_deref(), Some("slim"));
    }

    #[tokio::test]
    async fn test_load_character_fallback() {
        let mut world = World::new();
        let fallback_room = world.spawn((
            oxide_core::Name::new("The Limbo Fallback"),
            oxide_core::RoomKey("limbo:1".to_string()),
        ));

        let db = Mutex::new(oxide_data::Database::open_in_memory().unwrap());
        let account_id = {
            let db_guard = db.lock().await;
            oxide_data::create_account(db_guard.conn(), "testuser2", "hash").unwrap()
        };

        let _char_id = {
            let db_guard = db.lock().await;
            let entity_id = oxide_data::insert_entity(db_guard.conn(), "player").unwrap();
            oxide_data::create_character(
                db_guard.conn(),
                account_id,
                "FallbackChar",
                "human",
                "warrior",
                entity_id,
                Some("deleted_room:3"),
                Some("deleted_room:3"),
            )
            .unwrap()
        };

        let char_row = {
            let db_guard = db.lock().await;
            let _ = db_guard.conn().execute(
                "UPDATE characters SET current_room_key = 'deleted_room:1', recall_room_key = 'deleted_room:2' WHERE name = 'FallbackChar'",
                [],
            );
            oxide_data::get_characters_by_account(db_guard.conn(), account_id)
                .unwrap()
                .into_iter()
                .find(|c| c.name == "FallbackChar")
                .unwrap()
        };

        let mut flow = LoginFlow::new();
        let _lines = load_character(&mut flow, &mut world, &char_row, &db).await;

        let player_entity = flow.entity.unwrap();
        let position = world
            .query_one::<&Position>(player_entity)
            .unwrap()
            .get()
            .map(|p| p.room)
            .unwrap();
        assert_eq!(position, fallback_room);
    }

    #[test]
    fn test_alignment_setting() {
        let registry = TemplateRegistry::new();
        let mut flow = LoginFlow::new();
        flow.create_buffer.race = Some("human".to_string());
        flow.create_buffer.class = Some("warrior".to_string());
        flow.state = LoginState::CharacterCreateAlignment;

        // Select lawful good (index 1)
        let _ = handle_alignment_state(&mut flow, "1", Some(&registry));
        assert_eq!(flow.create_buffer.alignment.as_deref(), Some("lawful_good"));
    }
}
