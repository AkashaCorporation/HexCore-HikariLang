use thiserror::Error;

#[derive(Debug, Error)]
pub enum HKLError {
    #[error("Lexer error at {span:?}: {message}")]
    Lexer { message: String, span: Span },

    #[error("Parser error at {span:?}: {message}")]
    Parser { message: String, span: Span },

    #[error("Type error at {span:?}: {message}")]
    Type { message: String, span: Span },

    #[error("Runtime error at {span:?}: {message}")]
    Runtime { message: String, span: Span },

    #[error("HQL error at {span:?}: {message}")]
    HQL { message: String, span: Span },

    #[error("IO error: {0}")]
    IO(String),

    #[error("{0}")]
    Other(String),
}

impl Clone for HKLError {
    fn clone(&self) -> Self {
        match self {
            HKLError::Lexer { message, span } => HKLError::Lexer {
                message: message.clone(),
                span: span.clone(),
            },
            HKLError::Parser { message, span } => HKLError::Parser {
                message: message.clone(),
                span: span.clone(),
            },
            HKLError::Type { message, span } => HKLError::Type {
                message: message.clone(),
                span: span.clone(),
            },
            HKLError::Runtime { message, span } => HKLError::Runtime {
                message: message.clone(),
                span: span.clone(),
            },
            HKLError::HQL { message, span } => HKLError::HQL {
                message: message.clone(),
                span: span.clone(),
            },
            HKLError::IO(s) => HKLError::IO(s.clone()),
            HKLError::Other(s) => HKLError::Other(s.clone()),
        }
    }
}

impl From<std::io::Error> for HKLError {
    fn from(err: std::io::Error) -> Self {
        HKLError::IO(err.to_string())
    }
}

pub type Span = std::ops::Range<usize>;

pub type Result<T> = std::result::Result<T, HKLError>;
