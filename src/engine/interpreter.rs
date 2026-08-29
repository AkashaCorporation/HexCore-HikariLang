use super::environment::{Environment, ExecutionContext};
use super::runtime::{MockRuntimeHost, RuntimeCall, RuntimeHost};
use crate::error::{HKLError, Span};
use crate::parser::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String_(String),
    Bool(bool),
    Address(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Function(FunctionValue),
    Pipeline(PipelineValue),
    HQLResult(HQLMatchResultValue),
    EmuSnapshot(EmuSnapshotValue),
    IOC(IOCValue),
    IRNode(String),
    Binary(String),
    Void,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String_(a), Value::String_(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Address(a), Value::Address(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a.name == b.name && a.params == b.params,
            (Value::Pipeline(a), Value::Pipeline(b)) => a.name == b.name,
            (Value::HQLResult(a), Value::HQLResult(b)) => a == b,
            (Value::EmuSnapshot(a), Value::EmuSnapshot(b)) => a == b,
            (Value::IOC(a), Value::IOC(b)) => a == b,
            (Value::IRNode(a), Value::IRNode(b)) => a == b,
            (Value::Binary(a), Value::Binary(b)) => a == b,
            (Value::Void, Value::Void) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionValue {
    pub name: String,
    pub params: Vec<String>,
    pub body: StatementBlock,
    /// Optional expression body for lambdas (when statements are empty).
    pub expr_body: Option<Box<Expr>>,
    pub closure: Environment,
}

#[derive(Debug, Clone)]
pub struct PipelineValue {
    pub name: String,
    pub stages: Vec<Statement>,
    pub input: Vec<InputSpec>,
    pub session: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HQLMatchResultValue {
    pub signature_id: String,
    pub matches: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmuSnapshotValue {
    pub session_id: String,
    pub timestamp: u64,
    pub hooks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IOCValue {
    pub ioc_type: String,
    pub value: String,
    pub confidence: f64,
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::String_(_) => "String",
            Value::Bool(_) => "Bool",
            Value::Address(_) => "Address",
            Value::Array(_) => "Array",
            Value::Map(_) => "Map",
            Value::Function(_) => "Function",
            Value::Pipeline(_) => "Pipeline",
            Value::HQLResult(_) => "HQLResult",
            Value::EmuSnapshot(_) => "EmuSnapshot",
            Value::IOC(_) => "IOC",
            Value::IRNode(_) => "IRNode",
            Value::Binary(_) => "Binary",
            Value::Void => "Void",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String_(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Void => false,
            _ => true,
        }
    }

    pub fn to_string_value(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String_(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Address(a) => a.clone(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string_value()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Map(map) => {
                let items: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string_value()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::HQLResult(r) => format!(
                "HQLResult(id={}, matches={}, confidence={})",
                r.signature_id,
                r.matches.len(),
                r.confidence
            ),
            Value::EmuSnapshot(s) => format!(
                "EmuSnapshot(session={}, hooks={})",
                s.session_id,
                s.hooks.len()
            ),
            Value::IOC(i) => format!("IOC({}: {} @ {:.2})", i.ioc_type, i.value, i.confidence),
            Value::IRNode(n) => format!("IRNode({})", n),
            Value::Binary(b) => format!("Binary({})", b),
            _ => format!("<{}>", self.type_name()),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_value())
    }
}

pub struct Interpreter {
    pub environment: Environment,
    runtime: Box<dyn RuntimeHost>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self::with_runtime(MockRuntimeHost)
    }

    pub fn with_runtime<H>(runtime: H) -> Self
    where
        H: RuntimeHost + 'static,
    {
        Interpreter {
            environment: Environment::new(),
            runtime: Box::new(runtime),
        }
    }

    pub fn execute_program(&mut self, program: &Program) -> Result<Value, HKLError> {
        let mut last_value = Value::Void;

        for decl in &program.declarations {
            last_value = self.execute_declaration(decl)?;
        }

        // Auto-run the first pipeline with mock inputs (IDE/Decompiler plug later).
        if let Some(TopLevelDecl::Pipeline(pipeline)) = program.declarations.first() {
            last_value = self.run_pipeline(pipeline)?;
        }

        Ok(last_value)
    }

    pub fn run_pipeline(&mut self, pipeline: &PipelineDecl) -> Result<Value, HKLError> {
        let mut ctx = ExecutionContext::new(pipeline.name.clone());
        if let Some(session) = &pipeline.session {
            ctx.env
                .set("session".into(), Value::String_(session.clone()));
        }

        // Bind mock inputs so pipelines can execute without a real binary yet.
        for input in &pipeline.input {
            let mock = match &input.type_ref {
                TypeRef::Simple(name) => Value::Binary(format!("mock:{}", name)),
                TypeRef::Union(variants) => {
                    let names: Vec<String> = variants
                        .iter()
                        .map(|v| match v {
                            TypeRef::Simple(s) => s.clone(),
                            _ => "?".into(),
                        })
                        .collect();
                    Value::Binary(format!("mock:{}", names.join("|")))
                }
                _ => Value::Binary("mock:Binary".into()),
            };
            ctx.env.set(input.param.clone(), mock);
        }

        println!("── pipeline {} ──", pipeline.name);
        if let Some(s) = &pipeline.session {
            println!("   session: {}", s);
        }

        let mut last_value = Value::Void;
        for stage in &pipeline.body.stages {
            last_value = self.execute_statement(stage, &mut ctx)?;
        }

        Ok(last_value)
    }

    fn execute_declaration(&mut self, decl: &TopLevelDecl) -> Result<Value, HKLError> {
        match decl {
            TopLevelDecl::Pipeline(pipeline) => {
                let value = Value::Pipeline(PipelineValue {
                    name: pipeline.name.clone(),
                    stages: pipeline.body.stages.clone(),
                    input: pipeline.input.clone(),
                    session: pipeline.session.clone(),
                });
                self.environment.set(pipeline.name.clone(), value.clone());
                Ok(value)
            }
            TopLevelDecl::Function(func) => {
                let value = Value::Function(FunctionValue {
                    name: func.name.clone(),
                    params: func.params.iter().map(|p| p.name.clone()).collect(),
                    body: func.body.clone(),
                    expr_body: None,
                    closure: self.environment.clone(),
                });
                self.environment.set(func.name.clone(), value.clone());
                Ok(value)
            }
            TopLevelDecl::Import(_) => Ok(Value::Void),
            TopLevelDecl::Const(const_) => {
                let value =
                    self.evaluate_expr(&const_.value, &mut ExecutionContext::new("const".into()))?;
                self.environment.set(const_.name.clone(), value.clone());
                Ok(value)
            }
        }
    }

    pub fn execute_statement(
        &mut self,
        stmt: &Statement,
        ctx: &mut ExecutionContext,
    ) -> Result<Value, HKLError> {
        match stmt {
            Statement::LetBinding(let_) => {
                let value = self.evaluate_expr(&let_.value, ctx)?;
                ctx.env.set(let_.name.clone(), value.clone());
                Ok(value)
            }
            Statement::Assign(assign) => {
                let value = self.evaluate_expr(&assign.value, ctx)?;
                ctx.env.set(assign.name.clone(), value.clone());
                Ok(value)
            }
            Statement::HQLQuery(hql) => {
                let _target = self.evaluate_expr(&hql.target, ctx)?;
                // Mock HQL: produce a match result the rest of the pipeline can use.
                let result = Value::HQLResult(HQLMatchResultValue {
                    signature_id: "embedded_hql".into(),
                    matches: vec!["@0x401000".into(), "@0x402100".into()],
                    confidence: 0.82,
                });
                // Also expose as array-like via Map with .any() support via member
                let wrapped = Value::Map(HashMap::from([
                    (
                        "matches".into(),
                        Value::Array(vec![
                            Value::String_("@0x401000".into()),
                            Value::String_("@0x402100".into()),
                        ]),
                    ),
                    ("confidence".into(), Value::Float(0.82)),
                    ("any".into(), Value::Bool(true)), // used by matches.any() when treated as field
                ]));
                // Prefer the map so .any() works as member field; keep HQLResult available too.
                let _ = result;
                ctx.env.set(hql.variable.clone(), wrapped.clone());
                Ok(wrapped)
            }
            Statement::If(if_) => {
                let condition = self.evaluate_expr(&if_.condition, ctx)?;
                if condition.is_truthy() {
                    self.execute_block(&if_.then, ctx)
                } else if let Some(else_block) = &if_.else_block {
                    self.execute_block(else_block, ctx)
                } else {
                    Ok(Value::Void)
                }
            }
            Statement::For(for_) => {
                let iterable = self.evaluate_expr(&for_.iterable, ctx)?;
                match iterable {
                    Value::Array(arr) => {
                        let mut last_value = Value::Void;
                        for item in arr {
                            ctx.env.set(for_.variable.clone(), item);
                            last_value = self.execute_block(&for_.body, ctx)?;
                        }
                        Ok(last_value)
                    }
                    _ => Err(HKLError::Runtime {
                        message: format!("Cannot iterate over {}", iterable.type_name()),
                        span: for_.span.clone(),
                    }),
                }
            }
            Statement::While(while_) => {
                let mut last_value = Value::Void;
                let mut guard = 0usize;
                loop {
                    let condition = self.evaluate_expr(&while_.condition, ctx)?;
                    if !condition.is_truthy() {
                        break;
                    }
                    last_value = self.execute_block(&while_.body, ctx)?;
                    guard += 1;
                    if guard > 10_000 {
                        return Err(HKLError::Runtime {
                            message: "While loop exceeded iteration limit".into(),
                            span: while_.span.clone(),
                        });
                    }
                }
                Ok(last_value)
            }
            Statement::Emit(emit) => {
                let value = self.evaluate_expr(&emit.value, ctx)?;
                println!("EMIT {}", value.to_string_value());
                Ok(value)
            }
            Statement::Store(store) => {
                let target = store.target.as_deref().unwrap_or("session");
                println!("STORE {}", target);
                Ok(Value::Void)
            }
            Statement::Notify(notify) => {
                let message = self.evaluate_expr(&notify.message, ctx)?;
                if let Some(condition) = &notify.condition {
                    let cond_value = self.evaluate_expr(condition, ctx)?;
                    if cond_value.is_truthy() {
                        println!("NOTIFY: {}", message.to_string_value());
                    }
                } else {
                    println!("NOTIFY: {}", message.to_string_value());
                }
                Ok(Value::Void)
            }
            Statement::Expr(expr) => self.evaluate_expr(&expr.expr, ctx),
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.evaluate_expr(value, ctx)
                } else {
                    Ok(Value::Void)
                }
            }
            Statement::Input(spec) => {
                // Already bound in run_pipeline; keep for completeness.
                if ctx.env.get(&spec.param).is_none() {
                    ctx.env
                        .set(spec.param.clone(), Value::Binary("mock:Binary".into()));
                }
                Ok(Value::Void)
            }
            Statement::Session(name) => {
                ctx.env.set("session".into(), Value::String_(name.clone()));
                Ok(Value::Void)
            }
        }
    }

    pub fn execute_block(
        &mut self,
        block: &StatementBlock,
        ctx: &mut ExecutionContext,
    ) -> Result<Value, HKLError> {
        // Pipeline blocks share the surrounding environment so assignments
        // inside `if` / stages remain visible to later stages (script semantics).
        // Function calls create their own env via `call_function`.
        let mut last_value = Value::Void;
        for stmt in &block.statements {
            last_value = self.execute_statement(stmt, ctx)?;
        }
        Ok(last_value)
    }

    pub fn evaluate_expr(
        &mut self,
        expr: &Expr,
        ctx: &mut ExecutionContext,
    ) -> Result<Value, HKLError> {
        match expr {
            Expr::Literal(lit) => self.evaluate_literal(lit, ctx),
            Expr::Ident(name) => {
                if let Some(v) = ctx
                    .env
                    .get(name)
                    .or_else(|| self.environment.get(name))
                    .cloned()
                {
                    Ok(v)
                } else {
                    // Mock-friendly: fixtures may reference future-bound names
                    // (e.g. timeline) before IDE/Decompiler plug-ins exist.
                    println!(
                        "  [warn] undefined variable '{}', using mock placeholder",
                        name
                    );
                    Ok(Value::String_(format!("<undefined:{}>", name)))
                }
            }
            Expr::Binary(bin) => {
                let left = self.evaluate_expr(&bin.left, ctx)?;
                let right = self.evaluate_expr(&bin.right, ctx)?;
                self.evaluate_binary(bin.op.clone(), left, right, bin.span.clone())
            }
            Expr::Unary(unary) => {
                let operand = self.evaluate_expr(&unary.operand, ctx)?;
                self.evaluate_unary(unary.op.clone(), operand, unary.span.clone())
            }
            Expr::Pipe(pipe) => {
                let left = self.evaluate_expr(&pipe.left, ctx)?;
                // If right is a call, inject left as first arg
                match pipe.right.as_ref() {
                    Expr::Call(call) => {
                        let mut args = vec![left];
                        for a in &call.args {
                            args.push(self.evaluate_expr(a, ctx)?);
                        }
                        self.dispatch_call(
                            &call.callee,
                            &args,
                            &call.named_args,
                            call.span.clone(),
                            ctx,
                        )
                    }
                    Expr::Ident(name) => {
                        if self.runtime.supports(name) {
                            self.call_runtime(name, &[left], &[], pipe.span.clone())
                        } else {
                            Err(HKLError::Runtime {
                                message: format!("Unknown function/builtin: {}", name),
                                span: pipe.span.clone(),
                            })
                        }
                    }
                    other => {
                        let right = self.evaluate_expr(other, ctx)?;
                        match right {
                            Value::Function(func) => self.call_function(func, vec![left], ctx),
                            _ => Err(HKLError::Runtime {
                                message: format!("Cannot pipe into {}", right.type_name()),
                                span: pipe.span.clone(),
                            }),
                        }
                    }
                }
            }
            Expr::Call(call) => {
                let mut args = Vec::new();
                for a in &call.args {
                    args.push(self.evaluate_expr(a, ctx)?);
                }
                self.dispatch_call(
                    &call.callee,
                    &args,
                    &call.named_args,
                    call.span.clone(),
                    ctx,
                )
            }
            Expr::Member(member) => {
                let object = self.evaluate_expr(&member.object, ctx)?;
                self.member_access(object, &member.member, member.span.clone())
            }
            Expr::Index(index) => {
                let object = self.evaluate_expr(&index.object, ctx)?;
                let idx = self.evaluate_expr(&index.index, ctx)?;
                match (object, idx) {
                    (Value::Array(arr), Value::Int(i)) => {
                        let i = i as usize;
                        arr.get(i).cloned().ok_or_else(|| HKLError::Runtime {
                            message: format!("Index out of bounds: {}", i),
                            span: index.span.clone(),
                        })
                    }
                    (Value::Map(map), Value::String_(key)) => {
                        map.get(&key).cloned().ok_or_else(|| HKLError::Runtime {
                            message: format!("Key not found: {}", key),
                            span: index.span.clone(),
                        })
                    }
                    (obj, _) => Err(HKLError::Runtime {
                        message: format!("Cannot index {}", obj.type_name()),
                        span: index.span.clone(),
                    }),
                }
            }
            Expr::Match(match_) => {
                let scrutinee = self.evaluate_expr(&match_.scrutinee, ctx)?;
                for case in &match_.cases {
                    if let Some(binds) = self.match_pattern(&case.pattern, &scrutinee, ctx)? {
                        for (name, value) in binds {
                            ctx.env.set(name, value);
                        }
                        return self.evaluate_expr(&case.body, ctx);
                    }
                }
                Err(HKLError::Runtime {
                    message: "No matching case".into(),
                    span: match_.span.clone(),
                })
            }
            Expr::Lambda(lambda) => Ok(Value::Function(FunctionValue {
                name: "<lambda>".into(),
                params: lambda.params.clone(),
                body: StatementBlock {
                    statements: Vec::new(),
                    span: lambda.span.clone(),
                },
                expr_body: Some(lambda.body.clone()),
                closure: ctx.env.clone(),
            })),
            Expr::Block(block) => {
                let mut last_value = Value::Void;
                for stmt in &block.statements {
                    last_value = self.execute_statement(stmt, ctx)?;
                }
                if let Some(result) = &block.result {
                    self.evaluate_expr(result, ctx)
                } else {
                    Ok(last_value)
                }
            }
            Expr::TypeCast(cast) => self.evaluate_expr(&cast.expr, ctx),
            Expr::Ternary(ternary) => {
                let condition = self.evaluate_expr(&ternary.condition, ctx)?;
                if condition.is_truthy() {
                    self.evaluate_expr(&ternary.then_branch, ctx)
                } else {
                    self.evaluate_expr(&ternary.else_branch, ctx)
                }
            }
            Expr::Address(addr) => Ok(Value::Address(addr.clone())),
            Expr::Range(range) => {
                let start = self.evaluate_expr(&range.start, ctx)?;
                let end = self.evaluate_expr(&range.end, ctx)?;
                Ok(Value::Array(vec![start, end]))
            }
            Expr::PipelineRef(name) => ctx
                .env
                .get(name)
                .or_else(|| self.environment.get(name))
                .cloned()
                .ok_or_else(|| HKLError::Runtime {
                    message: format!("Undefined pipeline: {}", name),
                    span: 0..0,
                }),
            Expr::Filter(filter) => {
                let collection = self.evaluate_expr(&filter.collection, ctx)?;
                // Mock filter: return collection unchanged.
                // Predicate evaluation is deferred until real query engine plugs in
                // (avoids unbound free vars like `name` in `where is_suspicious(name)`).
                let _ = &filter.predicate;
                match collection {
                    Value::Array(arr) => Ok(Value::Array(arr)),
                    other => Ok(other),
                }
            }
        }
    }

    fn member_access(
        &mut self,
        object: Value,
        member: &str,
        span: Span,
    ) -> Result<Value, HKLError> {
        match &object {
            Value::Map(map) => {
                if let Some(v) = map.get(member) {
                    return Ok(v.clone());
                }
            }
            Value::HQLResult(r) => {
                return Ok(match member {
                    "any" => Value::Bool(!r.matches.is_empty()),
                    "confidence" => Value::Float(r.confidence),
                    "matches" => Value::Array(
                        r.matches
                            .iter()
                            .map(|m| Value::String_(m.clone()))
                            .collect(),
                    ),
                    "signature_id" => Value::String_(r.signature_id.clone()),
                    _ => {
                        return Err(HKLError::Runtime {
                            message: format!("Unknown field '{}' on HQLResult", member),
                            span,
                        })
                    }
                });
            }
            Value::EmuSnapshot(s) => {
                return Ok(match member {
                    "session_id" => Value::String_(s.session_id.clone()),
                    "timestamp" => Value::Int(s.timestamp as i64),
                    "hooks" => {
                        Value::Array(s.hooks.iter().map(|h| Value::String_(h.clone())).collect())
                    }
                    _ => {
                        return Err(HKLError::Runtime {
                            message: format!("Unknown field '{}' on EmuSnapshot", member),
                            span,
                        })
                    }
                });
            }
            Value::IOC(i) => {
                return Ok(match member {
                    "type" => Value::String_(i.ioc_type.clone()),
                    "value" => Value::String_(i.value.clone()),
                    "confidence" => Value::Float(i.confidence),
                    "high_confidence" => Value::Bool(i.confidence >= 0.7),
                    _ => {
                        return Err(HKLError::Runtime {
                            message: format!("Unknown field '{}' on IOC", member),
                            span,
                        })
                    }
                });
            }
            Value::Array(arr) => {
                if member == "any" {
                    return Ok(Value::Bool(!arr.is_empty()));
                }
                if member == "len" || member == "length" {
                    return Ok(Value::Int(arr.len() as i64));
                }
                if member == "high_confidence" {
                    return Ok(Value::Bool(arr.iter().any(|v| {
                        match v {
                            Value::IOC(i) => i.confidence >= 0.7,
                            Value::Map(map) => map
                                .get("confidence")
                                .map(|c| matches!(c, Value::Float(f) if *f >= 0.7))
                                .unwrap_or(false),
                            _ => false,
                        }
                    })));
                }
            }
            _ => {}
        }

        Err(HKLError::Runtime {
            message: format!(
                "Cannot access member '{}' on {}",
                member,
                object.type_name()
            ),
            span,
        })
    }

    fn dispatch_call(
        &mut self,
        callee: &Expr,
        args: &[Value],
        named_args: &[(String, Expr)],
        span: Span,
        ctx: &mut ExecutionContext,
    ) -> Result<Value, HKLError> {
        // Resolve named args into values
        let mut named_vals: Vec<(String, Value)> = Vec::new();
        for (k, e) in named_args {
            named_vals.push((k.clone(), self.evaluate_expr(e, ctx)?));
        }

        // member call: helix.decompile(...), matches.any()
        if let Expr::Member(m) = callee {
            // Module-style runtime calls first (do not resolve the module as a variable):
            // remill.lift, helix.decompile, elixir.emulate, or host-provided functions.
            if let Expr::Ident(module) = m.object.as_ref() {
                let full = format!("{}.{}", module, m.member);
                if self.runtime.supports(&full) {
                    return self.call_runtime(&full, args, &named_vals, span.clone());
                }
            }

            let object = self.evaluate_expr(&m.object, ctx)?;

            // Method-style .any()
            if m.member == "any" {
                return Ok(Value::Bool(
                    object.is_truthy()
                        && match &object {
                            Value::Array(a) => !a.is_empty(),
                            Value::HQLResult(r) => !r.matches.is_empty(),
                            Value::Map(map) => map
                                .get("any")
                                .map(|v| v.is_truthy())
                                .unwrap_or(!map.is_empty()),
                            _ => object.is_truthy(),
                        },
                ));
            }

            // high_confidence on array of IOCs: iocs.high_confidence
            if m.member == "high_confidence" {
                return match &object {
                    Value::Array(arr) => Ok(Value::Bool(arr.iter().any(|v| {
                        match v {
                            Value::IOC(i) => i.confidence >= 0.7,
                            Value::Map(map) => map
                                .get("confidence")
                                .map(|c| match c {
                                    Value::Float(f) => *f >= 0.7,
                                    _ => false,
                                })
                                .unwrap_or(false),
                            _ => false,
                        }
                    }))),
                    Value::IOC(i) => Ok(Value::Bool(i.confidence >= 0.7)),
                    _ => Ok(Value::Bool(false)),
                };
            }
        }

        if let Expr::Ident(name) = callee {
            // User function?
            if let Some(Value::Function(func)) = ctx
                .env
                .get(name)
                .or_else(|| self.environment.get(name))
                .cloned()
            {
                return self.call_function(func, args.to_vec(), ctx);
            }
            if self.runtime.supports(name) {
                return self.call_runtime(name, args, &named_vals, span);
            }
            return Err(HKLError::Runtime {
                message: format!("Unknown function/builtin: {}", name),
                span,
            });
        }

        // Evaluate callee as value
        let callee_val = self.evaluate_expr(callee, ctx)?;
        match callee_val {
            Value::Function(func) => self.call_function(func, args.to_vec(), ctx),
            _ => Err(HKLError::Runtime {
                message: format!("Cannot call {}", callee_val.type_name()),
                span,
            }),
        }
    }

    fn call_function(
        &mut self,
        func: FunctionValue,
        args: Vec<Value>,
        ctx: &mut ExecutionContext,
    ) -> Result<Value, HKLError> {
        if !func.params.is_empty() && func.params.len() != args.len() {
            return Err(HKLError::Runtime {
                message: format!(
                    "Expected {} arguments for {}, got {}",
                    func.params.len(),
                    func.name,
                    args.len()
                ),
                span: 0..0,
            });
        }

        let mut child_env = func.closure.child();
        for (param, arg) in func.params.iter().zip(args) {
            child_env.set(param.clone(), arg);
        }

        let old = std::mem::replace(&mut ctx.env, child_env);
        let result = if let Some(expr) = &func.expr_body {
            self.evaluate_expr(expr, ctx)
        } else {
            self.execute_block(&func.body, ctx)
        };
        ctx.env = old;
        result
    }

    fn evaluate_literal(
        &mut self,
        lit: &Literal,
        ctx: &mut ExecutionContext,
    ) -> Result<Value, HKLError> {
        match lit {
            Literal::Int(n) => Ok(Value::Int(*n)),
            Literal::Float(n) => Ok(Value::Float(*n)),
            Literal::String(s) => Ok(Value::String_(s.clone())),
            Literal::Bool(b) => Ok(Value::Bool(*b)),
            Literal::Hex(h) => Ok(Value::Address(h.clone())),
            Literal::Address(a) => Ok(Value::Address(a.clone())),
            Literal::Array(elems) => {
                let mut values = Vec::new();
                for e in elems {
                    values.push(self.evaluate_expr(e, ctx)?);
                }
                Ok(Value::Array(values))
            }
            Literal::Map(entries) => {
                let mut map = HashMap::new();
                for (key, val) in entries {
                    let key_val = self.evaluate_expr(key, ctx)?;
                    let val_val = self.evaluate_expr(val, ctx)?;
                    match key_val {
                        Value::String_(s) => {
                            map.insert(s, val_val);
                        }
                        other => {
                            return Err(HKLError::Runtime {
                                message: format!(
                                    "Map keys must be strings, got {}",
                                    other.type_name()
                                ),
                                span: 0..0,
                            });
                        }
                    }
                }
                Ok(Value::Map(map))
            }
        }
    }

    fn evaluate_binary(
        &self,
        op: BinaryOp,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, HKLError> {
        match op {
            BinaryOp::Add => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::String_(a), Value::String_(b)) => Ok(Value::String_(format!("{}{}", a, b))),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                _ => Err(HKLError::Runtime {
                    message: format!("Cannot add {} and {}", left.type_name(), right.type_name()),
                    span,
                }),
            },
            BinaryOp::Sub => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot subtract {} from {}",
                        right.type_name(),
                        left.type_name()
                    ),
                    span,
                }),
            },
            BinaryOp::Mul => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot multiply {} and {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                }),
            },
            BinaryOp::Div => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if *b == 0 {
                        Err(HKLError::Runtime {
                            message: "Division by zero".into(),
                            span,
                        })
                    } else {
                        Ok(Value::Int(a / b))
                    }
                }
                (Value::Float(a), Value::Float(b)) => {
                    if *b == 0.0 {
                        Err(HKLError::Runtime {
                            message: "Division by zero".into(),
                            span,
                        })
                    } else {
                        Ok(Value::Float(a / b))
                    }
                }
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot divide {} by {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                }),
            },
            BinaryOp::Mod => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if *b == 0 {
                        Err(HKLError::Runtime {
                            message: "Modulo by zero".into(),
                            span,
                        })
                    } else {
                        Ok(Value::Int(a % b))
                    }
                }
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot modulo {} by {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                }),
            },
            BinaryOp::Eq => Ok(Value::Bool(left == right)),
            BinaryOp::Ne => Ok(Value::Bool(left != right)),
            BinaryOp::Lt => cmp_ord(&left, &right, span, |o| o.is_lt()),
            BinaryOp::Gt => cmp_ord(&left, &right, span, |o| o.is_gt()),
            BinaryOp::Le => cmp_ord(&left, &right, span, |o| o.is_le()),
            BinaryOp::Ge => cmp_ord(&left, &right, span, |o| o.is_ge()),
            BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            BinaryOp::BitAnd => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot bitwise AND {} and {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                }),
            },
            BinaryOp::BitOr => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot bitwise OR {} and {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                }),
            },
            BinaryOp::BitXor => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot bitwise XOR {} and {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                }),
            },
            BinaryOp::Shl => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a << b)),
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot left shift {} by {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                }),
            },
            BinaryOp::Shr => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a >> b)),
                _ => Err(HKLError::Runtime {
                    message: format!(
                        "Cannot right shift {} by {}",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                }),
            },
        }
    }

    fn evaluate_unary(&self, op: UnaryOp, operand: Value, span: Span) -> Result<Value, HKLError> {
        match op {
            UnaryOp::Neg => match operand {
                Value::Int(n) => Ok(Value::Int(-n)),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(HKLError::Runtime {
                    message: format!("Cannot negate {}", operand.type_name()),
                    span,
                }),
            },
            UnaryOp::Not => Ok(Value::Bool(!operand.is_truthy())),
            UnaryOp::BitNot => match operand {
                Value::Int(n) => Ok(Value::Int(!n)),
                _ => Err(HKLError::Runtime {
                    message: format!("Cannot bitwise NOT {}", operand.type_name()),
                    span,
                }),
            },
        }
    }

    fn match_pattern(
        &mut self,
        pattern: &Pattern,
        value: &Value,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Vec<(String, Value)>>, HKLError> {
        Ok(match pattern {
            Pattern::Wildcard => Some(Vec::new()),
            Pattern::Ident(name) => Some(vec![(name.clone(), value.clone())]),
            Pattern::Literal(lit) => {
                let pattern_val = self.evaluate_literal(lit, ctx)?;
                if pattern_val == *value {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            Pattern::Array(patterns) => match value {
                Value::Array(arr) if patterns.len() == arr.len() => {
                    let mut binds = Vec::new();
                    for (pat, val) in patterns.iter().zip(arr) {
                        if let Some(mut b) = self.match_pattern(pat, val, ctx)? {
                            binds.append(&mut b);
                        } else {
                            return Ok(None);
                        }
                    }
                    Some(binds)
                }
                _ => None,
            },
            Pattern::Rest(_) => Some(Vec::new()),
        })
    }

    fn call_runtime(
        &mut self,
        name: &str,
        args: &[Value],
        named: &[(String, Value)],
        span: Span,
    ) -> Result<Value, HKLError> {
        self.runtime.call(RuntimeCall {
            name,
            args,
            named_args: named,
            span,
        })
    }
}

fn cmp_ord<F>(left: &Value, right: &Value, span: Span, pred: F) -> Result<Value, HKLError>
where
    F: Fn(std::cmp::Ordering) -> bool,
{
    let ord = match (left, right) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String_(a), Value::String_(b)) => a.cmp(b),
        _ => {
            return Err(HKLError::Runtime {
                message: format!(
                    "Cannot compare {} and {}",
                    left.type_name(),
                    right.type_name()
                ),
                span,
            });
        }
    };
    Ok(Value::Bool(pred(ord)))
}
