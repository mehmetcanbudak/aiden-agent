//! Rebuild an assistant turn as chronological rows (port of
//! `renderer/lib/assistant-message-presentation.ts`).
//!
//! A version-3 timeline anchors every step to a UTF-16 offset into the visible
//! assistant text. Splitting the text at those offsets recovers the order the
//! user actually saw: prose, the activity that happened during it, more prose.
//! Legacy or malformed timelines yield `None` so callers keep the older
//! activity-above-the-message layout instead of guessing.

use aiden_core::{AgentStep, GenerationClaimCheck, GenerationTimeline, GenerationTimelineStatus};

use super::activity_feed::is_active_step;

#[derive(Debug, Clone, PartialEq)]
pub enum AssistantPresentationRow {
    Text {
        /// Keyed by the stable start boundary: the tail row grows on every
        /// stream delta, and keying by content would remount it per token.
        key: String,
        content: String,
        start_offset: usize,
        end_offset: usize,
    },
    Activity {
        key: String,
        content_offset: usize,
        steps: Vec<AgentStep>,
    },
}

fn step_id(step: &AgentStep) -> &str {
    match step {
        AgentStep::Tool(tool) => tool.id.as_str(),
        AgentStep::Thinking(thinking) => thinking.id.as_str(),
    }
}

fn step_updated_at(step: &AgentStep) -> u64 {
    match step {
        AgentStep::Tool(tool) => tool.updated_at,
        AgentStep::Thinking(thinking) => thinking.updated_at,
    }
}

fn step_content_offset(step: &AgentStep) -> Option<usize> {
    match step {
        AgentStep::Tool(tool) => tool.content_offset,
        AgentStep::Thinking(thinking) => thinking.content_offset,
    }
}

/// Byte index of a UTF-16 offset. Returns `None` when the offset runs past the
/// text or lands inside a surrogate pair, which means the timeline and the
/// message disagree and the whole layout has to fall back.
fn byte_index_for_utf16(content: &str, target: usize) -> Option<usize> {
    if target == 0 {
        return Some(0);
    }
    let mut units = 0usize;
    for (byte_index, ch) in content.char_indices() {
        if units == target {
            return Some(byte_index);
        }
        units += ch.len_utf16();
        if units > target {
            return None;
        }
    }
    (units == target).then_some(content.len())
}

fn text_row(content: &str, start: usize, end: usize) -> Option<AssistantPresentationRow> {
    let start_byte = byte_index_for_utf16(content, start)?;
    let end_byte = byte_index_for_utf16(content, end)?;
    if end_byte < start_byte {
        return None;
    }
    let slice = &content[start_byte..end_byte];
    if slice.trim().is_empty() {
        return None;
    }
    Some(AssistantPresentationRow::Text {
        key: format!("text-{start}"),
        content: slice.to_string(),
        start_offset: start,
        end_offset: end,
    })
}

/// `assistantPresentationRows`.
pub fn assistant_presentation_rows(
    content: &str,
    timeline: Option<&GenerationTimeline>,
) -> Option<Vec<AssistantPresentationRow>> {
    let timeline = timeline?;
    if timeline.steps.is_empty() {
        return None;
    }
    let content_units = content.encode_utf16().count();
    let offsets: Vec<usize> = timeline
        .steps
        .iter()
        .map(step_content_offset)
        .collect::<Option<Vec<_>>>()?;
    if offsets.iter().enumerate().any(|(index, offset)| {
        *offset > content_units || (index > 0 && *offset < offsets[index - 1])
    }) {
        return None;
    }

    let mut rows: Vec<AssistantPresentationRow> = Vec::new();
    let mut cursor = 0usize;
    let mut index = 0usize;
    while index < timeline.steps.len() {
        let offset = offsets[index];
        if let Some(narrative) = text_row(content, cursor, offset) {
            rows.push(narrative);
        }

        let mut steps: Vec<AgentStep> = Vec::new();
        while index < timeline.steps.len() && offsets[index] == offset {
            steps.push(timeline.steps[index].clone());
            index += 1;
        }
        let key = steps
            .first()
            .map(|step| format!("activity-{offset}-{}", step_id(step)))
            .unwrap_or_else(|| format!("activity-{offset}-{index}"));
        rows.push(AssistantPresentationRow::Activity {
            key,
            content_offset: offset,
            steps,
        });
        cursor = offset;
    }

    if let Some(tail) = text_row(content, cursor, content_units) {
        rows.push(tail);
    }
    Some(rows)
}

/// `activityTimelineFragment` — limit one feed to the steps at a text boundary.
pub fn activity_timeline_fragment(
    timeline: &GenerationTimeline,
    steps: Vec<AgentStep>,
) -> GenerationTimeline {
    let running =
        timeline.status == GenerationTimelineStatus::Running && steps.iter().any(is_active_step);
    let claim_step_ids: Vec<String> = match &timeline.claim_check {
        Some(GenerationClaimCheck::UnverifiedSuccess { step_ids }) => step_ids
            .iter()
            .filter(|id| steps.iter().any(|step| step_id(step) == id.as_str()))
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    let finished_at = if running {
        None
    } else {
        timeline
            .finished_at
            .or_else(|| steps.last().map(step_updated_at))
            .or(Some(timeline.started_at))
    };
    GenerationTimeline {
        status: if running {
            GenerationTimelineStatus::Running
        } else if timeline.status == GenerationTimelineStatus::Running {
            GenerationTimelineStatus::Completed
        } else {
            timeline.status
        },
        finished_at,
        steps,
        claim_check: (!claim_step_ids.is_empty()).then_some(
            GenerationClaimCheck::UnverifiedSuccess {
                step_ids: claim_step_ids,
            },
        ),
        ..timeline.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiden_core::{AgentStepStatus, AgentThinkingStep, AgentToolStep};

    fn tool(id: &str, content_offset: Option<usize>) -> AgentStep {
        AgentStep::Tool(AgentToolStep {
            id: id.to_string(),
            order: 0,
            tool_call_id: format!("call-{id}"),
            tool_name: "read_file".into(),
            label: "Read file".into(),
            status: AgentStepStatus::Completed,
            started_at: 1,
            updated_at: 2,
            finished_at: Some(2),
            content_offset,
            target: None,
            detail: None,
            line_changes: None,
        })
    }

    fn timeline(steps: Vec<AgentStep>) -> GenerationTimeline {
        GenerationTimeline {
            version: aiden_core::GENERATION_TIMELINE_VERSION,
            generation_id: "generation-1".into(),
            status: GenerationTimelineStatus::Completed,
            started_at: 1,
            finished_at: Some(9),
            steps,
            claim_check: None,
        }
    }

    #[test]
    fn prose_is_split_around_the_activity_it_surrounds() {
        let content = "Before. After.";
        let rows =
            assistant_presentation_rows(content, Some(&timeline(vec![tool("tool-1", Some(7))])))
                .expect("rows");
        assert_eq!(rows.len(), 3);
        match &rows[0] {
            AssistantPresentationRow::Text { content, .. } => assert_eq!(content, "Before."),
            other => panic!("expected text, got {other:?}"),
        }
        match &rows[1] {
            AssistantPresentationRow::Activity { content_offset, .. } => {
                assert_eq!(*content_offset, 7)
            }
            other => panic!("expected activity, got {other:?}"),
        }
        match &rows[2] {
            AssistantPresentationRow::Text { content, .. } => assert_eq!(content, " After."),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn steps_sharing_an_offset_collapse_into_one_activity_row() {
        let rows = assistant_presentation_rows(
            "Doing work.",
            Some(&timeline(vec![
                tool("tool-1", Some(5)),
                tool("tool-2", Some(5)),
            ])),
        )
        .expect("rows");
        let activity_rows = rows
            .iter()
            .filter(|row| matches!(row, AssistantPresentationRow::Activity { .. }))
            .count();
        assert_eq!(activity_rows, 1);
    }

    #[test]
    fn legacy_and_inconsistent_timelines_fall_back() {
        // Versions 1 and 2 carry no offsets at all.
        assert!(
            assistant_presentation_rows("text", Some(&timeline(vec![tool("tool-1", None)])))
                .is_none()
        );
        // An offset past the text means the record and the message disagree.
        assert!(
            assistant_presentation_rows("hi", Some(&timeline(vec![tool("tool-1", Some(99))])))
                .is_none()
        );
        // Offsets only ever move forward.
        assert!(assistant_presentation_rows(
            "some text here",
            Some(&timeline(vec![
                tool("tool-1", Some(9)),
                tool("tool-2", Some(4)),
            ]))
        )
        .is_none());
        assert!(assistant_presentation_rows("text", None).is_none());
    }

    /// Offsets are UTF-16 code units, so an emoji counts as two.
    #[test]
    fn offsets_are_utf16_code_units() {
        let content = "ab🙂cd";
        assert_eq!(byte_index_for_utf16(content, 0), Some(0));
        assert_eq!(byte_index_for_utf16(content, 2), Some(2));
        // 4 units is past the surrogate pair, landing on 'c'.
        assert_eq!(byte_index_for_utf16(content, 4), Some(6));
        // 3 units would split the pair.
        assert_eq!(byte_index_for_utf16(content, 3), None);
        assert_eq!(byte_index_for_utf16(content, 6), Some(content.len()));
        assert_eq!(byte_index_for_utf16(content, 7), None);

        let rows =
            assistant_presentation_rows(content, Some(&timeline(vec![tool("tool-1", Some(4))])))
                .expect("rows");
        match &rows[0] {
            AssistantPresentationRow::Text { content, .. } => assert_eq!(content, "ab🙂"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn a_fragment_keeps_only_its_own_steps_and_claims() {
        let mut full = timeline(vec![tool("tool-1", Some(0)), tool("tool-2", Some(3))]);
        full.claim_check = Some(GenerationClaimCheck::UnverifiedSuccess {
            step_ids: vec!["tool-1".into(), "tool-2".into()],
        });
        let fragment = activity_timeline_fragment(&full, vec![tool("tool-2", Some(3))]);
        assert_eq!(fragment.steps.len(), 1);
        match fragment.claim_check {
            Some(GenerationClaimCheck::UnverifiedSuccess { step_ids }) => {
                assert_eq!(step_ids, vec!["tool-2".to_string()]);
            }
            None => panic!("expected the claim to survive for its own step"),
        }
    }

    #[test]
    fn a_settled_fragment_of_a_running_turn_reports_completed() {
        let mut running = timeline(vec![tool("tool-1", Some(0))]);
        running.status = GenerationTimelineStatus::Running;
        running.finished_at = None;
        let fragment = activity_timeline_fragment(&running, vec![tool("tool-1", Some(0))]);
        assert_eq!(fragment.status, GenerationTimelineStatus::Completed);
        assert!(fragment.finished_at.is_some());

        let active = AgentStep::Thinking(AgentThinkingStep {
            id: "think-1".into(),
            order: 0,
            started_at: 1,
            updated_at: 2,
            finished_at: None,
            duration_ms: None,
            content_offset: Some(0),
        });
        let live = activity_timeline_fragment(&running, vec![active]);
        assert_eq!(live.status, GenerationTimelineStatus::Running);
        assert!(live.finished_at.is_none());
    }
}
