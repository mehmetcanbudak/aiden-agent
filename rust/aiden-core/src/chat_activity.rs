//! Per-chat "working" signal (port of `renderer/shared/chat-activity.ts`,
//! `main/services/chat-activity-core.ts` and `renderer/lib/chat-activity.ts`).
//!
//! The sidebar shows which chats have work in flight, including ones the user
//! is not currently looking at. Stream ids make begin/settle idempotent, and
//! counts keep the contract correct if a chat ever hosts more than one kind of
//! background work at once.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::is_safe_subagent_identifier;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChatActivitySnapshot {
    pub revision: u64,
    pub active_chat_ids: Vec<String>,
}

/// `parseChatActivitySnapshot` — reject anything that is not a well-formed
/// snapshot of safe chat identifiers, and de-duplicate what survives.
pub fn parse_chat_activity_snapshot(value: &serde_json::Value) -> Option<ChatActivitySnapshot> {
    let candidate = value.as_object()?;
    let revision = candidate.get("revision")?.as_u64()?;
    let raw = candidate.get("activeChatIds")?.as_array()?;
    let mut seen = BTreeSet::new();
    let mut active_chat_ids = Vec::with_capacity(raw.len());
    for entry in raw {
        if !is_safe_subagent_identifier(entry) {
            return None;
        }
        let id = entry.as_str()?;
        if seen.insert(id.to_string()) {
            active_chat_ids.push(id.to_string());
        }
    }
    Some(ChatActivitySnapshot {
        revision,
        active_chat_ids,
    })
}

/// `ChatActivityRegistry` — projects stream ownership into the activity signal.
///
/// Upstream publishes through an `onChange` callback. Here `begin`/`settle`
/// report whether the visible set changed, so the caller notifies its own
/// observers; that keeps the registry a plain testable value.
#[derive(Debug, Default)]
pub struct ChatActivityRegistry {
    stream_chat_ids: HashMap<String, String>,
    active_stream_counts: BTreeMap<String, usize>,
    revision: u64,
}

impl ChatActivityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim a stream for a chat. Returns true when the chat became active.
    pub fn begin(&mut self, stream_id: &str, chat_id: &str) -> bool {
        match self.stream_chat_ids.get(stream_id) {
            Some(existing) if existing == chat_id => return false,
            Some(_) => {
                self.settle(stream_id);
            }
            None => {}
        }
        self.stream_chat_ids
            .insert(stream_id.to_string(), chat_id.to_string());
        let count = self
            .active_stream_counts
            .entry(chat_id.to_string())
            .or_insert(0);
        *count += 1;
        if *count == 1 {
            self.revision += 1;
            return true;
        }
        false
    }

    /// Release a stream. Returns true when the chat stopped being active.
    pub fn settle(&mut self, stream_id: &str) -> bool {
        let Some(chat_id) = self.stream_chat_ids.remove(stream_id) else {
            return false;
        };
        if let Some(count) = self.active_stream_counts.get_mut(&chat_id) {
            if *count > 1 {
                *count -= 1;
                return false;
            }
        }
        self.active_stream_counts.remove(&chat_id);
        self.revision += 1;
        true
    }

    pub fn snapshot(&self) -> ChatActivitySnapshot {
        ChatActivitySnapshot {
            revision: self.revision,
            active_chat_ids: self.active_stream_counts.keys().cloned().collect(),
        }
    }
}

/// `ChatActivityState` — what the sidebar holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatActivityState {
    pub revision: u64,
    pub active_chat_ids: BTreeSet<String>,
}

impl ChatActivityState {
    pub fn is_working(&self, chat_id: &str) -> bool {
        self.active_chat_ids.contains(chat_id)
    }
}

/// `applyChatActivitySnapshot` — an out-of-order publish is ignored, so a late
/// delivery cannot resurrect a stale set.
pub fn apply_chat_activity_snapshot(
    current: &ChatActivityState,
    snapshot: &ChatActivitySnapshot,
) -> ChatActivityState {
    if snapshot.revision < current.revision {
        return current.clone();
    }
    ChatActivityState {
        revision: snapshot.revision,
        active_chat_ids: snapshot.active_chat_ids.iter().cloned().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chat_is_active_while_any_of_its_streams_is() {
        let mut registry = ChatActivityRegistry::new();
        assert!(registry.begin("stream-1", "chat-a"));
        // A second stream in the same chat does not re-announce it.
        assert!(!registry.begin("stream-2", "chat-a"));
        assert_eq!(registry.snapshot().active_chat_ids, vec!["chat-a"]);

        // Nor does settling only one of them.
        assert!(!registry.settle("stream-1"));
        assert_eq!(registry.snapshot().active_chat_ids, vec!["chat-a"]);

        assert!(registry.settle("stream-2"));
        assert!(registry.snapshot().active_chat_ids.is_empty());
    }

    #[test]
    fn begin_and_settle_are_idempotent() {
        let mut registry = ChatActivityRegistry::new();
        assert!(registry.begin("stream-1", "chat-a"));
        assert!(!registry.begin("stream-1", "chat-a"));
        assert!(registry.settle("stream-1"));
        // Settling an unknown stream is not an error and changes nothing.
        assert!(!registry.settle("stream-1"));
        assert!(!registry.settle("never-seen"));
    }

    #[test]
    fn a_stream_that_moves_chats_releases_the_old_one() {
        let mut registry = ChatActivityRegistry::new();
        registry.begin("stream-1", "chat-a");
        registry.begin("stream-1", "chat-b");
        assert_eq!(registry.snapshot().active_chat_ids, vec!["chat-b"]);
    }

    #[test]
    fn the_revision_only_moves_when_the_visible_set_does() {
        let mut registry = ChatActivityRegistry::new();
        let start = registry.snapshot().revision;
        registry.begin("stream-1", "chat-a");
        let after_begin = registry.snapshot().revision;
        assert!(after_begin > start);
        registry.begin("stream-2", "chat-a");
        assert_eq!(registry.snapshot().revision, after_begin);
    }

    #[test]
    fn a_late_snapshot_cannot_resurrect_a_stale_set() {
        let current = ChatActivityState {
            revision: 5,
            active_chat_ids: BTreeSet::new(),
        };
        let stale = ChatActivitySnapshot {
            revision: 4,
            active_chat_ids: vec!["chat-a".into()],
        };
        assert_eq!(apply_chat_activity_snapshot(&current, &stale), current);

        let fresh = ChatActivitySnapshot {
            revision: 6,
            active_chat_ids: vec!["chat-a".into()],
        };
        let applied = apply_chat_activity_snapshot(&current, &fresh);
        assert_eq!(applied.revision, 6);
        assert!(applied.is_working("chat-a"));
    }

    #[test]
    fn a_malformed_snapshot_is_rejected_and_duplicates_collapse() {
        let parse = |value: serde_json::Value| parse_chat_activity_snapshot(&value);
        assert!(parse(serde_json::json!({"revision": 1})).is_none());
        assert!(parse(serde_json::json!({"revision": -1, "activeChatIds": []})).is_none());
        assert!(parse(serde_json::json!({"revision": 1, "activeChatIds": [""]})).is_none());
        assert!(
            parse(serde_json::json!({"revision": 1, "activeChatIds": ["../escape"]})).is_none()
        );
        assert!(parse(serde_json::json!([])).is_none());

        let parsed = parse(serde_json::json!({
            "revision": 2,
            "activeChatIds": ["chat-a", "chat-a", "chat-b"],
        }))
        .expect("snapshot");
        assert_eq!(parsed.revision, 2);
        assert_eq!(parsed.active_chat_ids, vec!["chat-a", "chat-b"]);
    }
}
