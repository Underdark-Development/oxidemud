use crate::context::with_current_world;
use oxide_core::Entity;
use rhai::Engine;

pub fn register(engine: &mut Engine) {
    engine.register_fn(
        "assert",
        |val: bool| -> Result<(), Box<rhai::EvalAltResult>> {
            if !val {
                Err("Assertion failed".into())
            } else {
                Ok(())
            }
        },
    );

    engine.register_fn(
        "register_skill",
        |id: String,
         name: String,
         command: String,
         script: String,
         help_text: String,
         short: String| {
            oxide_core::register_dynamic_skill(oxide_core::ScriptSkill {
                id,
                name,
                short,
                command: Some(command),
                is_spell: false,
                topic: "Skills".to_string(),
                help_text,
                script,
                restrictions: oxide_core::CommandRestrictions::default(),
            });
        },
    );

    engine.register_fn(
        "register_skill",
        |id: String,
         name: String,
         command: String,
         script: String,
         help_text: String,
         short: String,
         allowed_classes: rhai::Array| {
            let classes = allowed_classes
                .into_iter()
                .filter_map(|v| v.into_string().ok())
                .collect();
            let restrictions = oxide_core::CommandRestrictions {
                allowed_classes: classes,
                ..Default::default()
            };
            oxide_core::register_dynamic_skill(oxide_core::ScriptSkill {
                id,
                name,
                short,
                command: Some(command),
                is_spell: false,
                topic: "Skills".to_string(),
                help_text,
                script,
                restrictions,
            });
        },
    );

    engine.register_fn(
        "register_spell",
        |id: String,
         name: String,
         spell_name: String,
         script: String,
         help_text: String,
         short: String| {
            oxide_core::register_dynamic_skill(oxide_core::ScriptSkill {
                id,
                name,
                short,
                command: Some(spell_name),
                is_spell: true,
                topic: "Spells".to_string(),
                help_text,
                script,
                restrictions: oxide_core::CommandRestrictions::default(),
            });
        },
    );

    engine.register_fn(
        "register_spell",
        |id: String,
         name: String,
         spell_name: String,
         script: String,
         help_text: String,
         short: String,
         allowed_classes: rhai::Array| {
            let classes = allowed_classes
                .into_iter()
                .filter_map(|v| v.into_string().ok())
                .collect();
            let restrictions = oxide_core::CommandRestrictions {
                allowed_classes: classes,
                ..Default::default()
            };
            oxide_core::register_dynamic_skill(oxide_core::ScriptSkill {
                id,
                name,
                short,
                command: Some(spell_name),
                is_spell: true,
                topic: "Spells".to_string(),
                help_text,
                script,
                restrictions,
            });
        },
    );

    engine.register_fn(
        "register_entity_command",
        |entity: Entity, command_name: String, script: String, help_text: String| {
            with_current_world(|w| {
                if let Ok(mut q) = w.query_one::<&mut oxide_core::EntityCommands>(entity) {
                    if let Some(ec) = q.get() {
                        ec.add(command_name, script, help_text);
                        return;
                    }
                }
                let mut ec = oxide_core::EntityCommands::new();
                ec.add(command_name, script, help_text);
                let _ = w.insert(entity, (ec,));
            });
        },
    );
}
