use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::ApiError;

const MAX_PREVIEW_EDGE: u32 = 2048;
const DEFAULT_PREVIEW_EDGE: u32 = 1024;

/// HTTP client for RapidRAW External Control API (`/v1/*`).
#[derive(Clone)]
pub struct RapidRawClient {
    http: Client,
    base_url: String,
    token: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ApiEnvelope {
    ok: bool,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<ApiErrorBody>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ApiErrorBody {
    code: String,
    message: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewData {
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub data_base64: String,
    #[serde(default)]
    pub source_path: Option<String>,
}

impl RapidRawClient {
    pub fn new(config: &Config) -> Result<Self, ApiError> {
        let http = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| ApiError::new("client_build", e.to_string()))?;

        Ok(Self {
            http,
            base_url: format!("{}/v1", config.api_url.trim_end_matches('/')),
            token: config.api_token.clone(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn cap_max_edge(max_edge: Option<u32>) -> u32 {
        max_edge
            .unwrap_or(DEFAULT_PREVIEW_EDGE)
            .clamp(1, MAX_PREVIEW_EDGE)
    }

    async fn request<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(String, String)]>,
        body: Option<&B>,
    ) -> Result<Value, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let mut builder = self.http.request(method, &url);

        if let Some(token) = &self.token {
            builder = builder.bearer_auth(token);
        }
        if let Some(q) = query {
            builder = builder.query(q);
        }
        if let Some(b) = body {
            builder = builder.json(b);
        }

        let response = builder.send().await.map_err(|e| {
            if e.is_timeout() {
                ApiError::new("timeout", format!("Request timed out: {e}"))
            } else if e.is_connect() {
                ApiError::new(
                    "connection_refused",
                    format!(
                        "Cannot connect to RapidRAW at {}. Is External Control enabled? ({e})",
                        self.base_url
                    ),
                )
            } else {
                ApiError::new("transport_error", e.to_string())
            }
        })?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ApiError::new("read_body", e.to_string()))?;

        self.parse_envelope(status, &text)
    }

    fn parse_envelope(&self, status: StatusCode, text: &str) -> Result<Value, ApiError> {
        if text.trim().is_empty() {
            return Err(ApiError::new(
                "empty_response",
                format!("Empty response (HTTP {})", status.as_u16()),
            )
            .with_status(status.as_u16()));
        }

        let envelope: ApiEnvelope = serde_json::from_str(text).map_err(|e| {
            ApiError::new(
                "invalid_json",
                format!("Failed to parse API response: {e}; body={text}"),
            )
            .with_status(status.as_u16())
        })?;

        if !envelope.ok {
            let err = envelope.error.unwrap_or(ApiErrorBody {
                code: "unknown".into(),
                message: "API returned ok=false without error body".into(),
            });
            return Err(ApiError::new(err.code, err.message).with_status(status.as_u16()));
        }

        if !status.is_success() {
            return Err(ApiError::new(
                "http_error",
                format!("Unexpected HTTP {} with ok=true", status.as_u16()),
            )
            .with_status(status.as_u16()));
        }

        Ok(envelope.data)
    }

    async fn get(&self, path: &str, query: Option<&[(String, String)]>) -> Result<Value, ApiError> {
        self.request::<Value>(Method::GET, path, query, None).await
    }

    async fn post<B: Serialize>(&self, path: &str, body: &B) -> Result<Value, ApiError> {
        self.request(Method::POST, path, None, Some(body)).await
    }

    async fn post_empty(&self, path: &str) -> Result<Value, ApiError> {
        self.post(path, &json!({})).await
    }

    pub async fn get_status(&self) -> Result<Value, ApiError> {
        self.get("/status", None).await
    }

    pub async fn get_current_image(&self) -> Result<Value, ApiError> {
        self.get("/current-image", None).await
    }

    pub async fn get_image_metadata(&self, path: &str) -> Result<Value, ApiError> {
        self.get(
            "/images/metadata",
            Some(&[(("path".into()), path.to_string())]),
        )
        .await
    }

    pub async fn get_adjustments(&self) -> Result<Value, ApiError> {
        self.get("/adjustments", None).await
    }

    pub async fn get_adjustments_schema(&self) -> Result<Value, ApiError> {
        self.get("/adjustments/schema", None).await
    }

    pub async fn get_selection(&self) -> Result<Value, ApiError> {
        self.get("/selection", None).await
    }

    pub async fn get_preview(
        &self,
        max_edge: Option<u32>,
        path: Option<&str>,
    ) -> Result<PreviewData, ApiError> {
        let edge = Self::cap_max_edge(max_edge);
        let mut query = vec![("maxEdge".into(), edge.to_string())];
        if let Some(p) = path {
            query.push(("path".into(), p.to_string()));
        }
        let data = self.get("/preview", Some(&query)).await?;
        serde_json::from_value(data).map_err(|e| {
            ApiError::new("invalid_preview", format!("Preview response decode failed: {e}"))
        })
    }

    pub async fn set_adjustment(&self, key: &str, value: Value) -> Result<Value, ApiError> {
        self.post("/adjustments/set", &json!({ "key": key, "value": value }))
            .await
    }

    pub async fn adjust_parameter(&self, key: &str, delta: f64) -> Result<Value, ApiError> {
        self.post(
            "/adjustments/adjust",
            &json!({ "key": key, "delta": delta }),
        )
        .await
    }

    pub async fn list_presets(&self) -> Result<Value, ApiError> {
        self.get("/presets", None).await
    }

    pub async fn apply_preset(&self, name: &str) -> Result<Value, ApiError> {
        self.post("/presets/apply", &json!({ "name": name })).await
    }

    pub async fn save_preset(&self, name: &str) -> Result<Value, ApiError> {
        self.post("/presets/save", &json!({ "name": name })).await
    }

    /// Import presets from local absolute paths and/or an http(s) URL.
    pub async fn import_presets(
        &self,
        paths: &[String],
        url: Option<&str>,
    ) -> Result<Value, ApiError> {
        let mut body = json!({ "paths": paths });
        if let Some(u) = url {
            body["url"] = Value::String(u.to_string());
        }
        self.post("/presets/import", &body).await
    }

    pub async fn list_community_presets(&self) -> Result<Value, ApiError> {
        self.get("/presets/community", None).await
    }

    pub async fn import_community_preset(&self, name: &str) -> Result<Value, ApiError> {
        self.post("/presets/community/import", &json!({ "name": name }))
            .await
    }

    pub async fn undo(&self) -> Result<Value, ApiError> {
        self.post_empty("/undo").await
    }

    pub async fn redo(&self) -> Result<Value, ApiError> {
        self.post_empty("/redo").await
    }

    pub async fn reset_adjustments(&self) -> Result<Value, ApiError> {
        self.post_empty("/adjustments/reset").await
    }

    pub async fn list_images(&self, folder: &str, recursive: bool) -> Result<Value, ApiError> {
        self.get(
            "/images",
            Some(&[
                ("folder".into(), folder.to_string()),
                ("recursive".into(), recursive.to_string()),
            ]),
        )
        .await
    }

    pub async fn open_image(&self, path: &str) -> Result<Value, ApiError> {
        self.post("/images/open", &json!({ "path": path })).await
    }

    pub async fn next_image(&self) -> Result<Value, ApiError> {
        self.post_empty("/images/next").await
    }

    pub async fn previous_image(&self) -> Result<Value, ApiError> {
        self.post_empty("/images/previous").await
    }

    pub async fn rate_image(&self, path: Option<&str>, rating: i8) -> Result<Value, ApiError> {
        let mut body = json!({ "rating": rating });
        if let Some(p) = path {
            body["path"] = Value::String(p.to_string());
        }
        self.post("/rating", &body).await
    }

    pub async fn set_color_label(
        &self,
        path: Option<&str>,
        label: &str,
    ) -> Result<Value, ApiError> {
        let mut body = json!({ "label": label });
        if let Some(p) = path {
            body["path"] = Value::String(p.to_string());
        }
        self.post("/color-label", &body).await
    }

    pub async fn copy_adjustments(&self) -> Result<Value, ApiError> {
        self.post_empty("/adjustments/copy").await
    }

    pub async fn paste_adjustments(&self) -> Result<Value, ApiError> {
        self.post_empty("/adjustments/paste").await
    }

    pub async fn apply_adjustments_to_images(&self, paths: &[String]) -> Result<Value, ApiError> {
        self.post("/adjustments/apply-to", &json!({ "paths": paths }))
            .await
    }

    pub async fn export_image(
        &self,
        path: Option<&str>,
        output_path: &str,
        format: Option<&str>,
        quality: Option<u8>,
    ) -> Result<Value, ApiError> {
        let mut body = json!({ "outputPath": output_path });
        if let Some(p) = path {
            body["path"] = Value::String(p.to_string());
        }
        if let Some(f) = format {
            body["format"] = Value::String(f.to_string());
        }
        if let Some(q) = quality {
            body["quality"] = json!(q);
        }
        self.post("/export", &body).await
    }

    pub async fn export_batch(
        &self,
        paths: &[String],
        output_folder: &str,
        format: Option<&str>,
        quality: Option<u8>,
    ) -> Result<Value, ApiError> {
        let mut body = json!({
            "paths": paths,
            "outputFolder": output_folder,
        });
        if let Some(f) = format {
            body["format"] = Value::String(f.to_string());
        }
        if let Some(q) = quality {
            body["quality"] = json!(q);
        }
        self.post("/export/batch", &body).await
    }

    /// Create an AI mask (`sky` | `foreground` | `subject` | `depth`).
    /// Optional normalized box: `{ x, y, width, height }` in 0..1 image coords.
    pub async fn create_ai_mask(
        &self,
        mask_type: &str,
        box_region: Option<Value>,
    ) -> Result<Value, ApiError> {
        let mut body = json!({ "type": mask_type });
        if let Some(b) = box_region {
            body["box"] = b;
        }
        self.post("/masks/ai", &body).await
    }

    /// Apply local adjustments to the active mask (or named maskId).
    pub async fn adjust_mask(
        &self,
        adjustments: Value,
        mask_id: Option<&str>,
    ) -> Result<Value, ApiError> {
        let mut body = json!({ "adjustments": adjustments });
        if let Some(id) = mask_id {
            body["maskId"] = Value::String(id.to_string());
        }
        self.post("/masks/adjust", &body).await
    }

    /// AI erase / generative replace. `mode`: `quick` | `generative`.
    pub async fn ai_erase(
        &self,
        mode: &str,
        prompt: Option<&str>,
    ) -> Result<Value, ApiError> {
        let mut body = json!({ "mode": mode });
        if let Some(p) = prompt {
            body["prompt"] = Value::String(p.to_string());
        }
        self.post("/ai/erase", &body).await
    }

    /// Readiness of ONNX models, AI connector, and cloud features.
    pub async fn ai_status(&self) -> Result<Value, ApiError> {
        self.get("/ai/status", None).await
    }

    /// Optional AI denoise. Strength 0..1 when provided; method optional.
    pub async fn ai_denoise(
        &self,
        strength: Option<f64>,
        method: Option<&str>,
    ) -> Result<Value, ApiError> {
        let mut body = json!({});
        if let Some(s) = strength {
            body["strength"] = json!(s);
        }
        if let Some(m) = method {
            body["method"] = Value::String(m.to_string());
        }
        self.post("/ai/denoise", &body).await
    }
}

/// Decode JSON Value into a typed structure (used by unit-style helpers).
pub fn decode_data<T: DeserializeOwned>(data: Value) -> Result<T, ApiError> {
    serde_json::from_value(data).map_err(|e| ApiError::new("decode", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cap_max_edge_defaults_and_clamps() {
        assert_eq!(RapidRawClient::cap_max_edge(None), 1024);
        assert_eq!(RapidRawClient::cap_max_edge(Some(512)), 512);
        assert_eq!(RapidRawClient::cap_max_edge(Some(9999)), 2048);
        assert_eq!(RapidRawClient::cap_max_edge(Some(0)), 1);
    }

    #[test]
    fn parse_ok_envelope() {
        let client = RapidRawClient {
            http: Client::new(),
            base_url: "http://127.0.0.1:17355/v1".into(),
            token: None,
        };
        let body = r#"{"ok":true,"data":{"enabled":true,"port":17355}}"#;
        let data = client
            .parse_envelope(StatusCode::OK, body)
            .expect("ok envelope");
        assert_eq!(data["enabled"], json!(true));
        assert_eq!(data["port"], json!(17355));
    }

    #[test]
    fn parse_error_envelope() {
        let client = RapidRawClient {
            http: Client::new(),
            base_url: "http://127.0.0.1:17355/v1".into(),
            token: None,
        };
        let body = r#"{"ok":false,"error":{"code":"no_image","message":"No image open"}}"#;
        let err = client
            .parse_envelope(StatusCode::BAD_REQUEST, body)
            .expect_err("error envelope");
        assert_eq!(err.code, "no_image");
        assert!(err.message.contains("No image"));
        assert_eq!(err.http_status, Some(400));
    }

    #[test]
    fn parse_invalid_json() {
        let client = RapidRawClient {
            http: Client::new(),
            base_url: "http://127.0.0.1:17355/v1".into(),
            token: None,
        };
        let err = client
            .parse_envelope(StatusCode::OK, "not-json")
            .expect_err("bad json");
        assert_eq!(err.code, "invalid_json");
    }

    #[test]
    fn preview_data_deserializes_camel_case() {
        let raw = json!({
            "width": 800,
            "height": 600,
            "mimeType": "image/jpeg",
            "dataBase64": "abc",
            "sourcePath": "/tmp/a.raw"
        });
        let preview: PreviewData = serde_json::from_value(raw).unwrap();
        assert_eq!(preview.width, 800);
        assert_eq!(preview.mime_type, "image/jpeg");
        assert_eq!(preview.source_path.as_deref(), Some("/tmp/a.raw"));
    }
}
