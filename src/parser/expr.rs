use super::ast::*;
use crate::error::{HKLError, Span};
use crate::lexer::{Keyword, Token};

type ArgList = (Vec<Expr>, Vec<(String, Expr)>);

/// Recursive-descent expression parser over a token slice.
pub struct ExprParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> ExprParser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        ExprParser { tokens, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn parse_expression(&mut self) -> Result<Expr, HKLError> {
        self.parse_pipe()
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

    fn parse_pipe(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_or()?;
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            let right = self.parse_or()?;
            left = Expr::Pipe(PipeExpr {
                left: Box::new(left),
                right: Box::new(right),
                span: 0..0,
            });
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_and()?;
        while matches!(
            self.peek(),
            Some(Token::PipePipe) | Some(Token::Keyword(Keyword::Or))
        ) {
            self.advance();
            let right = self.parse_and()?;
            left = bin(BinaryOp::Or, left, right);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_equality()?;
        while matches!(
            self.peek(),
            Some(Token::AmpAmp) | Some(Token::Keyword(Keyword::And))
        ) {
            self.advance();
            let right = self.parse_equality()?;
            left = bin(BinaryOp::And, left, right);
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Some(Token::Eq) => BinaryOp::Eq,
                Some(Token::Ne) => BinaryOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = bin(op, left, right);
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_bitor()?;
        loop {
            let op = match self.peek() {
                Some(Token::Lt) => BinaryOp::Lt,
                Some(Token::Gt) => BinaryOp::Gt,
                Some(Token::Le) => BinaryOp::Le,
                Some(Token::Ge) => BinaryOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_bitor()?;
            left = bin(op, left, right);
        }
        Ok(left)
    }

    fn parse_bitor(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_bitxor()?;
        while matches!(self.peek(), Some(Token::PipeOp)) {
            // Don't treat `|` used as type-union terminator context — expressions only.
            self.advance();
            let right = self.parse_bitxor()?;
            left = bin(BinaryOp::BitOr, left, right);
        }
        Ok(left)
    }

    fn parse_bitxor(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_bitand()?;
        while matches!(self.peek(), Some(Token::Caret)) {
            self.advance();
            let right = self.parse_bitand()?;
            left = bin(BinaryOp::BitXor, left, right);
        }
        Ok(left)
    }

    fn parse_bitand(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_term()?;
        while matches!(self.peek(), Some(Token::Amp)) {
            self.advance();
            let right = self.parse_term()?;
            left = bin(BinaryOp::BitAnd, left, right);
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinaryOp::Add,
                Some(Token::Minus) => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            left = bin(op, left, right);
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, HKLError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinaryOp::Mul,
                Some(Token::Slash) => BinaryOp::Div,
                Some(Token::Percent) => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = bin(op, left, right);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, HKLError> {
        match self.peek() {
            Some(Token::Minus) => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span: 0..0,
                }))
            }
            Some(Token::Bang) | Some(Token::Keyword(Keyword::Not)) => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span: 0..0,
                }))
            }
            Some(Token::Caret) => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                    span: 0..0,
                }))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, HKLError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Some(Token::LParen) => {
                    self.advance();
                    let (args, named_args) = self.parse_arg_list()?;
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call(CallExpr {
                        callee: Box::new(expr),
                        args,
                        named_args,
                        span: 0..0,
                    });
                }
                // Call-with-block: generate_report { format: "Markdown", ... }
                Some(Token::LBrace) if matches!(expr, Expr::Ident(_) | Expr::Member(_)) => {
                    let map = self.parse_map_or_object()?;
                    expr = Expr::Call(CallExpr {
                        callee: Box::new(expr),
                        args: vec![map],
                        named_args: Vec::new(),
                        span: 0..0,
                    });
                }
                Some(Token::Dot) => {
                    self.advance();
                    let member = match self.advance() {
                        Some(Token::Ident(s)) => s.clone(),
                        Some(Token::Keyword(k)) => format!("{:?}", k).to_lowercase(),
                        other => {
                            return Err(parse_err(
                                &format!("Expected member name, got {:?}", other),
                                self.pos..self.pos,
                            ));
                        }
                    };
                    expr = Expr::Member(MemberExpr {
                        object: Box::new(expr),
                        member,
                        span: 0..0,
                    });
                }
                Some(Token::LBracket) => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index(IndexExpr {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span: 0..0,
                    });
                }
                Some(Token::DotDot) => {
                    self.advance();
                    let end = self.parse_primary()?;
                    expr = Expr::Range(RangeExpr {
                        start: Box::new(expr),
                        end: Box::new(end),
                        span: 0..0,
                    });
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_arg_list(&mut self) -> Result<ArgList, HKLError> {
        let mut args = Vec::new();
        let mut named_args = Vec::new();

        if matches!(self.peek(), Some(Token::RParen)) {
            return Ok((args, named_args));
        }

        loop {
            // named: ident : expr  (but not bare type-ish — only when next is colon after ident)
            if let Some(Token::Ident(name)) = self.peek().cloned() {
                let save = self.pos;
                self.advance();
                if matches!(self.peek(), Some(Token::Colon)) {
                    self.advance();
                    let value = self.parse_expression()?;
                    named_args.push((name, value));
                } else {
                    self.pos = save;
                    args.push(self.parse_expression()?);
                }
            } else if matches!(self.peek(), Some(Token::Keyword(Keyword::Timeout)))
                || matches!(self.peek(), Some(Token::Keyword(Keyword::Stalker)))
                || matches!(self.peek(), Some(Token::Keyword(Keyword::Confidence)))
                || matches!(self.peek(), Some(Token::Keyword(Keyword::Hook)))
            {
                // allow keyword-named args like timeout: 45
                let name = match self.advance() {
                    Some(Token::Keyword(Keyword::Timeout)) => "timeout".to_string(),
                    Some(Token::Keyword(Keyword::Stalker)) => "stalker".to_string(),
                    Some(Token::Keyword(Keyword::Confidence)) => "confidence".to_string(),
                    Some(Token::Keyword(Keyword::Hook)) => "hooks".to_string(),
                    _ => unreachable!(),
                };
                // hooks keyword maps from Hook; fixtures use hooks:
                let name = if name == "hooks" {
                    // If the source said `hooks:` it was Ident, not keyword.
                    // Keep as hooks for Hook keyword fallback.
                    "hooks".to_string()
                } else {
                    name
                };
                self.expect(&Token::Colon)?;
                let value = self.parse_expression()?;
                named_args.push((name, value));
            } else {
                // hooks is an Ident in fixtures
                args.push(self.parse_expression()?);
            }

            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
                if matches!(self.peek(), Some(Token::RParen)) {
                    break; // trailing comma
                }
                continue;
            }
            break;
        }

        Ok((args, named_args))
    }

    fn parse_primary(&mut self) -> Result<Expr, HKLError> {
        // filter collection where predicate
        if matches!(self.peek(), Some(Token::Keyword(Keyword::Filter))) {
            self.advance();
            let collection = self.parse_expression()?;
            if matches!(self.peek(), Some(Token::Keyword(Keyword::Where))) {
                self.advance();
                let predicate = self.parse_expression()?;
                return Ok(Expr::Filter(FilterExpr {
                    collection: Box::new(collection),
                    predicate: Box::new(predicate),
                    span: 0..0,
                }));
            }
            // bare filter(...) call style fallback — treat as ident
            return Ok(Expr::Ident("filter".into()));
        }

        match self.peek().cloned() {
            Some(Token::IntLit(n)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Int(n)))
            }
            Some(Token::FloatLit(n)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Float(n)))
            }
            Some(Token::StringLit(s)) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(s)))
            }
            Some(Token::BoolLit(b)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(b)))
            }
            Some(Token::HexLit(h)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Hex(h)))
            }
            Some(Token::AddressLit(a)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Address(a)))
            }
            Some(Token::Ident(s)) => {
                self.advance();
                Ok(Expr::Ident(s))
            }
            Some(Token::Keyword(Keyword::True)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            Some(Token::Keyword(Keyword::False)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            // Soft keywords usable as identifiers in expression position
            Some(Token::Keyword(Keyword::Binary)) => {
                self.advance();
                Ok(Expr::Ident("binary".into()))
            }
            Some(Token::Keyword(Keyword::Session)) => {
                self.advance();
                Ok(Expr::Ident("session".into()))
            }
            Some(Token::Keyword(Keyword::Timeout)) => {
                self.advance();
                Ok(Expr::Ident("timeout".into()))
            }
            Some(Token::Keyword(Keyword::Stalker)) => {
                self.advance();
                Ok(Expr::Ident("stalker".into()))
            }
            Some(Token::Keyword(Keyword::Confidence)) => {
                self.advance();
                Ok(Expr::Ident("confidence".into()))
            }
            Some(Token::Keyword(Keyword::Ioc)) => {
                self.advance();
                Ok(Expr::Ident("ioc".into()))
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Some(Token::LBracket) => {
                self.advance();
                let mut elems = Vec::new();
                if !matches!(self.peek(), Some(Token::RBracket)) {
                    loop {
                        elems.push(self.parse_expression()?);
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                            if matches!(self.peek(), Some(Token::RBracket)) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::Literal(Literal::Array(elems)))
            }
            Some(Token::LBrace) => self.parse_map_or_object(),
            Some(other) => Err(parse_err(
                &format!("Unexpected token in expression: {}", other),
                self.pos..self.pos + 1,
            )),
            None => Err(parse_err(
                "Unexpected EOF in expression",
                self.pos..self.pos,
            )),
        }
    }

    /// `{ key: value, shorthand, ... }`
    fn parse_map_or_object(&mut self) -> Result<Expr, HKLError> {
        self.expect(&Token::LBrace)?;
        let mut entries = Vec::new();

        if !matches!(self.peek(), Some(Token::RBrace)) {
            loop {
                // shorthand: ident  OR  key: value
                match self.peek().cloned() {
                    Some(Token::Ident(name)) => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Colon)) {
                            self.advance();
                            let value = self.parse_expression()?;
                            entries.push((Expr::Literal(Literal::String(name)), value));
                        } else {
                            // shorthand { strings } → "strings": strings
                            entries.push((
                                Expr::Literal(Literal::String(name.clone())),
                                Expr::Ident(name),
                            ));
                        }
                    }
                    Some(Token::StringLit(s)) => {
                        self.advance();
                        self.expect(&Token::Colon)?;
                        let value = self.parse_expression()?;
                        entries.push((Expr::Literal(Literal::String(s)), value));
                    }
                    Some(Token::Keyword(k)) => {
                        // allow keyword keys like format:
                        let name = keyword_as_ident(&k);
                        self.advance();
                        self.expect(&Token::Colon)?;
                        let value = self.parse_expression()?;
                        entries.push((Expr::Literal(Literal::String(name)), value));
                    }
                    _ => {
                        let key = self.parse_expression()?;
                        self.expect(&Token::Colon)?;
                        let value = self.parse_expression()?;
                        entries.push((key, value));
                    }
                }

                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                    if matches!(self.peek(), Some(Token::RBrace)) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(Expr::Literal(Literal::Map(entries)))
    }
}

fn bin(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    Expr::Binary(BinaryExpr {
        op,
        left: Box::new(left),
        right: Box::new(right),
        span: 0..0,
    })
}

fn keyword_as_ident(k: &Keyword) -> String {
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

fn parse_err(message: &str, span: Span) -> HKLError {
    HKLError::Parser {
        message: message.to_string(),
        span,
    }
}

/// Public helper used by tests / external callers that already have tokens.
pub fn parse_expression(tokens: &[Token]) -> Result<Expr, HKLError> {
    let mut p = ExprParser::new(tokens);
    let expr = p.parse_expression()?;
    if p.position() != tokens.len() {
        return Err(parse_err(
            &format!(
                "Unexpected trailing tokens starting at {:?}",
                tokens.get(p.position())
            ),
            p.position()..p.position() + 1,
        ));
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_literal() {
        let tokens = tokenize("42").unwrap();
        assert!(parse_expression(&tokens).is_ok());
    }

    #[test]
    fn test_binary_expr() {
        let tokens = tokenize("x + y").unwrap();
        assert!(parse_expression(&tokens).is_ok());
    }

    #[test]
    fn test_pipe_expr() {
        let tokens = tokenize("x |> f").unwrap();
        assert!(parse_expression(&tokens).is_ok());
    }

    #[test]
    fn test_call_named() {
        let tokens = tokenize(r#"pathfinder(binary, hints: "DWARF")"#).unwrap();
        let expr = parse_expression(&tokens).unwrap();
        match expr {
            Expr::Call(c) => {
                assert_eq!(c.args.len(), 1);
                assert_eq!(c.named_args.len(), 1);
                assert_eq!(c.named_args[0].0, "hints");
            }
            _ => panic!("expected call"),
        }
    }

    #[test]
    fn test_member_call() {
        let tokens = tokenize("helix.decompile(ir, confidence: 0.7)").unwrap();
        assert!(parse_expression(&tokens).is_ok());
    }
}
