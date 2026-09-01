//! Field accessors for Grok ACP session-update JSON.

use serde_json::Value;

/// The ACP update object: either the line itself or `params.update`.
pub(crate) fn update_from_line(entry: &Value) -> Option<&Value> {
    entry
        .pointer("/params/update")
        .or_else(|| entry.get("sessionUpdate").is_some().then_some(entry))
}

pub(crate) fn text_content(update: &Value) -> &str {
    update
        .pointer("/content/text")
        .and_then(Value::as_str)
        .unwrap_or("")
}

pub(crate) fn tool_name(update: &Value) -> &str {
    update
        .get("_meta")
        .and_then(|meta| meta.get("x.ai/tool"))
        .and_then(|tool| tool.get("name"))
        .and_then(Value::as_str)
        .or_else(|| update.get("toolName").and_then(Value::as_str))
        .or_else(|| update.get("title").and_then(Value::as_str))
        .unwrap_or("")
}

pub(crate) fn tool_name_or_tool(update: &Value) -> &str {
    match tool_name(update) {
        "" => "tool",
        name => name,
    }
}

pub(crate) fn tool_output_text(update: &Value) -> String {
    if let Some(arr) = update.get("content").and_then(Value::as_array) {
        let texts: Vec<&str> = arr
            .iter()
            .filter_map(|item| {
                item.pointer("/content/text")
                    .or_else(|| item.get("text"))
                    .and_then(Value::as_str)
            })
            .filter(|text| !text.is_empty())
            .collect();
        if !texts.is_empty() {
            return texts.join("\n");
        }
    }
    update
        .get("rawOutput")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
