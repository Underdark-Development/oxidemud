use oxide_core::templates::TemplateRegistry;
use oxide_core::{LearnedSkills, Wallet};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const POINT_BUY_COST: [u8; 11] = [1, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4];
pub const STANDARD_ARRAY: [u8; 6] = [15, 14, 13, 12, 10, 8];
pub const MAX_POINT_BUY_POINTS: u8 = 27;
pub const MAX_REROLLS: u8 = 3;

// ---------------------------------------------------------------------------
// Handler helpers
// ---------------------------------------------------------------------------

/// Validates a character name: 3-16 chars, letters, hyphens, apostrophes.
pub fn is_valid_character_name(s: &str) -> bool {
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
pub fn roll_4d6_drop_lowest() -> u8 {
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
pub fn roll_all_stats() -> [u8; 6] {
    let mut stats = [0u8; 6];
    for s in &mut stats {
        *s = roll_4d6_drop_lowest();
    }
    stats.sort_unstable_by(|a, b| b.cmp(a));
    stats
}

/// Match an option by 1-based index or by name (case-insensitive).
pub fn match_option_index_or_name<'a>(
    input_lower: &str,
    options: &'a [String],
) -> Option<&'a String> {
    if let Ok(idx) = input_lower.parse::<usize>() {
        if idx > 0 && idx <= options.len() {
            return Some(&options[idx - 1]);
        }
    }
    options.iter().find(|o| o.to_lowercase() == input_lower)
}

/// Parse a stat name abbreviation to index (0-5).
pub fn stat_index(s: &str) -> Option<usize> {
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
pub fn point_buy_cost(current: u8) -> Option<u8> {
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
) -> (oxide_core::Attributes, i32, LearnedSkills) {
    let mut skills = LearnedSkills::new();

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
