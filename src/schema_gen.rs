//! JSON Schema (draft-04) for taplo — generated from Rust types via schemars.

use crate::model::{Config, Entry, EntryOptions, WorktreeOverrides};
use schemars::generate::SchemaSettings;
use schemars::Schema;
use serde_json::{json, Value};

const CONFIG_INIT_KEYS: &[&str] = &[
    "root",
    "windows",
    "worktree_parent",
    "worktree_prefix",
    "entries",
    "worktrees",
];

const ENTRY_OPTION_KEYS: &[&str] = &["dir", "windows", "cmd"];

const WORKTREE_KEYS: &[&str] = &["root", "windows", "worktree_parent"];

/// Root document: `{ "<config-name>": Config, ... }` (serde flatten on Store).
pub fn store_schema_json() -> anyhow::Result<String> {
    let settings = SchemaSettings::draft07();
    let mut generator = settings.into_generator();

    let config = generator.root_schema_for::<Config>();
    let entry = generator.root_schema_for::<Entry>();
    let entry_options = generator.root_schema_for::<EntryOptions>();
    let worktree = generator.root_schema_for::<WorktreeOverrides>();

    let config_value = schema_to_value(config);
    let config_props = config_value
        .get("properties")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let config_block = json!({
        "type": "object",
        "properties": config_props,
        "required": ["entries"],
        "additionalProperties": false,
        "x-taplo": {
            "initKeys": CONFIG_INIT_KEYS,
            "docs": {
                "main": "A tmux-manager config (project). Top-level TOML: `[name]` + `[name.entries]` + optional `[name.worktrees.\"…\"]`."
            }
        }
    });

    let entry_options_value = schema_to_value(entry_options);
    let mut entry_options_props = entry_options_value
        .get("properties")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    inject_init_keys(&mut entry_options_props, ENTRY_OPTION_KEYS);

    let worktree_value = schema_to_value(worktree);
    let mut worktree_props = worktree_value
        .get("properties")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    inject_init_keys(&mut worktree_props, WORKTREE_KEYS);

    let root = json!({
        "$schema": "http://json-schema.org/draft-04/schema#",
        "title": "tmux-manager config",
        "description": "tmux session definitions for tm. Top-level keys are config names (e.g. kuib-ai, portfolios).",
        "type": "object",
        "patternProperties": {
            "^[a-zA-Z][a-zA-Z0-9._-]*$": config_block
        },
        "additionalProperties": false,
        "definitions": {
            "Entry": schema_to_value(entry),
            "EntryOptions": {
                "type": "object",
                "properties": entry_options_props,
                "required": ["dir"],
                "additionalProperties": false,
                "x-taplo": { "initKeys": ENTRY_OPTION_KEYS }
            },
            "WorktreeOverrides": {
                "type": "object",
                "properties": worktree_props,
                "additionalProperties": false,
                "x-taplo": { "initKeys": WORKTREE_KEYS }
            }
        }
    });

    Ok(serde_json::to_string_pretty(&root)?)
}

fn schema_to_value(schema: Schema) -> Value {
    serde_json::to_value(schema).expect("schema serializes to JSON")
}

fn inject_init_keys(props: &mut serde_json::Map<String, Value>, keys: &[&str]) {
    for key in keys {
        if let Some(prop) = props.get_mut(*key) {
            if let Some(obj) = prop.as_object_mut() {
                obj.insert("x-taplo".into(), json!({ "initKeys": keys }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_schema_has_config_fields() {
        let raw = store_schema_json().unwrap();
        let v: Value = serde_json::from_str(&raw).unwrap();
        let cfg = &v["patternProperties"]["^[a-zA-Z][a-zA-Z0-9._-]*$"]["properties"];
        for key in CONFIG_INIT_KEYS {
            assert!(cfg.get(key).is_some(), "missing field {key}");
        }
    }

    #[test]
    fn entry_schema_allows_string_or_object() {
        let raw = store_schema_json().unwrap();
        let v: Value = serde_json::from_str(&raw).unwrap();
        let entry = &v["definitions"]["Entry"];
        let variants = entry
            .get("anyOf")
            .or_else(|| entry.get("oneOf"))
            .and_then(|v| v.as_array())
            .expect("Entry is a union");
        assert!(variants.len() >= 2);
    }
}
