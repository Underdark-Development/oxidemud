use crate::context::push_script_context;
use crate::engine::ScriptEngine;
use oxide_core::script_dispatch::{collect_learned_skill_scripts, collect_say_scripts};
use oxide_core::{DamageType, Entity, HitContext, ScriptingBridge, World};
use rhai::Scope;


fn notify_script_error(entity: Entity, script: &str, err: &str) {
    if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
        bridge.send_to_entity(entity, &format!("[Script Error in '{}']: {}", script, err));
    }
}

impl ScriptingBridge for ScriptEngine {
    fn execute_trigger(
        &self,
        script: &str,
        entity: Entity,
        actor: Option<Entity>,
        target: Option<Entity>,
        world: &mut World,
    ) -> Result<bool, String> {
        let _guard = push_script_context(entity, actor, target, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("self", entity);
        scope.push("actor", actor);
        scope.push("target", target);

        if let Ok(mut q) = world.query_one::<&oxide_core::ScriptParams>(entity) {
            if let Some(params) = q.get() {
                for (k, v) in &params.0 {
                    scope.push(k.clone(), v.clone());
                }
            }
        }

        self.engine
            .call_fn::<bool>(&mut scope, &ast, "on_trigger", ())
            .map_err(|e| {
                if let Some(act) = actor.or(Some(entity)) {
                    notify_script_error(act, script, &e.to_string());
                }
                e.to_string()
            })
    }

    fn execute_combat_hit_hook(
        &self,
        attacker: Entity,
        target: Entity,
        is_offhand: bool,
        world: &mut World,
    ) -> Result<HitContext, String> {
        let _guard = push_script_context(target, Some(attacker), Some(target), world);
        let mut hit_ctx = HitContext {
            attacker,
            target,
            is_offhand,
            is_aborted: false,
            abort_reason: None,
            hit_modifier: 0,
            override_hit: None,
        };

        let skill_scripts = collect_learned_skill_scripts(world, target);
        for script_path in skill_scripts {
            if hit_ctx.is_aborted {
                break;
            }
            if let Ok(ast) = self.get_ast(&script_path) {
                let mut scope = Scope::new();
                scope.push("hit_ctx", hit_ctx.clone());

                match self
                    .engine
                    .call_fn::<HitContext>(&mut scope, &ast, "on_combat_hit", ())
                {
                    Ok(returned_ctx) => {
                        hit_ctx = returned_ctx;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        if !err_str.contains("Function not found") {
                            notify_script_error(target, &script_path, &err_str);
                        }
                    }
                }
            }
        }

        Ok(hit_ctx)
    }

    fn execute_combat_damage_hook(
        &self,
        attacker: Entity,
        target: Entity,
        damage: i32,
        damage_type: DamageType,
        world: &mut World,
    ) -> Result<(i32, DamageType), String> {
        let _guard = push_script_context(target, Some(attacker), Some(target), world);
        let mut final_damage = damage;
        let final_type = damage_type;

        let skill_scripts = collect_learned_skill_scripts(world, target);
        for script_path in skill_scripts {
            if let Ok(ast) = self.get_ast(&script_path) {
                let mut scope = Scope::new();

                let type_str = format!("{:?}", final_type).to_lowercase();
                match self.engine.call_fn::<i64>(
                    &mut scope,
                    &ast,
                    "on_combat_damage",
                    (final_damage as i64, type_str),
                ) {
                    Ok(mod_dmg) => {
                        final_damage = mod_dmg.max(0) as i32;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        if !err_str.contains("Function not found") {
                            notify_script_error(target, &script_path, &err_str);
                        }
                    }
                }
            }
        }

        Ok((final_damage, final_type))
    }

    fn execute_mob_ai(
        &self,
        script: &str,
        entity: Entity,
        world: &mut World,
    ) -> Result<Option<String>, String> {
        let _guard = push_script_context(entity, None, None, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("self", entity);

        if let Ok(mut q) = world.query_one::<&oxide_core::ScriptParams>(entity) {
            if let Some(params) = q.get() {
                for (k, v) in &params.0 {
                    scope.push(k.clone(), v.clone());
                }
            }
        }

        self.engine
            .call_fn::<Option<String>>(&mut scope, &ast, "on_ai_pulse", ())
            .map_err(|e| e.to_string())
    }

    fn execute_say_hook(
        &self,
        script_entity: Entity,
        speaker: Entity,
        message: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(script_entity, Some(speaker), None, world);
        let scripts_to_run = collect_say_scripts(world, script_entity);

        for (script_path, params) in scripts_to_run {
            let ast = self.get_ast(&script_path)?;
            let mut scope = Scope::new();
            scope.push("self", script_entity);
            scope.push("speaker", speaker);
            scope.push("message", message.to_string());

            for (k, v) in params {
                scope.push(k, v);
            }

            self.engine
                .call_fn::<()>(&mut scope, &ast, "on_say", ())
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    fn execute_use_skill(
        &self,
        script: &str,
        actor: Entity,
        target: Option<Entity>,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(actor, Some(actor), target, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("actor", actor);
        scope.push("target", target);

        self.engine
            .call_fn::<()>(&mut scope, &ast, "on_use", ())
            .map_err(|e| e.to_string())
    }

    fn execute_quest_hook(
        &self,
        script: &str,
        player: Entity,
        quest_id: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(player, Some(player), None, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("player", player);
        scope.push("quest_id", quest_id.to_string());

        let mut rewards_map = rhai::Map::new();
        if let Some(templates) = oxide_core::templates::get_global_templates() {
            if let Some(quest_def) = templates.quests.get(quest_id) {
                rewards_map.insert("xp".into(), (quest_def.rewards.xp as i64).into());
                rewards_map.insert("gold".into(), (quest_def.rewards.gold as i64).into());

                let mut items_arr = rhai::Array::new();
                for item in &quest_def.rewards.items {
                    let mut item_map = rhai::Map::new();
                    item_map.insert(
                        "item_template_id".into(),
                        item.item_template_id.clone().into(),
                    );
                    item_map.insert("count".into(), (item.count as i64).into());
                    items_arr.push(item_map.into());
                }
                rewards_map.insert("items".into(), items_arr.into());

                let mut faction_arr = rhai::Array::new();
                for fac in &quest_def.rewards.faction {
                    let mut fac_map = rhai::Map::new();
                    fac_map.insert("faction_id".into(), fac.faction_id.clone().into());
                    fac_map.insert("amount".into(), (fac.amount as i64).into());
                    faction_arr.push(fac_map.into());
                }
                rewards_map.insert("faction".into(), faction_arr.into());
            }
        }
        scope.push("rewards", rewards_map);

        self.engine
            .run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| e.to_string())
    }

    fn execute_script_skill(
        &self,
        script: &str,
        actor: Entity,
        args: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(actor, Some(actor), None, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("actor", actor);
        scope.push("args", args.to_string());

        if self
            .engine
            .call_fn::<()>(&mut scope, &ast, "on_use", ())
            .is_err()
            && self
                .engine
                .call_fn::<()>(&mut scope, &ast, "on_cast", ())
                .is_err()
            && self
                .engine
                .call_fn::<()>(&mut scope, &ast, "main", ())
                .is_err()
        {
            let _ = self.engine.run_ast_with_scope(&mut scope, &ast);
        }
        Ok(())
    }

    fn execute_entity_command(
        &self,
        entity: Entity,
        script: &str,
        actor: Entity,
        args: &str,
        world: &mut World,
    ) -> Result<(), String> {
        let _guard = push_script_context(entity, Some(actor), None, world);
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("self", entity);
        scope.push("actor", actor);
        scope.push("args", args.to_string());

        if self
            .engine
            .call_fn::<()>(&mut scope, &ast, "on_command", ())
            .is_err()
            && self
                .engine
                .call_fn::<()>(&mut scope, &ast, "main", ())
                .is_err()
        {
            let _ = self.engine.run_ast_with_scope(&mut scope, &ast);
        }
        Ok(())
    }

    fn evaluate_script_predicate(
        &self,
        script: &str,
        actor: Entity,
        target_entity: Option<Entity>,
        world: &mut World,
    ) -> Result<bool, String> {
        let _guard = push_script_context(actor, Some(actor), target_entity, world);
        let mut scope = Scope::new();
        scope.push("actor", actor);
        scope.push("target", target_entity);

        if script.ends_with(".rhai") {
            let ast = self.get_ast(script)?;
            if let Ok(res) = self.engine.call_fn::<bool>(&mut scope, &ast, "can_use", ()) {
                Ok(res)
            } else {
                self.engine
                    .eval_ast_with_scope::<bool>(&mut scope, &ast)
                    .map_err(|e| e.to_string())
            }
        } else {
            self.engine
                .eval_with_scope::<bool>(&mut scope, script)
                .map_err(|e| e.to_string())
        }
    }

    fn reload_script(&self, rel_path: &str) -> Result<(), String> {
        let full_path = self.script_dir.join(rel_path);
        if !full_path.exists() {
            let mut cache = self.ast_cache.write().unwrap_or_else(|e| e.into_inner());
            cache.remove(rel_path);
            return Ok(());
        }

        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read script {}: {}", rel_path, e))?;

        let processed = crate::engine::strip_tests(&content);
        let ast = self
            .engine
            .compile(&processed)
            .map_err(|e| format!("Compile error in {}: {}", rel_path, e))?;

        let mut cache = self.ast_cache.write().unwrap_or_else(|e| e.into_inner());
        cache.insert(rel_path.to_string(), ast);
        Ok(())
    }
}
