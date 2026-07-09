pub mod embed;
pub mod bridge;

pub use embed::{HQLProcessor, extract_hql_queries};
pub use bridge::{HQLQuery, AttributeCheck, AttrValue, OperandCheck, HQLMatchResult, parse_hql_query, hql_query_to_json};
