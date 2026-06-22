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
        let engine = Engine::new();
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
        let test_fns = find_test_functions(script);

        if test_fns.is_empty() {
            return results;
        }

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

pub fn strip_tests(script: &str) -> String {
    let mut output = Vec::new();
    let mut in_test_block = false;
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed == "//#test" {
            in_test_block = true;
            output.push("");
            continue;
        }
        if trimmed == "//#end" {
            in_test_block = false;
            output.push("");
            continue;
        }
        if in_test_block {
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
    fn test_run_tests_in_engine() {
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
