//! Lightweight JSON output validation for `nexus run --json-schema`.

use std::path::Path;

/// Append instruction so the model returns JSON only.
pub fn wrap_prompt_for_json(prompt: &str, schema: &serde_json::Value) -> String {
    format!(
        "{prompt}\n\n\
         [Output contract] Reply with a single valid JSON object only (no markdown fences). \
         It must satisfy this JSON Schema:\n```json\n{}\n```",
        serde_json::to_string_pretty(schema).unwrap_or_else(|_| schema.to_string())
    )
}

/// Extract JSON object from assistant text (raw or ```json fence).
pub fn extract_json(text: &str) -> anyhow::Result<serde_json::Value> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') {
        return Ok(serde_json::from_str(trimmed)?);
    }
    if let Some(start) = trimmed.find("```json") {
        let rest = &trimmed[start + 7..];
        if let Some(end) = rest.find("```") {
            return Ok(serde_json::from_str(rest[..end].trim())?);
        }
    }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return Ok(serde_json::from_str(&trimmed[start..=end])?);
        }
    }
    anyhow::bail!("no JSON object found in model output")
}

/// Full validation via engine `jsonschema` when `engine_url` is set.
pub async fn validate_via_engine(
    engine_url: &str,
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> anyhow::Result<()> {
    let url = format!("{}/v1/validate/json-schema", engine_url.trim_end_matches('/'));
    let res = reqwest::Client::new()
        .post(url)
        .json(&serde_json::json!({ "data": value, "schema": schema }))
        .send()
        .await?;
    let body: serde_json::Value = res.json().await?;
    if body.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(());
    }
    anyhow::bail!(
        "{}",
        body.get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("validation failed")
    )
}

/// Validate required keys and primitive types from a JSON Schema draft subset.
pub fn validate_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> anyhow::Result<()> {
    if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
        if !value.is_object() {
            anyhow::bail!("expected JSON object");
        }
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            let obj = value.as_object().unwrap();
            for key in required {
                if let Some(k) = key.as_str() {
                    if !obj.contains_key(k) {
                        anyhow::bail!("missing required field: {k}");
                    }
                }
            }
        }
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            let obj = value.as_object().unwrap();
            for (key, prop_schema) in props {
                if let Some(v) = obj.get(key) {
                    check_type(v, prop_schema)?;
                }
            }
        }
    }
    Ok(())
}

fn check_type(value: &serde_json::Value, prop_schema: &serde_json::Value) -> anyhow::Result<()> {
    let Some(expected) = prop_schema.get("type").and_then(|t| t.as_str()) else {
        return Ok(());
    };
    let ok = match expected {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => true,
    };
    if !ok {
        anyhow::bail!("field type mismatch: expected {expected}");
    }
    Ok(())
}

pub fn load_schema(path: &Path) -> anyhow::Result<serde_json::Value> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_from_fence() {
        let t = "Here:\n```json\n{\"ok\": true}\n```";
        let v = extract_json(t).unwrap();
        assert_eq!(v["ok"], true);
    }

    #[test]
    fn validate_required_field() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": { "name": { "type": "string" } }
        });
        validate_against_schema(&serde_json::json!({"name": "x"}), &schema).unwrap();
        assert!(validate_against_schema(&serde_json::json!({}), &schema).is_err());
    }
}
