use crate::error::Span;

// ─── Top-Level ───

#[derive(Debug, Clone)]
pub struct Program {
    pub declarations: Vec<TopLevelDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TopLevelDecl {
    Pipeline(PipelineDecl),
    Function(FunctionDecl),
    Import(ImportDecl),
    Const(ConstDecl),
}

// ─── Pipeline Declaration ───

#[derive(Debug, Clone)]
pub struct PipelineDecl {
    pub name: String,
    pub input: Vec<InputSpec>,
    pub session: Option<String>,
    pub body: PipelineBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InputSpec {
    pub param: String,
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone)]
pub struct PipelineBody {
    pub stages: Vec<Statement>,
}

// ─── Function Declaration ───

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: StatementBlock,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ref: Option<TypeRef>,
}

#[derive(Debug, Clone)]
pub struct StatementBlock {
    pub statements: Vec<Statement>,
    pub span: Span,
}

// ─── Import / Const ───

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub type_ref: Option<TypeRef>,
    pub value: Expr,
    pub span: Span,
}

// ─── Statements ───

#[derive(Debug, Clone)]
pub enum Statement {
    LetBinding(LetBinding),
    Assign(AssignStatement),
    HQLQuery(HQLQueryStatement),
    If(IfStatement),
    For(ForStatement),
    While(WhileStatement),
    Emit(EmitStatement),
    Store(StoreStatement),
    Notify(NotifyStatement),
    Expr(ExprStatement),
    Return(ReturnStatement),
    /// `input name: Type` inside pipeline body
    Input(InputSpec),
    /// `session: "name"` inside pipeline body
    Session(String),
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub type_ref: Option<TypeRef>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssignStatement {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HQLQueryStatement {
    pub variable: String,
    pub query: HQLQueryBlock,
    pub target: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HQLQueryBlock {
    pub content: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub condition: Expr,
    pub then: StatementBlock,
    pub else_block: Option<StatementBlock>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForStatement {
    pub variable: String,
    pub iterable: Expr,
    pub body: StatementBlock,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub condition: Expr,
    pub body: StatementBlock,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EmitStatement {
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StoreStatement {
    pub target: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct NotifyStatement {
    pub message: Expr,
    pub condition: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExprStatement {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub value: Option<Expr>,
    pub span: Span,
}

// ─── Expressions ───

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Pipe(PipeExpr),
    Call(CallExpr),
    Member(MemberExpr),
    Index(IndexExpr),
    Match(MatchExpr),
    Lambda(LambdaExpr),
    Block(BlockExpr),
    TypeCast(TypeCastExpr),
    Ternary(TernaryExpr),
    Address(String),
    Range(RangeExpr),
    PipelineRef(String),
    /// `filter <expr> where <expr>`
    Filter(FilterExpr),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Hex(String),
    Address(String),
    Array(Vec<Expr>),
    /// Object/map literal. Entries may be shorthand (`foo` → `"foo": foo`) or `key: value`.
    Map(Vec<(Expr, Expr)>),
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone)]
pub struct PipeExpr {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
    pub named_args: Vec<(String, Expr)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MemberExpr {
    pub object: Box<Expr>,
    pub member: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub object: Box<Expr>,
    pub index: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchExpr {
    pub scrutinee: Box<Expr>,
    pub cases: Vec<MatchCase>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Literal),
    Ident(String),
    Wildcard,
    Array(Vec<Pattern>),
    Rest(Option<String>),
}

#[derive(Debug, Clone)]
pub struct LambdaExpr {
    pub params: Vec<String>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BlockExpr {
    pub statements: Vec<Statement>,
    pub result: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeCastExpr {
    pub expr: Box<Expr>,
    pub target_type: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TernaryExpr {
    pub condition: Box<Expr>,
    pub then_branch: Box<Expr>,
    pub else_branch: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct RangeExpr {
    pub start: Box<Expr>,
    pub end: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FilterExpr {
    pub collection: Box<Expr>,
    pub predicate: Box<Expr>,
    pub span: Span,
}

// ─── Types ───

#[derive(Debug, Clone)]
pub enum TypeRef {
    Simple(String),
    Union(Vec<TypeRef>),
    Generic(String, Vec<TypeRef>),
    Pipeline(Vec<TypeRef>),
}
