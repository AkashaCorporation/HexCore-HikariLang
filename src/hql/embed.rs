use crate::error::{HKLError, Span};
use crate::parser::ast::HQLQueryBlock;
use super::bridge::{HQLQuery, parse_hql_query, hql_query_to_json};

pub struct HQLProcessor {
    queries: Vec<HQLQuery>,
}

impl HQLProcessor {
    pub fn new() -> Self {
        HQLProcessor {
            queries: Vec::new(),
        }
    }

    pub fn process_query(&mut self, block: &HQLQueryBlock) -> Result<HQLQuery, HKLError> {
        let query = parse_hql_query(&block.content, block.span.clone())?;
        self.queries.push(query.clone());
        Ok(query)
    }

    pub fn to_json(&self) -> serde_json::Value {
        let queries: Vec<serde_json::Value> = self.queries.iter()
            .map(hql_query_to_json)
            .collect();
        serde_json::Value::Array(queries)
    }

    pub fn clear(&mut self) {
        self.queries.clear();
    }
}

pub fn extract_hql_queries(content: &str) -> Vec<(String, Span)> {
    let mut queries = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();

    while i < chars.len() {
        // Look for hql """
        if i + 4 <= chars.len() && chars[i] == 'h' && chars[i + 1] == 'q' && chars[i + 2] == 'l' {
            // Check for whitespace after hql
            if i + 3 < chars.len() && chars[i + 3] == ' ' {
                // Look for """
                let mut j = i + 4;
                while j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }

                if j + 2 < chars.len() && chars[j] == '"' && chars[j + 1] == '"' && chars[j + 2] == '"' {
                    let start = j + 3;
                    // Find closing """
                    let mut end = start;
                    while end + 2 < chars.len() {
                        if chars[end] == '"' && chars[end + 1] == '"' && chars[end + 2] == '"' {
                            break;
                        }
                        end += 1;
                    }

                    if end + 2 < chars.len() {
                        let query_content: String = chars[start..end].iter().collect();
                        queries.push((query_content, start..end));
                        i = end + 3;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    queries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hql_queries() {
        let content = r#"
            pipeline Test {
                matches = hql """
                    fn where
                        calls("VirtualProtect")
                """ on decomp;
            }
        "#;

        let queries = extract_hql_queries(content);
        assert_eq!(queries.len(), 1);
        assert!(queries[0].0.contains("calls(\"VirtualProtect\")"));
    }
}
