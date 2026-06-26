use std::collections::HashMap;

/// In-memory component storing key-value parameters injected into Rhai scripting scopes.
#[derive(Debug, Clone, Default)]
pub struct ScriptParams(pub HashMap<String, String>);
