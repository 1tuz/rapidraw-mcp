use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::wrapper::Parameters,
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api_client::RapidRawClient;

fn ok_json(data: Value) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string()),
    )]))
}

fn map_api(err: crate::error::ApiError) -> McpError {
    err.into_mcp()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PathArg {
    #[schemars(description = "Absolute filesystem path to an image")]
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PreviewArgs {
    #[schemars(description = "Max edge length in pixels (capped at 2048, default 1024)")]
    pub max_edge: Option<u32>,
    #[schemars(description = "Optional absolute image path; defaults to current open image")]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetAdjustmentArgs {
    #[schemars(description = "Adjustment key (see RapidRAW adjustments schema)")]
    pub key: String,
    #[schemars(description = "New value (number, bool, string, or object depending on key)")]
    pub value: Value,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AdjustParameterArgs {
    #[schemars(description = "Numeric adjustment key")]
    pub key: String,
    #[schemars(description = "Delta to add to the current value")]
    pub delta: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PresetNameArgs {
    #[schemars(description = "Preset name")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImportPresetsArgs {
    #[schemars(
        description = "Absolute local preset file paths (.rrpreset, .xmp, .lrtemplate, .json) under $HOME or temp"
    )]
    pub paths: Option<Vec<String>>,
    #[schemars(
        description = "http(s) URL to a preset file to download and import (max 10MiB; private hosts blocked)"
    )]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListImagesArgs {
    #[schemars(description = "Absolute folder path inside RapidRAW library roots")]
    pub folder: String,
    #[schemars(description = "Recurse into subfolders (default false)")]
    pub recursive: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RateImageArgs {
    #[schemars(description = "Optional absolute path; defaults to current library selection")]
    pub path: Option<String>,
    #[schemars(description = "Star rating 0–5")]
    pub rating: i8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ColorLabelArgs {
    #[schemars(description = "Optional absolute path; defaults to current library selection")]
    pub path: Option<String>,
    #[schemars(description = "Color label string; empty string clears the label")]
    pub label: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApplyToArgs {
    #[schemars(description = "Absolute image paths to receive copied adjustments")]
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportArgs {
    #[schemars(description = "Optional source path; defaults to current open image")]
    pub path: Option<String>,
    #[schemars(description = "Absolute output file path (must be under allowed roots)")]
    pub output_path: String,
    #[schemars(description = "Export format: jpeg, png, tiff, … (default jpeg)")]
    pub format: Option<String>,
    #[schemars(description = "JPEG quality 1–100 (default 92)")]
    pub quality: Option<u8>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportBatchArgs {
    #[schemars(description = "Absolute image paths to export")]
    pub paths: Vec<String>,
    #[schemars(description = "Absolute output folder under allowed roots")]
    pub output_folder: String,
    #[schemars(description = "Export format (default jpeg)")]
    pub format: Option<String>,
    #[schemars(description = "JPEG quality 1–100")]
    pub quality: Option<u8>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BoundingBox {
    #[schemars(description = "Left edge, normalized 0..1")]
    pub x: f64,
    #[schemars(description = "Top edge, normalized 0..1")]
    pub y: f64,
    #[schemars(description = "Width, normalized 0..1")]
    pub width: f64,
    #[schemars(description = "Height, normalized 0..1")]
    pub height: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateAiMaskArgs {
    #[schemars(
        description = "Mask type: sky | foreground | subject | depth"
    )]
    pub r#type: String,
    #[serde(rename = "box", alias = "box_region")]
    #[schemars(
        rename = "box",
        description = "Optional normalized bounding box to guide subject/foreground selection"
    )]
    pub box_region: Option<BoundingBox>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AdjustMaskArgs {
    #[schemars(
        description = "Local adjustment map applied inside the active (or specified) mask, e.g. {\"exposure\":0.3,\"temperature\":10}"
    )]
    pub adjustments: Value,
    #[schemars(description = "Optional mask id; defaults to the active mask")]
    pub mask_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AiEraseArgs {
    #[schemars(description = "Erase mode: quick (inpaint/heal) or generative (prompted replace)")]
    pub mode: String,
    #[schemars(description = "Optional prompt for generative mode, e.g. remove wires / people / trees")]
    pub prompt: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AiDenoiseArgs {
    #[schemars(description = "Denoise strength 0..1 (optional; server default if omitted)")]
    pub strength: Option<f64>,
    #[schemars(description = "Optional method: ai | bm3d | raw9")]
    pub method: Option<String>,
}

#[derive(Clone)]
pub struct RapidRawMcp {
    client: RapidRawClient,
}

#[tool_router]
impl RapidRawMcp {
    pub fn new(client: RapidRawClient) -> Self {
        Self { client }
    }

    #[tool(
        name = "rapidraw_get_status",
        description = "Get RapidRAW External Control status: protocol version, enabled flag, port, and session mirror (current image, undo/redo, selection)."
    )]
    async fn rapidraw_get_status(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.get_status().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_get_current_image",
        description = "Get the currently open image path and whether an image is ready in the editor."
    )]
    async fn rapidraw_get_current_image(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.get_current_image().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_get_image_metadata",
        description = "Load sidecar metadata (including adjustments) for an absolute image path."
    )]
    async fn rapidraw_get_image_metadata(
        &self,
        Parameters(args): Parameters<PathArg>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .get_image_metadata(&args.path)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_get_adjustments",
        description = "Get current editor adjustments for the open image."
    )]
    async fn rapidraw_get_adjustments(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.get_adjustments().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_get_selection",
        description = "Get library selection: selectedPaths and currentFolder."
    )]
    async fn rapidraw_get_selection(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.get_selection().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_get_preview",
        description = "Render a JPEG preview of the current or specified image. Returns an image/jpeg content block plus JSON metadata (width, height, sourcePath). max_edge is capped at 2048."
    )]
    async fn rapidraw_get_preview(
        &self,
        Parameters(args): Parameters<PreviewArgs>,
    ) -> Result<CallToolResult, McpError> {
        let preview = self
            .client
            .get_preview(args.max_edge, args.path.as_deref())
            .await
            .map_err(map_api)?;

        let meta = json!({
            "width": preview.width,
            "height": preview.height,
            "mimeType": preview.mime_type,
            "sourcePath": preview.source_path,
            "maxEdgeApplied": RapidRawClient::cap_max_edge(args.max_edge),
        });

        Ok(CallToolResult::success(vec![
            ContentBlock::image(preview.data_base64, preview.mime_type),
            ContentBlock::text(
                serde_json::to_string_pretty(&meta).unwrap_or_else(|_| meta.to_string()),
            ),
        ]))
    }

    #[tool(
        name = "rapidraw_set_adjustment",
        description = "Set an adjustment key to an absolute value on the current image."
    )]
    async fn rapidraw_set_adjustment(
        &self,
        Parameters(args): Parameters<SetAdjustmentArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .set_adjustment(&args.key, args.value)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_adjust_parameter",
        description = "Nudge a numeric adjustment by delta (relative change)."
    )]
    async fn rapidraw_adjust_parameter(
        &self,
        Parameters(args): Parameters<AdjustParameterArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .adjust_parameter(&args.key, args.delta)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_list_presets",
        description = "List available RapidRAW presets."
    )]
    async fn rapidraw_list_presets(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.list_presets().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_apply_preset",
        description = "Apply a named preset to the current image."
    )]
    async fn rapidraw_apply_preset(
        &self,
        Parameters(args): Parameters<PresetNameArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .apply_preset(&args.name)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_save_preset",
        description = "Save current adjustments as a named preset."
    )]
    async fn rapidraw_save_preset(
        &self,
        Parameters(args): Parameters<PresetNameArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .save_preset(&args.name)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_import_presets",
        description = "Import presets into RapidRAW from local absolute file paths and/or an http(s) URL (.rrpreset/.xmp/.lrtemplate/.json). After import, use rapidraw_list_presets then rapidraw_apply_preset."
    )]
    async fn rapidraw_import_presets(
        &self,
        Parameters(args): Parameters<ImportPresetsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let paths = args.paths.unwrap_or_default();
        ok_json(
            self.client
                .import_presets(&paths, args.url.as_deref())
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_list_community_presets",
        description = "List official RapidRAW community presets from the GitHub manifest (name + creator). Use rapidraw_import_community_preset to install one into the app."
    )]
    async fn rapidraw_list_community_presets(&self) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .list_community_presets()
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_import_community_preset",
        description = "Download/install a community preset by exact name into RapidRAW (from CyberTimon/RapidRAW-Presets). Then apply with rapidraw_apply_preset."
    )]
    async fn rapidraw_import_community_preset(
        &self,
        Parameters(args): Parameters<PresetNameArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .import_community_preset(&args.name)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_undo",
        description = "Undo the last adjustment change in the editor."
    )]
    async fn rapidraw_undo(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.undo().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_redo",
        description = "Redo the last undone adjustment change."
    )]
    async fn rapidraw_redo(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.redo().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_reset_adjustments",
        description = "Reset all adjustments on the current image to defaults."
    )]
    async fn rapidraw_reset_adjustments(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.reset_adjustments().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_list_images",
        description = "List images in a library folder (optionally recursive)."
    )]
    async fn rapidraw_list_images(
        &self,
        Parameters(args): Parameters<ListImagesArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .list_images(&args.folder, args.recursive.unwrap_or(false))
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_open_image",
        description = "Open an absolute image path in the RapidRAW editor."
    )]
    async fn rapidraw_open_image(
        &self,
        Parameters(args): Parameters<PathArg>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(self.client.open_image(&args.path).await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_next_image",
        description = "Navigate to the next image in the current folder/filmstrip."
    )]
    async fn rapidraw_next_image(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.next_image().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_previous_image",
        description = "Navigate to the previous image in the current folder/filmstrip."
    )]
    async fn rapidraw_previous_image(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.previous_image().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_rate_image",
        description = "Set star rating (0–5) on a path or the current library selection."
    )]
    async fn rapidraw_rate_image(
        &self,
        Parameters(args): Parameters<RateImageArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .rate_image(args.path.as_deref(), args.rating)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_set_color_label",
        description = "Set or clear a color label on a path or the current library selection."
    )]
    async fn rapidraw_set_color_label(
        &self,
        Parameters(args): Parameters<ColorLabelArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .set_color_label(args.path.as_deref(), &args.label)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_copy_adjustments",
        description = "Copy adjustments from the current image into RapidRAW clipboard."
    )]
    async fn rapidraw_copy_adjustments(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.copy_adjustments().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_paste_adjustments",
        description = "Paste previously copied adjustments onto the current image."
    )]
    async fn rapidraw_paste_adjustments(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.paste_adjustments().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_apply_adjustments_to_images",
        description = "Apply current (or clipboard) adjustments to a list of absolute image paths."
    )]
    async fn rapidraw_apply_adjustments_to_images(
        &self,
        Parameters(args): Parameters<ApplyToArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .apply_adjustments_to_images(&args.paths)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_export_image",
        description = "Export one image to an absolute output path under allowed library roots."
    )]
    async fn rapidraw_export_image(
        &self,
        Parameters(args): Parameters<ExportArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .export_image(
                    args.path.as_deref(),
                    &args.output_path,
                    args.format.as_deref(),
                    args.quality,
                )
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_export_batch",
        description = "Batch-export multiple images into an absolute output folder."
    )]
    async fn rapidraw_export_batch(
        &self,
        Parameters(args): Parameters<ExportBatchArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .export_batch(
                    &args.paths,
                    &args.output_folder,
                    args.format.as_deref(),
                    args.quality,
                )
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_create_ai_mask",
        description = "Create an AI mask on the current image. type: sky | foreground | subject | depth. Optional normalized box {x,y,width,height} guides subject/foreground. After creating, use rapidraw_adjust_mask for local edits and rapidraw_get_preview to verify (preview loop)."
    )]
    async fn rapidraw_create_ai_mask(
        &self,
        Parameters(args): Parameters<CreateAiMaskArgs>,
    ) -> Result<CallToolResult, McpError> {
        let box_json = args.box_region.map(|b| {
            json!({
                "x": b.x,
                "y": b.y,
                "width": b.width,
                "height": b.height,
            })
        });
        ok_json(
            self.client
                .create_ai_mask(&args.r#type, box_json)
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_adjust_mask",
        description = "Apply local adjustments inside the active AI/manual mask (or mask_id). Pass an adjustments object (exposure, contrast, temperature, …). Follow with rapidraw_get_preview to inspect the result."
    )]
    async fn rapidraw_adjust_mask(
        &self,
        Parameters(args): Parameters<AdjustMaskArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .adjust_mask(args.adjustments, args.mask_id.as_deref())
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_ai_erase",
        description = "Remove objects (trees, wires, people, …) via AI erase. mode=quick for heal/inpaint; mode=generative with optional prompt for generative replace. Operate on the current selection/mask. Always follow with rapidraw_get_preview."
    )]
    async fn rapidraw_ai_erase(
        &self,
        Parameters(args): Parameters<AiEraseArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .ai_erase(&args.mode, args.prompt.as_deref())
                .await
                .map_err(map_api)?,
        )
    }

    #[tool(
        name = "rapidraw_ai_status",
        description = "Check AI readiness: ONNX models, local connector, and cloud features before running mask/erase/denoise workflows."
    )]
    async fn rapidraw_ai_status(&self) -> Result<CallToolResult, McpError> {
        ok_json(self.client.ai_status().await.map_err(map_api)?)
    }

    #[tool(
        name = "rapidraw_ai_denoise",
        description = "Optional AI denoise on the current image. strength 0..1; method ai|bm3d|raw9. Follow with rapidraw_get_preview."
    )]
    async fn rapidraw_ai_denoise(
        &self,
        Parameters(args): Parameters<AiDenoiseArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok_json(
            self.client
                .ai_denoise(args.strength, args.method.as_deref())
                .await
                .map_err(map_api)?,
        )
    }
}

#[tool_handler(
    name = "rapidraw-mcp",
    version = "0.1.0",
    instructions = "MCP bridge to RapidRAW External Control API on localhost. Enable External Control in RapidRAW Settings first. Tools cover library/editor ops plus AI workflows: create_ai_mask → adjust_mask → get_preview; ai_erase (quick|generative); ai_status; ai_denoise. Always use rapidraw_get_preview in a loop after destructive/AI edits. Paths must be absolute and under configured library roots."
)]
impl ServerHandler for RapidRawMcp {}
