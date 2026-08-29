# HKL Runtime Host Contract

## Status

This is the first integration boundary between the HikariLang interpreter and
concrete HexCore analysis engines.

The repository still defaults to `MockRuntimeHost`, so existing fixtures remain
runnable without HexCore. A production integration supplies one
`RuntimeHost` implementation to `Interpreter::with_runtime` or
`PipelineEngine::with_runtime`.

## Why this boundary exists

The language runtime previously contained a hard-coded `match` for Pathfinder,
Remill, Helix, Elixir, IOC extraction, reporting, and static helpers. A second
builtin registry also existed in `engine/builtins.rs`, but the interpreter did
not use it. That left two competing extension paths and coupled language
semantics to backend behavior.

`RuntimeHost` makes the direction explicit:

```text
HKL source
  -> lexer
  -> parser
  -> typechecker
  -> interpreter
  -> RuntimeHost
       -> Pathfinder / function recovery
       -> Remill / IR lifting
       -> Helix / decompilation
       -> Elixir / execution
       -> static helpers / reporting
```

The interpreter evaluates expressions first. The host receives only normalized
`Value` arguments and the source span associated with the call.

## Contract

```rust
pub trait RuntimeHost: Send {
    fn supports(&self, name: &str) -> bool;
    fn call(&mut self, call: RuntimeCall<'_>) -> Result<Value, HKLError>;
}
```

A `RuntimeCall` contains:

- the canonical function name, such as `helix.decompile`;
- evaluated positional arguments;
- evaluated named arguments;
- the HKL source span for diagnostics.

`supports` is semantically important for module-style calls. A call such as
`helix.decompile(...)` is sent to the host only when the host claims that exact
name. Otherwise, member syntax remains available for language-level operations
such as `matches.any()`.

When a host claims a call, its error is authoritative and is returned to the
pipeline. Backend failures are not reinterpreted as missing members.

## Minimal integration example

```rust
use hikari_lang::engine::{PipelineEngine, RuntimeCall, RuntimeHost, Value};
use hikari_lang::error::HKLError;

struct HexCoreRuntime;

impl RuntimeHost for HexCoreRuntime {
    fn supports(&self, name: &str) -> bool {
        matches!(name, "pathfinder" | "remill.lift" | "helix.decompile")
    }

    fn call(&mut self, call: RuntimeCall<'_>) -> Result<Value, HKLError> {
        match call.name {
            "pathfinder" => {
                // Convert normalized HKL values into a HexCore request.
                // Return a typed HKL Value after the engine finishes.
                Ok(Value::IRNode("cfg-from-hexcore".into()))
            }
            _ => Err(HKLError::Runtime {
                message: format!("runtime call not implemented: {}", call.name),
                span: call.span,
            }),
        }
    }
}

let engine = PipelineEngine::with_runtime(HexCoreRuntime);
```

## Deliberate non-decisions in this wave

This contract does **not** define:

- Job or Watcher syntax and lifecycle;
- replacement of the existing JSON Job System;
- HQL execution or HQL bridge internals;
- a production CLI transport;
- HXDB/session transaction semantics;
- cancellation, progress streaming, or asynchronous engine calls.

Those concerns must be aligned with the HexCore 3.8.4 integration contracts
before they become stable HKL APIs. The current host boundary is intentionally
small enough to accept those decisions later without changing the parser or
rewriting the interpreter.

## Compatibility

`MockRuntimeHost` preserves the existing mock outputs and remains the default.
The legacy `BuiltinFn` exports are retained temporarily for source
compatibility, but backend dispatch now has one canonical path through
`RuntimeHost`.
