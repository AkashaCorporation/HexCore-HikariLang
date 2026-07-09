use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum HKLType {
    // Native binary analysis types
    Binary { format: Option<BinaryFormat> },
    Function,
    BasicBlock,
    IRNode,
    EmuSnapshot,
    IOC,
    Pattern { pattern_type: PatternType },
    String { encoding: Option<StringEncoding> },
    Address,
    Range,
    Pipeline { stages: Vec<HKLType> },

    // Primitives
    Bool,
    Int { width: Option<u32> },
    Float,
    String_,

    // Collections
    Array(Box<HKLType>),
    Map(Box<HKLType>, Box<HKLType>),

    // Function signatures
    Fn { params: Vec<HKLType>, returns: Box<HKLType> },

    // Special
    Void,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryFormat {
    PE64,
    PE32,
    ELF64,
    ELF32,
    MachO,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    YARA,
    Sigma,
    HQL,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringEncoding {
    Ascii,
    Utf8,
    Wide,
}

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub params: Vec<Param>,
    pub returns: HKLType,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_: HKLType,
}

pub fn builtin_signatures() -> HashMap<String, FunctionSig> {
    let mut sigs = HashMap::new();

    sigs.insert("pathfinder".into(), FunctionSig {
        params: vec![
            Param { name: "binary".into(), type_: HKLType::Binary { format: None } },
            Param { name: "hints".into(), type_: HKLType::String_ },
        ],
        returns: HKLType::IRNode,
    });

    sigs.insert("remill.lift".into(), FunctionSig {
        params: vec![
            Param { name: "binary".into(), type_: HKLType::Binary { format: None } },
            Param { name: "cfg".into(), type_: HKLType::IRNode },
        ],
        returns: HKLType::IRNode,
    });

    sigs.insert("helix.decompile".into(), FunctionSig {
        params: vec![
            Param { name: "ir".into(), type_: HKLType::IRNode },
            Param { name: "confidence".into(), type_: HKLType::Float },
        ],
        returns: HKLType::Function,
    });

    sigs.insert("elixir.emulate".into(), FunctionSig {
        params: vec![
            Param { name: "binary".into(), type_: HKLType::Binary { format: None } },
            Param { name: "hooks".into(), type_: HKLType::String_ },
            Param { name: "timeout".into(), type_: HKLType::Int { width: Some(32) } },
        ],
        returns: HKLType::EmuSnapshot,
    });

    sigs.insert("detect_ioc".into(), FunctionSig {
        params: vec![
            Param { name: "snapshot".into(), type_: HKLType::EmuSnapshot },
        ],
        returns: HKLType::Array(Box::new(HKLType::IOC)),
    });

    sigs.insert("generate_report".into(), FunctionSig {
        params: vec![
            Param { name: "format".into(), type_: HKLType::String_ },
            Param { name: "include".into(), type_: HKLType::Array(Box::new(HKLType::String_)) },
        ],
        returns: HKLType::String_,
    });

    sigs.insert("chacha_detection".into(), FunctionSig {
        params: vec![],
        returns: HKLType::Pattern { pattern_type: PatternType::HQL },
    });

    sigs.insert("refcount_scanner".into(), FunctionSig {
        params: vec![],
        returns: HKLType::Pattern { pattern_type: PatternType::HQL },
    });

    sigs.insert("api_hash_resolver".into(), FunctionSig {
        params: vec![],
        returns: HKLType::Pattern { pattern_type: PatternType::HQL },
    });

    sigs.insert("get_imports".into(), FunctionSig {
        params: vec![
            Param { name: "binary".into(), type_: HKLType::Binary { format: None } },
        ],
        returns: HKLType::Array(Box::new(HKLType::String_)),
    });

    sigs.insert("extract_strings".into(), FunctionSig {
        params: vec![
            Param { name: "binary".into(), type_: HKLType::Binary { format: None } },
        ],
        returns: HKLType::Array(Box::new(HKLType::String_)),
    });

    sigs.insert("filter".into(), FunctionSig {
        params: vec![
            Param { name: "items".into(), type_: HKLType::Array(Box::new(HKLType::Unknown)) },
            Param { name: "predicate".into(), type_: HKLType::Fn {
                params: vec![HKLType::Unknown],
                returns: Box::new(HKLType::Bool),
            }},
        ],
        returns: HKLType::Array(Box::new(HKLType::Unknown)),
    });

    sigs
}

impl std::fmt::Display for HKLType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HKLType::Binary { format } => match format {
                Some(fmt) => write!(f, "Binary<{:?}>", fmt),
                None => write!(f, "Binary"),
            },
            HKLType::Function => write!(f, "Function"),
            HKLType::BasicBlock => write!(f, "BasicBlock"),
            HKLType::IRNode => write!(f, "IRNode"),
            HKLType::EmuSnapshot => write!(f, "EmuSnapshot"),
            HKLType::IOC => write!(f, "IOC"),
            HKLType::Pattern { pattern_type } => write!(f, "Pattern<{}>", format!("{:?}", pattern_type)),
            HKLType::String { .. } => write!(f, "String"),
            HKLType::Address => write!(f, "Address"),
            HKLType::Range => write!(f, "Range"),
            HKLType::Pipeline { .. } => write!(f, "Pipeline"),
            HKLType::Bool => write!(f, "Bool"),
            HKLType::Int { width } => match width {
                Some(w) => write!(f, "Int<{}>", w),
                None => write!(f, "Int"),
            },
            HKLType::Float => write!(f, "Float"),
            HKLType::String_ => write!(f, "String"),
            HKLType::Array(elem) => write!(f, "Array<{}>", elem),
            HKLType::Map(key, val) => write!(f, "Map<{}, {}>", key, val),
            HKLType::Fn { params, returns } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", returns)
            }
            HKLType::Void => write!(f, "Void"),
            HKLType::Unknown => write!(f, "?"),
        }
    }
}
