use rhai::Engine;

pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let engine = Engine::new();

        ScriptEngine { engine }
    }

    pub fn eval(&self, script: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.engine.run(script)?;
        Ok(())
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
}
