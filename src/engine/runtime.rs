use super::interpreter::{EmuSnapshotValue, HQLMatchResultValue, IOCValue, Value};
use crate::error::{HKLError, Span};
use std::collections::HashMap;

/// A normalized backend invocation emitted by the HKL interpreter.
///
/// The request deliberately contains only evaluated HKL values. It does not
/// expose parser AST nodes, HexCore job/watch semantics, HQL internals, or a
/// persistence implementation. Those integrations can evolve independently
/// behind a [`RuntimeHost`] implementation.
#[derive(Debug, Clone)]
pub struct RuntimeCall<'a> {
    pub name: &'a str,
    pub args: &'a [Value],
    pub named_args: &'a [(String, Value)],
    pub span: Span,
}

impl<'a> RuntimeCall<'a> {
    pub fn named_arg(&self, name: &str) -> Option<&Value> {
        self.named_args
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value))
    }
}

/// Host boundary between the HKL language runtime and concrete HexCore engines.
///
/// A host must advertise support before a module-style call is dispatched.
/// This lets expressions such as `matches.any()` keep their language-level
/// method semantics while backend failures from supported calls are propagated
/// without being mistaken for an unsupported function.
pub trait RuntimeHost: Send {
    fn supports(&self, name: &str) -> bool;

    fn call(&mut self, call: RuntimeCall<'_>) -> Result<Value, HKLError>;
}

/// Default development host that preserves the repository's existing mock
/// behavior until a production HexCore host is wired in.
#[derive(Debug, Default)]
pub struct MockRuntimeHost;

const MOCK_FUNCTIONS: &[&str] = &[
    "pathfinder",
    "remill.lift",
    "helix.decompile",
    "elixir.emulate",
    "detect_ioc",
    "generate_report",
    "extract_strings",
    "get_imports",
    "filter",
    "is_suspicious",
    "chacha_detection",
    "refcount_scanner",
    "api_hash_resolver",
];

impl RuntimeHost for MockRuntimeHost {
    fn supports(&self, name: &str) -> bool {
        MOCK_FUNCTIONS.contains(&name)
    }

    fn call(&mut self, call: RuntimeCall<'_>) -> Result<Value, HKLError> {
        match call.name {
            "pathfinder" => {
                println!("  [mock] pathfinder(...)");
                Ok(Value::IRNode("cfg_mock".into()))
            }
            "remill.lift" => {
                println!("  [mock] remill.lift(...)");
                Ok(Value::IRNode("ir_lifted".into()))
            }
            "helix.decompile" => {
                let confidence = call
                    .named_arg("confidence")
                    .and_then(|value| match value {
                        Value::Float(value) => Some(*value),
                        Value::Int(value) => Some(*value as f64),
                        _ => None,
                    })
                    .unwrap_or(0.5);

                println!("  [mock] helix.decompile(confidence={})", confidence);
                Ok(Value::Map(HashMap::from([
                    ("name".into(), Value::String_("decompiled_func".into())),
                    (
                        "body".into(),
                        Value::String_("// mock decompiled pseudocode".into()),
                    ),
                    ("confidence".into(), Value::Float(confidence)),
                ])))
            }
            "elixir.emulate" => {
                let hooks = call
                    .named_arg("hooks")
                    .map(Value::to_string_value)
                    .unwrap_or_else(|| "default".into());
                let timeout = call
                    .named_arg("timeout")
                    .and_then(|value| match value {
                        Value::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(30);
                let stalker = call
                    .named_arg("stalker")
                    .map(Value::is_truthy)
                    .unwrap_or(false);

                println!(
                    "  [mock] elixir.emulate(hooks={}, timeout={}, stalker={})",
                    hooks, timeout, stalker
                );
                Ok(Value::EmuSnapshot(EmuSnapshotValue {
                    session_id: "emu_mock_session".into(),
                    timestamp: 1_700_000_000,
                    hooks: vec![hooks],
                }))
            }
            "detect_ioc" => {
                println!("  [mock] detect_ioc(...)");
                Ok(Value::Array(vec![
                    Value::IOC(IOCValue {
                        ioc_type: "url".into(),
                        value: "http://malicious.example.com".into(),
                        confidence: 0.85,
                    }),
                    Value::IOC(IOCValue {
                        ioc_type: "mutex".into(),
                        value: "Global\\AshakaV5".into(),
                        confidence: 0.92,
                    }),
                ]))
            }
            "generate_report" => {
                let report = if let Some(Value::Map(options)) = call.args.first() {
                    let format = options
                        .get("format")
                        .map(Value::to_string_value)
                        .unwrap_or_else(|| "Markdown".into());
                    format!(
                        "# Mock Report ({})\n\nGenerated by HikariScript runtime (mock).\n",
                        format
                    )
                } else {
                    "# Mock Report\n\nGenerated by HikariScript runtime (mock).\n".into()
                };

                println!("  [mock] generate_report");
                Ok(Value::String_(report))
            }
            "extract_strings" => {
                println!("  [mock] extract_strings(...)");
                Ok(Value::Array(vec![
                    Value::String_("Mock string 1".into()),
                    Value::String_("/tmp/payload.bin".into()),
                    Value::String_("Ashaka".into()),
                ]))
            }
            "get_imports" => {
                println!("  [mock] get_imports(...)");
                Ok(Value::Array(vec![
                    Value::String_("kernel32.dll!VirtualProtect".into()),
                    Value::String_("ntdll.dll!NtAllocateVirtualMemory".into()),
                    Value::String_("ws2_32.dll!connect".into()),
                ]))
            }
            "filter" | "is_suspicious" => {
                if call.name == "is_suspicious" {
                    return Ok(Value::Bool(true));
                }

                if let Some(Value::Array(values)) = call.args.first() {
                    Ok(Value::Array(values.clone()))
                } else {
                    Ok(Value::Array(Vec::new()))
                }
            }
            "chacha_detection" | "refcount_scanner" | "api_hash_resolver" => {
                Ok(Value::HQLResult(HQLMatchResultValue {
                    signature_id: call.name.into(),
                    matches: Vec::new(),
                    confidence: 0.0,
                }))
            }
            _ => Err(HKLError::Runtime {
                message: format!("Unknown runtime function: {}", call.name),
                span: call.span,
            }),
        }
    }
}
