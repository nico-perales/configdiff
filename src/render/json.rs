//! Machine-readable JSON rendering of a diff.

use serde_json::{Map, Value as Json, json};

use crate::diff::{Change, ChangeKind, Diff};

#[must_use]
pub fn render(diff: &Diff, pretty: bool) -> String {
    let s = diff.summary();
    let total = s.added + s.removed + s.changed + s.type_changed;

    let changes: Vec<Json> = diff.changes().iter().map(change_to_json).collect();

    let doc = json!({
        "summary": {
            "added": s.added,
            "removed": s.removed,
            "changed": s.changed,
            "type_changed": s.type_changed,
            "total": total,
        },
        "changes": changes,
    });

    if pretty {
        serde_json::to_string_pretty(&doc).unwrap_or_default()
    } else {
        serde_json::to_string(&doc).unwrap_or_default()
    }
}

fn change_to_json(change: &Change) -> Json {
    let mut obj = Map::new();
    obj.insert("path".into(), Json::String(change.path.to_string()));
    match &change.kind {
        ChangeKind::Added { new } => {
            obj.insert("kind".into(), "added".into());
            obj.insert("new".into(), to_json(new));
        }
        ChangeKind::Removed { old } => {
            obj.insert("kind".into(), "removed".into());
            obj.insert("old".into(), to_json(old));
        }
        ChangeKind::Changed { old, new } => {
            obj.insert("kind".into(), "changed".into());
            obj.insert("old".into(), to_json(old));
            obj.insert("new".into(), to_json(new));
        }
        ChangeKind::TypeChanged { old, new } => {
            obj.insert("kind".into(), "type_changed".into());
            obj.insert("old_type".into(), old.type_name().into());
            obj.insert("new_type".into(), new.type_name().into());
            obj.insert("old".into(), to_json(old));
            obj.insert("new".into(), to_json(new));
        }
    }
    Json::Object(obj)
}

fn to_json(value: &crate::Value) -> Json {
    serde_json::to_value(value).unwrap_or(Json::Null)
}
