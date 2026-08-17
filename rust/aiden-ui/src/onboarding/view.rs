//! Onboarding flow rendering. The step content mirrors the TS card copy; the
//! motion is a quiet 180ms opacity crossfade per step, gated by the reduced
//! motion preference (GPUI 0.2 has no transform animations, so the design
//! docs' scale/offset recipes are approximated with opacity only).

use std::sync::Arc;
use std::time::Duration;

use aiden_core::appearance::ReduceMotion;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, img, px, Animation, AnimationExt as _, AnyElement, Context, FontWeight, Image,
    ImageFormat, InteractiveElement as _, IntoElement, ObjectFit, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, StyledImage as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _, Theme,
};

use super::state::{ProviderChoice, Step};
use super::OnboardingView;
use crate::typography;

const APP_ICON_PNG: &[u8] = include_bytes!("../../../../resources/app-icon.png");

/// Current Main's product-curated Pi ordering. The compact cards already
/// represent OpenAI and Anthropic, so those entries are filtered below while
/// the remaining featured providers stay ahead of Pi's catalog order.
const ONBOARDING_FEATURED_PI_PROVIDER_IDS: &[&str] = &[
    "openai",
    "anthropic",
    "google",
    "xai",
    "openrouter",
    "deepseek",
    "vercel-ai-gateway",
    "opencode",
    "opencode-go",
    "zai-coding-cn",
    "kimi-coding",
];

fn onboarding_more_provider_indices<'a>(
    provider_ids: impl IntoIterator<Item = &'a str>,
) -> Vec<usize> {
    let provider_ids = provider_ids.into_iter().collect::<Vec<_>>();
    let mut ordered = Vec::with_capacity(provider_ids.len());

    for featured_id in ONBOARDING_FEATURED_PI_PROVIDER_IDS {
        if matches!(*featured_id, "openai" | "anthropic") {
            continue;
        }
        if let Some(index) = provider_ids.iter().position(|id| id == featured_id) {
            ordered.push(index);
        }
    }

    for (index, provider_id) in provider_ids.iter().enumerate() {
        if matches!(*provider_id, "openai" | "anthropic")
            || ONBOARDING_FEATURED_PI_PROVIDER_IDS.contains(provider_id)
        {
            continue;
        }
        ordered.push(index);
    }

    ordered
}

#[derive(Clone, Copy)]
enum TourFeatureSize {
    Hero,
    Tall,
    Standard,
    Wide,
}

impl TourFeatureSize {
    const fn column_span(self) -> u16 {
        match self {
            Self::Hero => 4,
            Self::Tall | Self::Standard => 2,
            Self::Wide => 3,
        }
    }

    const fn row_span(self) -> u16 {
        match self {
            Self::Hero | Self::Tall => 2,
            Self::Standard | Self::Wide => 1,
        }
    }

    const fn height(self) -> f32 {
        match self {
            Self::Hero | Self::Tall => 246.0,
            Self::Standard | Self::Wide => 118.0,
        }
    }
}

struct TourFeature {
    id: &'static str,
    group: &'static str,
    title: &'static str,
    description: &'static str,
    image: &'static [u8],
    icon: IconName,
    size: TourFeatureSize,
}

const TOUR_GROUPS: [(&str, &str); 3] = [
    ("create", "Build in your workspace"),
    ("extend", "Choose and extend"),
    ("control", "Automate and stay in control"),
];

const TOUR_FEATURES: [TourFeature; 22] = [
    TourFeature {
        id: "workspace",
        group: "create",
        title: "Workspace Agent",
        description: "Read, search, edit, and run commands inside the workspace you choose.",
        image: include_bytes!("../../../../renderer/assets/onboarding/aiden-workspace.png"),
        icon: IconName::Folder,
        size: TourFeatureSize::Hero,
    },
    TourFeature {
        id: "computer-use",
        group: "create",
        title: "Computer Use",
        description: "Inspect and operate Mac apps when you opt in, with approval before every action.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/computer-use.png"),
        icon: IconName::Eye,
        size: TourFeatureSize::Tall,
    },
    TourFeature {
        id: "subagents",
        group: "create",
        title: "Native Subagents",
        description: "Delegate scout, planner, and reviewer jobs, then inspect their live results.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/native-subagents.png"),
        icon: IconName::Bot,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "files-editor",
        group: "create",
        title: "Files & Text Editor",
        description: "Browse, search, edit, and safely save workspace text files beside the chat.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/files-editor.png"),
        icon: IconName::File,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "review-diffs",
        group: "create",
        title: "Review & Diffs",
        description: "Inspect staged, unstaged, and branch-to-branch diffs before you commit.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/review-diffs.png"),
        icon: IconName::GalleryVerticalEnd,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "terminal",
        group: "create",
        title: "Integrated Terminal",
        description: "Run a workspace shell in tabs or split panes, then reopen it with sanitized local history.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/terminal.png"),
        icon: IconName::SquareTerminal,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "git-workflows",
        group: "create",
        title: "Git Workflows",
        description: "Switch branches, create reviewed commits, and push with stale-state guards.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/git-workflows.png"),
        icon: IconName::GitHub,
        size: TourFeatureSize::Wide,
    },
    TourFeature {
        id: "workspaces",
        group: "create",
        title: "Workspaces & Worktrees",
        description: "Use folders, scratch spaces, and isolated worktrees while preserving context.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/workspaces-worktrees.png"),
        icon: IconName::Building2,
        size: TourFeatureSize::Wide,
    },
    TourFeature {
        id: "models",
        group: "extend",
        title: "Model Freedom",
        description: "Choose from 30+ Pi providers, ChatGPT sign-in, Apple models, or local endpoints.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/model-freedom.png"),
        icon: IconName::Bot,
        size: TourFeatureSize::Hero,
    },
    TourFeature {
        id: "model-pad",
        group: "extend",
        title: "Personal Model Pad",
        description: "Arrange favorite models on your own map of capability and response pace.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/model-pad.png"),
        icon: IconName::Map,
        size: TourFeatureSize::Tall,
    },
    TourFeature {
        id: "thinking",
        group: "extend",
        title: "Thinking Controls",
        description: "Tune supported models' reasoning effort and follow thinking as it streams.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/thinking-controls.png"),
        icon: IconName::Bot,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "vision",
        group: "extend",
        title: "Attachments & Vision",
        description: "Attach text and images for vision-capable models to inspect in conversation.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/attachments-vision.png"),
        icon: IconName::Eye,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "web-search",
        group: "extend",
        title: "Web Search",
        description: "Give the workspace agent live Exa search when you choose to connect it.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/web-search.png"),
        icon: IconName::Globe,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "skills",
        group: "extend",
        title: "Reusable Skills",
        description: "Create reusable instructions, then type $ to attach one to your next message.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/skills.png"),
        icon: IconName::BookOpen,
        size: TourFeatureSize::Wide,
    },
    TourFeature {
        id: "mcp",
        group: "extend",
        title: "MCP Connectors",
        description: "Connect services or any MCP server and expose only the tools you enable.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/mcp-connectors.png"),
        icon: IconName::Frame,
        size: TourFeatureSize::Wide,
    },
    TourFeature {
        id: "assistant",
        group: "control",
        title: "Aiden Assistant",
        description: "Ask about the app and prepare confirmed automations from a private dock.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/aiden-assistant.png"),
        icon: IconName::Bot,
        size: TourFeatureSize::Hero,
    },
    TourFeature {
        id: "schedules",
        group: "control",
        title: "Scheduled Automations",
        description: "Schedule recurring model work or trusted scripts, then run or pause anytime.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/scheduled-automations.png"),
        icon: IconName::Calendar,
        size: TourFeatureSize::Tall,
    },
    TourFeature {
        id: "voice",
        group: "control",
        title: "Voice & Dictation",
        description: "Speak into the composer or dictate system-wide with cloud or on-device voice.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/voice-dictation.png"),
        icon: IconName::Bell,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "commands",
        group: "control",
        title: "Command Palette",
        description: "Use Command-K or / for app commands, and $ to attach a reusable skill.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/command-palette.png"),
        icon: IconName::Search,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "usage",
        group: "control",
        title: "Private Usage Profile",
        description: "See on-device activity, token mix, cost coverage, and your top models.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/usage-profile.png"),
        icon: IconName::ChartPie,
        size: TourFeatureSize::Standard,
    },
    TourFeature {
        id: "permissions",
        group: "control",
        title: "Permissioned by Default",
        description: "Choose No access, Ask first, or Full per workspace; keys stay encrypted.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/permissions.png"),
        icon: IconName::CircleCheck,
        size: TourFeatureSize::Wide,
    },
    TourFeature {
        id: "themes",
        group: "control",
        title: "Themes & Accessibility",
        description: "Tune light or dark themes, fonts, contrast, motion, and diff markers.",
        image: include_bytes!("../../../../renderer/assets/onboarding/features/themes-accessibility.png"),
        icon: IconName::Palette,
        size: TourFeatureSize::Wide,
    },
];

fn tour_tile_consumes_key(key: &str) -> bool {
    matches!(key, "enter" | "space")
}

/// Selection cards live inside the onboarding root's Enter key context. Stop
/// that bubbling action while a card owns focus so Enter activates the card
/// (via GPUI's native keyboard click) instead of advancing the whole flow.
fn selection_card_consumes_key(key: &str) -> bool {
    matches!(key, "enter" | "space")
}

/// Whether motion is allowed for this appearance preference + the injected
/// OS flag (mirrors the pill's `MotionGate`; GPUI cannot probe the OS).
fn motion_allowed(reduce_motion: ReduceMotion, system_reduced: bool) -> bool {
    match reduce_motion {
        ReduceMotion::On => false,
        ReduceMotion::Off => true,
        ReduceMotion::System => !system_reduced,
    }
}

/// A quiet 180ms ease-out cubic (the design docs' restrained entrance).
fn quiet_ease(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

impl Render for OnboardingView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let step = self.machine.current();
        let step_index = self.machine.step_index();
        let total = self.machine.total_steps();
        // System is a live process preference, not a baked-in false. This is
        // also refreshed by the native accessibility observer while the main
        // window is open; onboarding uses the same conservative probe before
        // it has a ChatService owner.
        let motion = motion_allowed(
            self.machine.reduce_motion,
            crate::services::appearance::current_system_reduced_motion(cx),
        );
        let finish = step == Step::Finish;

        // Focus management: the name field on Welcome, the primary action
        // everywhere else (macOS-style forward focus on step change).
        self.manage_focus(step, window, cx);

        let step_content = v_flex()
            .id("onboarding-step-content")
            .track_scroll(&self.step_scroll)
            .flex_1()
            .w_full()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px_6()
            .py_5()
            .child(self.step_content(step, cx));
        let step_content: AnyElement = if motion {
            step_content
                .with_animation(
                    ("onboarding-crossfade", step_index),
                    Animation::new(Duration::from_millis(180)).with_easing(quiet_ease),
                    |el, progress| el.opacity(progress),
                )
                .into_any_element()
        } else {
            step_content.into_any_element()
        };

        div()
            .id("onboarding-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .key_context("onboarding")
            .on_action(cx.listener(Self::on_next))
            .on_action(cx.listener(Self::on_back))
            .on_action(cx.listener(Self::on_skip))
            .child(
                h_flex()
                    .id("onboarding-card")
                    .w(px(860.0))
                    .h(px(600.0))
                    .overflow_hidden()
                    .rounded(px(16.0))
                    .bg(theme.popover)
                    .shadow_lg()
                    .child(self.setup_rail(step_index, &theme))
                    .child(
                        v_flex()
                            .min_w(px(0.0))
                            .flex_1()
                            .size_full()
                            .rounded_r(px(16.0))
                            .bg(theme.popover)
                            .child(self.card_header(step_index, total, &theme, cx))
                            .child(step_content)
                            .when_some(self.machine.error, |el, message| {
                                el.child(
                                    div()
                                        .w_full()
                                        .px_6()
                                        .pb_2()
                                        .text_size(typography::small(&theme))
                                        .line_height(px(17.0))
                                        .text_color(theme.danger)
                                        .child(message),
                                )
                            })
                            .child(self.card_footer(finish, &theme, cx)),
                    ),
            )
            .when(self.pi_setup.is_some(), |el| {
                el.child(self.pi_provider_setup_modal(&theme, cx))
            })
    }
}

impl OnboardingView {
    fn setup_rail(&self, step_index: usize, theme: &Theme) -> impl IntoElement {
        v_flex()
            .id("onboarding-setup-rail")
            .h_full()
            .w(px(220.0))
            .flex_shrink_0()
            .rounded_l(px(16.0))
            .justify_between()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.sidebar)
            .px_5()
            .pt_7()
            .pb_5()
            .child(
                v_flex()
                    .child(
                        img(Arc::new(Image::from_bytes(
                            ImageFormat::Png,
                            APP_ICON_PNG.to_vec(),
                        )))
                        .size(px(56.0)),
                    )
                    .child(
                        div()
                            .mt_5()
                            .text_size(px(20.0))
                            .line_height(px(24.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Set up Aiden"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_size(typography::small(theme))
                            .line_height(px(20.0))
                            .text_color(theme.muted_foreground)
                            .child("Add your profile and one model connection. You can change either later in Settings."),
                    ),
            )
            .child(
                v_flex()
                    .id("onboarding-setup-progress")
                    .gap_2()
                    .children(Step::ALL.iter().enumerate().map(|(index, step)| {
                        let reached = index <= step_index;
                        h_flex()
                            .items_center()
                            .gap_2()
                            .text_color(if reached {
                                theme.foreground
                            } else {
                                theme.muted_foreground
                            })
                            .child(
                                div()
                                    .size(px(20.0))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_full()
                                    .bg(if reached { theme.accent } else { theme.muted })
                                    .text_color(if reached {
                                        theme.accent_foreground
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .when(index < step_index, |el| {
                                        el.child(Icon::new(IconName::Check).xsmall())
                                    })
                                    .when(index >= step_index, |el| {
                                        el.child((index + 1).to_string())
                                    }),
                            )
                            .child(
                                div()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .text_size(typography::small(theme))
                                    .line_height(px(17.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(step.label()),
                            )
                    })),
            )
    }

    /// Focus the name input on Welcome and the primary action elsewhere, once
    /// per step change.
    fn manage_focus(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        if self.focused_step == self.machine.step_index() {
            return;
        }
        self.focused_step = self.machine.step_index();
        self.step_scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        if step == Step::Welcome {
            let name = self.name_input.clone();
            name.update(cx, |input, inner| input.focus(window, inner));
        } else {
            window.focus(&self.next_focus);
        }
    }

    fn card_header(
        &self,
        step_index: usize,
        total: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .id("onboarding-header")
            .w_full()
            .h(px(56.0))
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(theme.border)
            .px_6()
            .child(
                div()
                    .text_size(typography::small(theme))
                    .line_height(px(17.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child(format!("Step {} of {}", step_index + 1, total)),
            )
            .child(
                Button::new("onboarding-skip")
                    .ghost()
                    .small()
                    .h(px(28.0))
                    .rounded(px(999.0))
                    .label("Skip")
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.skip_pressed(cx);
                    })),
            )
    }

    fn card_footer(&self, finish: bool, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("onboarding-footer")
            .w_full()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_6()
            .py_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Button::new("onboarding-back")
                    .ghost()
                    .rounded(px(999.0))
                    .icon(IconName::ChevronLeft)
                    .label("Back")
                    .disabled(self.machine.step_index() == 0 || self.busy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.back_pressed(cx);
                    })),
            )
            .child(
                Button::new("onboarding-next")
                    .primary()
                    .rounded(px(999.0))
                    .label(if self.discovering {
                        "Discovering models…"
                    } else if self.busy && !finish {
                        "Adding provider…"
                    } else if finish {
                        "Start using Aiden"
                    } else {
                        "Next"
                    })
                    .child(Icon::new(IconName::ChevronRight).small())
                    .track_focus(&self.next_focus)
                    .disabled(!self.machine.can_continue() || self.busy)
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.on_next_pressed(window, cx);
                    })),
            )
    }

    fn step_content(&mut self, step: Step, cx: &mut Context<Self>) -> AnyElement {
        match step {
            Step::Welcome => self.welcome_step(cx),
            Step::Provider => self.provider_step(cx),
            Step::Finish => self.finish_step(cx),
        }
    }

    fn step_heading(&self, title: &str, body: &str, theme: &Theme) -> AnyElement {
        v_flex()
            .gap_1p5()
            .child(
                div()
                    .text_size(px(20.0))
                    .line_height(px(24.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_size(typography::small(theme))
                    .line_height(px(17.0))
                    .text_color(theme.muted_foreground)
                    .child(body.to_string()),
            )
            .into_any_element()
    }

    // -----------------------------------------------------------------------
    // Welcome (TS "profile" step)
    // -----------------------------------------------------------------------

    fn welcome_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        v_flex()
            .id("onboarding-welcome")
            .w_full()
            .max_w(px(480.0))
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(
                        Icon::new(IconName::User)
                            .mt_0p5()
                            .with_size(px(20.0))
                            .text_color(theme.accent),
                    )
                    .child(self.step_heading(
                        "What should Aiden call you?",
                        "This personalizes your profile and model context on this Mac.",
                        &theme,
                    )),
            )
            .child(
                v_flex()
                    .mt_6()
                    .gap_2()
                    .child(
                        div()
                            .text_size(typography::small(&theme))
                            .line_height(px(17.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Name"),
                    )
                    .child(Input::new(&self.name_input)),
            )
            .child(
                h_flex()
                    .mt_4()
                    .gap_2()
                    .items_center()
                    .child(
                        Icon::default()
                            .path("native-icons/lock.svg")
                            .small()
                            .text_color(theme.accent),
                    )
                    .child(
                        div()
                            .text_size(typography::small(&theme))
                            .line_height(px(17.0))
                            .text_color(theme.muted_foreground)
                            .child("Stored privately on this Mac."),
                    ),
            )
            .into_any_element()
    }

    // -----------------------------------------------------------------------
    // Provider (TS "provider" step)
    // -----------------------------------------------------------------------

    fn provider_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let selected_pi_provider = self
            .selected_pi_provider_id
            .as_ref()
            .and_then(|selected| {
                self.pi_provider_statuses
                    .iter()
                    .find(|status| status.provider.id == *selected)
            })
            .cloned();
        let more_providers = onboarding_more_provider_indices(
            self.pi_provider_statuses
                .iter()
                .map(|status| status.provider.id.as_str()),
        )
        .into_iter()
        .map(|index| self.pi_provider_statuses[index].clone())
        .collect::<Vec<_>>();
        let primary_selected = self.selected_pi_provider_id.is_none();
        let selected_choice = self.machine.choice;

        v_flex()
            .id("onboarding-provider")
            .w_full()
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(
                        Icon::default()
                            .path("native-icons/network.svg")
                            .mt_0p5()
                            .with_size(px(20.0))
                            .text_color(theme.accent),
                    )
                    .child(self.step_heading(
                        "Add a model provider",
                        "Choose one connection to get started.",
                        &theme,
                    )),
            )
            .child(
                v_flex()
                    .mt_6()
                    .w_full()
                    .gap_2()
                    .children(ProviderChoice::ALL.chunks(2).map(|row| {
                        h_flex().w_full().gap_2().children(row.iter().map(|choice| {
                            self.provider_card(
                                *choice,
                                primary_selected && *choice == selected_choice,
                                &theme,
                                cx,
                            )
                        }))
                    })),
            )
            .child(
                h_flex()
                    .id("onboarding-more-provider-trigger")
                    .mt_2()
                    .w_full()
                    .min_h(px(58.0))
                    .items_center()
                    .gap_2p5()
                    .px_3()
                    .py_2()
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(theme.input)
                    .bg(theme.foreground.alpha(0.029))
                    .opacity(if self.busy { 0.5 } else { 1.0 })
                    .when(
                        crate::services::appearance::pointer_cursors_enabled(cx),
                        |el| el.cursor_pointer(),
                    )
                    .hover(|style| style.bg(theme.muted))
                    .focusable()
                    .tab_stop(true)
                    .focus(|style| style.bg(theme.list_active).border_color(theme.ring))
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        if this.busy {
                            return;
                        }
                        this.show_more_providers = !this.show_more_providers;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .size(px(32.0))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::default()
                                    .path("native-icons/blocks.svg")
                                    .with_size(px(20.0)),
                            ),
                    )
                    .child(
                        v_flex()
                            .min_w(px(0.0))
                            .flex_1()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_size(typography::small(&theme))
                                    .line_height(px(17.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Choose from more"),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(typography::small(&theme))
                                    .line_height(px(16.0))
                                    .text_color(theme.muted_foreground)
                                    .child(selected_pi_provider.as_ref().map_or_else(
                                        || {
                                            format!(
                                                "{} additional provider{}",
                                                more_providers.len(),
                                                if more_providers.len() == 1 { "" } else { "s" }
                                            )
                                        },
                                        |status| format!("{} selected", status.provider.label),
                                    )),
                            ),
                    )
                    .child(
                        Icon::new(if self.show_more_providers {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        })
                        .small()
                        .flex_shrink_0()
                        .text_color(theme.muted_foreground),
                    ),
            )
            .when(self.show_more_providers, |el| {
                el.child(
                    v_flex()
                        .id("onboarding-more-providers")
                        .mt_2()
                        .w_full()
                        .gap_1p5()
                        .p_2()
                        .rounded(px(12.0))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.popover)
                        .children(more_providers.chunks(2).map(|row| {
                            h_flex()
                                .w_full()
                                .gap_1p5()
                                .children(row.iter().map(|status| {
                                    self.more_provider_card(
                                        status,
                                        self.selected_pi_provider_id.as_deref()
                                            == Some(status.provider.id.as_str()),
                                        &theme,
                                        cx,
                                    )
                                }))
                        })),
                )
            })
            .when(primary_selected && selected_choice.requires_key(), |el| {
                el.child(
                    h_flex()
                        .mt_4()
                        .w_full()
                        .items_start()
                        .gap_3()
                        .child(
                            v_flex()
                                .flex_1()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(typography::small(&theme))
                                        .line_height(px(17.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("API key"),
                                )
                                .child(
                                    Input::new(&self.api_key_input)
                                        .h(px(40.0))
                                        .disabled(self.busy),
                                ),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(typography::small(&theme))
                                        .line_height(px(17.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Base URL (optional)"),
                                )
                                .child(
                                    Input::new(&self.base_url_input)
                                        .h(px(40.0))
                                        .disabled(self.busy),
                                ),
                        ),
                )
            })
            .when(
                primary_selected && selected_choice == ProviderChoice::Tailscale,
                |el| {
                    el.child(
                        h_flex().mt_4().w_full().gap_3().child(
                            v_flex()
                                .w(gpui::relative(0.5))
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(typography::small(&theme))
                                        .line_height(px(17.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Model URL"),
                                )
                                .child(
                                    Input::new(&self.base_url_input)
                                        .h(px(40.0))
                                        .disabled(self.busy),
                                ),
                        ),
                    )
                },
            )
            .into_any_element()
    }

    fn provider_card(
        &self,
        choice: ProviderChoice,
        selected: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .id(SharedString::from(
                format!("provider-{:?}", choice).to_ascii_lowercase(),
            ))
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(77.0))
            .items_start()
            .gap_2p5()
            .px_3()
            .py_2p5()
            .rounded(px(12.0))
            .border_1()
            .border_color(if selected { theme.accent } else { theme.input })
            .bg(if selected {
                theme.accent.alpha(0.10)
            } else {
                theme.foreground.alpha(0.029)
            })
            .opacity(if self.busy { 0.5 } else { 1.0 })
            .when(
                crate::services::appearance::pointer_cursors_enabled(cx),
                |el| el.cursor_pointer(),
            )
            .hover(|style| style.bg(theme.muted))
            .focusable()
            .tab_stop(true)
            .focus(|style| style.bg(theme.list_active).border_color(theme.ring))
            .on_key_down(|event: &gpui::KeyDownEvent, _window, cx| {
                if selection_card_consumes_key(event.keystroke.key.as_str()) {
                    cx.stop_propagation();
                }
            })
            .on_click(cx.listener(move |this, _event, window, cx| {
                if this.busy {
                    return;
                }
                this.machine.choice = choice;
                this.machine.api_key.clear();
                this.machine.base_url.clear();
                this.machine.defer_pi_setup = false;
                this.open_pi_provider_setup_on_complete = false;
                this.selected_pi_provider_id = None;
                this.api_key_input
                    .update(cx, |input, inner| input.set_value("", window, inner));
                this.base_url_input.update(cx, |input, inner| {
                    input.set_value("", window, inner);
                    input.set_placeholder(
                        if choice == ProviderChoice::Tailscale {
                            "https://model.tailnet.ts.net/v1"
                        } else {
                            "Use provider default"
                        },
                        window,
                        inner,
                    );
                });
                cx.notify();
            }))
            .child(
                div()
                    .size(px(32.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        if choice == ProviderChoice::Tailscale {
                            Icon::default().path("native-icons/network.svg")
                        } else {
                            crate::chat::chat_pane::provider_icon_asset_path(
                                choice.icon_provider_id(),
                                "",
                            )
                            .map_or_else(
                                || Icon::new(IconName::Globe),
                                |path| Icon::default().path(path),
                            )
                        }
                        .with_size(px(20.0)),
                    ),
            )
            .child(
                v_flex()
                    .min_w(px(0.0))
                    .flex_1()
                    .gap_0p5()
                    .child(
                        div()
                            .truncate()
                            .text_size(typography::small(theme))
                            .line_height(px(17.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child(choice.title()),
                    )
                    .child(
                        div()
                            .max_w(px(182.0))
                            .text_size(typography::small(theme))
                            .line_height(px(16.0))
                            .text_color(theme.muted_foreground)
                            .child(choice.description()),
                    ),
            )
            .child(
                Icon::new(IconName::Check)
                    .small()
                    .flex_shrink_0()
                    .text_color(theme.accent)
                    .opacity(if selected { 1.0 } else { 0.0 }),
            )
    }

    fn more_provider_card(
        &self,
        status: &crate::services::pi_provider_setup::PiProviderStatus,
        selected: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let provider_id = status.provider.id.clone();
        let label = status.provider.label.clone();
        let can_choose = !self.busy
            && (status.configured || status.auth_methods.iter().any(|method| method.available));
        let setup_label = if status.configured {
            "Ready on this Mac".to_string()
        } else {
            let labels = status
                .auth_methods
                .iter()
                .filter(|method| method.available)
                .map(|method| method.label.as_str())
                .take(2)
                .collect::<Vec<_>>();
            if labels.is_empty() {
                "Requires system credentials".to_string()
            } else {
                labels.join(" or ")
            }
        };

        h_flex()
            .id(SharedString::from(format!("onboarding-more-{provider_id}")))
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(56.0))
            .items_center()
            .gap_2p5()
            .px_2p5()
            .py_2()
            .rounded(px(12.0))
            .border_1()
            .border_color(if selected {
                theme.accent
            } else {
                theme.input.opacity(0.0)
            })
            .bg(if selected {
                theme.accent.alpha(0.10)
            } else {
                theme.foreground.opacity(0.0)
            })
            .opacity(if can_choose { 1.0 } else { 0.5 })
            .when(
                can_choose && crate::services::appearance::pointer_cursors_enabled(cx),
                |el| el.cursor_pointer(),
            )
            .hover(|style| style.border_color(theme.border).bg(theme.muted))
            .focusable()
            .tab_stop(can_choose)
            .focus(|style| style.bg(theme.list_active).border_color(theme.ring))
            .when(can_choose, |el| {
                el.on_click(cx.listener(move |this, _event, window, cx| {
                    this.selected_pi_provider_id = Some(provider_id.clone());
                    this.machine.api_key.clear();
                    this.machine.base_url.clear();
                    this.api_key_input
                        .update(cx, |input, inner| input.set_value("", window, inner));
                    this.base_url_input
                        .update(cx, |input, inner| input.set_value("", window, inner));
                    this.machine.error = None;
                    cx.notify();
                }))
            })
            .child(
                div()
                    .size(px(32.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_lg()
                    .bg(theme.background)
                    .child(
                        crate::chat::chat_pane::provider_icon_asset_path(&status.provider.id, "")
                            .map_or_else(
                                || Icon::new(IconName::Globe),
                                |path| Icon::default().path(path),
                            )
                            .with_size(px(18.0)),
                    ),
            )
            .child(
                v_flex()
                    .min_w(px(0.0))
                    .flex_1()
                    .gap_0p5()
                    .child(
                        div()
                            .truncate()
                            .text_size(typography::small(theme))
                            .line_height(px(17.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child(label),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(typography::small(theme))
                            .line_height(px(17.0))
                            .text_color(theme.muted_foreground)
                            .child(setup_label),
                    ),
            )
            .child(
                Icon::new(IconName::Check)
                    .small()
                    .flex_shrink_0()
                    .text_color(theme.accent)
                    .opacity(if selected { 1.0 } else { 0.0 }),
            )
    }

    fn pi_provider_setup_modal(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(modal) = self.pi_setup.as_ref() else {
            return div().into_any_element();
        };
        let busy = modal.busy;
        let label = modal.provider.label.clone();
        let error = modal.error.clone();
        v_flex()
            .id("onboarding-pi-provider-setup-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.18))
            .on_mouse_down(gpui::MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation()
            })
            .on_click(cx.listener(|this, _event, window, cx| {
                cx.stop_propagation();
                this.close_pi_provider_setup(window, cx);
            }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_pi_provider_setup(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                v_flex()
                    .id("onboarding-pi-provider-setup-dialog")
                    .w(px(440.0))
                    .max_w(gpui::relative(0.9))
                    .gap_3()
                    .p_4()
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .shadow_lg()
                    .occlude()
                    .on_mouse_down(gpui::MouseButton::Left, |_event, _window, cx| {
                        cx.stop_propagation()
                    })
                    .on_click(|_event, _window, cx| cx.stop_propagation())
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Set up {label}")),
                    )
                    .child(
                        div()
                            .text_size(typography::small(theme))
                            .line_height(px(17.0))
                            .text_color(theme.muted_foreground)
                            .child(
                                "This credential is encrypted on this Mac and bound to Pi's exact provider catalog.",
                            ),
                    )
                    .child(Input::new(&self.api_key_input).mask_toggle().disabled(busy))
                    .when_some(error, |el, error| {
                        el.child(
                            div()
                                .text_size(typography::small(theme))
                                .line_height(px(17.0))
                                .text_color(theme.danger)
                                .child(error),
                        )
                    })
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                div().track_focus(&self.pi_setup_cancel_focus).tab_stop(true).child(
                                    Button::new("onboarding-pi-provider-cancel")
                                        .ghost()
                                        .small()
                                        .tab_stop(false)
                                        .label("Cancel")
                                        .disabled(busy)
                                        .on_click(cx.listener(
                                            |this, _event, window, cx| {
                                                this.close_pi_provider_setup(window, cx);
                                            },
                                        )),
                                ),
                            )
                            .child(
                                div().track_focus(&self.pi_setup_save_focus).tab_stop(true).child(
                                    Button::new("onboarding-pi-provider-save")
                                        .primary()
                                        .small()
                                        .tab_stop(false)
                                        .label(if busy { "Saving…" } else { "Save" })
                                        .disabled(busy)
                                        .on_click(cx.listener(
                                            |this, _event, window, cx| {
                                                this.save_pi_provider_setup(window, cx);
                                            },
                                        )),
                                ),
                            ),
                    ),
            )
            .into_any_element()
    }

    // -----------------------------------------------------------------------
    // Finish (TS "tour" step + the first-run marker write)
    // -----------------------------------------------------------------------

    fn finish_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();

        v_flex()
            .id("onboarding-finish")
            .w_full()
            .gap_5()
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(
                        Icon::new(IconName::Check)
                            .mt_0p5()
                            .with_size(px(20.0))
                            .text_color(theme.accent),
                    )
                    .child(self.step_heading(
                        "Everything Aiden brings together",
                        "Explore all 22 shipped features. Scroll, then hover or focus a tile to learn more.",
                        &theme,
                    )),
            )
            .children(TOUR_GROUPS.iter().enumerate().map(|(group_index, (group_id, group_title))| {
                let feature_count = TOUR_FEATURES
                    .iter()
                    .filter(|feature| feature.group == *group_id)
                    .count();
                v_flex()
                    .when(group_index > 0, |el| el.mt_2())
                    .gap_2p5()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .px_0p5()
                            .child(
                                div()
                                    .text_size(typography::small(&theme))
                                    .line_height(px(17.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.muted_foreground)
                                    .child(*group_title),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .line_height(px(13.0))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{feature_count} features")),
                            ),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(6)
                            .gap_2p5()
                            .children(
                                TOUR_FEATURES
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, feature)| feature.group == *group_id)
                                    .map(|(feature_index, feature)| {
                                        let group = feature.id;
                                        let focused = self.focused_tour_feature == Some(feature_index);
                                        let image = img(Arc::new(Image::from_bytes(
                                            ImageFormat::Png,
                                            feature.image.to_vec(),
                                        )))
                                        .size_full()
                                        .object_fit(ObjectFit::Contain);
                                        let image = match feature.size {
                                            TourFeatureSize::Hero => div()
                                                .absolute()
                                                .right(px(-12.0))
                                                .top(px(-12.0))
                                                .h(gpui::relative(1.16))
                                                .w(gpui::relative(0.72))
                                                .child(image),
                                            TourFeatureSize::Tall => div()
                                                .absolute()
                                                .left(px(4.0))
                                                .right(px(4.0))
                                                .top(px(4.0))
                                                .h(gpui::relative(0.72))
                                                .child(image),
                                            TourFeatureSize::Standard => div()
                                                .absolute()
                                                .right(px(4.0))
                                                .top(px(4.0))
                                                .size(px(76.0))
                                                .child(image),
                                            TourFeatureSize::Wide => div()
                                                .absolute()
                                                .right(px(4.0))
                                                .top_0()
                                                .h_full()
                                                .w(gpui::relative(0.46))
                                                .child(image),
                                        };
                                        let title_width = match feature.size {
                                            TourFeatureSize::Hero => 0.42,
                                            TourFeatureSize::Tall => 0.92,
                                            TourFeatureSize::Standard => 0.58,
                                            TourFeatureSize::Wide => 0.54,
                                        };
                                        div()
                                            .id(SharedString::from(format!(
                                                "feature-{}",
                                                feature.id
                                            )))
                                            .group(group)
                                            .relative()
                                            .h(px(feature.size.height()))
                                            .col_span(feature.size.column_span())
                                            .row_span(feature.size.row_span())
                                            .overflow_hidden()
                                            .rounded(px(12.0))
                                            .border_1()
                                            .border_color(theme.input)
                                            .bg(theme.foreground.alpha(0.029))
                                            .track_focus(&self.tour_focuses[feature_index])
                                            .tab_stop(true)
                                            .hover(|style| {
                                                style
                                                    .bg(theme.foreground.alpha(0.129))
                                                    .border_color(theme.border)
                                            })
                                            .focus(|style| {
                                                style
                                                    .bg(theme.list_active)
                                                    .border_color(theme.ring)
                                            })
                                            .on_key_down(
                                                |event: &gpui::KeyDownEvent, _window, cx| {
                                                    if tour_tile_consumes_key(
                                                        event.keystroke.key.as_str(),
                                                    ) {
                                                        cx.stop_propagation();
                                                    }
                                                },
                                            )
                                            .child(
                                                div()
                                                    .absolute()
                                                    .inset_0()
                                                    .group_hover(group, |style| {
                                                        style.opacity(0.0)
                                                    })
                                                    .opacity(if focused { 0.0 } else { 1.0 })
                                                    .child(image)
                                                    .child(
                                                        div()
                                                            .absolute()
                                                            .left(px(12.0))
                                                            .right(px(12.0))
                                                            .bottom(px(12.0))
                                                            .max_w(gpui::relative(title_width))
                                                            .text_size(typography::small(&theme))
                                                            .line_height(px(16.0))
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .child(feature.title),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .absolute()
                                                    .inset_0()
                                                    .justify_end()
                                                    .p_3()
                                                    .bg(theme.popover)
                                                    .opacity(if focused { 1.0 } else { 0.0 })
                                                    .group_hover(group, |style| {
                                                        style.opacity(1.0)
                                                    })
                                                    .child(
                                                        Icon::new(feature.icon.clone())
                                                            .small()
                                                            .absolute()
                                                            .right(px(12.0))
                                                            .top(px(12.0))
                                                            .text_color(theme.accent),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(typography::small(&theme))
                                                            .line_height(px(16.0))
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .child(feature.title),
                                                    )
                                                    .child(
                                                        div()
                                                            .mt_1()
                                                            .text_size(px(12.0))
                                                            .line_height(px(16.0))
                                                            .text_color(theme.muted_foreground)
                                                            .child(feature.description),
                                                    ),
                                            )
                                    }),
                            ),
                    )
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        onboarding_more_provider_indices, selection_card_consumes_key, tour_tile_consumes_key,
        TOUR_FEATURES,
    };

    #[test]
    fn more_providers_match_current_main_featured_then_catalog_order() {
        let ids = [
            "amazon-bedrock",
            "anthropic",
            "deepseek",
            "google",
            "kimi-coding",
            "openai",
            "openrouter",
            "radius",
            "xai",
        ];
        let ordered = onboarding_more_provider_indices(ids)
            .into_iter()
            .map(|index| ids[index]);

        assert_eq!(
            ordered.collect::<Vec<_>>(),
            vec![
                "google",
                "xai",
                "openrouter",
                "deepseek",
                "kimi-coding",
                "amazon-bedrock",
                "radius",
            ]
        );
    }

    #[test]
    fn every_advertised_tour_feature_has_a_bounded_square_rgba_png() {
        assert_eq!(TOUR_FEATURES.len(), 22);

        let mut ids = BTreeSet::new();
        for feature in &TOUR_FEATURES {
            assert!(ids.insert(feature.id), "duplicate tile id: {}", feature.id);
            assert!(!feature.title.trim().is_empty());
            assert!(!feature.description.trim().is_empty());
            assert!(!feature.description.contains("Hover any tile"));

            let png = feature.image;
            assert!(png.len() < 1_200_000, "{} asset is oversized", feature.id);
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "{} signature", feature.id);
            assert_eq!(&png[12..16], b"IHDR", "{} IHDR", feature.id);
            assert_eq!(
                u32::from_be_bytes(png[16..20].try_into().unwrap()),
                1024,
                "{} width",
                feature.id
            );
            assert_eq!(
                u32::from_be_bytes(png[20..24].try_into().unwrap()),
                1024,
                "{} height",
                feature.id
            );
            assert_eq!(png[24], 8, "{} bit depth", feature.id);
            assert_eq!(png[25], 6, "{} must be RGBA", feature.id);
        }

        assert_eq!(ids.len(), TOUR_FEATURES.len());
    }

    #[test]
    fn informational_tour_tiles_contain_activation_keys() {
        assert!(tour_tile_consumes_key("enter"));
        assert!(tour_tile_consumes_key("space"));
        assert!(!tour_tile_consumes_key("tab"));
        assert!(!tour_tile_consumes_key("escape"));
    }

    #[test]
    fn selection_cards_consume_enter_and_space_without_advancing_onboarding() {
        assert!(selection_card_consumes_key("enter"));
        assert!(selection_card_consumes_key("space"));
        assert!(!selection_card_consumes_key("tab"));
        assert!(!selection_card_consumes_key("escape"));
    }

    #[test]
    fn onboarding_selection_cards_keep_native_focus_and_keyboard_click_contract() {
        let source = include_str!("view.rs");
        let marker = "fn provider_card";
        let block = source
            .split_once(marker)
            .and_then(|(_, rest)| rest.split_once("fn "))
            .map(|(body, _)| body)
            .unwrap_or(source);
        assert!(block.contains(".focusable()"), "{marker} must be focusable");
        assert!(
            block.contains(".tab_stop(true)"),
            "{marker} must be keyboard reachable"
        );
        assert!(
            block.contains("selection_card_consumes_key"),
            "{marker} must consume Enter/Space"
        );
    }
}
