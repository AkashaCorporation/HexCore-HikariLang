use hikari_lang::engine::{PipelineEngine, RuntimeCall, RuntimeHost, Value};
use hikari_lang::error::HKLError;
use hikari_lang::lexer::tokenize;
use hikari_lang::parser::{parse_program, PipelineDecl, Program, TopLevelDecl};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
struct RecordedCall {
    name: String,
    args: Vec<Value>,
    named_args: Vec<(String, Value)>,
}

struct RecordingHost {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl RuntimeHost for RecordingHost {
    fn supports(&self, name: &str) -> bool {
        name == "custom.analyze"
    }

    fn call(&mut self, call: RuntimeCall<'_>) -> Result<Value, HKLError> {
        self.calls.lock().unwrap().push(RecordedCall {
            name: call.name.to_owned(),
            args: call.args.to_vec(),
            named_args: call.named_args.to_vec(),
        });

        Ok(Value::String_("host-ok".into()))
    }
}

struct FailingHost;

impl RuntimeHost for FailingHost {
    fn supports(&self, name: &str) -> bool {
        name == "helix.decompile"
    }

    fn call(&mut self, call: RuntimeCall<'_>) -> Result<Value, HKLError> {
        Err(HKLError::Runtime {
            message: "backend exploded".into(),
            span: call.span,
        })
    }
}

struct EmptyHost;

impl RuntimeHost for EmptyHost {
    fn supports(&self, _name: &str) -> bool {
        false
    }

    fn call(&mut self, call: RuntimeCall<'_>) -> Result<Value, HKLError> {
        panic!("unsupported runtime call reached host: {}", call.name)
    }
}

fn parse(source: &str) -> Program {
    let tokens = tokenize(source).expect("source should tokenize");
    parse_program(&tokens).expect("source should parse")
}

fn first_pipeline(program: &Program) -> &PipelineDecl {
    match program.declarations.first() {
        Some(TopLevelDecl::Pipeline(pipeline)) => pipeline,
        _ => panic!("expected first declaration to be a pipeline"),
    }
}

#[test]
fn pipeline_engine_dispatches_evaluated_values_to_custom_host() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let host = RecordingHost {
        calls: Arc::clone(&calls),
    };
    let program = parse(
        r#"
        pipeline HostProbe {
            input binary: Binary
            let result = custom.analyze(binary, confidence: 0.75);
            emit result;
        }
        "#,
    );

    let mut engine = PipelineEngine::with_runtime(host);
    let result = engine
        .execute_pipeline(
            first_pipeline(&program),
            HashMap::from([(
                "binary".into(),
                Value::Binary("sample-under-test.exe".into()),
            )]),
        )
        .expect("custom host call should succeed");

    assert_eq!(result, Value::String_("host-ok".into()));

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "custom.analyze");
    assert_eq!(
        calls[0].args,
        vec![Value::Binary("sample-under-test.exe".into())]
    );
    assert_eq!(
        calls[0].named_args,
        vec![("confidence".into(), Value::Float(0.75))]
    );
}

#[test]
fn supported_backend_failures_are_not_reinterpreted_as_method_calls() {
    let program = parse(
        r#"
        pipeline FailureProbe {
            input binary: Binary
            let result = helix.decompile(binary);
            emit result;
        }
        "#,
    );
    let mut engine = PipelineEngine::with_runtime(FailingHost);

    let error = engine
        .execute_pipeline(
            first_pipeline(&program),
            HashMap::from([("binary".into(), Value::Binary("sample.exe".into()))]),
        )
        .expect_err("backend failure should propagate");

    match error {
        HKLError::Runtime { message, .. } => assert_eq!(message, "backend exploded"),
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn language_level_methods_still_work_when_host_does_not_claim_them() {
    let program = parse(
        r#"
        pipeline MethodProbe {
            let values = [1];
            let result = values.any();
            emit result;
        }
        "#,
    );
    let mut engine = PipelineEngine::with_runtime(EmptyHost);

    let result = engine
        .execute_pipeline(first_pipeline(&program), HashMap::new())
        .expect("language method should not be sent to runtime host");

    assert_eq!(result, Value::Bool(true));
}
