use crate::scripting::with_dynamic_skills;
use crate::{Entity, ItemTriggers, LearnedSkills, Npc, Room, ScriptParams, World};
use std::collections::HashMap;

/// Collect script paths and dynamic parameters for a `say` event on `script_entity`.
pub fn collect_say_scripts(
    world: &World,
    script_entity: Entity,
) -> Vec<(String, HashMap<String, String>)> {
    let mut scripts_to_run = Vec::new();

    if let Ok(mut q) = world.query_one::<&Npc>(script_entity) {
        if let Some(npc) = q.get() {
            if let Some(ref s) = npc.script {
                let mut params = HashMap::new();
                if let Ok(mut q_params) = world.query_one::<&ScriptParams>(script_entity) {
                    if let Some(p) = q_params.get() {
                        params = p.0.clone();
                    }
                }
                scripts_to_run.push((s.clone(), params));
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&Room>(script_entity) {
        if let Some(room) = q.get() {
            if let Some(ref s) = room.script {
                let mut params = HashMap::new();
                if let Ok(mut q_params) = world.query_one::<&ScriptParams>(script_entity) {
                    if let Some(p) = q_params.get() {
                        params = p.0.clone();
                    }
                }
                scripts_to_run.push((s.clone(), params));
            }
        }
    }

    if let Ok(mut q) = world.query_one::<&ItemTriggers>(script_entity) {
        if let Some(triggers) = q.get() {
            for trigger in &triggers.0 {
                if trigger.event == "say" {
                    if let Some(ref s) = trigger.script {
                        scripts_to_run.push((s.clone(), trigger.params.clone()));
                    }
                }
            }
        }
    }

    scripts_to_run
}

/// Collect skill script paths for an entity's learned skills.
pub fn collect_learned_skill_scripts(world: &World, entity: Entity) -> Vec<String> {
    let learned_skills: Vec<String> = if let Ok(mut q) = world.query_one::<&LearnedSkills>(entity)
    {
        q.get()
            .map(|s| s.skills.keys().cloned().collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut scripts = Vec::new();
    for skill_id in learned_skills {
        if let Some(script_path) = resolve_skill_script_path(&skill_id) {
            scripts.push(script_path);
        }
    }
    scripts
}

pub fn resolve_skill_script_path(skill_id: &str) -> Option<String> {
    if let Some(script) =
        with_dynamic_skills(|reg| reg.skills.get(skill_id).map(|s| s.script.clone()))
    {
        return Some(script);
    }
    Some(format!("skills/{}.rhai", skill_id))
}
