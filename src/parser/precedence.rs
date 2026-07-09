// Operator precedence table for HKL (documentation / tooling).
// Lower number = lower precedence (evaluated last).

pub const PRECEDENCE: &[(u8, &[&str])] = &[
    (1, &["||", "or"]),
    (2, &["&&", "and"]),
    (3, &["==", "!=", "<", ">", "<=", ">="]),
    (4, &["|>"]),
    (5, &["+", "-"]),
    (6, &["*", "/", "%"]),
    (7, &["^", "&", "|"]),
    (8, &["<<", ">>"]),
];

pub fn get_precedence(op: &str) -> u8 {
    for (prec, ops) in PRECEDENCE {
        if ops.contains(&op) {
            return *prec;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precedence() {
        assert!(get_precedence("||") < get_precedence("&&"));
        assert!(get_precedence("&&") < get_precedence("=="));
        assert!(get_precedence("==") < get_precedence("|>"));
        assert!(get_precedence("|>") < get_precedence("+"));
        assert!(get_precedence("+") < get_precedence("*"));
    }
}
