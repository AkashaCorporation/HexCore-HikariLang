use super::ast::*;
use super::expr::ExprParser;
use crate::error::{HKLError, Span};
use crate::lexer::{Keyword, Token};

pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, HKLError> {
        let mut declarations = Vec::new();
        while self.pos < self.tokens.len() {
            declarations.push(self.parse_top_level()?);
        }
        Ok(Program {
            declarations,
            span: 0..0,
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<(), HKLError> {
        match self.peek() {
            Some(t) if t == expected => {
                self.pos += 1;
                Ok(())
            }
            Some(t) => Err(parse_err(
                &format!("Expected {}, got {}", expected, t),
                self.pos..self.pos + 1,
            )),
            None => Err(parse_err(
                &format!("Expected {}, got EOF", expected),
                self.pos..self.pos,
            )),
        }
    }

    fn expect_ident(&mut self) -> Result<String, HKLError> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s.clone()),
            // Soft keywords may appear as names (e.g. `input binary: Binary`)
            Some(Token::Keyword(k)) => Ok(keyword_name(k)),
            Some(t) => Err(parse_err(
                &format!("Expected identifier, got {}", t),
                self.pos.saturating_sub(1)..self.pos,
            )),
            None => Err(parse_err(
                "Expected identifier, got EOF",
                self.pos..self.pos,
            )),
        }
    }

    fn parse_top_level(&mut self) -> Result<TopLevelDecl, HKLError> {
        match self.peek() {
            Some(Token::Keyword(Keyword::Pipeline)) => {
                Ok(TopLevelDecl::Pipeline(self.parse_pipeline()?))
            }
            Some(Token::Keyword(Keyword::Fn)) => Ok(TopLevelDecl::Function(self.parse_function()?)),
            Some(t) => Err(parse_err(
                &format!("Expected pipeline or fn, got {}", t),
                self.pos..self.pos + 1,
            )),
            None => Err(parse_err("Unexpected EOF", self.pos..self.pos)),
        }
    }

    pub fn parse_pipeline(&mut self) -> Result<PipelineDecl, HKLError> {
        self.expect(&Token::Keyword(Keyword::Pipeline))?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut inputs = Vec::new();
        let mut session = None;
        let mut stages = Vec::new();

        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            match self.peek() {
                Some(Token::Keyword(Keyword::Input)) => {
                    self.advance();
                    let param = self.expect_ident()?;
                    self.expect(&Token::Colon)?;
                    let type_ref = self.parse_type_ref()?;
                    // optional trailing semicolon
                    if matches!(self.peek(), Some(Token::Semicolon)) {
                        self.advance();
                    }
                    inputs.push(InputSpec { param, type_ref });
                }
                Some(Token::Keyword(Keyword::Session)) => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    match self.advance() {
                        Some(Token::StringLit(s)) => {
                            session = Some(s.clone());
                        }
                        Some(t) => {
                            return Err(parse_err(
                                &format!("Expected session string, got {}", t),
                                self.pos.saturating_sub(1)..self.pos,
                            ));
                        }
                        None => {
                            return Err(parse_err(
                                "Expected session string, got EOF",
                                self.pos..self.pos,
                            ));
                        }
                    }
                    if matches!(self.peek(), Some(Token::Semicolon)) {
                        self.advance();
                    }
                }
                _ => {
                    stages.push(self.parse_statement()?);
                }
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(PipelineDecl {
            name,
            input: inputs,
            session,
            body: PipelineBody { stages },
            span: 0..0,
        })
    }

    pub fn parse_function(&mut self) -> Result<FunctionDecl, HKLError> {
        self.expect(&Token::Keyword(Keyword::Fn))?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;

        let mut params = Vec::new();
        if !matches!(self.peek(), Some(Token::RParen)) {
            loop {
                let pname = self.expect_ident()?;
                let type_ref = if matches!(self.peek(), Some(Token::Colon)) {
                    self.advance();
                    Some(self.parse_type_ref()?)
                } else {
                    None
                };
                params.push(Param {
                    name: pname,
                    type_ref,
                });
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.expect(&Token::RParen)?;

        let return_type = if matches!(self.peek(), Some(Token::ThinArrow)) {
            self.advance();
            Some(self.parse_type_ref()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(FunctionDecl {
            name,
            params,
            return_type,
            body,
            span: 0..0,
        })
    }

    fn parse_type_ref(&mut self) -> Result<TypeRef, HKLError> {
        let first = self.parse_simple_type()?;
        if matches!(self.peek(), Some(Token::PipeOp)) {
            let mut variants = vec![first];
            while matches!(self.peek(), Some(Token::PipeOp)) {
                self.advance();
                variants.push(self.parse_simple_type()?);
            }
            Ok(TypeRef::Union(variants))
        } else {
            Ok(first)
        }
    }

    fn parse_simple_type(&mut self) -> Result<TypeRef, HKLError> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(TypeRef::Simple(s.clone())),
            Some(Token::Keyword(Keyword::Binary)) => Ok(TypeRef::Simple("Binary".into())),
            Some(Token::Keyword(Keyword::Function)) => Ok(TypeRef::Simple("Function".into())),
            Some(Token::Keyword(Keyword::BasicBlock)) => Ok(TypeRef::Simple("BasicBlock".into())),
            Some(Token::Keyword(Keyword::Pattern)) => Ok(TypeRef::Simple("Pattern".into())),
            Some(Token::Keyword(Keyword::Ioc)) => Ok(TypeRef::Simple("IOC".into())),
            // Allow any soft keyword as a type name (e.g. custom aliases)
            Some(Token::Keyword(k)) => {
                let mut name = keyword_name(k);
                // Capitalize common type keywords for typechecker resolution
                if name == "binary" {
                    name = "Binary".into();
                }
                Ok(TypeRef::Simple(name))
            }
            Some(t) => Err(parse_err(
                &format!("Expected type name, got {}", t),
                self.pos.saturating_sub(1)..self.pos,
            )),
            None => Err(parse_err("Expected type name, got EOF", self.pos..self.pos)),
        }
    }

    fn parse_block(&mut self) -> Result<StatementBlock, HKLError> {
        self.expect(&Token::LBrace)?;
        let mut statements = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            statements.push(self.parse_statement()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(StatementBlock {
            statements,
            span: 0..0,
        })
    }

    pub fn parse_statement(&mut self) -> Result<Statement, HKLError> {
        match self.peek() {
            Some(Token::Keyword(Keyword::Let)) => self.parse_let(),
            Some(Token::Keyword(Keyword::If)) => self.parse_if(),
            Some(Token::Keyword(Keyword::For)) => self.parse_for(),
            Some(Token::Keyword(Keyword::While)) => self.parse_while(),
            Some(Token::Keyword(Keyword::Emit)) => self.parse_emit(),
            Some(Token::Keyword(Keyword::Store)) => self.parse_store(),
            Some(Token::Keyword(Keyword::Notify)) => self.parse_notify(),
            Some(Token::Keyword(Keyword::Return)) => self.parse_return(),
            // HQL: matches = hql """...""" on expr;
            Some(Token::Ident(_)) => {
                // Peek ahead for assignment / hql
                if self.pos + 1 < self.tokens.len()
                    && matches!(self.tokens[self.pos + 1], Token::Assign)
                {
                    self.parse_assign_or_hql()
                } else {
                    self.parse_expr_stmt()
                }
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_let(&mut self) -> Result<Statement, HKLError> {
        self.expect(&Token::Keyword(Keyword::Let))?;
        let name = self.expect_ident()?;
        let type_ref = if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            Some(self.parse_type_ref()?)
        } else {
            None
        };
        self.expect(&Token::Assign)?;
        let value = self.parse_expr()?;
        self.optional_semicolon();
        Ok(Statement::LetBinding(LetBinding {
            name,
            type_ref,
            value,
            span: 0..0,
        }))
    }

    fn parse_assign_or_hql(&mut self) -> Result<Statement, HKLError> {
        let name = self.expect_ident()?;
        self.expect(&Token::Assign)?;

        // hql """...""" on expr
        if matches!(self.peek(), Some(Token::HQLBlock(_))) {
            let content = match self.advance() {
                Some(Token::HQLBlock(c)) => c.clone(),
                _ => unreachable!(),
            };
            self.expect(&Token::Keyword(Keyword::On))?;
            let target = self.parse_expr()?;
            self.optional_semicolon();
            return Ok(Statement::HQLQuery(HQLQueryStatement {
                variable: name,
                query: HQLQueryBlock {
                    content,
                    span: 0..0,
                },
                target,
                span: 0..0,
            }));
        }

        // Bare "hql" shouldn't appear as keyword — HQLBlock is one token.
        // Also handle: name = expr
        let value = self.parse_expr()?;
        self.optional_semicolon();
        Ok(Statement::Assign(AssignStatement {
            name,
            value,
            span: 0..0,
        }))
    }

    fn parse_if(&mut self) -> Result<Statement, HKLError> {
        self.expect(&Token::Keyword(Keyword::If))?;
        let condition = self.parse_expr()?;
        let then = self.parse_block()?;
        let else_block = if matches!(self.peek(), Some(Token::Keyword(Keyword::Else))) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };
        self.optional_semicolon();
        Ok(Statement::If(IfStatement {
            condition,
            then,
            else_block,
            span: 0..0,
        }))
    }

    fn parse_for(&mut self) -> Result<Statement, HKLError> {
        self.expect(&Token::Keyword(Keyword::For))?;
        let variable = self.expect_ident()?;
        self.expect(&Token::Keyword(Keyword::In))?;
        let iterable = self.parse_expr()?;
        let body = self.parse_block()?;
        self.optional_semicolon();
        Ok(Statement::For(ForStatement {
            variable,
            iterable,
            body,
            span: 0..0,
        }))
    }

    fn parse_while(&mut self) -> Result<Statement, HKLError> {
        self.expect(&Token::Keyword(Keyword::While))?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        self.optional_semicolon();
        Ok(Statement::While(WhileStatement {
            condition,
            body,
            span: 0..0,
        }))
    }

    fn parse_emit(&mut self) -> Result<Statement, HKLError> {
        self.expect(&Token::Keyword(Keyword::Emit))?;
        let value = self.parse_expr()?;
        self.optional_semicolon();
        Ok(Statement::Emit(EmitStatement { value, span: 0..0 }))
    }

    fn parse_store(&mut self) -> Result<Statement, HKLError> {
        self.expect(&Token::Keyword(Keyword::Store))?;
        let target = match self.peek() {
            Some(Token::Ident(s)) => {
                let s = s.clone();
                self.advance();
                Some(s)
            }
            Some(Token::Keyword(Keyword::Session)) => {
                self.advance();
                Some("session".into())
            }
            _ => None,
        };
        self.optional_semicolon();
        Ok(Statement::Store(StoreStatement { target, span: 0..0 }))
    }

    fn parse_notify(&mut self) -> Result<Statement, HKLError> {
        self.expect(&Token::Keyword(Keyword::Notify))?;
        let message = self.parse_expr()?;
        let condition = if matches!(self.peek(), Some(Token::Keyword(Keyword::If))) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.optional_semicolon();
        Ok(Statement::Notify(NotifyStatement {
            message,
            condition,
            span: 0..0,
        }))
    }

    fn parse_return(&mut self) -> Result<Statement, HKLError> {
        self.expect(&Token::Keyword(Keyword::Return))?;
        let value = if matches!(
            self.peek(),
            Some(Token::Semicolon) | Some(Token::RBrace) | None
        ) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.optional_semicolon();
        Ok(Statement::Return(ReturnStatement { value, span: 0..0 }))
    }

    fn parse_expr_stmt(&mut self) -> Result<Statement, HKLError> {
        let expr = self.parse_expr()?;
        self.optional_semicolon();
        Ok(Statement::Expr(ExprStatement { expr, span: 0..0 }))
    }

    fn parse_expr(&mut self) -> Result<Expr, HKLError> {
        let mut ep = ExprParser::new(self.tokens);
        ep.set_position(self.pos);
        let expr = ep.parse_expression()?;
        self.pos = ep.position();
        Ok(expr)
    }

    fn optional_semicolon(&mut self) {
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
    }
}

fn parse_err(message: &str, span: Span) -> HKLError {
    HKLError::Parser {
        message: message.to_string(),
        span,
    }
}

fn keyword_name(k: &Keyword) -> String {
    match k {
        Keyword::Pipeline => "pipeline",
        Keyword::Let => "let",
        Keyword::Fn => "fn",
        Keyword::If => "if",
        Keyword::Else => "else",
        Keyword::Match => "match",
        Keyword::For => "for",
        Keyword::While => "while",
        Keyword::Return => "return",
        Keyword::Input => "input",
        Keyword::Output => "output",
        Keyword::Stage => "stage",
        Keyword::Transform => "transform",
        Keyword::Parallel => "parallel",
        Keyword::On => "on",
        Keyword::Where => "where",
        Keyword::And => "and",
        Keyword::Or => "or",
        Keyword::Not => "not",
        Keyword::True => "true",
        Keyword::False => "false",
        Keyword::Store => "store",
        Keyword::Notify => "notify",
        Keyword::Export => "export",
        Keyword::Import => "import",
        Keyword::Use => "use",
        Keyword::Oracle => "oracle",
        Keyword::Emulate => "emulate",
        Keyword::Detect => "detect",
        Keyword::Filter => "filter",
        Keyword::Emit => "emit",
        Keyword::In => "in",
        Keyword::Binary => "binary",
        Keyword::Function => "function",
        Keyword::BasicBlock => "basicblock",
        Keyword::Pattern => "pattern",
        Keyword::Ioc => "ioc",
        Keyword::Session => "session",
        Keyword::Hook => "hook",
        Keyword::Timeout => "timeout",
        Keyword::Stalker => "stalker",
        Keyword::Severity => "severity",
        Keyword::Mitre => "mitre",
        Keyword::Confidence => "confidence",
    }
    .to_string()
}

/// Parse a full program from tokens.
pub fn parse_program(tokens: &[Token]) -> Result<Program, HKLError> {
    let mut p = Parser::new(tokens);
    p.parse_program()
}

/// Compatibility wrapper used by the old chumsky-style API.
pub fn program_parser() -> ProgramParser {
    ProgramParser
}

pub struct ProgramParser;

impl ProgramParser {
    pub fn parse(&self, tokens: Vec<Token>) -> Result<Program, HKLError> {
        parse_program(&tokens)
    }
}

pub fn pipeline_parser() -> PipelineParser {
    PipelineParser
}

pub struct PipelineParser;

impl PipelineParser {
    pub fn parse(&self, tokens: Vec<Token>) -> Result<PipelineDecl, HKLError> {
        let mut p = Parser::new(&tokens);
        p.parse_pipeline()
    }
}

pub fn function_parser() -> FunctionParser {
    FunctionParser
}

pub struct FunctionParser;

impl FunctionParser {
    pub fn parse(&self, tokens: Vec<Token>) -> Result<FunctionDecl, HKLError> {
        let mut p = Parser::new(&tokens);
        p.parse_function()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_let_binding() {
        let tokens = tokenize("let x = 42;").unwrap();
        let mut p = Parser::new(&tokens);
        assert!(p.parse_statement().is_ok());
    }

    #[test]
    fn test_simple_pipeline() {
        let source = r#"
            pipeline QuickTriage {
                input binary: Binary

                let strings = extract_strings(binary);
            }
        "#;
        let tokens = tokenize(source).unwrap();
        let program = parse_program(&tokens).unwrap();
        assert_eq!(program.declarations.len(), 1);
        match &program.declarations[0] {
            TopLevelDecl::Pipeline(p) => {
                assert_eq!(p.name, "QuickTriage");
                assert_eq!(p.input.len(), 1);
                assert_eq!(p.input[0].param, "binary");
            }
            _ => panic!("expected pipeline"),
        }
    }

    #[test]
    fn test_fixture_simple_pipe() {
        let source = include_str!("../../tests/fixtures/simple_pipe.hkl");
        let tokens = tokenize(source).unwrap();
        let program = parse_program(&tokens);
        assert!(program.is_ok(), "{:?}", program.err());
    }
}
