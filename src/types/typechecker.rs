use std::collections::HashMap;
use crate::error::HKLError;
use crate::parser::ast::*;
use super::core::{HKLType, FunctionSig, builtin_signatures};

#[derive(Clone)]
pub struct TypeChecker {
    env: HashMap<String, HKLType>,
    parent: Option<Box<TypeChecker>>,
    builtins: HashMap<String, FunctionSig>,
    errors: Vec<HKLError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            env: HashMap::new(),
            parent: None,
            builtins: builtin_signatures(),
            errors: Vec::new(),
        }
    }

    pub fn child(&self) -> Self {
        TypeChecker {
            env: HashMap::new(),
            parent: Some(Box::new(self.clone())),
            builtins: self.builtins.clone(),
            errors: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<HKLType> {
        self.env
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }

    pub fn set(&mut self, name: String, type_: HKLType) {
        self.env.insert(name, type_);
    }

    pub fn errors(&self) -> &[HKLError] {
        &self.errors
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), Vec<HKLError>> {
        for decl in &program.declarations {
            self.check_declaration(decl);
        }
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_declaration(&mut self, decl: &TopLevelDecl) {
        match decl {
            TopLevelDecl::Pipeline(pipeline) => self.check_pipeline(pipeline),
            TopLevelDecl::Function(func) => self.check_function(func),
            TopLevelDecl::Import(_) => {}
            TopLevelDecl::Const(const_) => self.check_const(const_),
        }
    }

    fn check_pipeline(&mut self, pipeline: &PipelineDecl) {
        let mut child = self.child();

        for input in &pipeline.input {
            let type_ = self.resolve_type_ref(&input.type_ref);
            child.set(input.param.clone(), type_);
        }

        if pipeline.session.is_some() {
            child.set("session".into(), HKLType::String_);
        }

        for stage in &pipeline.body.stages {
            child.check_statement(stage);
        }

        self.errors.extend(child.errors);
    }

    fn check_function(&mut self, func: &FunctionDecl) {
        let mut child = self.child();

        for param in &func.params {
            let type_ = param
                .type_ref
                .as_ref()
                .map(|t| self.resolve_type_ref(t))
                .unwrap_or(HKLType::Unknown);
            child.set(param.name.clone(), type_);
        }

        for stmt in &func.body.statements {
            child.check_statement(stmt);
        }

        self.errors.extend(child.errors);
    }

    fn check_const(&mut self, const_: &ConstDecl) {
        let type_ = const_
            .type_ref
            .as_ref()
            .map(|t| self.resolve_type_ref(t))
            .unwrap_or_else(|| self.infer_expr(&const_.value));

        self.set(const_.name.clone(), type_);
    }

    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::LetBinding(let_) => {
                let type_ = let_
                    .type_ref
                    .as_ref()
                    .map(|t| self.resolve_type_ref(t))
                    .unwrap_or_else(|| self.infer_expr(&let_.value));

                let inferred = self.infer_expr(&let_.value);
                if !self.types_compatible(&type_, &inferred) {
                    self.errors.push(HKLError::Type {
                        message: format!("Type mismatch: expected {}, got {}", type_, inferred),
                        span: let_.span.clone(),
                    });
                }

                self.set(let_.name.clone(), type_);
            }
            Statement::Assign(assign) => {
                let inferred = self.infer_expr(&assign.value);
                self.set(assign.name.clone(), inferred);
            }
            Statement::HQLQuery(hql) => {
                let _target_type = self.infer_expr(&hql.target);
                self.set(
                    hql.variable.clone(),
                    HKLType::Array(Box::new(HKLType::Unknown)),
                );
            }
            Statement::If(if_) => {
                let cond_type = self.infer_expr(&if_.condition);
                if !matches!(cond_type, HKLType::Bool | HKLType::Unknown) {
                    // allow non-bool truthiness for mock language
                }

                let mut then_child = self.child();
                for stmt in &if_.then.statements {
                    then_child.check_statement(stmt);
                }
                // merge bindings that assignments create — keep simple: just collect errors
                self.errors.extend(then_child.errors);

                if let Some(else_block) = &if_.else_block {
                    let mut else_child = self.child();
                    for stmt in &else_block.statements {
                        else_child.check_statement(stmt);
                    }
                    self.errors.extend(else_child.errors);
                }
            }
            Statement::For(for_) => {
                let iter_type = self.infer_expr(&for_.iterable);
                let elem_type = match iter_type {
                    HKLType::Array(elem) => *elem,
                    _ => HKLType::Unknown,
                };

                let mut child = self.child();
                child.set(for_.variable.clone(), elem_type);
                for stmt in &for_.body.statements {
                    child.check_statement(stmt);
                }
                self.errors.extend(child.errors);
            }
            Statement::While(while_) => {
                let _cond_type = self.infer_expr(&while_.condition);
                let mut child = self.child();
                for stmt in &while_.body.statements {
                    child.check_statement(stmt);
                }
                self.errors.extend(child.errors);
            }
            Statement::Emit(emit) => {
                self.infer_expr(&emit.value);
            }
            Statement::Store(_) => {}
            Statement::Notify(notify) => {
                self.infer_expr(&notify.message);
                if let Some(condition) = &notify.condition {
                    self.infer_expr(condition);
                }
            }
            Statement::Expr(expr) => {
                self.infer_expr(&expr.expr);
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.infer_expr(value);
                }
            }
            Statement::Input(spec) => {
                let type_ = self.resolve_type_ref(&spec.type_ref);
                self.set(spec.param.clone(), type_);
            }
            Statement::Session(_) => {
                self.set("session".into(), HKLType::String_);
            }
        }
    }

    fn infer_expr(&mut self, expr: &Expr) -> HKLType {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => HKLType::Int { width: Some(64) },
                Literal::Float(_) => HKLType::Float,
                Literal::String(_) => HKLType::String_,
                Literal::Bool(_) => HKLType::Bool,
                Literal::Hex(_) | Literal::Address(_) => HKLType::Address,
                Literal::Array(elems) => {
                    if let Some(first) = elems.first() {
                        let elem_type = self.infer_expr(first);
                        HKLType::Array(Box::new(elem_type))
                    } else {
                        HKLType::Array(Box::new(HKLType::Unknown))
                    }
                }
                Literal::Map(entries) => {
                    if let Some((_, val)) = entries.first() {
                        let val_type = self.infer_expr(val);
                        HKLType::Map(Box::new(HKLType::String_), Box::new(val_type))
                    } else {
                        HKLType::Map(Box::new(HKLType::String_), Box::new(HKLType::Unknown))
                    }
                }
            },
            Expr::Ident(name) => self.get(name).unwrap_or(HKLType::Unknown),
            Expr::Binary(bin) => {
                let left = self.infer_expr(&bin.left);
                let right = self.infer_expr(&bin.right);

                match bin.op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        if left == HKLType::Float || right == HKLType::Float {
                            HKLType::Float
                        } else if left == HKLType::String_ || right == HKLType::String_ {
                            HKLType::String_
                        } else {
                            HKLType::Int { width: Some(64) }
                        }
                    }
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Gt
                    | BinaryOp::Le
                    | BinaryOp::Ge
                    | BinaryOp::And
                    | BinaryOp::Or => HKLType::Bool,
                    BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => HKLType::Int { width: Some(64) },
                }
            }
            Expr::Unary(unary) => {
                let operand = self.infer_expr(&unary.operand);
                match unary.op {
                    UnaryOp::Neg | UnaryOp::BitNot => operand,
                    UnaryOp::Not => HKLType::Bool,
                }
            }
            Expr::Pipe(pipe) => {
                let _left = self.infer_expr(&pipe.left);
                self.infer_expr(&pipe.right)
            }
            Expr::Call(call) => {
                if let Expr::Ident(name) = call.callee.as_ref() {
                    if let Some(sig) = self.builtins.get(name) {
                        return sig.returns.clone();
                    }
                }
                if let Expr::Member(m) = call.callee.as_ref() {
                    if let Expr::Ident(module) = m.object.as_ref() {
                        let full = format!("{}.{}", module, m.member);
                        if let Some(sig) = self.builtins.get(&full) {
                            return sig.returns.clone();
                        }
                    }
                    if m.member == "any" || m.member == "high_confidence" {
                        return HKLType::Bool;
                    }
                }
                HKLType::Unknown
            }
            Expr::Member(member) => {
                let _object_type = self.infer_expr(&member.object);
                if member.member == "any" || member.member == "high_confidence" {
                    HKLType::Bool
                } else {
                    HKLType::Unknown
                }
            }
            Expr::Index(index) => {
                let object_type = self.infer_expr(&index.object);
                match object_type {
                    HKLType::Array(elem) => *elem,
                    HKLType::Map(_, val) => *val,
                    _ => HKLType::Unknown,
                }
            }
            Expr::Match(_) => HKLType::Unknown,
            Expr::Lambda(_) => HKLType::Unknown,
            Expr::Block(block) => {
                let mut child = self.child();
                for stmt in &block.statements {
                    child.check_statement(stmt);
                }

                let result_type = if let Some(result) = &block.result {
                    child.infer_expr(result)
                } else {
                    HKLType::Void
                };
                self.errors.extend(child.errors);
                result_type
            }
            Expr::TypeCast(cast) => {
                self.infer_expr(&cast.expr);
                self.resolve_type_ref(&cast.target_type)
            }
            Expr::Ternary(ternary) => {
                self.infer_expr(&ternary.condition);
                let then_type = self.infer_expr(&ternary.then_branch);
                let else_type = self.infer_expr(&ternary.else_branch);
                if self.types_compatible(&then_type, &else_type) {
                    then_type
                } else {
                    HKLType::Unknown
                }
            }
            Expr::Address(_) => HKLType::Address,
            Expr::Range(_) => HKLType::Range,
            Expr::PipelineRef(_) => HKLType::Unknown,
            Expr::Filter(filter) => {
                let collection = self.infer_expr(&filter.collection);
                self.infer_expr(&filter.predicate);
                match collection {
                    HKLType::Array(_) => collection,
                    _ => HKLType::Array(Box::new(HKLType::Unknown)),
                }
            }
        }
    }

    fn resolve_type_ref(&self, type_ref: &TypeRef) -> HKLType {
        match type_ref {
            TypeRef::Simple(name) => match name.as_str() {
                "Binary" => HKLType::Binary { format: None },
                "PE64" | "PE32" | "ELF64" | "ELF32" | "MachO" => HKLType::Binary { format: None },
                "Function" => HKLType::Function,
                "BasicBlock" => HKLType::BasicBlock,
                "IRNode" => HKLType::IRNode,
                "EmuSnapshot" => HKLType::EmuSnapshot,
                "IOC" => HKLType::IOC,
                "Pattern" => {
                    HKLType::Pattern {
                        pattern_type: super::core::PatternType::HQL,
                    }
                }
                "string" | "String" => HKLType::String_,
                "int" | "Int" => HKLType::Int { width: Some(64) },
                "float" | "Float" => HKLType::Float,
                "bool" | "Bool" => HKLType::Bool,
                "void" | "Void" => HKLType::Void,
                _ => HKLType::Unknown,
            },
            TypeRef::Union(variants) => {
                if let Some(first) = variants.first() {
                    self.resolve_type_ref(first)
                } else {
                    HKLType::Unknown
                }
            }
            TypeRef::Generic(name, params) => match name.as_str() {
                "Array" => {
                    if let Some(param) = params.first() {
                        HKLType::Array(Box::new(self.resolve_type_ref(param)))
                    } else {
                        HKLType::Array(Box::new(HKLType::Unknown))
                    }
                }
                "Map" => {
                    if params.len() >= 2 {
                        HKLType::Map(
                            Box::new(self.resolve_type_ref(&params[0])),
                            Box::new(self.resolve_type_ref(&params[1])),
                        )
                    } else {
                        HKLType::Map(Box::new(HKLType::Unknown), Box::new(HKLType::Unknown))
                    }
                }
                _ => HKLType::Unknown,
            },
            TypeRef::Pipeline(params) => HKLType::Pipeline {
                stages: params.iter().map(|p| self.resolve_type_ref(p)).collect(),
            },
        }
    }

    fn types_compatible(&self, expected: &HKLType, actual: &HKLType) -> bool {
        if expected == actual {
            return true;
        }

        match (expected, actual) {
            (HKLType::Unknown, _) | (_, HKLType::Unknown) => true,
            (HKLType::Binary { format: None }, HKLType::Binary { .. }) => true,
            (HKLType::Binary { .. }, HKLType::Binary { format: None }) => true,
            (HKLType::Array(a), HKLType::Array(b)) => self.types_compatible(a, b),
            (HKLType::Map(k1, v1), HKLType::Map(k2, v2)) => {
                self.types_compatible(k1, k2) && self.types_compatible(v1, v2)
            }
            _ => false,
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse_program;

    #[test]
    fn test_let_binding_type_check() {
        let source = r#"
            pipeline Test {
                input binary: Binary

                let x = 42;
            }
        "#;
        let tokens = tokenize(source).unwrap();
        let program = parse_program(&tokens).unwrap();

        let mut checker = TypeChecker::new();
        let result = checker.check_program(&program);
        assert!(result.is_ok());
    }
}
