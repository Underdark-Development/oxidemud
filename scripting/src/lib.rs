use rhai::Engine;

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
}

pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
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
        ScriptEngine { engine }
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
        Self::new()
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
        let engine = ScriptEngine::new();
        engine.eval("let x = 1 + 2;").unwrap();
    }

    #[test]
    fn test_eval_string() {
        let engine = ScriptEngine::new();
        engine.eval(r#"let msg = "hello";"#).unwrap();
    }

    #[test]
    fn test_eval_syntax_error() {
        let engine = ScriptEngine::new();
        let result = engine.eval("let x = ;");
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_undefined_var() {
        let engine = ScriptEngine::new();
        let result = engine.eval("let y = undefined_var;");
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_accessors() {
        let engine = ScriptEngine::new();
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
        let engine = ScriptEngine::new();
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
        let engine = ScriptEngine::new();
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
