use oxide_core::{DamageType, Entity, HitContext, ItemTriggers, Npc, Room, ScriptingBridge, World};
use rhai::{Engine, Scope, AST};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
}

/// A thread-safe wrapper around the ECS `World` raw pointer.
/// Rhai scripts run synchronously under the World lock, so this raw pointer
/// remains valid for the duration of the script's execution.
#[derive(Clone)]
pub struct ScriptWorld {
    world_ptr: *mut World,
}

// SAFETY: Rhai scripts run synchronously under the main game loop / world lock.
// The pointer is only accessed while the lock is held.
unsafe impl Send for ScriptWorld {}
unsafe impl Sync for ScriptWorld {}

impl ScriptWorld {
    pub fn new(world: &mut World) -> Self {
        ScriptWorld { world_ptr: world }
    }

    /// Access the underlying mutable reference to `World`.
    ///
    /// # SAFETY
    /// The caller must ensure that the pointer remains valid and is not aliased
    /// concurrently (which is guaranteed during synchronous script execution).
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_mut(&self) -> &mut World {
        &mut *self.world_ptr
    }

    /// Access the underlying immutable reference to `World`.
    ///
    /// # SAFETY
    /// The caller must ensure that the pointer remains valid.
    pub unsafe fn as_ref(&self) -> &World {
        &*self.world_ptr
    }
}

pub struct ScriptEngine {
    engine: Engine,
    script_dir: PathBuf,
    ast_cache: RwLock<HashMap<String, AST>>,
}

impl ScriptEngine {
    pub fn new(script_dir: impl Into<PathBuf>) -> Self {
        let mut engine = Engine::new();

        // Security limits
        engine.set_max_operations(50_000);
        engine.set_max_call_levels(32);
        engine.set_max_string_size(10_000);

        // Register custom types
        engine.register_type_with_name::<Entity>("Entity");
        engine.register_type_with_name::<ScriptWorld>("World");
        engine.register_type_with_name::<HitContext>("HitContext");
        engine.register_type_with_name::<DamageType>("DamageType");

        // Entity bindings
        engine.register_fn("id", |entity: Entity| entity.id() as i64);
        engine.register_fn("to_string", |entity: Entity| {
            format!("Entity({})", entity.id())
        });

        // HitContext bindings
        engine.register_get_set(
            "is_aborted",
            |ctx: &mut HitContext| ctx.is_aborted,
            |ctx: &mut HitContext, val: bool| ctx.is_aborted = val,
        );
        engine.register_get_set(
            "hit_modifier",
            |ctx: &mut HitContext| ctx.hit_modifier as i64,
            |ctx: &mut HitContext, val: i64| ctx.hit_modifier = val as i32,
        );
        engine.register_fn("abort", |ctx: &mut HitContext, reason: String| {
            ctx.is_aborted = true;
            ctx.abort_reason = Some(reason);
        });
        engine.register_fn("override_hit", |ctx: &mut HitContext, outcome: bool| {
            ctx.override_hit = Some(outcome);
        });

        // DamageType bindings
        engine.register_fn("to_string", |dt: DamageType| format!("{:?}", dt));

        // World operations
        // Spawning / Despawning
        engine.register_fn("despawn", |world: ScriptWorld, entity: Entity| unsafe {
            let _ = world.as_mut().despawn(entity);
        });

        // Querying components
        engine.register_fn("get_hp", |world: ScriptWorld, entity: Entity| -> i64 {
            unsafe {
                let w = world.as_ref();
                if let Ok(mut q) = w.query_one::<&oxide_core::Health>(entity) {
                    q.get().map(|h| h.current as i64).unwrap_or(0)
                } else {
                    0
                }
            }
        });
        engine.register_fn(
            "set_hp",
            |world: ScriptWorld, entity: Entity, hp: i64| unsafe {
                let w = world.as_mut();
                if let Ok(mut q) = w.query_one::<&mut oxide_core::Health>(entity) {
                    if let Some(h) = q.get() {
                        h.current = hp as i32;
                    }
                }
            },
        );
        engine.register_fn("get_max_hp", |world: ScriptWorld, entity: Entity| -> i64 {
            unsafe {
                let w = world.as_ref();
                if let Ok(mut q) = w.query_one::<&oxide_core::Health>(entity) {
                    q.get().map(|h| h.max as i64).unwrap_or(0)
                } else {
                    0
                }
            }
        });
        engine.register_fn("get_level", |world: ScriptWorld, entity: Entity| -> i64 {
            unsafe {
                let w = world.as_ref();
                if let Ok(mut q) = w.query_one::<&oxide_core::Level>(entity) {
                    q.get().map(|l| l.0 as i64).unwrap_or(1)
                } else {
                    1
                }
            }
        });
        engine.register_fn("get_name", |world: ScriptWorld, entity: Entity| -> String {
            unsafe {
                let w = world.as_ref();
                if let Ok(mut q) = w.query_one::<&oxide_core::Name>(entity) {
                    q.get()
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "Someone".to_string())
                } else {
                    "Someone".to_string()
                }
            }
        });

        // Messaging
        engine.register_fn("send_to", |entity: Entity, msg: String| {
            if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                bridge.send_to_entity(entity, &msg);
            }
        });
        engine.register_fn(
            "echo_room",
            |world: ScriptWorld, entity: Entity, msg: String| unsafe {
                let w = world.as_ref();
                let room = w
                    .query_one::<&oxide_core::Position>(entity)
                    .ok()
                    .and_then(|mut q| q.get().map(|p| p.room));
                if let Some(r) = room {
                    if let Some(bridge) = oxide_core::scripting::get_message_bridge() {
                        bridge.echo_to_room(r, &msg);
                    }
                }
            },
        );

        // Follower control
        engine.register_fn(
            "follow",
            |world: ScriptWorld, entity: Entity, target: Entity| unsafe {
                let w = world.as_mut();
                let _ = w.insert(
                    entity,
                    (oxide_core::Following {
                        target,
                        autofollow: true,
                    },),
                );
            },
        );
        engine.register_fn("unfollow", |world: ScriptWorld, entity: Entity| unsafe {
            let w = world.as_mut();
            let _ = w.remove_one::<oxide_core::Following>(entity);
        });

        // Mob spawning & Template spawn
        engine.register_fn(
            "spawn_mob",
            |_world: ScriptWorld, _template_id: String, _room_entity: Entity| -> Entity {
                Entity::from(hecs::Entity::from_bits(0).unwrap()) // Fallback if not hooked.
            },
        );

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

        ScriptEngine {
            engine,
            script_dir: script_dir.into(),
            ast_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Retrieve or compile an AST for the given script path relative to the script directory.
    pub fn get_ast(&self, rel_path: &str) -> Result<AST, String> {
        {
            let cache = self.ast_cache.read().unwrap();
            if let Some(ast) = cache.get(rel_path) {
                return Ok(ast.clone());
            }
        }

        let full_path = self.script_dir.join(rel_path);
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read script {}: {}", rel_path, e))?;

        let processed = strip_tests(&content);
        let ast = self
            .engine
            .compile(&processed)
            .map_err(|e| format!("Compile error in {}: {}", rel_path, e))?;

        let mut cache = self.ast_cache.write().unwrap();
        cache.insert(rel_path.to_string(), ast.clone());
        Ok(ast)
    }

    pub fn eval(&self, script: &str) -> Result<(), Box<dyn std::error::Error>> {
        let processed = strip_tests(script);
        self.engine.run(&processed)?;
        Ok(())
    }

    pub fn eval_with_options(
        &self,
        script: &str,
        include_tests: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let processed = if include_tests {
            script.to_string()
        } else {
            strip_tests(script)
        };
        self.engine.run(&processed)?;
        Ok(())
    }

    pub fn run_tests(&self, script: &str) -> Vec<TestResult> {
        let mut results = Vec::new();
        let blocks = parse_test_blocks(script);

        if !blocks.is_empty() {
            for (idx, block) in blocks.iter().enumerate() {
                let test_script = construct_test_script(script, &blocks, idx);
                match self.engine.run(&test_script) {
                    Ok(_) => results.push(TestResult {
                        name: block.name.clone(),
                        success: true,
                        error: None,
                    }),
                    Err(e) => results.push(TestResult {
                        name: block.name.clone(),
                        success: false,
                        error: Some(e.to_string()),
                    }),
                }
            }
        } else {
            // Fallback: run test_* functions if any
            let test_fns = find_test_functions(script);
            if !test_fns.is_empty() {
                let mut scope = rhai::Scope::new();
                match self.engine.compile(script) {
                    Ok(ast) => {
                        for test_name in test_fns {
                            match self.engine.call_fn::<()>(&mut scope, &ast, &test_name, ()) {
                                Ok(_) => results.push(TestResult {
                                    name: test_name,
                                    success: true,
                                    error: None,
                                }),
                                Err(e) => results.push(TestResult {
                                    name: test_name,
                                    success: false,
                                    error: Some(e.to_string()),
                                }),
                            }
                        }
                    }
                    Err(e) => {
                        results.push(TestResult {
                            name: "Compilation".to_string(),
                            success: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }
        results
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
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
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("self", entity);
        scope.push("actor", actor);
        scope.push("target", target);
        scope.push("world", ScriptWorld::new(world));

        self.engine
            .call_fn::<bool>(&mut scope, &ast, "on_trigger", ())
            .map_err(|e| e.to_string())
    }

    fn execute_combat_hit_hook(
        &self,
        attacker: Entity,
        target: Entity,
        is_offhand: bool,
        world: &mut World,
    ) -> Result<HitContext, String> {
        let mut hit_ctx = HitContext {
            attacker,
            target,
            is_offhand,
            is_aborted: false,
            abort_reason: None,
            hit_modifier: 0,
            override_hit: None,
        };

        // If target has parry, execute the parry script.
        let has_parry = if let Ok(mut q) = world.query_one::<&oxide_core::LearnedSkills>(target) {
            q.get().map(|s| s.has("parry")).unwrap_or(false)
        } else {
            false
        };

        if has_parry {
            if let Ok(ast) = self.get_ast("skills/parry.rhai") {
                let mut scope = Scope::new();
                scope.push("world", ScriptWorld::new(world));
                scope.push("hit_ctx", hit_ctx.clone());

                if let Ok(returned_ctx) =
                    self.engine
                        .call_fn::<HitContext>(&mut scope, &ast, "on_combat_hit", ())
                {
                    hit_ctx = returned_ctx;
                }
            }
        }

        Ok(hit_ctx)
    }

    fn execute_combat_damage_hook(
        &self,
        _attacker: Entity,
        _target: Entity,
        damage: i32,
        damage_type: DamageType,
        _world: &mut World,
    ) -> Result<(i32, DamageType), String> {
        Ok((damage, damage_type))
    }

    fn execute_mob_ai(
        &self,
        script: &str,
        entity: Entity,
        world: &mut World,
    ) -> Result<Option<String>, String> {
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("self", entity);
        scope.push("world", ScriptWorld::new(world));

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
        let mut scripts_to_run: Vec<String> = Vec::new();

        if let Ok(mut q) = world.query_one::<&Npc>(script_entity) {
            if let Some(npc) = q.get() {
                if let Some(ref s) = npc.script {
                    scripts_to_run.push(s.clone());
                }
            }
        }

        if let Ok(mut q) = world.query_one::<&Room>(script_entity) {
            if let Some(room) = q.get() {
                if let Some(ref s) = room.script {
                    scripts_to_run.push(s.clone());
                }
            }
        }

        if let Ok(mut q) = world.query_one::<&ItemTriggers>(script_entity) {
            if let Some(triggers) = q.get() {
                for trigger in &triggers.0 {
                    if trigger.event == "say" {
                        if let Some(ref s) = trigger.script {
                            scripts_to_run.push(s.clone());
                        }
                    }
                }
            }
        }

        for script_path in scripts_to_run {
            let ast = self.get_ast(&script_path)?;
            let mut scope = Scope::new();
            scope.push("self", script_entity);
            scope.push("speaker", speaker);
            scope.push("message", message.to_string());
            scope.push("world", ScriptWorld::new(world));

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
        let ast = self.get_ast(script)?;
        let mut scope = Scope::new();
        scope.push("actor", actor);
        scope.push("target", target);
        scope.push("world", ScriptWorld::new(world));

        self.engine
            .call_fn::<()>(&mut scope, &ast, "on_use", ())
            .map_err(|e| e.to_string())
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new("content/scripts")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestBlock {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn parse_test_blocks(script: &str) -> Vec<TestBlock> {
    let mut blocks = Vec::new();
    let mut current_start = None;
    let mut current_name = None;

    for (line_idx, line) in script.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("//#test") {
            let name = stripped.trim().to_string();
            let name = if name.is_empty() {
                format!("Test at line {}", line_idx + 1)
            } else {
                name
            };
            current_start = Some(line_idx);
            current_name = Some(name);
        } else if trimmed == "//#end" {
            if let (Some(start), Some(name)) = (current_start.take(), current_name.take()) {
                blocks.push(TestBlock {
                    name,
                    start_line: start,
                    end_line: line_idx,
                });
            }
        }
    }
    blocks
}

pub fn construct_test_script(script: &str, blocks: &[TestBlock], target_idx: usize) -> String {
    let target = &blocks[target_idx];
    let mut output = Vec::new();

    for (line_idx, line) in script.lines().enumerate() {
        if line_idx >= target.start_line && line_idx <= target.end_line {
            if line_idx == target.start_line || line_idx == target.end_line {
                output.push("");
            } else {
                output.push(line);
            }
        } else {
            let in_other_block = blocks.iter().enumerate().any(|(idx, b)| {
                idx != target_idx && line_idx >= b.start_line && line_idx <= b.end_line
            });
            if in_other_block {
                output.push("");
            } else {
                output.push(line);
            }
        }
    }
    output.join("\n")
}

pub fn strip_tests(script: &str) -> String {
    let blocks = parse_test_blocks(script);
    let mut output = Vec::new();

    for (line_idx, line) in script.lines().enumerate() {
        let in_any_block = blocks
            .iter()
            .any(|b| line_idx >= b.start_line && line_idx <= b.end_line);
        if in_any_block {
            output.push("");
        } else {
            output.push(line);
        }
    }
    output.join("\n")
}

fn find_test_functions(script: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in script.lines() {
        let line = line.trim();
        if line.starts_with("fn ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name_part = parts[1];
                if let Some(idx) = name_part.find('(') {
                    let name = &name_part[..idx];
                    if name.starts_with("test_") {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Line count preserved
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
        println!("Test block results: {:?}", results);
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
}
