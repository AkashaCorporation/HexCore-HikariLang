use crate::error::{HKLError, Span};

#[derive(Debug, Clone)]
pub struct HQLQuery {
    pub target: Option<String>,
    pub attributes: Vec<AttributeCheck>,
    pub contains: Vec<HQLQuery>,
    pub operands: Vec<OperandCheck>,
    pub min_depth: Option<u32>,
    pub max_depth: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct AttributeCheck {
    pub field: String,
    pub value: AttrValue,
}

#[derive(Debug, Clone)]
pub enum AttrValue {
    Exact(String),
    ExactNum(f64),
    ExactBool(bool),
    Glob(String),
    Regex(String),
}

#[derive(Debug, Clone)]
pub struct OperandCheck {
    pub position: usize,
    pub query: HQLQuery,
}

#[derive(Debug, Clone)]
pub struct HQLMatchResult {
    pub signature_id: String,
    pub matches: Vec<String>,
    pub confidence: f64,
}

pub fn parse_hql_query(content: &str, _span: Span) -> Result<HQLQuery, HKLError> {
    // Simple HQL parser for embedded queries
    // This is a simplified version - the full parser would be more complex

    let mut target = None;
    let mut attributes = Vec::new();
    let contains = Vec::new();

    // Parse the query content
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("fn where") || line.starts_with("function where") {
            // Function query
            target = Some("CFunctionDecl".to_string());
        } else if line.contains("calls(") {
            // Call pattern
            if let Some(start) = line.find("calls(\"") {
                if let Some(end) = line[start + 7..].find("\")") {
                    let func_name = &line[start + 7..start + 7 + end];
                    attributes.push(AttributeCheck {
                        field: "callee".to_string(),
                        value: AttrValue::Exact(func_name.to_string()),
                    });
                }
            }
        } else if line.contains("contains_string(") {
            // String containment pattern
            if let Some(start) = line.find("contains_string(\"") {
                if let Some(end) = line[start + 16..].find("\")") {
                    let string_val = &line[start + 16..start + 16 + end];
                    attributes.push(AttributeCheck {
                        field: "value".to_string(),
                        value: AttrValue::Exact(string_val.to_string()),
                    });
                }
            }
        } else if line.contains("has_xor_loop(") {
            // XOR loop pattern
            if let Some(start) = line.find("key_size: ") {
                let rest = &line[start + 10..];
                if let Some(end) = rest.find(")") {
                    let range_str = &rest[..end];
                    // Parse range like "8..32"
                    if let Some(range_end) = range_str.find("..") {
                        let _min = range_str[..range_end].parse::<f64>().unwrap_or(0.0);
                        let _max = range_str[range_end + 2..].parse::<f64>().unwrap_or(0.0);
                        attributes.push(AttributeCheck {
                            field: "operator".to_string(),
                            value: AttrValue::Exact("^".to_string()),
                        });
                    }
                }
            }
        }
    }

    Ok(HQLQuery {
        target,
        attributes,
        contains,
        operands: Vec::new(),
        min_depth: None,
        max_depth: None,
    })
}

pub fn hql_query_to_json(query: &HQLQuery) -> serde_json::Value {
    let mut json = serde_json::Map::new();

    if let Some(target) = &query.target {
        json.insert(
            "target".to_string(),
            serde_json::Value::String(target.clone()),
        );
    }

    if !query.attributes.is_empty() {
        let attrs: Vec<serde_json::Value> = query
            .attributes
            .iter()
            .map(|attr| {
                let mut attr_json = serde_json::Map::new();
                attr_json.insert(
                    "field".to_string(),
                    serde_json::Value::String(attr.field.clone()),
                );

                let value = match &attr.value {
                    AttrValue::Exact(s) => serde_json::Value::String(s.clone()),
                    AttrValue::ExactNum(n) => serde_json::Value::Number(
                        serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0)),
                    ),
                    AttrValue::ExactBool(b) => serde_json::Value::Bool(*b),
                    AttrValue::Glob(s) => serde_json::Value::String(s.clone()),
                    AttrValue::Regex(s) => serde_json::Value::String(format!("re:{}", s)),
                };
                attr_json.insert("value".to_string(), value);

                serde_json::Value::Object(attr_json)
            })
            .collect();
        json.insert("attributes".to_string(), serde_json::Value::Array(attrs));
    }

    if !query.contains.is_empty() {
        let contains: Vec<serde_json::Value> =
            query.contains.iter().map(hql_query_to_json).collect();
        json.insert("contains".to_string(), serde_json::Value::Array(contains));
    }

    serde_json::Value::Object(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_hql() {
        let query = r#"
            fn where
                calls("VirtualProtect") and
                contains_string("Ashaka")
        "#;
        let result = parse_hql_query(query, 0..0);
        assert!(result.is_ok());

        let query = result.unwrap();
        assert_eq!(query.target, Some("CFunctionDecl".to_string()));
        assert_eq!(query.attributes.len(), 2);
    }

    #[test]
    fn test_hql_to_json() {
        let query = HQLQuery {
            target: Some("CCallExpr".to_string()),
            attributes: vec![AttributeCheck {
                field: "callee".to_string(),
                value: AttrValue::Exact("VirtualProtect".to_string()),
            }],
            contains: vec![],
            operands: vec![],
            min_depth: None,
            max_depth: None,
        };

        let json = hql_query_to_json(&query);
        assert!(json.is_object());
        assert_eq!(json["target"], "CCallExpr");
    }
}
