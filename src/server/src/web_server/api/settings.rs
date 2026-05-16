use axum::{http::StatusCode, Json};
use serde_json::{json, Map, Value};

#[derive(serde::Deserialize)]
pub(crate) struct WebVerboseSettingsPatch {
    show_thinking: Option<bool>,
    show_tool_use: Option<bool>,
}

/// GET /api/settings/web -- read web transcript visibility settings.
pub async fn get_web_settings_handler() -> Json<crate::api_types::WebVerboseSettings> {
    Json(web_verbose_settings())
}

/// PATCH /api/settings/web -- update web transcript visibility settings.
pub async fn update_web_settings_handler(
    Json(body): Json<WebVerboseSettingsPatch>,
) -> Result<Json<crate::api_types::WebVerboseSettings>, (StatusCode, String)> {
    common::config::update_settings_json(|root| {
        let channels = object_entry(ensure_object(root), "channels");
        let web = object_entry(channels, "web");
        let verbose = object_entry(web, "verbose");
        if let Some(show_thinking) = body.show_thinking {
            verbose.insert("show_thinking".to_string(), json!(show_thinking));
        }
        if let Some(show_tool_use) = body.show_tool_use {
            verbose.insert("show_tool_use".to_string(), json!(show_tool_use));
        }
    })
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;

    Ok(Json(web_verbose_settings()))
}

fn web_verbose_settings() -> crate::api_types::WebVerboseSettings {
    let cfg = common::config::ensure_loaded();
    let verbose = cfg.channel_verbose("web");
    crate::api_types::WebVerboseSettings {
        show_thinking: verbose.show_thinking,
        show_tool_use: verbose.show_tool_use,
    }
}

fn object_entry<'a>(object: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    let child = object.entry(key.to_string()).or_insert_with(|| json!({}));
    ensure_object(child)
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    value.as_object_mut().expect("value was forced to object")
}
