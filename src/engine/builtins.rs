use super::interpreter::Value;
use crate::error::{HKLError, Span};
use std::collections::HashMap;

pub trait BuiltinFn: Send + Sync {
    fn call(&self, args: &[Value], span: Span) -> Result<Value, HKLError>;
}

pub fn default_builtins() -> HashMap<String, Box<dyn BuiltinFn>> {
    let mut builtins: HashMap<String, Box<dyn BuiltinFn>> = HashMap::new();

    for name in [
        "pathfinder",
        "helix.decompile",
        "elixir.emulate",
        "remill.lift",
        "detect_ioc",
        "generate_report",
        "chacha_detection",
        "refcount_scanner",
        "api_hash_resolver",
        "get_imports",
        "extract_strings",
        "filter",
    ] {
        builtins.insert(name.to_string(), Box::new(NamedMock(name.to_string())));
    }

    builtins
}

struct NamedMock(String);

impl BuiltinFn for NamedMock {
    fn call(&self, _args: &[Value], _span: Span) -> Result<Value, HKLError> {
        Ok(Value::String_(format!("mock:{}", self.0)))
    }
}
