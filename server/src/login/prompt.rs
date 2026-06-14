use tokio::sync::Mutex;

use mud_core::templates::TemplateRegistry;
use mud_core::{Name, World};

use crate::registry::ConnectionRegistry;

use super::state::LoginState;
use super::LoginFlow;

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
    _flow: &mut LoginFlow,
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

    for (i, (_id, race)) in races.iter().enumerate() {
        lines.push(format!("{}. {} — {}", i + 1, race.name, race.description));
    }
    lines.push(format!("Pick a race by number (1-{}):", races.len()));
    lines
}

/// Return the class-selection prompt as a vector of output lines.
pub fn show_character_class_prompt(flow: &LoginFlow, templates: &TemplateRegistry) -> Vec<String> {
    let mut lines = Vec::new();
    let race_id = flow.create_buffer.race.as_deref().unwrap_or("");
    let available = templates.available_classes_for_race(race_id);

    lines.push(String::new());
    lines.push("--- Choose a Class ---".to_string());

    for (i, class) in available.iter().enumerate() {
        lines.push(format!("{}. {} — {}", i + 1, class.name, class.description));
    }
    lines.push(format!("Pick a class by number (1-{}):", available.len()));
    lines
}

/// Return the character-confirmation screen as a vector of output lines.
pub fn show_character_confirm(flow: &mut LoginFlow, templates: &TemplateRegistry) -> Vec<String> {
    let mut lines = Vec::new();
    let (name, race_id, class_id) = {
        (
            flow.create_buffer
                .name
                .clone()
                .unwrap_or_else(|| "?".to_string()),
            flow.create_buffer
                .race
                .clone()
                .unwrap_or_else(|| "?".to_string()),
            flow.create_buffer
                .class
                .clone()
                .unwrap_or_else(|| "?".to_string()),
        )
    };

    let race = templates.get_race(&race_id);
    let class = templates.get_class(&class_id);

    let default_attrs = mud_core::templates::RaceAttributes::default();
    let default_mods = mud_core::templates::ClassAttributeMods::default();

    let (race_name, race_attrs) = race
        .map(|r| (r.name.as_str(), &r.attributes))
        .unwrap_or(("?", &default_attrs));

    let (class_name, class_mods) = class
        .map(|c| (c.name.as_str(), &c.attribute_mods))
        .unwrap_or(("?", &default_mods));

    let str = (race_attrs.strength as i16 + class_mods.strength as i16) as u8;
    let dex = (race_attrs.dexterity as i16 + class_mods.dexterity as i16) as u8;
    let int = (race_attrs.intelligence as i16 + class_mods.intelligence as i16) as u8;
    let wis = (race_attrs.wisdom as i16 + class_mods.wisdom as i16) as u8;
    let con = (race_attrs.constitution as i16 + class_mods.constitution as i16) as u8;
    let cha = (race_attrs.charisma as i16 + class_mods.charisma as i16) as u8;

    lines.push(String::new());
    lines.push("--- Character Summary ---".to_string());
    lines.push(format!("  Name:       {name}"));
    lines.push(format!("  Race:       {race_name}"));
    lines.push(format!("  Class:      {class_name}"));
    lines.push(format!(
        "  Attributes: STR {str}, DEX {dex}, INT {int}, WIS {wis}, CON {con}, CHA {cha}"
    ));
    if let Some(r) = race {
        if !r.racial_abilities.is_empty() {
            lines.push(format!("  Abilities:  {}", r.racial_abilities.join(", ")));
        }
    }
    if let Some(c) = class {
        if !c.auto_skills.is_empty() {
            lines.push(format!("  Skills:     {}", c.auto_skills.join(", ")));
        }
    }
    lines.push(String::new());
    lines.push("Accept this character? (y/n)".to_string());
    lines
}

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
