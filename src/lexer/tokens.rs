use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    HexLit(String),
    AddressLit(String),

    // Identifiers
    Ident(String),

    // Keywords
    Keyword(Keyword),

    // Operators
    Assign,     // =
    Pipe,       // |>
    FatArrow,   // =>
    ThinArrow,  // ->
    DotDot,     // ..
    ColonColon, // ::
    At,         // @
    Hash,       // #

    // Comparison
    Eq, // ==
    Ne, // !=
    Lt, // <
    Gt, // >
    Le, // <=
    Ge, // >=

    // Arithmetic / bitwise
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %
    Caret,   // ^
    Amp,     // &
    PipeOp,  // |

    // Logical
    Bang,     // !
    AmpAmp,   // &&
    PipePipe, // ||

    // Delimiters
    LBrace,    // {
    RBrace,    // }
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Semicolon, // ;
    Colon,     // :
    Dot,       // .

    // Special
    HQLBlock(String), // hql """..."""
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Keyword {
    Pipeline,
    Let,
    Fn,
    If,
    Else,
    Match,
    For,
    While,
    Return,
    Input,
    Output,
    Stage,
    Transform,
    Parallel,
    On,
    Where,
    And,
    Or,
    Not,
    True,
    False,
    Store,
    Notify,
    Export,
    Import,
    Use,
    Oracle,
    Emulate,
    Detect,
    Filter,
    Emit,
    In,
    Binary,
    Function,
    BasicBlock,
    Pattern,
    Ioc,
    Session,
    Hook,
    Timeout,
    Stalker,
    Severity,
    Mitre,
    Confidence,
}

impl Keyword {
    pub fn from_ident(s: &str) -> Option<Keyword> {
        Some(match s {
            "pipeline" => Keyword::Pipeline,
            "let" => Keyword::Let,
            "fn" => Keyword::Fn,
            "if" => Keyword::If,
            "else" => Keyword::Else,
            "match" => Keyword::Match,
            "for" => Keyword::For,
            "while" => Keyword::While,
            "return" => Keyword::Return,
            "input" => Keyword::Input,
            "output" => Keyword::Output,
            "stage" => Keyword::Stage,
            "transform" => Keyword::Transform,
            "parallel" => Keyword::Parallel,
            "on" => Keyword::On,
            "where" => Keyword::Where,
            "and" => Keyword::And,
            "or" => Keyword::Or,
            "not" => Keyword::Not,
            "true" => Keyword::True,
            "false" => Keyword::False,
            "store" => Keyword::Store,
            "notify" => Keyword::Notify,
            "export" => Keyword::Export,
            "import" => Keyword::Import,
            "use" => Keyword::Use,
            "oracle" => Keyword::Oracle,
            "emulate" => Keyword::Emulate,
            "detect" => Keyword::Detect,
            "filter" => Keyword::Filter,
            "emit" => Keyword::Emit,
            "in" => Keyword::In,
            "binary" => Keyword::Binary,
            "function" => Keyword::Function,
            "basicblock" => Keyword::BasicBlock,
            "pattern" => Keyword::Pattern,
            "ioc" => Keyword::Ioc,
            "session" => Keyword::Session,
            "hook" => Keyword::Hook,
            "timeout" => Keyword::Timeout,
            "stalker" => Keyword::Stalker,
            "severity" => Keyword::Severity,
            "mitre" => Keyword::Mitre,
            "confidence" => Keyword::Confidence,
            _ => return None,
        })
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::IntLit(n) => write!(f, "{}", n),
            Token::FloatLit(n) => write!(f, "{}", n),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            Token::BoolLit(b) => write!(f, "{}", b),
            Token::HexLit(h) => write!(f, "{}", h),
            Token::AddressLit(a) => write!(f, "@{}", a),
            Token::Ident(s) => write!(f, "{}", s),
            Token::Keyword(k) => write!(f, "{:?}", k),
            Token::Assign => write!(f, "="),
            Token::Pipe => write!(f, "|>"),
            Token::FatArrow => write!(f, "=>"),
            Token::ThinArrow => write!(f, "->"),
            Token::DotDot => write!(f, ".."),
            Token::ColonColon => write!(f, "::"),
            Token::At => write!(f, "@"),
            Token::Hash => write!(f, "#"),
            Token::Eq => write!(f, "=="),
            Token::Ne => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::Le => write!(f, "<="),
            Token::Ge => write!(f, ">="),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Caret => write!(f, "^"),
            Token::Amp => write!(f, "&"),
            Token::PipeOp => write!(f, "|"),
            Token::Bang => write!(f, "!"),
            Token::AmpAmp => write!(f, "&&"),
            Token::PipePipe => write!(f, "||"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::Dot => write!(f, "."),
            Token::HQLBlock(s) => write!(f, "hql \"\"\"{}\"\"\"", s),
        }
    }
}
