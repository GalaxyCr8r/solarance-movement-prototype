# SpacetimeDB Module

## IMPORTANT: Do not run `cargo` in this directory

This is a SpacetimeDB WASM module. Running `cargo build` or `cargo check` directly will fail with linker errors because the SpacetimeDB ABI symbols (`_bytes_sink_write`, `_console_log`, etc.) are only available inside the WASM runtime — not on the host.

**To build and check for errors:**

```bash
spacetime build --module-path spacetimedb
```

**To publish:**

```bash
spacetime publish <db-name> --module-path spacetimedb
```

**To clear and republish:**

```bash
spacetime publish <db-name> --clear-database -y --module-path spacetimedb
```

**To view logs:**

```bash
spacetime logs <db-name>
```
