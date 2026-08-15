//! Aiden settings, matched to `renderer/components/settings/assistant-settings.tsx`.

use aiden_core::keybindings::GlobalShortcutState;
use gpui::{
    div, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, Styled as _, Window,
};
use gpui_component::{button::Button, h_flex, v_flex, ActiveTheme, Sizable as _};

use super::{settings_field, settings_fieldset, SettingsSection, SettingsView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssistantBadgeTone {
    Neutral,
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AssistantFactCopy {
    id: &'static str,
    title: &'static str,
    value: &'static str,
    detail: &'static str,
}

const ASSISTANT_FACTS: [AssistantFactCopy; 4] = [
    AssistantFactCopy {
        id: "assistant-model",
        title: "Chat model",
        value: "Follows composer",
        detail: "Interactive Aiden chats use the provider and model selected in the main composer.",
    },
    AssistantFactCopy {
        id: "assistant-history",
        title: "Conversation history",
        value: "On this Mac",
        detail: "Aiden conversations are stored on this Mac in a private assistant workspace.",
    },
    AssistantFactCopy {
        id: "assistant-access",
        title: "Access",
        value: "Automations only",
        detail: "Aiden can list automations and propose new read-only Ask Aiden tasks with your confirmation. It still cannot inspect live settings or projects, edit files, run commands, or use connected tools.",
    },
    AssistantFactCopy {
        id: "assistant-background",
        title: "Background suggestions",
        value: "Not active",
        detail: "Project monitoring and proactive nudges are not active in this build, so background controls are not shown yet.",
    },
];

#[derive(Default)]
pub struct AssistantState {
    _runtime_facts: (),
}

impl AssistantState {
    pub fn hydrate(&mut self, _settings: &serde_json::Map<String, serde_json::Value>) {}
}

impl SettingsView {
    pub(crate) fn assistant_section(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let well = crate::services::appearance::well_surface(cx);
        let global_status = self
            .services
            .shortcuts
            .read(cx)
            .snapshot()
            .global
            .iter()
            .find(|status| status.command_id == aiden_core::CommandId::AssistantOpen)
            .cloned();
        let (shortcut_value, shortcut_badge, shortcut_tone) = match global_status {
            Some(status) => match status.state {
                GlobalShortcutState::Active => (
                    aiden_core::keybindings::pretty_accelerator(status.binding.as_deref())
                        .to_string(),
                    "Active",
                    AssistantBadgeTone::Positive,
                ),
                GlobalShortcutState::Unavailable => (
                    aiden_core::keybindings::pretty_accelerator(status.binding.as_deref())
                        .to_string(),
                    "Unavailable",
                    AssistantBadgeTone::Negative,
                ),
                GlobalShortcutState::Disabled => (
                    aiden_core::keybindings::pretty_accelerator(status.binding.as_deref())
                        .to_string(),
                    "Off",
                    AssistantBadgeTone::Neutral,
                ),
            },
            None => ("—".to_string(), "Unavailable", AssistantBadgeTone::Negative),
        };

        let shortcut_control = h_flex()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap_2()
            .child(assistant_badge(shortcut_badge, shortcut_tone, &theme))
            .child(
                div()
                    .text_size(gpui::px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(shortcut_value),
            )
            .child(
                Button::new("assistant-manage-shortcut")
                    .small()
                    .label("Manage shortcuts")
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.active = SettingsSection::Shortcut;
                        cx.notify();
                    })),
            );
        let shortcut = settings_field(
            "assistant-global-shortcut",
            "Global shortcut",
            "Open the Aiden dock from anywhere and move focus to its composer.",
            shortcut_control,
            false,
            &theme,
        );

        let facts = ASSISTANT_FACTS
            .iter()
            .enumerate()
            .map(|(index, fact)| {
                settings_field(
                    fact.id,
                    fact.title,
                    fact.detail,
                    assistant_badge(fact.value, AssistantBadgeTone::Neutral, &theme),
                    index + 1 != ASSISTANT_FACTS.len(),
                    &theme,
                )
            })
            .collect();

        v_flex()
            .id("assistant-section")
            .w_full()
            .child(settings_fieldset("Open Aiden", vec![shortcut], well))
            .child(settings_fieldset("How Aiden works", facts, well))
    }
}

fn assistant_badge(
    label: impl Into<SharedString>,
    tone: AssistantBadgeTone,
    theme: &gpui_component::Theme,
) -> impl IntoElement {
    let (background, foreground) = match tone {
        AssistantBadgeTone::Neutral => (theme.muted, theme.foreground),
        AssistantBadgeTone::Positive => (theme.success.opacity(0.10), theme.success),
        AssistantBadgeTone::Negative => (theme.danger.opacity(0.10), theme.danger),
    };
    div()
        .h(gpui::px(24.))
        .flex()
        .items_center()
        .rounded_full()
        .bg(background)
        .text_color(foreground)
        .text_size(gpui::px(13.))
        .font_weight(FontWeight::MEDIUM)
        .px_2()
        .child(label.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_matches_the_electron_v0_28_39_surface() {
        let facts = ASSISTANT_FACTS
            .iter()
            .map(|fact| (fact.title, fact.value, fact.detail))
            .collect::<Vec<_>>();
        assert_eq!(facts.len(), 4);
        assert!(facts
            .iter()
            .any(|fact| fact.0 == "Conversation history" && fact.1 == "On this Mac"));
        assert!(facts
            .iter()
            .any(|fact| fact.0 == "Access" && fact.1 == "Automations only"));
        assert!(facts
            .iter()
            .any(|fact| fact.0 == "Background suggestions" && fact.1 == "Not active"));
    }
}
