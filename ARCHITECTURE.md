# Architecture

## Overview

`rapidraw-mcp` is a thin **stdio MCP server** that translates tool calls into
HTTP requests against RapidRAW’s External Control API.

```
┌─────────────┐   JSON-RPC/stdio    ┌────────────────┐   HTTP localhost   ┌──────────────┐
│ MCP client  │ ──────────────────► │  rapidraw-mcp  │ ─────────────────► │ RapidRAW /v1 │
│ (Cursor…)   │ ◄────────────────── │  (rmcp 3.x)    │ ◄───────────────── │ :17355       │
└─────────────┘   ContentBlock(s)   └────────────────┘   {ok,data,error}  └──────────────┘
```

## Modules

| Module | Role |
|--------|------|
| `config` | Env: `RAPIDRAW_API_URL`, `RAPIDRAW_API_TOKEN`, `RAPIDRAW_TIMEOUT_SECS` |
| `api_client` | Typed `reqwest` wrapper over `/v1/*`; envelope parse; preview edge cap |
| `error` | API errors → MCP `ErrorData` |
| `tools` | `#[tool]` handlers via `rmcp` macros; one tool per logical operation |
| `main` | stderr tracing + `serve(stdio())` |

## Transport rules

- **stdout**: MCP JSON-RPC only (owned by `rmcp` stdio transport)
- **stderr**: `tracing` logs (`RUST_LOG` / default INFO)

## Response shape

RapidRAW returns:

```json
{ "ok": true, "data": { … } }
{ "ok": false, "error": { "code": "…", "message": "…" } }
```

The client unwraps `data` on success and maps `error` into MCP tool errors.

## Preview special case

`rapidraw_get_preview` returns **two** content blocks:

1. `image/jpeg` (`ContentBlock::image` with base64 payload)
2. text JSON meta (`width`, `height`, `mimeType`, `sourcePath`, `maxEdgeApplied`)

`maxEdge` is clamped to **1…2048** (default **1024**), matching RapidRAW server limits.

## AI / mask workflows

Suggested agent loops (always end with `rapidraw_get_preview`):

```
ai_status ──► create_ai_mask(sky|subject|…) ──► adjust_mask ──► get_preview ──► …
ai_status ──► ai_erase(quick|generative) ──► get_preview ──► undo?
ai_denoise? ──► get_preview
```

HTTP surface (added alongside core `/v1` editor routes):

| Path | Purpose |
|------|---------|
| `POST /v1/masks/ai` | Create AI mask |
| `POST /v1/masks/adjust` | Local adjustments on mask |
| `POST /v1/ai/erase` | Quick / generative erase |
| `GET /v1/ai/status` | Model readiness |
| `POST /v1/ai/denoise` | Optional denoise |

## Why a separate process?
- Keeps RapidRAW itself free of MCP protocol surface
- Stdio subprocess model matches Cursor / Claude Desktop
- Token and URL stay in the MCP host env, not in the photo app UI path
