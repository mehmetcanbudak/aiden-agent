//! Onboarding flow (port of `renderer/components/onboarding-flow.tsx` +
//! `onboarding-flow.test.tsx`).
//!
//! [`OnboardingView`] is an `Entity + Render` flow view backed by the pure
//! step-state machine in [`state`]. The orchestrator opens it with
//! [`open_onboarding_window`] and is notified of completion through the
//! [`OnboardingServices::on_complete`] foreground callback (the flow's
//! `InputState` inputs require a gpui-component `Root`, so the window root is
//! `Root` and the completion event cannot be subscribed to directly).
//!
//! Completion mechanism: the view emits the gpui event
//! [`OnboardingEvent::Completed`] once the first-run marker
//! (`aiden:onboarding:v1:complete`, the exact TS `localStorage` key) is queued
//! into `settings.json`. An optional [`OnboardingServices::on_complete`]
//! callback (called on the foreground with `&mut App`) is also available for
//! creators that cannot subscribe to events.
//!
//! API deviation from the plan sketch (`pub fn new(cx, services)`): the view
//! owns gpui-component `InputState` entities, which require a `Window` to
//! construct, so the signature is `new(window, cx, services)` — the same shape
//! as `app::AppState::new(stores, window, cx)`.

mod state;
mod view;

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

use aiden_core::appearance::{parse_appearance_config, ReduceMotion};
use aiden_data::config_store::provider_connection_snapshot;
use aiden_data::portable_config::{
    ProviderModelMetadata, ProviderModelMetadataSource, ProviderModelType, StoredProvider,
};
use aiden_providers::live_discovery::{self, RuntimeKind};
use gpui::{
    actions, div, px, size, App, AppContext as _, Bounds, Context, Entity, EventEmitter,
    FocusHandle, Focusable as _, KeyBinding, ParentElement as _, ScrollHandle, Styled as _,
    Subscription, Task, Window, WindowBounds, WindowHandle, WindowOptions,
};
use gpui_component::{
    input::{InputEvent, InputState},
    ActiveTheme as _, Root, TitleBar, WindowExt as _,
};
use gpui_tokio_bridge::Tokio;

use crate::services::appearance::SETTINGS_APPEARANCE_KEY;
use crate::services::codex_auth::{CodexAuthAttemptGuard, CodexDialogLease};
use crate::services::pi_provider_setup::{PiProviderStatus, PiSetupLease};
use crate::services::stores::Stores;
use crate::typography;

use state::{
    NextOutcome, OnboardingProvider, ProviderChoice, Step, MODEL_SELECTION_SETTINGS_KEY,
    ONBOARDING_COMPLETE_KEY, PROFILE_NAME_SETTINGS_KEY,
};

enum CodexAuthUpdate {
    DeviceCode(crate::services::codex_auth::CodexDeviceAuthorization),
    Finished(Result<(), String>),
}

actions!(onboarding, [OnboardingNext, OnboardingBack, OnboardingSkip]);

/// Emitted once when the flow finishes (or is skipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingEvent {
    Completed,
}

/// Foreground completion callback invoked (with `&mut App`) when the flow
/// completes, in addition to the [`OnboardingEvent`] emission.
pub type PiProviderSetupRequest = (String, String, u64);
pub type CompletionCallback = Box<dyn Fn(Option<PiProviderSetupRequest>, &mut App)>;

struct OnboardingPiSetupModal {
    provider: OnboardingProvider,
    lease: PiSetupLease,
    busy: bool,
    error: Option<String>,
    return_focus: Option<FocusHandle>,
}

/// Native close is never allowed while the onboarding window is retained.
/// Completion removes this window programmatically after the durable marker
/// and main-window handoff are queued. Keeping the platform close vetoed even
/// after `completed_emitted` avoids a race where a red-✕ click lands between
/// the marker write and that deferred handoff, which would strand the process
/// with an invalid retained window handle.
pub(crate) const fn onboarding_native_close_allowed(_completed_emitted: bool) -> bool {
    false
}

/// Everything the flow needs from its creator.
pub struct OnboardingServices {
    pub stores: Stores,
    on_complete: Option<CompletionCallback>,
    force_show: bool,
    initial_step: Option<usize>,
}

impl OnboardingServices {
    pub fn new(stores: Stores) -> Self {
        Self {
            stores,
            on_complete: None,
            force_show: false,
            initial_step: None,
        }
    }

    /// Optional foreground callback invoked (with `&mut App`) when the flow
    /// completes. The gpui [`OnboardingEvent`] is emitted either way.
    pub fn with_on_complete(mut self, callback: CompletionCallback) -> Self {
        self.on_complete = Some(callback);
        self
    }

    /// Dev-only caller override used for visual QA without deleting the
    /// durable onboarding marker or any user settings.
    pub fn with_force_show(mut self, force_show: bool) -> Self {
        self.force_show = force_show;
        self
    }

    pub fn with_initial_step(mut self, initial_step: Option<usize>) -> Self {
        self.initial_step = initial_step;
        self
    }
}

/// The onboarding flow entity. All step logic lives in
/// [`OnboardingMachine`] (pure, unit-tested); this view renders it and
/// performs the per-step store writes.
pub struct OnboardingView {
    machine: OnboardingMachine,
    stores: Stores,
    on_complete: Option<CompletionCallback>,
    force_show: bool,
    name_input: Entity<InputState>,
    api_key_input: Entity<InputState>,
    base_url_input: Entity<InputState>,
    /// Async persistence in flight (TS `saving` — disables the buttons).
    busy: bool,
    /// Local-runtime model enumeration in flight (TS `discovering`).
    discovering: bool,
    booted: bool,
    completed_emitted: bool,
    open_pi_provider_setup_on_complete: bool,
    /// Last step the view focused, so focus moves only on step changes.
    focused_step: usize,
    /// Focus target for the primary action on non-Welcome steps.
    next_focus: FocusHandle,
    /// Per-tile focus handles make the native bento reveal the same detail
    /// surface for keyboard focus that current Main reveals with CSS.
    tour_focuses: Vec<FocusHandle>,
    focused_tour_feature: Option<usize>,
    step_scroll: ScrollHandle,
    pi_provider_statuses: Vec<PiProviderStatus>,
    selected_pi_provider_id: Option<String>,
    show_more_providers: bool,
    pi_setup: Option<OnboardingPiSetupModal>,
    pi_setup_cancel_focus: FocusHandle,
    pi_setup_save_focus: FocusHandle,
    codex_revision: u64,
    codex_attempt: Option<CodexAuthAttemptGuard>,
    _boot: Option<Task<anyhow::Result<()>>>,
    _subscriptions: Vec<Subscription>,
}

impl OnboardingView {
    /// `window` is required for the input entities (see the module docs for
    /// the deviation note).
    pub fn new(window: &mut Window, cx: &mut Context<Self>, services: OnboardingServices) -> Self {
        // Keyboard nav: Enter advances (the TS Enter-on-name behavior, extended
        // to every step), cmd-left goes back, cmd-right advances, Escape skips.
        // While an input is focused its own "Input" context consumes Enter /
        // cmd-left / cmd-right, so typing never advances accidentally.
        cx.bind_keys([
            KeyBinding::new("enter", OnboardingNext, Some("onboarding")),
            KeyBinding::new("cmd-right", OnboardingNext, Some("onboarding")),
            KeyBinding::new("cmd-left", OnboardingBack, Some("onboarding")),
            KeyBinding::new("escape", OnboardingSkip, Some("onboarding")),
        ]);

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Your name"));
        let api_key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Paste key")
                .masked(true)
        });
        let base_url_input = cx.new(|cx| InputState::new(window, cx).placeholder("Base URL"));

        let mut machine = OnboardingMachine::new();
        if let Some(initial_step) = services.initial_step {
            machine.start_at_step_for_visual_qa(initial_step);
        }
        let mut this = Self {
            machine,
            stores: services.stores,
            on_complete: services.on_complete,
            force_show: services.force_show,
            name_input,
            api_key_input,
            base_url_input,
            busy: false,
            discovering: false,
            booted: false,
            completed_emitted: false,
            open_pi_provider_setup_on_complete: false,
            focused_step: usize::MAX,
            next_focus: cx.focus_handle(),
            tour_focuses: (0..22).map(|_| cx.focus_handle()).collect(),
            focused_tour_feature: None,
            step_scroll: ScrollHandle::new(),
            pi_provider_statuses: Vec::new(),
            selected_pi_provider_id: None,
            show_more_providers: false,
            pi_setup: None,
            pi_setup_cancel_focus: cx.focus_handle(),
            pi_setup_save_focus: cx.focus_handle(),
            codex_revision: 0,
            codex_attempt: None,
            _boot: None,
            _subscriptions: Vec::new(),
        };

        // Keep the machine in sync with the inputs; Enter submits (TS).
        this._subscriptions.push(cx.subscribe_in(
            &this.name_input,
            window,
            |this, _source, event, window, cx| match event {
                InputEvent::Change => {
                    this.machine.name = this.name_input.read(cx).value().to_string();
                    cx.notify();
                }
                InputEvent::PressEnter { secondary: false } => this.on_next_pressed(window, cx),
                InputEvent::PressEnter { secondary: true }
                | InputEvent::Focus
                | InputEvent::Blur => {}
            },
        ));
        this._subscriptions.push(cx.subscribe_in(
            &this.api_key_input,
            window,
            |this, _source, event, window, cx| match event {
                InputEvent::Change => {
                    this.machine.api_key = this.api_key_input.read(cx).value().to_string();
                    cx.notify();
                }
                InputEvent::PressEnter { secondary: false } => {
                    if this.pi_setup.is_some() {
                        this.save_pi_provider_setup(window, cx);
                    } else {
                        this.on_next_pressed(window, cx);
                    }
                }
                InputEvent::PressEnter { secondary: true }
                | InputEvent::Focus
                | InputEvent::Blur => {}
            },
        ));
        this._subscriptions.push(cx.subscribe_in(
            &this.base_url_input,
            window,
            |this, _source, event, window, cx| match event {
                InputEvent::Change => {
                    this.machine.base_url = this.base_url_input.read(cx).value().to_string();
                    cx.notify();
                }
                InputEvent::PressEnter { secondary: false } => this.on_next_pressed(window, cx),
                InputEvent::PressEnter { secondary: true }
                | InputEvent::Focus
                | InputEvent::Blur => {}
            },
        ));

        for (index, handle) in this.tour_focuses.clone().into_iter().enumerate() {
            this._subscriptions
                .push(cx.on_focus(&handle, window, move |this, _window, cx| {
                    this.focused_tour_feature = Some(index);
                    cx.notify();
                }));
            this._subscriptions
                .push(cx.on_blur(&handle, window, move |this, _window, cx| {
                    if this.focused_tour_feature == Some(index) {
                        this.focused_tour_feature = None;
                        cx.notify();
                    }
                }));
        }

        this.boot(cx);
        this
    }

    /// Load settings on the background, then either close immediately
    /// (already completed) or populate the three-step machine.
    fn boot(&mut self, cx: &mut Context<Self>) {
        let stores = self.stores.clone();
        let task = cx.spawn(async move |this, cx| -> anyhow::Result<()> {
            let (settings, codex_configured, pi_provider_statuses) = cx
                .background_spawn(async move {
                    let settings = stores.config.get_settings().unwrap_or_default();
                    let codex_configured = stores.codex_auth.is_configured().unwrap_or(false);
                    let pi_provider_statuses = stores.pi_providers.list();
                    (settings, codex_configured, pi_provider_statuses)
                })
                .await;
            this.update(cx, |this, cx| {
                if !this.force_show && !should_show_onboarding(&settings) {
                    // Marker already set (e.g. re-entrant open): self-close.
                    this.complete_onboarding(cx);
                    return;
                }
                let reduce_motion = settings
                    .get(SETTINGS_APPEARANCE_KEY)
                    .and_then(|value| parse_appearance_config(value).ok())
                    .map(|config| config.reduce_motion)
                    .unwrap_or(ReduceMotion::System);
                this.machine.set_reduce_motion(reduce_motion);
                this.machine.codex_configured = codex_configured;
                if codex_configured {
                    this.machine.record_codex_configured();
                }
                this.pi_provider_statuses = pi_provider_statuses;
                this.booted = true;
                cx.notify();
            })?;
            Ok(())
        });
        self._boot = Some(task);
    }

    // -----------------------------------------------------------------------
    // Navigation + persistence orchestration
    // -----------------------------------------------------------------------

    fn on_next(&mut self, _: &OnboardingNext, window: &mut Window, cx: &mut Context<Self>) {
        self.on_next_pressed(window, cx);
    }

    /// Advance the machine and run the per-step persistence. Shared by the
    /// Next button, the Enter bindings, and the input PressEnter events.
    fn on_next_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || !self.booted || self.completed_emitted {
            return;
        }
        let step = self.machine.current();
        if step == Step::Provider {
            if self.selected_pi_provider_id.is_some() {
                self.continue_with_selected_pi_provider(window, cx);
                return;
            }
            if self.machine.choice == ProviderChoice::ChatGpt && !self.machine.codex_configured {
                self.start_codex_sign_in(window, cx);
                return;
            }
        }
        if let Some(message) = self.machine.validate() {
            self.machine.error = Some(message);
            cx.notify();
            return;
        }
        match step {
            Step::Provider => self.save_provider_then_advance(cx),
            _ => {
                let from = step;
                match self.machine.advance() {
                    NextOutcome::Advanced => self.persist_after_step(from, cx),
                    NextOutcome::Completed => self.complete_onboarding(cx),
                    // `advance()` never yields `Blocked` (only `next()` does, and
                    // the wired view calls `advance()` directly after `validate()`).
                    // Treat the unreachable case as a quiet no-op instead of
                    // panicking inside an ObjC event callback (panic_cannot_unwind).
                    NextOutcome::Blocked => {
                        tracing::error!(
                            "onboarding advance returned Blocked after a passing validate"
                        );
                    }
                }
                cx.notify();
            }
        }
    }

    /// Persist what the step we just left collected. Mirrors current Main's
    /// profile save; provider and its implicit default model are saved by the
    /// provider-specific async path.
    fn persist_after_step(&mut self, from: Step, cx: &mut Context<Self>) {
        let stores = self.stores.clone();
        match from {
            Step::Welcome => {
                let name = self.machine.name.trim().to_string();
                cx.spawn(async move |_, cx| {
                    let _ = cx
                        .background_spawn(async move {
                            let mut patch = serde_json::Map::new();
                            patch.insert(
                                PROFILE_NAME_SETTINGS_KEY.to_string(),
                                serde_json::Value::String(name),
                            );
                            let _ = stores.config.set_settings(&patch, &|| true);
                        })
                        .await;
                })
                .detach();
            }
            Step::Provider | Step::Finish => {}
        }
    }

    fn persist_default_model_selection(&self, cx: &mut Context<Self>) {
        let Some((provider_id, model)) = self.machine.default_selection() else {
            return;
        };
        let stores = self.stores.clone();
        cx.spawn(async move |_, cx| {
            let _ = cx
                .background_spawn(async move {
                    let mut patch = serde_json::Map::new();
                    patch.insert(
                        MODEL_SELECTION_SETTINGS_KEY.to_string(),
                        serde_json::json!({ "providerId": provider_id, "model": model }),
                    );
                    let _ = stores.config.set_settings(&patch, &|| true);
                })
                .await;
        })
        .detach();
    }

    fn continue_with_selected_pi_provider(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(provider_id) = self.selected_pi_provider_id.clone() else {
            return;
        };
        let Some(status) = self
            .stores
            .pi_providers
            .list()
            .into_iter()
            .find(|status| status.provider.id == provider_id)
        else {
            self.machine.error = Some("That provider is no longer available. Choose another.");
            cx.notify();
            return;
        };
        let provider = onboarding_provider_from_pi_status(&status);
        if status.configured {
            self.machine.record_provider_saved(provider);
            self.persist_default_model_selection(cx);
            let _ = self.machine.advance();
            cx.notify();
            return;
        }
        if !status.auth_methods.iter().any(|method| method.available) {
            self.machine.error =
                Some("That provider's setup method is unavailable in this Rust release.");
            cx.notify();
            return;
        }

        self.api_key_input.update(cx, |input, inner| {
            input.set_value("", window, inner);
            input.set_placeholder("Paste API key", window, inner);
        });
        let focus = self.api_key_input.read(cx).focus_handle(cx);
        self.pi_setup = Some(OnboardingPiSetupModal {
            provider,
            lease: self.stores.pi_providers.begin_setup(),
            busy: false,
            error: None,
            return_focus: window.focused(cx),
        });
        cx.defer_in(window, move |_this, window, _cx| focus.focus(window));
        cx.notify();
    }

    fn close_pi_provider_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pi_setup.as_ref().is_some_and(|modal| modal.busy) {
            return;
        }
        let return_focus = self.pi_setup.take().and_then(|modal| modal.return_focus);
        if let Some(return_focus) = return_focus {
            cx.defer_in(window, move |_this, window, _cx| return_focus.focus(window));
        }
        cx.notify();
    }

    fn save_pi_provider_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(modal) = self.pi_setup.as_mut() else {
            return;
        };
        if modal.busy {
            return;
        }
        let key = self.api_key_input.read(cx).value().to_string();
        let provider = modal.provider.clone();
        let lease = modal.lease;
        modal.busy = true;
        modal.error = None;
        let authority = self.stores.pi_providers.clone();
        cx.spawn_in(window, async move |this, cx| {
            let provider_id = provider.id.clone();
            let result = cx
                .background_spawn(
                    async move { authority.commit_api_key(&provider_id, &key, lease) },
                )
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                let Some(modal) = this.pi_setup.as_mut() else {
                    return;
                };
                modal.busy = false;
                match result {
                    Ok(()) => {
                        this.pi_provider_statuses = this.stores.pi_providers.list();
                        this.machine.record_provider_saved(provider);
                        this.persist_default_model_selection(cx);
                        let _ = this.machine.advance();
                        this.close_pi_provider_setup(window, cx);
                    }
                    Err(error) => {
                        modal.error = Some(error.to_string());
                        cx.notify();
                    }
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// Provider step: write the provider record + API key on the background,
    /// persist its implicit default model, then advance directly to the tour.
    fn save_provider_then_advance(&mut self, cx: &mut Context<Self>) {
        if self.machine.choice == ProviderChoice::ChatGpt && self.machine.codex_configured {
            self.machine.record_codex_configured();
            self.persist_default_model_selection(cx);
            let _ = self.machine.advance();
            cx.notify();
            return;
        }
        let choice = self.machine.choice;
        let is_local_runtime = matches!(choice, ProviderChoice::LmStudio | ProviderChoice::Ollama);
        self.busy = true;
        self.discovering = is_local_runtime;
        cx.notify();
        let pending = self.machine.pending_provider_save();
        let deferred_pi_setup = self.machine.defer_pi_setup;
        let should_persist_selection = !deferred_pi_setup;
        let stores = self.stores.clone();
        let operation = Tokio::spawn(cx, async move {
            let Some(provider) = &pending.provider else {
                return Ok::<Option<OnboardingProvider>, String>(None);
            };
            if deferred_pi_setup {
                return Ok(Some(provider.clone()));
            }

            let mut stored = stored_provider_from_onboarding(provider);
            if is_local_runtime {
                if let Some(existing) = stores
                    .config
                    .get_provider(&provider.id)
                    .map_err(|error| format!("Couldn't load that provider: {error}"))?
                {
                    // Match Main's reserved-local intent rule: an existing LM
                    // Studio/Ollama record owns its endpoint, protocol, label,
                    // auth posture, metadata, and extra forward-compatible data.
                    stored = existing;
                }
                let runtime = match choice {
                    ProviderChoice::LmStudio => RuntimeKind::LmStudio,
                    ProviderChoice::Ollama => RuntimeKind::Ollama,
                    _ => unreachable!("local-runtime branch is choice-fenced"),
                };
                let api_key = if stored.needs_key {
                    stores
                        .config
                        .get_bound_provider_key(&stored)
                        .map_err(|error| format!("Couldn't load that provider key: {error}"))?
                } else {
                    None
                };
                if stored.needs_key && api_key.is_none() {
                    return Err("Enter the API key for this connection before testing.".to_string());
                }
                let discovered = live_discovery::discover_models_with_auth(
                    &stored.base_url,
                    runtime,
                    &live_discovery::DiscoveryOptions::default(),
                    api_key.as_deref(),
                )
                .await
                .map_err(|error| format!("Couldn't reach {}: {error}", provider.label))?;
                if discovered.is_empty() {
                    return Err(
                        "Endpoint reached, but no chat models were found. Load one in the server, then try again."
                            .to_string(),
                    );
                }
                let discovered_ids = discovered
                    .iter()
                    .map(|model| model.id.clone())
                    .collect::<Vec<_>>();
                let default_model = stored
                    .default_model
                    .as_ref()
                    .filter(|model| discovered_ids.contains(model))
                    .cloned()
                    .unwrap_or_else(|| discovered_ids[0].clone());
                let source = match runtime {
                    RuntimeKind::LmStudio => ProviderModelMetadataSource::Lmstudio,
                    RuntimeKind::Ollama => ProviderModelMetadataSource::Ollama,
                    RuntimeKind::Generic => ProviderModelMetadataSource::Provider,
                };
                stored.models = discovered_ids;
                stored.model_metadata = Some(
                    discovered
                        .into_iter()
                        .map(|model| {
                            let id = model.id;
                            (
                                id,
                                ProviderModelMetadata {
                                    source,
                                    name: model.name,
                                    r#type: Some(ProviderModelType::Llm),
                                    vision: None,
                                    tool_call: None,
                                    reasoning: None,
                                    thinking_levels: None,
                                    thinking_can_disable: None,
                                    context_length: model.context_window.map(u64::from),
                                    parameter_count: None,
                                    format: None,
                                },
                            )
                        })
                        .collect::<BTreeMap<_, _>>(),
                );
                stored.default_model = Some(default_model);
            }

            stores
                .config
                .save_provider(&stored, &|| true)
                .map_err(|error| format!("Couldn't add that provider: {error}"))?;
            if let Some(key) = &pending.api_key {
                stores
                    .keys
                    .set_bound(&stored.id, key, &provider_connection_snapshot(&stored))
                    .map_err(|error| format!("Couldn't save the API key: {error}"))?;
            }
            Ok(Some(onboarding_provider_from_stored(&stored)))
        });
        cx.spawn(async move |this, cx| {
            let result = operation
                .await
                .map_err(|_| "Provider setup was interrupted. Try again.".to_string())
                .and_then(|result| result);
            match result {
                Ok(saved) => {
                    let _ = this.update(cx, |this, cx| {
                        if let Some(provider) = saved {
                            this.machine.record_provider_saved(provider);
                            if should_persist_selection {
                                this.persist_default_model_selection(cx);
                            }
                        }
                        this.busy = false;
                        this.discovering = false;
                        match this.machine.advance() {
                            NextOutcome::Advanced => {}
                            NextOutcome::Completed => this.complete_onboarding(cx),
                            // Same invariant as `on_next_pressed`: `advance()`
                            // never yields Blocked. Stay panic-free on the
                            // foreground update dispatched from this task.
                            NextOutcome::Blocked => {
                                tracing::error!(
                                    "onboarding advance returned Blocked after the provider write"
                                );
                            }
                        }
                        cx.notify();
                    });
                }
                Err(message) => {
                    let _ = this.update(cx, |this, cx| {
                        this.busy = false;
                        this.discovering = false;
                        this.machine.error = Some(static_str(&message));
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn start_codex_sign_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.codex_revision = self.codex_revision.wrapping_add(1);
        let revision = self.codex_revision;
        self.busy = true;
        self.machine.error = None;
        let auth_store = self.stores.codex_auth.clone();
        let attempt = CodexAuthAttemptGuard::new(auth_store.clone());
        let cancelled = attempt.cancelled();
        let auth_revision = attempt.revision();
        self.codex_attempt = Some(attempt);
        let dialog_lease = CodexDialogLease::default();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        Tokio::spawn(cx, async move {
            let oauth = crate::services::codex_auth::CodexDeviceOAuth::default();
            let authorization = match oauth.begin(&cancelled).await {
                Ok(authorization) => authorization,
                Err(error) => {
                    let _ = tx.send(CodexAuthUpdate::Finished(Err(error.to_string())));
                    return;
                }
            };
            if tx
                .send(CodexAuthUpdate::DeviceCode(authorization.clone()))
                .is_err()
            {
                return;
            }
            let result = oauth
                .complete(&authorization, &cancelled)
                .await
                .and_then(|credential| {
                    auth_store
                        .commit_auth_attempt(auth_revision, &credential)
                        .and_then(|committed| {
                            if committed {
                                Ok(())
                            } else {
                                Err(aiden_providers::ProviderError::Auth(
                                    "ChatGPT sign-in was cancelled.".to_string(),
                                ))
                            }
                        })
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(CodexAuthUpdate::Finished(result));
        })
        .detach();

        let return_focus = window.focused(cx);
        cx.spawn_in(window, async move |this, cx| -> anyhow::Result<()> {
            while let Some(update) = rx.recv().await {
                let done = matches!(update, CodexAuthUpdate::Finished(_));
                this.update_in(cx, |this, window, cx| {
                    if matches!(update, CodexAuthUpdate::Finished(_))
                        && dialog_lease.take_owned_dialog()
                    {
                        window.close_dialog(cx);
                        if let Some(focus) = &return_focus {
                            focus.focus(window);
                        }
                    }
                    if !crate::services::codex_auth::auth_revision_is_current(
                        this.codex_revision,
                        revision,
                    ) {
                        return;
                    }
                    match update {
                        CodexAuthUpdate::DeviceCode(authorization) => {
                            cx.open_url(crate::services::codex_auth::DEVICE_VERIFICATION_URI);
                            let code = authorization.user_code;
                            let cancel = this
                                .codex_attempt
                                .as_ref()
                                .map(CodexAuthAttemptGuard::cancelled);
                            let auth_store = this.stores.codex_auth.clone();
                            let return_focus = return_focus.clone();
                            let dialog_lease = dialog_lease.clone();
                            dialog_lease.mark_open();
                            window.open_dialog(cx, move |dialog, _window, cx| {
                                let cancel = cancel.clone();
                                let auth_store = auth_store.clone();
                                let return_focus = return_focus.clone();
                                let cancel_lease = dialog_lease.clone();
                                let close_lease = dialog_lease.clone();
                                dialog
                                    .title("Sign in to ChatGPT")
                                    .overlay_closable(false)
                                    .child(
                                        gpui_component::v_flex()
                                            .gap_3()
                                            .child("Enter this temporary code on OpenAI's verification page:")
                                            .child(div().text_2xl().font_weight(gpui::FontWeight::SEMIBOLD).child(code.clone()))
                                            .child(div().text_size(typography::small(cx.theme())).text_color(cx.theme().muted_foreground).child("OAuth tokens stay encrypted in this Mac's Keychain.")),
                                    )
                                    .footer(|_, cancel_button, window, cx| vec![cancel_button(window, cx)])
                                    .on_cancel(move |_, _, _| {
                                        if let Some(cancel) = &cancel {
                                            cancel.store(true, Ordering::SeqCst);
                                        }
                                        auth_store.invalidate_auth_attempts();
                                        cancel_lease.request_focus_restore();
                                        true
                                    })
                                    .on_close(move |_, window, _| {
                                        close_lease.mark_closed();
                                        if close_lease.should_restore_focus() {
                                            if let Some(focus) = &return_focus {
                                                focus.focus(window);
                                            }
                                        }
                                    })
                            });
                        }
                        CodexAuthUpdate::Finished(result) => {
                            this.busy = false;
                            this.codex_attempt = None;
                            match result {
                                Ok(()) => {
                                    this.machine.record_codex_configured();
                                    this.persist_default_model_selection(cx);
                                    let _ = this.machine.advance();
                                }
                                Err(error) => this.machine.error = Some(static_str(&error)),
                            }
                        }
                    }
                    cx.notify();
                })?;
                if done {
                    break;
                }
            }
            Ok(())
        })
        .detach();
        cx.notify();
    }

    fn on_back(&mut self, _: &OnboardingBack, _window: &mut Window, cx: &mut Context<Self>) {
        self.back_pressed(cx);
    }

    /// Skip the whole flow (TS "Skip" button / our Escape binding).
    fn on_skip(&mut self, _: &OnboardingSkip, _window: &mut Window, cx: &mut Context<Self>) {
        self.skip_pressed(cx);
    }

    /// The Back button / cmd-left handler.
    fn back_pressed(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.completed_emitted {
            return;
        }
        self.machine.back();
        cx.notify();
    }

    /// The Skip button / Escape handler.
    fn skip_pressed(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.completed_emitted {
            return;
        }
        self.machine.skip();
        self.complete_onboarding(cx);
    }

    fn pi_provider_setup_target(&self) -> Option<PiProviderSetupRequest> {
        let provider = self.machine.saved_provider()?;
        self.stores
            .pi_providers
            .list()
            .iter()
            .find(|status| status.provider.id == provider.id)
            .map(|status| (provider.id.clone(), provider.label.clone(), status.revision))
    }

    /// Queue the first-run marker into settings.json, then run the completion
    /// callback and emit the event.
    ///
    /// Two ordering rules keep the completion path crash/race-free:
    ///
    /// 1. The marker is written BEFORE the callback runs. The callback opens
    ///    the main window; if a quit lands right after "Start using Aiden",
    ///    a fire-and-forget marker write could be lost and onboarding would
    ///    reappear on the next launch.
    ///
    /// 2. The callback is deferred out of the current update cycle. gpui
    ///    takes a window out of the app's window map for the duration of any
    ///    event dispatch (`update_window_id` → `windows.get_mut(id)?.take()?`),
    ///    and the callback closes THIS window (`handle.update(remove_window)`)
    ///    before opening the main one. Invoked synchronously from a button /
    ///    key handler on this window, that nested `handle.update` fails with
    ///    "window not found" and the onboarding window stays open on top of
    ///    the main window — both visible at once. Deferring runs the callback
    ///    at the end of the effect cycle, after this window is back in the map.
    fn complete_onboarding(&mut self, cx: &mut Context<Self>) {
        if self.completed_emitted {
            return;
        }
        self.completed_emitted = true;
        let pi_provider_setup = self
            .open_pi_provider_setup_on_complete
            .then(|| self.pi_provider_setup_target())
            .flatten();
        let stores = self.stores.clone();
        cx.spawn(async move |this, cx| {
            let _ = cx
                .background_spawn(async move {
                    let mut patch = serde_json::Map::new();
                    patch.insert(
                        ONBOARDING_COMPLETE_KEY.to_string(),
                        serde_json::Value::String("true".into()),
                    );
                    stores.config.set_settings(&patch, &|| true)
                })
                .await;
            this.update(cx, |this, cx| {
                if let Some(callback) = this.on_complete.take() {
                    cx.defer(move |cx| callback(pi_provider_setup, cx));
                }
                cx.emit(OnboardingEvent::Completed);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl EventEmitter<OnboardingEvent> for OnboardingView {}

fn onboarding_provider_from_pi_status(status: &PiProviderStatus) -> OnboardingProvider {
    let provider = &status.provider;
    OnboardingProvider {
        id: provider.id.clone(),
        kind: provider.kind,
        label: provider.label.clone(),
        base_url: provider.base_url.clone(),
        models: provider.models.clone(),
        default_model: provider.default_model.clone(),
        needs_key: provider.needs_key,
        deployment: provider
            .deployment
            .unwrap_or(aiden_data::portable_config::ProviderDeployment::Hosted),
    }
}

fn stored_provider_from_onboarding(provider: &OnboardingProvider) -> StoredProvider {
    StoredProvider {
        id: provider.id.clone(),
        kind: provider.kind,
        label: provider.label.clone(),
        base_url: provider.base_url.clone(),
        models: provider.models.clone(),
        model_metadata: None,
        default_model: provider.default_model.clone(),
        needs_key: provider.needs_key,
        deployment: Some(provider.deployment),
        is_preset: None,
        is_builtin: None,
        extra: serde_json::Map::new(),
    }
}

fn onboarding_provider_from_stored(provider: &StoredProvider) -> OnboardingProvider {
    OnboardingProvider {
        id: provider.id.clone(),
        kind: provider.kind,
        label: provider.label.clone(),
        base_url: provider.base_url.clone(),
        models: provider.models.clone(),
        default_model: provider.default_model.clone(),
        needs_key: provider.needs_key,
        deployment: provider
            .deployment
            .unwrap_or(aiden_data::portable_config::ProviderDeployment::Hosted),
    }
}

/// Leak a heap string into `'static` for the machine's error slot (the errors
/// are short-lived, user-facing messages).
fn static_str(message: &str) -> &'static str {
    Box::leak(message.to_string().into_boxed_str())
}

/// Open the onboarding flow in its own window. The window's root view is a
/// gpui-component `Root` (the flow's `InputState` inputs paint through the
/// Root layer), with [`OnboardingView`] as the Root's child; the returned
/// handle therefore targets `Root`, and completion is delivered through the
/// [`OnboardingServices::on_complete`] callback rather than an event
/// subscription.
pub fn open_onboarding_window(
    cx: &mut App,
    services: OnboardingServices,
) -> anyhow::Result<WindowHandle<Root>> {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(1000.0), px(700.0)),
            cx,
        ))),
        titlebar: Some(TitleBar::title_bar_options()),
        window_background: gpui::WindowBackgroundAppearance::Blurred,
        app_id: Some("com.sambitcreate.aiden-agent.onboarding".to_string()),
        tabbing_identifier: Some("aiden-onboarding".to_string()),
        ..Default::default()
    };

    cx.open_window(options, |window, cx| {
        let view = cx.new(|cx| OnboardingView::new(window, cx, services));
        let weak_view = view.downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let allow = weak_view
                .update(cx, |view, _cx| {
                    onboarding_native_close_allowed(view.completed_emitted)
                })
                .unwrap_or(false);
            if !allow {
                // Keep the incomplete flow visible and make a Dock click or
                // native close attempt an activation rather than a teardown.
                window.activate_window();
                cx.activate(true);
            }
            allow
        });
        cx.new(|cx| Root::new(view, window, cx))
    })
}

/// Re-exported for the orchestrator's first-run check in `main.rs`.
pub use state::should_show_onboarding;
/// Re-exported for the orchestrator and tests.
pub use state::OnboardingMachine;

#[cfg(test)]
mod lifecycle_tests {
    use super::{onboarding_native_close_allowed, OnboardingMachine};
    use crate::onboarding::state::{NextOutcome, Step};

    #[test]
    fn incomplete_native_close_is_vetoed_without_completing_or_losing_draft() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".to_string();
        assert_eq!(machine.next(), NextOutcome::Advanced);
        machine.api_key = "draft-key".to_string();
        let retained = machine.clone();

        assert!(!onboarding_native_close_allowed(false));
        assert!(!machine.is_complete());
        assert_eq!(retained.current(), Step::Provider);
        assert_eq!(retained.step_index(), machine.step_index());
        assert_eq!(retained.name, "Ada");
        assert_eq!(retained.api_key, "draft-key");
    }

    #[test]
    fn native_close_remains_vetoed_until_programmatic_completion_handoff() {
        assert!(!onboarding_native_close_allowed(false));
        assert!(!onboarding_native_close_allowed(true));
    }
}
