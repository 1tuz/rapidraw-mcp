# rapidraw-mcp

MCP server that bridges AI clients (Cursor, Claude Desktop, …) to
[RapidRAW](https://github.com/CyberTimon/RapidRAW) via its localhost
**External Control** HTTP API (`/v1`).

```
MCP client  --stdio-->  rapidraw-mcp  --HTTP-->  RapidRAW :17355/v1
```

## Requirements

- Rust 1.88+ (edition 2021; `rmcp` 3.x MSRV)
- RapidRAW with **External Control** enabled in Settings
- Optional bearer token if configured in RapidRAW

## Build

```bash
cargo build --release
# binary: target/release/rapidraw-mcp
```

## Run

```bash
export RAPIDRAW_API_URL=http://127.0.0.1:17355   # default
# export RAPIDRAW_API_TOKEN=…                    # optional
# export RAPIDRAW_TIMEOUT_SECS=30                # default
./target/release/rapidraw-mcp
```

Logs go to **stderr** only. **stdout** is MCP JSON-RPC (do not redirect casually).

## Cursor

See [`examples/cursor-mcp.json`](examples/cursor-mcp.json). Merge into your
Cursor MCP config (`~/.cursor/mcp.json` or project `.cursor/mcp.json`).

## Claude Desktop

See [`examples/claude-desktop.json`](examples/claude-desktop.json).

## Tools

Full list: [`docs/TOOLS.md`](docs/TOOLS.md). Names are prefixed `rapidraw_*`:

- Library / editor: status, preview, adjustments, presets, navigation, rating, export
- **AI / masks (must-have workflows):**
  - `rapidraw_create_ai_mask` → sky / subject / foreground / depth (+ optional box)
  - `rapidraw_adjust_mask` → local adjustments inside active mask
  - `rapidraw_ai_erase` → `quick` or `generative` (remove trees/wires/people)
  - `rapidraw_ai_status` → ONNX / connector / cloud readiness
  - `rapidraw_ai_denoise` → optional denoise
  - Always close the loop with `rapidraw_get_preview`

## Tests

```bash
cargo test
./scripts/e2e_smoke.sh   # skips if RapidRAW is not running
```

## Docs

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — data flow and modules
- [`SECURITY.md`](SECURITY.md) — localhost binding, tokens, path rules
- [`docs/TOOLS.md`](docs/TOOLS.md) — tool ↔ HTTP mapping

## License

MIT
