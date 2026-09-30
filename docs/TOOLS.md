# Tools

Each MCP tool maps 1:1 to a RapidRAW External Control HTTP endpoint under `/v1`.

| MCP tool | HTTP | Notes |
|----------|------|-------|
| `rapidraw_get_status` | `GET /v1/status` | Protocol, port, session mirror |
| `rapidraw_get_current_image` | `GET /v1/current-image` | Current path + ready flag |
| `rapidraw_get_image_metadata` | `GET /v1/images/metadata?path=` | Sidecar metadata |
| `rapidraw_get_adjustments` | `GET /v1/adjustments` | Current editor adjustments |
| `rapidraw_get_selection` | `GET /v1/selection` | selectedPaths, currentFolder |
| `rapidraw_get_preview` | `GET /v1/preview?maxEdge=&path=` | JPEG ContentBlock + JSON meta; edge capped ≤2048 |
| `rapidraw_set_adjustment` | `POST /v1/adjustments/set` | `{ key, value }` |
| `rapidraw_adjust_parameter` | `POST /v1/adjustments/adjust` | `{ key, delta }` |
| `rapidraw_list_presets` | `GET /v1/presets` | |
| `rapidraw_apply_preset` | `POST /v1/presets/apply` | `{ name }` |
| `rapidraw_save_preset` | `POST /v1/presets/save` | `{ name }` |
| `rapidraw_undo` | `POST /v1/undo` | |
| `rapidraw_redo` | `POST /v1/redo` | |
| `rapidraw_reset_adjustments` | `POST /v1/adjustments/reset` | |
| `rapidraw_list_images` | `GET /v1/images?folder=&recursive=` | |
| `rapidraw_open_image` | `POST /v1/images/open` | `{ path }` |
| `rapidraw_next_image` | `POST /v1/images/next` | |
| `rapidraw_previous_image` | `POST /v1/images/previous` | |
| `rapidraw_rate_image` | `POST /v1/rating` | `{ path?, rating }` 0–5 |
| `rapidraw_set_color_label` | `POST /v1/color-label` | `{ path?, label }` |
| `rapidraw_copy_adjustments` | `POST /v1/adjustments/copy` | |
| `rapidraw_paste_adjustments` | `POST /v1/adjustments/paste` | |
| `rapidraw_apply_adjustments_to_images` | `POST /v1/adjustments/apply-to` | `{ paths }` |
| `rapidraw_export_image` | `POST /v1/export` | `{ path?, outputPath, format?, quality? }` |
| `rapidraw_export_batch` | `POST /v1/export/batch` | `{ paths, outputFolder, format?, quality? }` |
| `rapidraw_create_ai_mask` | `POST /v1/masks/ai` | `{ type, box? }` — sky\|foreground\|subject\|depth |
| `rapidraw_adjust_mask` | `POST /v1/masks/adjust` | `{ adjustments, maskId? }` local edits |
| `rapidraw_ai_erase` | `POST /v1/ai/erase` | `{ mode, prompt? }` — quick\|generative |
| `rapidraw_ai_status` | `GET /v1/ai/status` | ONNX / connector / cloud readiness |
| `rapidraw_ai_denoise` | `POST /v1/ai/denoise` | `{ strength?, method? }` optional |

Related (HTTP only, not a separate MCP tool): `GET /v1/adjustments/schema`.

## AI / mask workflows (preview loop)

Always call `rapidraw_ai_status` first when unsure models are ready. After every
mask/erase/denoise mutation, call `rapidraw_get_preview` to inspect.

### Select sky (or subject) and adjust locally

1. `rapidraw_create_ai_mask` — `type: "sky"` (or `subject` / `foreground` / `depth`; optional `box`)
2. `rapidraw_adjust_mask` — e.g. `{ "exposure": -0.4, "highlights": -20 }`
3. `rapidraw_get_preview` — verify; tweak via step 2 again

### Remove trees / wires / people

1. `rapidraw_ai_status` — confirm generative/erase ready
2. Select region (mask) if required by RapidRAW, then `rapidraw_ai_erase`
   - `mode: "quick"` — heal/inpaint
   - `mode: "generative"`, `prompt: "remove power lines"` — prompted replace
3. `rapidraw_get_preview` — verify; undo with `rapidraw_undo` if needed

### Denoise (optional)

1. `rapidraw_ai_denoise` — optional `strength`, `method`
2. `rapidraw_get_preview`

## Envelope

Success: `{ "ok": true, "data": … }`  
Failure: `{ "ok": false, "error": { "code", "message" } }` → MCP tool error.
