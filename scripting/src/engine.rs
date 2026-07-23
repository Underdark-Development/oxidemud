use crate::bindings;
use crate::TestResult;
use rhai::{Engine, AST};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

pub struct ScriptEngine {
    pub(crate) engine: Engine,
    pub(crate) script_dir: PathBuf,
    pub(crate) ast_cache: RwLock<HashMap<String, AST>>,
}

impl ScriptEngine {
    pub fn new(script_dir: impl Into<PathBuf>) -> Self {
        let mut engine = Engine::new();
        bindings::register_all(&mut engine);

        ScriptEngine {
            engine,
            script_dir: script_dir.into(),
            ast_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Retrieve or compile an AST for the given script path relative to the script directory.
    pub fn get_ast(&self, rel_path: &str) -> Result<AST, String> {
        {
            let cache = self.ast_cache.read().unwrap_or_else(|e| e.into_inner());
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

        let mut cache = self.ast_cache.write().unwrap_or_else(|e| e.into_inner());
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
