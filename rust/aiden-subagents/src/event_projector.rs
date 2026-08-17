//! Port of `main/services/subagents/subagent-event-projector.ts` — converts
//! child lifecycle facts into a strict renderer-safe V1 projection. Raw tool
//! arguments/results, reasoning, prompts, and runtime transport data never
//! enter the projection.

use aiden_core::subagent_runs::{
    parse_subagent_run_snapshot_v1, SubagentMilestoneKind, SubagentRunSnapshotV1, SubagentRunState,
    MAX_SUBAGENT_ACTIVITY_CHARS, MAX_SUBAGENT_ERROR_CHARS, MAX_SUBAGENT_LATEST_TEXT_CHARS,
    MAX_SUBAGENT_MILESTONES, MAX_SUBAGENT_TASK_PREVIEW_CHARS, MAX_SUBAGENT_TERMINAL_MARKDOWN_CHARS,
    MAX_SUBAGENT_WARNING_CHARS, SUBAGENT_RUN_SNAPSHOT_VERSION,
};
use aiden_core::subagent_safe_text::sanitize_subagent_snapshot_text;

use crate::contracts::{SubagentTaskRequest, SubagentTaskResult};

const MAX_DURABLE_LIVE_MILESTONES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentRunIdentity {
    pub run_id: String,
    pub group_id: String,
    pub child_id: String,
}

pub struct SubagentRunProjectorInput {
    pub generation_id: String,
    pub chat_id: String,
    pub workspace_id: String,
    pub model_id: String,
    /// Synchronous authority/admission seam that runs before a new run is
    /// published.
    pub prepare_snapshot: Option<Box<dyn Fn(&SubagentRunSnapshotV1) + Send + Sync>>,
    pub on_snapshot: Option<Box<dyn Fn(&SubagentRunSnapshotV1) + Send + Sync>>,
    pub on_control_snapshot: Option<Box<dyn Fn(&SubagentRunSnapshotV1) + Send + Sync>>,
    pub now: Option<Box<dyn Fn() -> u64 + Send + Sync>>,
}

fn normalize_projected_text(value: &str) -> String {
    let sanitized = sanitize_subagent_snapshot_text(value);
    let normalized: String = sanitized
        .chars()
        .map(|character| {
            let code = character as u32;
            if code == 0x2028 || code == 0x2029 {
                '\n'
            } else if code == 10 {
                character
            } else if code <= 31 || (127..=159).contains(&code) {
                ' '
            } else {
                character
            }
        })
        .collect();
    sanitize_subagent_snapshot_text(&normalized)
}

/// Bound a projected value to what the strict snapshot parser accepts.
///
/// Lengths are counted in characters, matching `safe_text` in the parser; the
/// previous byte count both under-filled multi-byte values and could slice a
/// character in half. Truncating can also create a new credential-like suffix
/// out of a value that was safe whole, so the privacy projection re-runs after
/// every cut and the result is re-measured, bounded by a few attempts.
///
/// Upstream additionally guards against leaving an unpaired high surrogate at
/// the boundary; `chars()` cannot split a scalar value, so that has no analogue
/// here.
fn bounded(value: &str, maximum: usize, marker: &str) -> String {
    let mut safe = normalize_projected_text(value).trim().to_string();
    let safe_marker = normalize_projected_text(marker);
    let marker_len = safe_marker.chars().count();
    let mut attempt = 0;
    while attempt < 8 && safe.chars().count() > maximum {
        let keep = maximum.saturating_sub(marker_len);
        let prefix: String = safe.chars().take(keep).collect();
        safe = normalize_projected_text(&format!("{prefix}{safe_marker}"))
            .trim()
            .to_string();
        attempt += 1;
    }
    if safe.chars().count() <= maximum {
        return safe;
    }
    safe_marker.chars().take(maximum).collect()
}

/// As [`bounded_single_line`], but a value that projects away entirely falls
/// back to a fixed label rather than an empty string the parser would reject.
fn bounded_required_single_line(value: &str, maximum: usize, fallback: &str) -> String {
    let bounded = bounded_single_line(value, maximum);
    if bounded.is_empty() {
        fallback.to_string()
    } else {
        bounded
    }
}

fn bounded_single_line(value: &str, maximum: usize) -> String {
    let single_line: String = normalize_projected_text(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    bounded(&single_line, maximum, "…")
}

fn safe_tool_milestone(tool_name: &str) -> (&'static str, SubagentMilestoneKind) {
    match tool_name {
        "read_file" => ("Reading a workspace file", SubagentMilestoneKind::Reading),
        "list_dir" => (
            "Listing a workspace directory",
            SubagentMilestoneKind::Listing,
        ),
        "glob" => (
            "Matching workspace file names",
            SubagentMilestoneKind::Matching,
        ),
        "grep" => ("Searching workspace text", SubagentMilestoneKind::Searching),
        _ => (
            "Using a bounded read-only tool",
            SubagentMilestoneKind::Inspecting,
        ),
    }
}

fn append_milestone(
    current: Option<&[SubagentMilestoneKind]>,
    next: SubagentMilestoneKind,
) -> Vec<SubagentMilestoneKind> {
    let milestones = current.unwrap_or(&[]);
    if milestones.last() == Some(&next) || milestones.len() >= MAX_SUBAGENT_MILESTONES {
        return milestones.to_vec();
    }
    let mut milestones = milestones.to_vec();
    milestones.push(next);
    milestones
}

fn state_for_result(result: &SubagentTaskResult) -> SubagentRunState {
    match result.status.as_str() {
        "completed" => SubagentRunState::Completed,
        "failed" => SubagentRunState::Failed,
        "timed_out" => SubagentRunState::TimedOut,
        _ => SubagentRunState::Interrupted,
    }
}

/// The bounded projector. Persistence callbacks run synchronously in this port
/// (the TS enqueues them on a promise tail with the same ordering).
pub struct SubagentEventProjector {
    records: std::collections::HashMap<String, SubagentRunSnapshotV1>,
    durable_live_milestones: std::collections::HashMap<String, usize>,
    now: Box<dyn Fn() -> u64 + Send + Sync>,
    input: SubagentRunProjectorInput,
    persistence_error: Option<String>,
}

impl SubagentEventProjector {
    pub fn new(input: SubagentRunProjectorInput) -> Self {
        let SubagentRunProjectorInput {
            generation_id,
            chat_id,
            workspace_id,
            model_id,
            prepare_snapshot,
            on_snapshot,
            on_control_snapshot,
            now,
        } = input;
        SubagentEventProjector {
            records: std::collections::HashMap::new(),
            durable_live_milestones: std::collections::HashMap::new(),
            now: now.unwrap_or_else(|| Box::new(now_millis)),
            input: SubagentRunProjectorInput {
                generation_id,
                chat_id,
                workspace_id,
                model_id,
                prepare_snapshot,
                on_snapshot,
                on_control_snapshot,
                now: None,
            },
            persistence_error: None,
        }
    }

    pub fn begin(
        &mut self,
        identity: &SubagentRunIdentity,
        request: &SubagentTaskRequest,
    ) -> Result<(), String> {
        if self.records.contains_key(&identity.run_id) {
            return Err("Subagent run identity was reused.".to_string());
        }
        let now = (self.now)();
        let snapshot = SubagentRunSnapshotV1 {
            version: SUBAGENT_RUN_SNAPSHOT_VERSION,
            run_id: identity.run_id.clone(),
            group_id: identity.group_id.clone(),
            generation_id: self.input.generation_id.clone(),
            child_id: identity.child_id.clone(),
            chat_id: self.input.chat_id.clone(),
            workspace_id: self.input.workspace_id.clone(),
            revision: 1,
            role: aiden_core::subagent_runs::SubagentSnapshotRole::from_str(&request.role)
                .expect("validated role"),
            label: bounded_required_single_line(&request.label, 120, "Subagent task"),
            task_preview: bounded_required_single_line(
                &request.task,
                MAX_SUBAGENT_TASK_PREVIEW_CHARS,
                "Private task details redacted.",
            ),
            state: SubagentRunState::Queued,
            activity: Some("Waiting for an execution slot".to_string()),
            started_at: now,
            updated_at: now,
            finished_at: None,
            model_id: bounded_required_single_line(&self.input.model_id, 160, "Unknown model"),
            turns: 0,
            tools: 0,
            tokens: 0,
            milestones: Some(Vec::new()),
            latest_text: None,
            terminal_markdown: None,
            error: None,
            warnings: Vec::new(),
        };
        self.publish(snapshot, true)?;
        self.durable_live_milestones
            .insert(identity.run_id.clone(), 0);
        Ok(())
    }

    pub fn starting(&mut self, run_id: &str) -> Result<(), String> {
        self.update(
            run_id,
            UpdatePatch {
                state: Some(SubagentRunState::Starting),
                activity: Some("Starting a fresh child agent".to_string()),
                ..UpdatePatch::default()
            },
            None,
        )
    }

    pub fn running(&mut self, run_id: &str) -> Result<(), String> {
        self.update(
            run_id,
            UpdatePatch {
                state: Some(SubagentRunState::Running),
                activity: Some("Reviewing workspace context".to_string()),
                ..UpdatePatch::default()
            },
            None,
        )
    }

    pub fn turn_started(&mut self, run_id: &str) -> Result<(), String> {
        let current = self.require(run_id)?;
        self.update(
            run_id,
            UpdatePatch {
                turns: Some(current.turns + 1),
                durable: false,
                ..UpdatePatch::default()
            },
            None,
        )
    }

    pub fn tool_started(&mut self, run_id: &str, tool_name: &str) -> Result<(), String> {
        let current = self.require(run_id)?;
        let (activity, milestone) = safe_tool_milestone(tool_name);
        let activity = bounded(activity, MAX_SUBAGENT_ACTIVITY_CHARS, "…");
        let milestones = self
            .durable_live_milestones
            .get(run_id)
            .copied()
            .unwrap_or(0);
        let durable = activity != current.activity.as_deref().unwrap_or("")
            && milestones < MAX_DURABLE_LIVE_MILESTONES;
        if durable {
            self.durable_live_milestones
                .insert(run_id.to_string(), milestones + 1);
        }
        self.update(
            run_id,
            UpdatePatch {
                tools: Some(current.tools + 1),
                activity: Some(activity),
                milestones: Some(Some(append_milestone(
                    current.milestones.as_deref(),
                    milestone,
                ))),
                durable,
                ..UpdatePatch::default()
            },
            None,
        )
    }

    pub fn text_delta(&mut self, run_id: &str) -> Result<(), String> {
        let current = self.require(run_id)?;
        if current.activity.as_deref() == Some("Writing a bounded report") {
            return Ok(());
        }
        self.update(
            run_id,
            UpdatePatch {
                activity: Some("Writing a bounded report".to_string()),
                milestones: Some(Some(append_milestone(
                    current.milestones.as_deref(),
                    SubagentMilestoneKind::Composing,
                ))),
                ..UpdatePatch::default()
            },
            None,
        )
    }

    pub fn usage(&mut self, run_id: &str, tokens: u64) -> Result<(), String> {
        let current = self.require(run_id)?;
        let next_tokens = current.tokens.saturating_add(tokens);
        self.update(
            run_id,
            UpdatePatch {
                tokens: Some(next_tokens),
                durable: false,
                ..UpdatePatch::default()
            },
            None,
        )
    }

    pub fn finish(&mut self, run_id: &str, result: &SubagentTaskResult) -> Result<(), String> {
        let now = (self.now)();
        let projected_warning = result.warning.as_deref().unwrap_or("").to_string();
        let warning = if projected_warning.is_empty() {
            None
        } else {
            Some(bounded(&projected_warning, MAX_SUBAGENT_WARNING_CHARS, "…"))
        };
        let terminal_source = if !result.summary.is_empty() {
            result.summary.clone()
        } else if !projected_warning.is_empty() {
            projected_warning.clone()
        } else {
            "[No textual result.]".to_string()
        };
        let terminal_markdown = bounded(
            &terminal_source,
            MAX_SUBAGENT_TERMINAL_MARKDOWN_CHARS,
            "\n\n… [report truncated]",
        );
        let latest_text = bounded(&terminal_source, MAX_SUBAGENT_LATEST_TEXT_CHARS, "…");
        let latest_text = if latest_text.is_empty() {
            None
        } else {
            Some(latest_text)
        };
        let error_source = if !projected_warning.is_empty() {
            projected_warning
        } else {
            "The child could not complete this task.".to_string()
        };
        let error = bounded(&error_source, MAX_SUBAGENT_ERROR_CHARS, "…");
        let state = state_for_result(result);
        self.update(
            run_id,
            UpdatePatch {
                state: Some(state),
                activity: Some(if matches!(state, SubagentRunState::Completed) {
                    // `activity: undefined` deletes the field.
                    String::new()
                } else {
                    String::new()
                }),
                finished_at: Some(now),
                latest_text: Some(latest_text),
                terminal_markdown: Some(if terminal_markdown.is_empty() {
                    "[No textual result.]".to_string()
                } else {
                    terminal_markdown
                }),
                error: Some(if result.status == "failed" {
                    Some(error)
                } else {
                    None
                }),
                warnings: Some(warning.into_iter().collect()),
                durable: true,
                ..UpdatePatch::default()
            },
            Some(now),
        )
    }

    pub fn snapshot(&self) -> Vec<SubagentRunSnapshotV1> {
        let mut records: Vec<SubagentRunSnapshotV1> = self.records.values().cloned().collect();
        records.sort_by(|left, right| {
            left.started_at
                .cmp(&right.started_at)
                .then_with(|| left.run_id.cmp(&right.run_id))
        });
        records
    }

    pub fn flush(&self) -> Result<(), String> {
        if let Some(error) = &self.persistence_error {
            return Err(error.clone());
        }
        Ok(())
    }

    fn require(&self, run_id: &str) -> Result<SubagentRunSnapshotV1, String> {
        self.records
            .get(run_id)
            .cloned()
            .ok_or_else(|| "Unknown subagent run.".to_string())
    }

    fn update(
        &mut self,
        run_id: &str,
        patch: UpdatePatch,
        updated_at: Option<u64>,
    ) -> Result<(), String> {
        let durable = patch.durable;
        let current = self.require(run_id)?;
        if current.finished_at.is_some() {
            return Ok(());
        }
        let updated_at = updated_at.unwrap_or_else(|| (self.now)());
        let monotonic_updated_at = current.updated_at.max(updated_at);
        let mut next = current.clone();
        if let Some(state) = patch.state {
            next.state = state;
        }
        if let Some(activity) = patch.activity {
            if activity.is_empty() {
                next.activity = None;
            } else {
                next.activity = Some(activity);
            }
        }
        if let Some(finished_at) = patch.finished_at {
            next.finished_at = Some(monotonic_updated_at.max(finished_at));
        }
        if let Some(turns) = patch.turns {
            next.turns = turns;
        }
        if let Some(tools) = patch.tools {
            next.tools = tools;
        }
        if let Some(tokens) = patch.tokens {
            next.tokens = tokens;
        }
        if let Some(milestones) = patch.milestones {
            next.milestones = milestones;
        }
        if let Some(latest_text) = patch.latest_text {
            next.latest_text = latest_text;
        }
        if let Some(terminal_markdown) = patch.terminal_markdown {
            next.terminal_markdown = Some(terminal_markdown);
        }
        if let Some(error) = patch.error {
            next.error = error;
        }
        if let Some(warnings) = patch.warnings {
            next.warnings = warnings;
        }
        next.revision = current.revision + 1;
        next.updated_at = monotonic_updated_at;
        self.publish(next, durable)
    }

    fn publish(&mut self, candidate: SubagentRunSnapshotV1, durable: bool) -> Result<(), String> {
        let value = serde_json::to_value(&candidate).map_err(|error| error.to_string())?;
        let snapshot = parse_subagent_run_snapshot_v1(&value)
            .ok_or_else(|| "Invalid renderer-safe subagent snapshot.".to_string())?;
        if !self.records.contains_key(&snapshot.run_id) {
            if let Some(prepare) = &self.input.prepare_snapshot {
                prepare(&snapshot);
            }
        }
        self.records
            .insert(snapshot.run_id.clone(), snapshot.clone());
        if durable {
            if let Some(on_snapshot) = &self.input.on_snapshot {
                on_snapshot(&snapshot);
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct UpdatePatch {
    state: Option<SubagentRunState>,
    activity: Option<String>,
    finished_at: Option<u64>,
    turns: Option<u64>,
    tools: Option<u64>,
    tokens: Option<u64>,
    milestones: Option<Option<Vec<SubagentMilestoneKind>>>,
    latest_text: Option<Option<String>>,
    terminal_markdown: Option<String>,
    error: Option<Option<String>>,
    warnings: Option<Vec<String>>,
    durable: bool,
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parser measures characters, so the bound must too. Counting bytes
    /// both under-filled multi-byte values and could slice a character in half,
    /// which panics.
    #[test]
    fn bounds_are_counted_in_characters_not_bytes() {
        // Each character is 3 bytes, so a byte-based bound would cut at 10
        // bytes -- inside the fourth character.
        let value = "日本語のテキストです";
        assert_eq!(value.chars().count(), 10);
        assert!(value.len() > 10);
        let whole = bounded(value, 10, "…");
        assert_eq!(whole, value);
        assert!(whole.chars().count() <= 10);

        let cut = bounded(value, 5, "…");
        assert!(cut.chars().count() <= 5);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn a_short_value_is_returned_whole() {
        assert_eq!(bounded("hello", 32, "…"), "hello");
        assert_eq!(bounded("  padded  ", 32, "…"), "padded");
    }

    #[test]
    fn an_over_long_value_keeps_the_marker_within_the_bound() {
        let long = "a".repeat(500);
        let cut = bounded(&long, 24, "…");
        assert_eq!(cut.chars().count(), 24);
        assert!(cut.ends_with('…'));
    }

    /// A maximum smaller than the marker still has to satisfy the parser.
    #[test]
    fn a_bound_narrower_than_the_marker_is_still_respected() {
        let cut = bounded(&"a".repeat(50), 1, "…");
        assert!(cut.chars().count() <= 1);
    }

    #[test]
    fn a_value_that_projects_away_falls_back_instead_of_going_empty() {
        // Control characters normalize to spaces, which trim away entirely.
        assert_eq!(
            bounded_required_single_line("\u{1}\u{2}", 120, "Subagent task"),
            "Subagent task"
        );
        assert_eq!(
            bounded_required_single_line("Real label", 120, "Subagent task"),
            "Real label"
        );
    }

    #[test]
    fn single_line_collapses_whitespace() {
        assert_eq!(
            bounded_single_line("two   lines\nhere", 64),
            "two lines here"
        );
    }

    fn input() -> SubagentRunProjectorInput {
        SubagentRunProjectorInput {
            generation_id: "generation-1".to_string(),
            chat_id: "chat-1".to_string(),
            workspace_id: "workspace-1".to_string(),
            model_id: "model-1".to_string(),
            prepare_snapshot: None,
            on_snapshot: None,
            on_control_snapshot: None,
            now: Some(Box::new(|| 100)),
        }
    }

    fn request() -> SubagentTaskRequest {
        serde_json::from_value(serde_json::json!({
            "role": "scout",
            "label": "Look around",
            "task": "Explore the workspace and report findings.",
        }))
        .unwrap()
    }

    fn result(status: &str) -> SubagentTaskResult {
        serde_json::from_value(serde_json::json!({
            "role": "scout",
            "label": "Look around",
            "status": status,
            "summary": "Found the security boundary.",
            "warning": null,
        }))
        .unwrap()
    }

    #[test]
    fn lifecycle_projection_is_renderer_safe() {
        let mut projector = SubagentEventProjector::new(input());
        let identity = SubagentRunIdentity {
            run_id: "run-1".into(),
            group_id: "group-1".into(),
            child_id: "child-1".into(),
        };
        projector.begin(&identity, &request()).unwrap();
        let queued = projector.snapshot();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].state, SubagentRunState::Queued);
        assert_eq!(queued[0].label, "Look around");
        projector.starting("run-1").unwrap();
        projector.running("run-1").unwrap();
        projector.turn_started("run-1").unwrap();
        projector.tool_started("run-1", "read_file").unwrap();
        assert_eq!(projector.snapshot()[0].tools, 1);
        assert_eq!(projector.snapshot()[0].turns, 1);
        projector.finish("run-1", &result("completed")).unwrap();
        let finished = projector.snapshot()[0].clone();
        assert_eq!(finished.state, SubagentRunState::Completed);
        assert!(finished.finished_at.is_some());
        assert!(finished
            .terminal_markdown
            .as_deref()
            .unwrap()
            .contains("security boundary"));
    }

    #[test]
    fn finish_failed_sets_error_and_bounds_text() {
        let mut projector = SubagentEventProjector::new(input());
        let identity = SubagentRunIdentity {
            run_id: "run-1".into(),
            group_id: "group-1".into(),
            child_id: "child-1".into(),
        };
        projector.begin(&identity, &request()).unwrap();
        let mut failed = result("failed");
        failed.summary = "x".repeat(3_000);
        projector.finish("run-1", &failed).unwrap();
        let snapshot = projector.snapshot()[0].clone();
        assert_eq!(snapshot.state, SubagentRunState::Failed);
        assert!(snapshot.error.is_some());
        // Characters, not bytes: `safe_text` in the parser counts scalar
        // values, and the ellipsis marker is three bytes wide.
        assert!(
            snapshot.latest_text.as_deref().unwrap().chars().count()
                <= MAX_SUBAGENT_LATEST_TEXT_CHARS
        );
        assert!(
            snapshot
                .terminal_markdown
                .as_deref()
                .unwrap()
                .chars()
                .count()
                <= MAX_SUBAGENT_TERMINAL_MARKDOWN_CHARS
        );
    }

    #[test]
    fn milestone_append_is_bounded_and_deduplicated() {
        let mut projector = SubagentEventProjector::new(input());
        let identity = SubagentRunIdentity {
            run_id: "run-1".into(),
            group_id: "group-1".into(),
            child_id: "child-1".into(),
        };
        projector.begin(&identity, &request()).unwrap();
        for _ in 0..20 {
            projector.tool_started("run-1", "grep").unwrap();
        }
        let milestones = projector.snapshot()[0].milestones.clone().unwrap();
        assert_eq!(milestones.len(), 1);
        assert_eq!(milestones[0], SubagentMilestoneKind::Searching);
    }

    #[test]
    fn control_snapshot_projection_and_flush_error() {
        let mut projector = SubagentEventProjector::new(input());
        let identity = SubagentRunIdentity {
            run_id: "run-1".into(),
            group_id: "group-1".into(),
            child_id: "child-1".into(),
        };
        projector.begin(&identity, &request()).unwrap();
        // A late update after finish is fenced.
        projector.finish("run-1", &result("completed")).unwrap();
        projector.running("run-1").unwrap();
        assert_eq!(projector.snapshot()[0].state, SubagentRunState::Completed);
    }
}
