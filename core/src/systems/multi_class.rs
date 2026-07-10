use crate::{
    components::CombatStats, templates::TemplateRegistry, Alignment, Entity, FactionStanding,
    LearnedSkills, Level, MultiClassInfo, PrestigeGate, QuestLog, Race, World,
};

pub fn satisfies_prestige_gate(
    world: &World,
    player: Entity,
    gate: &PrestigeGate,
    _templates: &TemplateRegistry,
) -> Result<(), String> {
    // 1. Level requirement
    if let Some(req_level) = gate.requires_level {
        let total_level = world
            .query_one::<&Level>(player)
            .ok()
            .and_then(|mut q| q.get().map(|l| l.0))
            .unwrap_or(1);
        if total_level < req_level {
            return Err(format!(
                "Requires level {}, but you are only level {}",
                req_level, total_level
            ));
        }
    }

    // 2. Class requirements
    for (class_id, req_level) in &gate.requires_class {
        let actual_level = world
            .query_one::<&MultiClassInfo>(player)
            .ok()
            .and_then(|mut q| q.get().map(|mc| mc.class_level(class_id)))
            .unwrap_or_else(|| {
                let has_class_component = world
                    .query_one::<&crate::components::Class>(player)
                    .ok()
                    .and_then(|mut q| {
                        q.get()
                            .map(|c| c.0.to_lowercase() == class_id.to_lowercase())
                    })
                    .unwrap_or(false);
                if has_class_component {
                    world
                        .query_one::<&Level>(player)
                        .ok()
                        .and_then(|mut q| q.get().map(|l| l.0))
                        .unwrap_or(1)
                } else {
                    0
                }
            });
        if actual_level < *req_level {
            return Err(format!(
                "Requires class '{}' at level {}, but yours is level {}",
                class_id, req_level, actual_level
            ));
        }
    }

    // 3. Skill requirements
    for (skill_id, req_rank) in &gate.requires_skills {
        let actual_rank = world
            .query_one::<&LearnedSkills>(player)
            .ok()
            .and_then(|mut q| q.get().map(|ls| ls.rank(skill_id)))
            .unwrap_or(0);
        if actual_rank < *req_rank {
            return Err(format!(
                "Requires skill '{}' at rank {}, but yours is rank {}",
                skill_id, req_rank, actual_rank
            ));
        }
    }

    // 4. Race requirement
    if let Some(ref req_race) = gate.requires_race {
        let actual_race = world
            .query_one::<&Race>(player)
            .ok()
            .and_then(|mut q| q.get().map(|r| r.0.clone()))
            .unwrap_or_default();
        if actual_race.to_lowercase() != req_race.to_lowercase() {
            return Err(format!(
                "Requires race '{}', but you are '{}'",
                req_race, actual_race
            ));
        }
    }

    // 5. Alignment requirement
    if let Some(ref req_align) = gate.requires_alignment {
        let actual_align = world
            .query_one::<&Alignment>(player)
            .ok()
            .and_then(|mut q| q.get().map(|a| a.0.clone()))
            .unwrap_or_default();
        if actual_align.to_lowercase() != req_align.to_lowercase() {
            return Err(format!(
                "Requires alignment '{}', but yours is '{}'",
                req_align, actual_align
            ));
        }
    }

    // 6. Quest requirement
    if let Some(ref req_quest) = gate.requires_quest {
        let completed = world
            .query_one::<&QuestLog>(player)
            .ok()
            .and_then(|mut q| q.get().map(|ql| ql.completed.contains(req_quest)))
            .unwrap_or(false);
        if !completed {
            return Err(format!("Requires quest '{}' to be completed", req_quest));
        }
    }

    // 7. Faction requirement
    if let Some(ref req_faction) = gate.requires_faction {
        if let Some((fac_id, req_std_str)) = req_faction.split_once(':') {
            if let Ok(req_std) = req_std_str.parse::<i32>() {
                let actual_std = world
                    .query_one::<&FactionStanding>(player)
                    .ok()
                    .and_then(|mut q| q.get().map(|fs| fs.standing(fac_id)))
                    .unwrap_or(0);
                if actual_std < req_std {
                    return Err(format!(
                        "Requires standing {} with faction '{}', but yours is {}",
                        req_std, fac_id, actual_std
                    ));
                }
            }
        }
    }

    Ok(())
}

pub fn calculate_multiclass_combat_stats(
    mc_info: &MultiClassInfo,
    templates: &TemplateRegistry,
) -> CombatStats {
    let mut total_stats = CombatStats::default();
    for entry in &mc_info.classes {
        if let Some(class_template) = templates.get_class(&entry.id) {
            let stats = class_template.calculate_combat_stats(entry.level);
            total_stats.base_attack_bonus += stats.base_attack_bonus;
            total_stats.fort_save += stats.fort_save;
            total_stats.ref_save += stats.ref_save;
            total_stats.will_save += stats.will_save;
        }
    }
    total_stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiclass_xp_penalty() {
        let mut mc = MultiClassInfo::new();

        // Single favored class -> 1.0 (no penalty)
        mc.add_class("warrior".to_string(), 5, true);
        assert_eq!(mc.xp_penalty_multiplier(), 1.0);

        // Add second class, non-favored -> 1.0 (only 1 non-favored class, so non_favored_count = 1 <= 1)
        mc.add_class("mage".to_string(), 2, false);
        assert_eq!(mc.xp_penalty_multiplier(), 1.0);

        // Add third class, non-favored -> (2 non-favored classes) -> (2 - 1) * 20% = 20% penalty -> 0.80 multiplier
        mc.add_class("cleric".to_string(), 1, false);
        assert_eq!(mc.xp_penalty_multiplier(), 0.80);

        // Add fourth class, non-favored -> (3 non-favored classes) -> 40% penalty -> 0.60 multiplier
        mc.add_class("thief".to_string(), 1, false);
        assert_eq!(mc.xp_penalty_multiplier(), 0.60);
    }
}
