use super::interpreter::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Environment {
    variables: HashMap<String, Value>,
    parent: Option<Box<Environment>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(&self) -> Self {
        Environment {
            variables: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.variables
            .get_mut(name)
            .or_else(|| self.parent.as_mut().and_then(|p| p.get_mut(name)))
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn update(&mut self, name: &str, value: Value) -> bool {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            true
        } else if let Some(parent) = &mut self.parent {
            parent.update(name, value)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub env: Environment,
    pub pipeline_name: String,
    pub stage_index: usize,
    pub hooks: Vec<String>,
    pub timeout: Option<u64>,
    pub stalker: bool,
}

impl ExecutionContext {
    pub fn new(pipeline_name: String) -> Self {
        ExecutionContext {
            env: Environment::new(),
            pipeline_name,
            stage_index: 0,
            hooks: Vec::new(),
            timeout: None,
            stalker: false,
        }
    }
}
