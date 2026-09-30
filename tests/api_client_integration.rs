use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rapidraw_mcp::{Config, RapidRawClient};

fn client_for(server: &MockServer) -> RapidRawClient {
    let config = Config::default()
        .with_url(server.uri())
        .with_token(Some("test-token".into()))
        .with_timeout(std::time::Duration::from_secs(5));
    RapidRawClient::new(&config).expect("client")
}

#[tokio::test]
async fn get_status_ok() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/status"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": {
                "protocolVersion": "1.0.0",
                "enabled": true,
                "port": 17355
            }
        })))
        .mount(&server)
        .await;

    let data = client_for(&server).get_status().await.unwrap();
    assert_eq!(data["protocolVersion"], "1.0.0");
    assert_eq!(data["enabled"], true);
}

#[tokio::test]
async fn get_status_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/status"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "ok": false,
            "error": { "code": "unauthorized", "message": "bad token" }
        })))
        .mount(&server)
        .await;

    let err = client_for(&server).get_status().await.unwrap_err();
    assert_eq!(err.code, "unauthorized");
    assert_eq!(err.http_status, Some(401));
}

#[tokio::test]
async fn set_adjustment_posts_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/adjustments/set"))
        .and(body_partial_json(json!({ "key": "exposure", "value": 0.5 })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": { "key": "exposure", "value": 0.5 }
        })))
        .mount(&server)
        .await;

    let data = client_for(&server)
        .set_adjustment("exposure", json!(0.5))
        .await
        .unwrap();
    assert_eq!(data["key"], "exposure");
}

#[tokio::test]
async fn get_preview_caps_edge_and_decodes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/preview"))
        .and(query_param("maxEdge", "2048"))
        .and(query_param("path", "/Photos/a.raw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": {
                "width": 1024,
                "height": 768,
                "mimeType": "image/jpeg",
                "dataBase64": "qqqq",
                "sourcePath": "/Photos/a.raw"
            }
        })))
        .mount(&server)
        .await;

    let preview = client_for(&server)
        .get_preview(Some(99999), Some("/Photos/a.raw"))
        .await
        .unwrap();
    assert_eq!(preview.width, 1024);
    assert_eq!(preview.mime_type, "image/jpeg");
    assert_eq!(preview.data_base64, "qqqq");
}

#[tokio::test]
async fn list_images_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/images"))
        .and(query_param("folder", "/Photos"))
        .and(query_param("recursive", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": { "images": ["/Photos/a.raw", "/Photos/b.raw"] }
        })))
        .mount(&server)
        .await;

    let data = client_for(&server)
        .list_images("/Photos", true)
        .await
        .unwrap();
    assert_eq!(data["images"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn export_and_batch() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/export"))
        .and(body_partial_json(json!({
            "outputPath": "/Photos/out.jpg",
            "format": "jpeg"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": { "output": "/Photos/out.jpg" }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/export/batch"))
        .and(body_partial_json(json!({
            "paths": ["/Photos/a.raw"],
            "outputFolder": "/Photos/export"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": { "output": "/Photos/export" }
        })))
        .mount(&server)
        .await;

    let c = client_for(&server);
    let one = c
        .export_image(None, "/Photos/out.jpg", Some("jpeg"), Some(90))
        .await
        .unwrap();
    assert_eq!(one["output"], "/Photos/out.jpg");

    let batch = c
        .export_batch(
            &["/Photos/a.raw".into()],
            "/Photos/export",
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(batch["output"], "/Photos/export");
}

#[tokio::test]
async fn undo_redo_empty_post() {
    let server = MockServer::start().await;
    for p in ["/v1/undo", "/v1/redo", "/v1/adjustments/reset"] {
        Mock::given(method("POST"))
            .and(path(p))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ok": true,
                "data": {}
            })))
            .mount(&server)
            .await;
    }

    let c = client_for(&server);
    c.undo().await.unwrap();
    c.redo().await.unwrap();
    c.reset_adjustments().await.unwrap();
}

#[tokio::test]
async fn connection_refused_maps_error() {
    let config = Config::default()
        .with_url("http://127.0.0.1:1")
        .with_timeout(std::time::Duration::from_millis(200));
    let client = RapidRawClient::new(&config).unwrap();
    let err = client.get_status().await.unwrap_err();
    assert_eq!(err.code, "connection_refused");
}

#[tokio::test]
async fn ai_mask_and_erase_endpoints() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/masks/ai"))
        .and(body_partial_json(json!({
            "type": "sky",
            "box": { "x": 0.1, "y": 0.0, "width": 0.8, "height": 0.4 }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": { "maskId": "m1", "type": "sky" }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/masks/adjust"))
        .and(body_partial_json(json!({
            "adjustments": { "exposure": -0.3 },
            "maskId": "m1"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": { "maskId": "m1" }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/ai/erase"))
        .and(body_partial_json(json!({
            "mode": "generative",
            "prompt": "remove power lines"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": { "jobId": "e1" }
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/ai/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": {
                "onnxReady": true,
                "connectorReady": true,
                "cloudReady": false
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/ai/denoise"))
        .and(body_partial_json(json!({ "strength": 0.5, "method": "ai" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "data": { "applied": true }
        })))
        .mount(&server)
        .await;

    let c = client_for(&server);

    let status = c.ai_status().await.unwrap();
    assert_eq!(status["onnxReady"], true);

    let mask = c
        .create_ai_mask(
            "sky",
            Some(json!({ "x": 0.1, "y": 0.0, "width": 0.8, "height": 0.4 })),
        )
        .await
        .unwrap();
    assert_eq!(mask["maskId"], "m1");

    let adj = c
        .adjust_mask(json!({ "exposure": -0.3 }), Some("m1"))
        .await
        .unwrap();
    assert_eq!(adj["maskId"], "m1");

    let erase = c
        .ai_erase("generative", Some("remove power lines"))
        .await
        .unwrap();
    assert_eq!(erase["jobId"], "e1");

    let denoise = c.ai_denoise(Some(0.5), Some("ai")).await.unwrap();
    assert_eq!(denoise["applied"], true);
}

