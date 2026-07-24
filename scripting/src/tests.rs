#[cfg(test)]
mod tests {
    use crate::context::push_script_context;
    use crate::engine::{construct_test_script, parse_test_blocks, strip_tests, ScriptEngine};
    use oxide_core::{Room, World};
    use rhai::Scope;
    use std::collections::HashMap;

    #[test]
    fn test_eval_arithmetic() {
        let engine = ScriptEngine::new("src");
        engine.eval("let x = 1 + 2;").unwrap();
    }

    #[test]
    fn test_eval_string() {
        let engine = ScriptEngine::new("src");
        engine.eval(r#"let msg = "hello";"#).unwrap();
    }

    #[test]
    fn test_eval_syntax_error() {
        let engine = ScriptEngine::new("src");
        let result = engine.eval("let x = ;");
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_undefined_var() {
        let engine = ScriptEngine::new("src");
        let result = engine.eval("let y = undefined_var;");
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_accessors() {
        let engine = ScriptEngine::new("src");
        assert!(engine.engine().compile("let a = 1;").is_ok());
    }

    #[test]
    fn test_parse_test_blocks() {
        let script = r#"
let a = 1;
//#test my test 1
assert(a == 1);
//#end
//#test
assert(a == 2);
//#end
"#;
        let blocks = parse_test_blocks(script);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].name, "my test 1");
        assert_eq!(blocks[0].start_line, 2);
        assert_eq!(blocks[0].end_line, 4);

        assert_eq!(blocks[1].name, "Test at line 6");
        assert_eq!(blocks[1].start_line, 5);
        assert_eq!(blocks[1].end_line, 7);
    }

    #[test]
    fn test_construct_test_script() {
        let script = "let a = 1;\n//#test t1\nassert(a == 1);\n//#end\nlet b = 2;\n//#test t2\nassert(b == 2);\n//#end";
        let blocks = parse_test_blocks(script);
        assert_eq!(blocks.len(), 2);

        let s1 = construct_test_script(script, &blocks, 0);
        let s1_lines: Vec<&str> = s1.split('\n').collect();
        assert_eq!(s1_lines.len(), 8);
        assert_eq!(s1_lines[0], "let a = 1;");
        assert_eq!(s1_lines[1], ""); // marker
        assert_eq!(s1_lines[2], "assert(a == 1);"); // target content
        assert_eq!(s1_lines[3], ""); // marker
        assert_eq!(s1_lines[4], "let b = 2;");
        assert_eq!(s1_lines[5], ""); // other block cleared
        assert_eq!(s1_lines[6], ""); // other block cleared
        assert_eq!(s1_lines[7], ""); // other block cleared

        let s2 = construct_test_script(script, &blocks, 1);
        let s2_lines: Vec<&str> = s2.split('\n').collect();
        assert_eq!(s2_lines.len(), 8);
        assert_eq!(s2_lines[0], "let a = 1;");
        assert_eq!(s2_lines[1], "");
        assert_eq!(s2_lines[2], "");
        assert_eq!(s2_lines[3], "");
        assert_eq!(s2_lines[4], "let b = 2;");
        assert_eq!(s2_lines[5], "");
        assert_eq!(s2_lines[6], "assert(b == 2);");
        assert_eq!(s2_lines[7], "");
    }

    #[test]
    fn test_strip_tests_behavior() {
        let script = r#"
let a = 1;
//#test
fn test_foo() {
    assert(a == 1);
}
//#end
let b = 2;
"#;
        let stripped = strip_tests(script);
        assert!(!stripped.contains("test_foo"));
        assert!(stripped.contains("let a = 1;"));
        assert!(stripped.contains("let b = 2;"));
        assert_eq!(script.lines().count(), stripped.lines().count());
    }

    #[test]
    fn test_run_tests_with_blocks() {
        let engine = ScriptEngine::new("src");
        let script = r#"
let val = 10;
//#test success block
assert(val == 10);
//#end
//#test fail block
assert(val == 20);
//#end
"#;
        let results = engine.run_tests(script);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "success block");
        if !results[0].success {
            panic!("Test 0 failed: {:?}", results[0].error);
        }
        assert_eq!(results[1].name, "fail block");
        assert!(!results[1].success);
        assert!(results[1]
            .error
            .as_ref()
            .unwrap()
            .contains("Assertion failed"));
    }

    #[test]
    fn test_run_tests_fallback_fns() {
        let engine = ScriptEngine::new("src");
        let script = r#"
fn test_success() {
    let x = 1;
}
fn test_fail() {
    throw "assert failure";
}
"#;
        let results = engine.run_tests(script);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "test_success");
        assert!(results[0].success);
        assert_eq!(results[1].name, "test_fail");
        assert!(!results[1].success);
    }

    #[test]
    fn test_script_say_hook_execution() {
        use oxide_core::{Direction, Exit, Position, RoomExits};
        let engine = ScriptEngine::default();
        let mut world = World::new();

        let room = world.spawn((Room::new("Test Room", "Desc"),));
        let _ = world.insert(room, (Position::new(room),));

        let mut exits = RoomExits(vec![Exit::new(Direction::North, room)]);
        exits.0[0].set_closed(true);
        exits.0[0].set_locked(true);
        let _ = world.insert(room, (exits,));

        let mut scope = Scope::new();
        scope.push("self", room);

        let _guard = push_script_context(room, None, None, &mut world);

        let exits_list = engine
            .engine()
            .eval_with_scope::<rhai::Array>(&mut scope, "room_exits(self)")
            .unwrap();
        assert_eq!(exits_list.len(), 1);
        assert_eq!(exits_list[0].to_string(), "north");

        let is_closed = engine
            .engine()
            .eval_with_scope::<bool>(&mut scope, r#"is_exit_closed(self, "north")"#)
            .unwrap();
        assert!(is_closed);

        engine
            .engine()
            .run_with_scope(
                &mut scope,
                r#"
            set_exit_locked(self, "north", false);
            set_exit_closed(self, "north", false);
        "#,
            )
            .unwrap();

        let is_closed_now = engine
            .engine()
            .eval_with_scope::<bool>(&mut scope, r#"is_exit_closed(self, "north")"#)
            .unwrap();
        assert!(!is_closed_now);
    }

    #[test]
    fn test_say_hook_dynamic_parameters() {
        use oxide_core::ScriptParams;
        let engine = ScriptEngine::new("../content/scripts");
        let mut world = World::new();

        let room_id = world.spawn((
            Room::new("Para Room", "Desc").with_script(Some("rooms/open_sesame.rhai".to_string())),
            {
                let mut p = HashMap::new();
                p.insert("keyword".to_string(), "please open".to_string());
                p.insert("direction".to_string(), "north".to_string());
                ScriptParams(p)
            },
        ));

        let mut exits = oxide_core::RoomExits(vec![oxide_core::Exit::new(
            oxide_core::Direction::North,
            room_id,
        )]);
        exits.0[0].set_closed(true);
        exits.0[0].set_locked(true);
        world.insert(room_id, (exits,)).unwrap();

        let speaker = world.spawn(());

        use oxide_core::ScriptingBridge;
        engine
            .execute_say_hook(room_id, speaker, "please open", &mut world)
            .unwrap();

        let is_closed = world
            .query_one::<&oxide_core::RoomExits>(room_id)
            .unwrap()
            .get()
            .unwrap()
            .0[0]
            .is_closed();
        assert!(!is_closed);
    }

    #[test]
    fn test_reload_script() {
        use oxide_core::ScriptingBridge;
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("temp_scripts_test_{}", std::process::id()));
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let script_file = temp_dir.join("test_script.rhai");
        std::fs::write(&script_file, b"fn on_trigger() { true }").unwrap();

        let engine = ScriptEngine::new(&temp_dir);

        let _ast = engine.get_ast("test_script.rhai").unwrap();

        std::fs::write(&script_file, b"fn on_trigger() { false }").unwrap();
        engine.reload_script("test_script.rhai").unwrap();

        let mut world = World::new();
        let entity = world.spawn(());
        let res = engine
            .execute_trigger("test_script.rhai", entity, None, None, &mut world)
            .unwrap();
        assert!(!res);

        let _ = std::fs::remove_file(&script_file);
        engine.reload_script("test_script.rhai").unwrap();

        {
            let cache = engine.ast_cache.read().unwrap();
            assert!(!cache.contains_key("test_script.rhai"));
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_quest_chaining_and_rewards_scripting() {
        use oxide_core::templates::{QuestDef, QuestRewards, TemplateRegistry};
        use oxide_core::{
            Equipment, Experience, Inventory, Level, QuestLog, ScriptingBridge, Wallet,
        };

        let engine = ScriptEngine::default();
        let mut world = World::new();

        let mut templates = TemplateRegistry::new();

        let quest_a = QuestDef {
            id: "quest_a".to_string(),
            name: "Quest A".to_string(),
            description: "First quest".to_string(),
            level_requirement: 0,
            repeatable: false,
            auto_complete: false,
            giver_npc: None,
            turn_in_npc: None,
            prerequisites: Vec::new(),
            objectives: Vec::new(),
            rewards: QuestRewards {
                xp: 100,
                gold: 50,
                items: Vec::new(),
                faction: Vec::new(),
            },
            scripts: None,
            params: HashMap::new(),
        };

        let quest_b = QuestDef {
            id: "quest_b".to_string(),
            name: "Quest B".to_string(),
            description: "Second quest".to_string(),
            level_requirement: 0,
            repeatable: false,
            auto_complete: false,
            giver_npc: None,
            turn_in_npc: None,
            prerequisites: vec!["quest_a".to_string()],
            objectives: Vec::new(),
            rewards: QuestRewards::default(),
            scripts: None,
            params: HashMap::new(),
        };

        templates.quests.insert("quest_a".to_string(), quest_a);
        templates.quests.insert("quest_b".to_string(), quest_b);

        oxide_core::templates::register_global_templates(std::sync::Arc::new(templates));

        let player = world.spawn((
            QuestLog::new(),
            Level(1),
            Experience(0),
            Wallet::new(0, 0, 0, 0),
            Inventory(Vec::new()),
            Equipment::new(),
        ));

        let global_templates = oxide_core::templates::get_global_templates().unwrap();
        oxide_core::accept_quest(&mut world, player, "quest_a", &global_templates).unwrap();
        {
            let mut q_log = world.query_one::<&mut QuestLog>(player).unwrap();
            let log = q_log.get().unwrap();
            log.completed.insert("quest_a".to_string());
        }

        let script = r#"
            assert(rewards.xp == 100);
            assert(rewards.gold == 50);
            accept_quest(player, "quest_b");
        "#;
        let ast = engine.engine().compile(script).unwrap();

        {
            let mut cache = engine.ast_cache.write().unwrap();
            cache.insert("quest_a_complete.rhai".to_string(), ast);
        }

        engine
            .execute_quest_hook("quest_a_complete.rhai", player, "quest_a", &mut world)
            .unwrap();

        let mut q_log = world.query_one::<&QuestLog>(player).unwrap();
        let log = q_log.get().unwrap();
        assert!(log.active.contains_key("quest_b"));
    }

    #[test]
    fn test_parry_active_stance_and_combat_end_deactivation() {
        use oxide_core::{ActiveScriptEffects, CombatState, LearnedSkills, ScriptingBridge};

        let engine = ScriptEngine::new("../content/scripts");
        let mut world = World::new();

        let attacker = world.spawn(());
        let defender = world.spawn((
            LearnedSkills::default(),
            ActiveScriptEffects::default(),
            CombatState::Engaged {
                target: attacker,
                round_started: std::time::Instant::now(),
                stance: None,
            },
        ));

        {
            let mut q = world.query_one::<&mut LearnedSkills>(defender).unwrap();
            let skills = q.get().unwrap();
            skills.set_rank("parry", 100);
        }

        let hit_ctx = engine
            .execute_combat_hit_hook(attacker, defender, false, &mut world)
            .unwrap();
        assert!(!hit_ctx.is_aborted);

        {
            let mut q = world
                .query_one::<&mut ActiveScriptEffects>(defender)
                .unwrap();
            let effects = q.get().unwrap();
            effects.add_or_replace(oxide_core::ActiveScriptEffect {
                id: "parrying".to_string(),
                display_name: "Parrying Stance".to_string(),
                source: "Parry".to_string(),
                description: "Parrying stance".to_string(),
                remaining_secs: 3600,
                expire_message: Some("You stop parrying.".to_string()),
                affects_display: None,
                show_remaining_time: false,
                visible_in_affects: true,
                name_prefix: None,
                name_suffix: None,
                short_desc_override: None,
                visible_on_look: false,
                look_aura: None,
                expire_conditions: vec![oxide_core::EffectExpireCondition::ExitCombat],
                params: HashMap::new(),
            });
        }

        let parried = (0..50).any(|_| {
            engine
                .execute_combat_hit_hook(attacker, defender, false, &mut world)
                .unwrap()
                .is_aborted
        });
        assert!(parried);

        oxide_core::transition_combat_state(&mut world, defender, CombatState::NotInCombat);

        let has_parry_effect = world
            .query_one::<&ActiveScriptEffects>(defender)
            .unwrap()
            .get()
            .unwrap()
            .has("parrying");
        assert!(!has_parry_effect);
    }

    #[test]
    fn test_evaluate_script_predicate_and_combat_damage_hook() {
        use oxide_core::{DamageType, LearnedSkills, Level, ScriptingBridge};

        let engine = ScriptEngine::default();
        let mut world = World::new();

        let player = world.spawn((Level(10), LearnedSkills::default()));
        {
            let mut q = world.query_one::<&mut LearnedSkills>(player).unwrap();
            let ls = q.get().unwrap();
            ls.set_rank("stoneskin", 100);
        }

        let is_level_10 = engine
            .evaluate_script_predicate("get_level(actor) >= 10", player, None, &mut world)
            .unwrap();
        assert!(is_level_10);

        let is_level_20 = engine
            .evaluate_script_predicate("get_level(actor) >= 20", player, None, &mut world)
            .unwrap();
        assert!(!is_level_20);

        let script = r#"
            fn on_combat_damage(dmg, dtype) {
                return dmg / 2;
            }
        "#;
        let ast = engine.engine().compile(script).unwrap();
        {
            let mut cache = engine.ast_cache.write().unwrap();
            cache.insert("skills/stoneskin.rhai".to_string(), ast);
        }

        let attacker = world.spawn(());
        let (final_dmg, _final_type) = engine
            .execute_combat_damage_hook(attacker, player, 40, DamageType::Slash, &mut world)
            .unwrap();
        assert_eq!(final_dmg, 20);
    }
}
