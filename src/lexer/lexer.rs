use super::tokens::{Keyword, Token};
use crate::error::{HKLError, Span};

/// Hand-written lexer — fast to compile, predictable, and easy to extend.
pub fn tokenize(source: &str) -> Result<Vec<Token>, HKLError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0usize;

    while i < chars.len() {
        // Whitespace
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }

        // Line comment
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // hql """..."""
        if starts_with_word(&chars, i, "hql") {
            let after = i + 3;
            if after < chars.len() && (chars[after].is_whitespace() || chars[after] == '"') {
                let mut j = after;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j + 2 < chars.len()
                    && chars[j] == '"'
                    && chars[j + 1] == '"'
                    && chars[j + 2] == '"'
                {
                    j += 3;
                    let start = j;
                    while j + 2 < chars.len()
                        && !(chars[j] == '"' && chars[j + 1] == '"' && chars[j + 2] == '"')
                    {
                        j += 1;
                    }
                    if j + 2 >= chars.len() {
                        return Err(lex_err("Unterminated HQL block", start..j));
                    }
                    let content: String = chars[start..j].iter().collect();
                    tokens.push(Token::HQLBlock(content));
                    i = j + 3;
                    continue;
                }
            }
        }

        // Address @hex or @0xhex
        if chars[i] == '@' {
            let start = i;
            i += 1;
            if i + 1 < chars.len()
                && chars[i] == '0'
                && (chars[i + 1] == 'x' || chars[i + 1] == 'X')
            {
                i += 2;
            }
            let hex_start = i;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            if i == hex_start {
                return Err(lex_err("Expected hex digits after @", start..i));
            }
            let hex: String = chars[hex_start..i].iter().collect();
            tokens.push(Token::AddressLit(hex));
            continue;
        }

        // Hex literal 0x...
        if chars[i] == '0' && i + 1 < chars.len() && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
            let start = i;
            i += 2;
            let hex_start = i;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            if i == hex_start {
                return Err(lex_err("Expected hex digits after 0x", start..i));
            }
            let hex: String = chars[start..i].iter().collect();
            tokens.push(Token::HexLit(hex));
            continue;
        }

        // String "..."
        if chars[i] == '"' {
            let start = i;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    s.push(match chars[i] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '"' => '"',
                        c => c,
                    });
                    i += 1;
                } else {
                    s.push(chars[i]);
                    i += 1;
                }
            }
            if i >= chars.len() {
                return Err(lex_err("Unterminated string", start..i));
            }
            i += 1; // closing "
            tokens.push(Token::StringLit(s));
            continue;
        }

        // Number (int or float)
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            // float if .digit (but not ..)
            if i < chars.len()
                && chars[i] == '.'
                && i + 1 < chars.len()
                && chars[i + 1].is_ascii_digit()
            {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n: f64 = s.parse().map_err(|_| lex_err("Invalid float", start..i))?;
                tokens.push(Token::FloatLit(n));
            } else {
                let s: String = chars[start..i].iter().collect();
                let n: i64 = s
                    .parse()
                    .map_err(|_| lex_err("Invalid integer", start..i))?;
                tokens.push(Token::IntLit(n));
            }
            continue;
        }

        // Identifier / keyword
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            if s == "true" {
                tokens.push(Token::BoolLit(true));
            } else if s == "false" {
                tokens.push(Token::BoolLit(false));
            } else if let Some(kw) = Keyword::from_ident(&s) {
                tokens.push(Token::Keyword(kw));
            } else {
                tokens.push(Token::Ident(s));
            }
            continue;
        }

        // Multi-char operators
        if i + 1 < chars.len() {
            let two: String = chars[i..i + 2].iter().collect();
            let tok = match two.as_str() {
                "|>" => Some(Token::Pipe),
                "=>" => Some(Token::FatArrow),
                "->" => Some(Token::ThinArrow),
                ".." => Some(Token::DotDot),
                "::" => Some(Token::ColonColon),
                "==" => Some(Token::Eq),
                "!=" => Some(Token::Ne),
                "<=" => Some(Token::Le),
                ">=" => Some(Token::Ge),
                "&&" => Some(Token::AmpAmp),
                "||" => Some(Token::PipePipe),
                _ => None,
            };
            if let Some(t) = tok {
                tokens.push(t);
                i += 2;
                continue;
            }
        }

        // Single-char tokens
        let tok = match chars[i] {
            '=' => Token::Assign,
            '<' => Token::Lt,
            '>' => Token::Gt,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '^' => Token::Caret,
            '&' => Token::Amp,
            '|' => Token::PipeOp,
            '!' => Token::Bang,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            ':' => Token::Colon,
            '.' => Token::Dot,
            '#' => Token::Hash,
            c => {
                return Err(lex_err(&format!("Unexpected character '{}'", c), i..i + 1));
            }
        };
        tokens.push(tok);
        i += 1;
    }

    Ok(tokens)
}

fn starts_with_word(chars: &[char], i: usize, word: &str) -> bool {
    let w: Vec<char> = word.chars().collect();
    if i + w.len() > chars.len() {
        return false;
    }
    if chars[i..i + w.len()] != w[..] {
        return false;
    }
    // word boundary
    let end = i + w.len();
    end >= chars.len() || !(chars[end].is_ascii_alphanumeric() || chars[end] == '_')
}

fn lex_err(message: &str, span: Span) -> HKLError {
    HKLError::Lexer {
        message: message.to_string(),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_literal() {
        let tokens = tokenize("0x401000").unwrap();
        assert_eq!(tokens, vec![Token::HexLit("0x401000".into())]);
    }

    #[test]
    fn test_address_literal() {
        let tokens = tokenize("@0x401000").unwrap();
        assert_eq!(tokens, vec![Token::AddressLit("401000".into())]);
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize("\"hello world\"").unwrap();
        assert_eq!(tokens, vec![Token::StringLit("hello world".into())]);
    }

    #[test]
    fn test_keywords() {
        let tokens = tokenize("pipeline let fn if else emit").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Keyword(Keyword::Pipeline),
                Token::Keyword(Keyword::Let),
                Token::Keyword(Keyword::Fn),
                Token::Keyword(Keyword::If),
                Token::Keyword(Keyword::Else),
                Token::Keyword(Keyword::Emit),
            ]
        );
    }

    #[test]
    fn test_operators() {
        let tokens = tokenize("= == != <= >= && || |> => ..").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Assign,
                Token::Eq,
                Token::Ne,
                Token::Le,
                Token::Ge,
                Token::AmpAmp,
                Token::PipePipe,
                Token::Pipe,
                Token::FatArrow,
                Token::DotDot,
            ]
        );
    }

    #[test]
    fn test_hql_block() {
        let tokens = tokenize(r#"hql """fn where calls("x")""""#).unwrap();
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Token::HQLBlock(s) => assert!(s.contains("calls")),
            _ => panic!("expected HQLBlock"),
        }
    }

    #[test]
    fn test_assign_vs_eq() {
        let tokens = tokenize("x = 1; y == 2").unwrap();
        assert!(matches!(tokens[1], Token::Assign));
        assert!(matches!(tokens[5], Token::Eq));
    }
}
