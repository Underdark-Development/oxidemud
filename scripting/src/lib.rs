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
