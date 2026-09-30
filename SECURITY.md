# Security

## Trust boundary

RapidRAW External Control listens on **127.0.0.1 only**. `rapidraw-mcp` is a
local stdio bridge: any MCP client that can launch this binary can call whatever
the RapidRAW API allows for that session.

## Recommendations

1. **Enable only when needed** — turn External Control off in RapidRAW Settings when idle.
2. **Use a bearer token** — set a token in RapidRAW and pass it as `RAPIDRAW_API_TOKEN` to the MCP process.
3. **Do not expose the API** — never port-forward or bind RapidRAW External Control to non-loopback interfaces.
4. **Path allowlists** — RapidRAW rejects paths outside configured library roots (`path_forbidden`). Do not weaken that on the app side for MCP convenience.
5. **Secrets in MCP config** — prefer env vars / OS secret stores over committing tokens into `mcp.json`.
6. **Logs** — this server logs to stderr; avoid enabling debug logs that dump image base64 or tokens.

## What this server does *not* do

- No remote HTTP MCP endpoint (stdio only)
- No filesystem access of its own (all paths go through RapidRAW validation)
- No privilege escalation beyond the RapidRAW process permissions

## Supply chain

Depends on official `rmcp` (Model Context Protocol Rust SDK) and `reqwest` with
`rustls-tls`. Pin / audit versions before distribution.
