# coa-opencode

**Switchable Opencode backend** - Client/Server agent abstraction.

## STRUCTURE

```
src/
├── lib.rs            # AgentService trait + types
├── config.rs         # Provider-agnostic configuration
├── backend.rs        # OpencodeBackend implementation
├── client.rs         # HttpAgentClient
└── examples/
    └── server.rs     # Standalone server example
```

## KEY PATTERNS

### Backend Modes
```rust
// Auto-detect from env (default: daemon)
let backend = OpencodeBackend::from_env();

// Explicit CLI mode
let backend = OpencodeBackend::new(OpencodeBackendMode::Cli {
    opencode_bin: "opencode".into(),
    working_dir: Some(PathBuf::from(".opencode")),
});

// Explicit Daemon mode
let backend = OpencodeBackend::new(OpencodeBackendMode::Daemon {
    client: Client::new(),
    base_url: "http://localhost:8080".into(),
});
```

### Environment Variables
- `OPENCODE_BACKEND_MODE=cli|daemon` (default: daemon)
- `OPENCODE_SERVER_ADDR` (default: http://localhost:8080)
- `OPENCODE_BIN` (default: opencode)
- `OPENCODE_WORKING_DIR` (optional)

### AgentService Trait
```rust
#[async_trait]
impl AgentService for MyBackend {
    async fn list_agents(&self) -> Result<Vec<String>>;
    async fn run_agent(&self, id: &str, input: AgentRunInput) -> Result<AgentRunOutput>;
    // ... etc
}
```

## GAPS / TODO

- **Streaming**: Not yet implemented (returns complete AgentRunOutput)
- **Tool execution**: Types exist but no execution loop
- **Configuration**: Currently reads .opencode/ directly - needs abstraction
- **Error handling**: Uses String errors, needs typed errors

## ANTI-PATTERNS

- **Hardcoded models**: CLI mode has static model list
- **Direct .opencode/ access**: Should use config abstraction
- **String errors**: Use thiserror types instead

## RUNNING

```bash
# Run with daemon mode (default)
cargo run --example server

# Run with CLI mode
OPENCODE_BACKEND_MODE=cli cargo run --example server
```
