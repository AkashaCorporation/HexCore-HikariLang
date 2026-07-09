# HexCore · HikariLang

**HikariLang** (also *HikariScript* / **HKL**) is the binary-analysis workflow language for [HexCore](https://github.com/AkashaCorporation).

Write pipelines that orchestrate static analysis, IR lifting, decompilation, semantic queries (HQL), and dynamic emulation — as a single declarative script.

> **Status:** language core is runnable end-to-end with **mock backends**.  
> IDE and Decompiler teams plug real engines later; the CLI surface is stable enough to develop against.

---

## Quick start

**Requirements:** [Rust](https://rustup.rs/) 1.70+ (edition 2021)

```bash
git clone https://github.com/AkashaCorporation/HexCore-HikariLang.git
cd HexCore-HikariLang

cargo build
cargo test

# Run example pipelines (mock analysis backends)
cargo run -- run tests/fixtures/simple_pipe.hkl
cargo run -- run tests/fixtures/hunt_ashaka.hkl

# Type-check without executing
cargo run -- check tests/fixtures/hunt_ashaka.hkl

# Print the AST
cargo run -- ast tests/fixtures/simple_pipe.hkl
```

After `cargo build`, the CLI binary is:

| Platform | Path |
|----------|------|
| Windows  | `target/debug/hkl.exe` |
| Unix     | `target/debug/hkl` |

```bash
./target/debug/hkl run tests/fixtures/simple_pipe.hkl
./target/debug/hkl --help
```

---

## What is an HKL pipeline?

A pipeline declares **inputs**, optional **session** metadata, and a sequence of analysis stages.

```hkl
pipeline QuickTriage {
    input binary: Binary

    let strings = extract_strings(binary);
    let imports = get_imports(binary);
    let suspicious = filter imports where is_suspicious(name);

    emit { strings, imports: suspicious };
}
```

Richer workflows mix static + semantic + dynamic phases (see `tests/fixtures/hunt_ashaka.hkl`):

```hkl
pipeline HuntAshakaV5 {
    input binary: PE64 | ELF64
    session: "ashaka_hunt_2026"

    let cfg = pathfinder(binary, hints: "DWARF | PDB");
    let ir = remill.lift(binary, cfg);
    let decomp = helix.decompile(ir, confidence: 0.7);

    matches = hql """
        fn where
            calls("VirtualProtect") and
            contains_string("Ashaka")
    """ on decomp;

    if matches.any() {
        snapshot = elixir.emulate(binary,
            hooks: "Win32Full | LinuxSyscall",
            timeout: 45,
            stalker: true
        );
        iocs = detect_ioc(snapshot);
    }

    store session;
    notify "threat-intel@akashacorp" if iocs.high_confidence;
}
```

---

## CLI

| Command | Description |
|---------|-------------|
| `hkl run <file.hkl>` | Parse, type-aware run of the pipeline |
| `hkl check <file.hkl>` | Type-check only |
| `hkl ast <file.hkl>` | Dump debug AST |
| `hkl fmt <file.hkl>` | Formatter *(stub — IDE team)* |
| `hkl export -f yara\|sigma <file.hkl>` | Pattern export *(stub)* |

**Optional flags on `run`** (reserved for plug-ins):

```bash
hkl run pipeline.hkl --binary sample.exe --session my_session
```

---

## Architecture

```
.sourceforge
  │
  ▼
┌─────────┐   ┌──────────┐   ┌──────────────┐   ┌─────────────┐
│  Lexer  │──▶│  Parser  │──▶│ Typechecker  │──▶│ Interpreter │
└─────────┘   └──────────┘   └──────────────┘   └──────┬──────┘
                                                       │
                       ┌───────────────────────────────┼────────────────┐
                       ▼                               ▼                ▼
                 Mock builtins                   HQL bridge        Pipeline engine
              (pathfinder, helix, …)           (→ hexcore-hql)    (stages / emit)
```

| Path | Role |
|------|------|
| `src/lexer/` | Tokenizer (keywords, HQL blocks, hex/address lits) |
| `src/parser/` | Recursive-descent parser + AST |
| `src/types/` | Typechecker & builtin signatures |
| `src/engine/` | Interpreter, pipeline runner, mock builtins |
| `src/hql/` | Embedded HQL parse/bridge (stub → hexcore-hql) |
| `tests/fixtures/` | Example `.hkl` programs |

### Mock builtins (replace with real backends)

| Builtin | Intended backend |
|---------|------------------|
| `pathfinder` | CFG / function recovery |
| `remill.lift` | IR lifting |
| `helix.decompile` | Decompiler |
| `elixir.emulate` | Dynamic / stalker emu |
| `detect_ioc` | IOC extraction |
| `generate_report` | Reporting |
| `extract_strings` / `get_imports` / `filter` | Static triage helpers |

Plug points for IDE / Decompiler teams:

- `src/engine/interpreter.rs` → `call_builtin`
- `src/engine/builtins.rs`
- `src/hql/`
- CLI flags `--binary` / `--session`

---

## Project layout (what gets committed)

Only source and project metadata. Build output is **not** in git:

```
HexCore-HikariLang/
├── Cargo.toml
├── Cargo.lock          # locked deps for reproducible CLI builds
├── README.md
├── .gitignore
├── src/
│   ├── main.rs         # hkl CLI
│   ├── lib.rs
│   ├── lexer/
│   ├── parser/
│   ├── types/
│   ├── engine/
│   ├── hql/
│   └── error.rs
└── tests/
    └── fixtures/
        ├── simple_pipe.hkl
        └── hunt_ashaka.hkl
```

`target/` (compiled artifacts), IDE folders, and OS junk are ignored via `.gitignore`.

---

## Development

```bash
# Unit tests (lexer, parser, HQL, typechecker)
cargo test

# Debug build
cargo build

# Release binary
cargo build --release
# → target/release/hkl[.exe]
```

### Language surface (current)

- Pipelines with `input`, `session`, stages
- `let` / bare assignment, `if`, `for`, `while`, `emit`, `store`, `notify`
- Expressions: calls, named args, member access, pipes `|>`, maps/objects, arrays
- Embedded HQL: `matches = hql """ ... """ on <expr>`
- Union input types: `PE64 | ELF64`

---

## Roadmap (high level)

- [ ] Wire real HexCore backends (Pathfinder, Helix, Elixir, Remill)
- [ ] Full HQL integration via hexcore-hql
- [ ] Formatter (`hkl fmt`) and export (`yara` / `sigma`)
- [ ] Language server / IDE hooks
- [ ] Session persistence and multi-pipeline programs

---

## License

Proprietary — © Akasha Corporation. Internal HexCore component.

---

## Related

- **HexCore** ecosystem (HQL, Pythia, decompiler, IDE) — sibling repositories under [AkashaCorporation](https://github.com/AkashaCorporation)
