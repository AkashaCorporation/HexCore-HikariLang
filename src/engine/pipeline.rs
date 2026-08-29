use super::environment::ExecutionContext;
use super::interpreter::{Interpreter, Value};
use super::runtime::RuntimeHost;
use crate::error::HKLError;
use crate::parser::ast::*;
use std::collections::HashMap;

pub struct PipelineEngine {
    interpreter: Interpreter,
}

impl PipelineEngine {
    pub fn new() -> Self {
        PipelineEngine {
            interpreter: Interpreter::new(),
        }
    }

    /// Builds a pipeline engine backed by a concrete HexCore runtime host.
    ///
    /// The default engine retains the development mock host. Production
    /// integration can inject one host without changing parser or interpreter
    /// semantics.
    pub fn with_runtime<H>(runtime: H) -> Self
    where
        H: RuntimeHost + 'static,
    {
        PipelineEngine {
            interpreter: Interpreter::with_runtime(runtime),
        }
    }

    pub fn execute_pipeline(
        &mut self,
        pipeline: &PipelineDecl,
        inputs: HashMap<String, Value>,
    ) -> Result<Value, HKLError> {
        let mut ctx = ExecutionContext::new(pipeline.name.clone());

        for (name, value) in inputs {
            ctx.env.set(name, value);
        }

        if let Some(session) = &pipeline.session {
            ctx.env
                .set("session".into(), Value::String_(session.clone()));
        }

        let mut last_value = Value::Void;
        for stage in &pipeline.body.stages {
            last_value = self.interpreter.execute_statement(stage, &mut ctx)?;
        }

        Ok(last_value)
    }

    pub fn execute_stage(
        &mut self,
        stage: &Statement,
        ctx: &mut ExecutionContext,
    ) -> Result<Value, HKLError> {
        self.interpreter.execute_statement(stage, ctx)
    }

    pub fn execute_hql(
        &mut self,
        _query: &HQLQueryBlock,
        _target: &Value,
    ) -> Result<Value, HKLError> {
        // Mock HQL execution — real engine plugs in later.
        // HQL integration intentionally remains outside this runtime-host wave.
        Ok(Value::Array(Vec::new()))
    }
}

impl Default for PipelineEngine {
    fn default() -> Self {
        Self::new()
    }
}
