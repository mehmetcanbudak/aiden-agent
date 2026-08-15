//! Pure onboarding state machine (port of `renderer/components/onboarding-flow.tsx`
//! + `onboarding-flow.test.tsx`).
//!
//! The Electron flow is three steps (`profile → provider → tour`). The
//! native flow intentionally keeps that exact information architecture:
//!
//! | this port | TS original | notes |
//! |---|---|---|
//! | `Welcome` | `profile` | name entry, persisted under `profileName` |
//! | `Provider` | `provider` | real Aiden providers; key → keychain, default model selected implicitly |
//! | `Finish` | `tour` | "Aiden is ready" feature bento; writes the first-run marker |
//!
//! Preserved TS contracts (unit-tested below): `shouldShowOnboarding` reads the
//! marker `aiden:onboarding:v1:complete`, `make_onboarding_provider` emits the
//! exact current-Main onboarding provider IDs and model lists, and the
//! provider-step validation copy matches the TS toasts.

use aiden_core::appearance::ReduceMotion;
use aiden_data::portable_config::{ProviderDeployment, ProviderKind};

/// The first-run-complete marker. The TS stored it in `localStorage` under
/// `STORAGE_KEY = "aiden:onboarding:v1:complete"`; the Rust port persists the
/// same key string into `settings.json` via the config store so the disk
/// contract stays byte-compatible.
pub const ONBOARDING_COMPLETE_KEY: &str = "aiden:onboarding:v1:complete";
/// `settings.json` key for the profile name (TS `profileService.setName`).
pub const PROFILE_NAME_SETTINGS_KEY: &str = "profileName";
/// `settings.json` key for the provider+model selection (matches the chat
/// service's `MODEL_SELECTION_KEY`).
pub const MODEL_SELECTION_SETTINGS_KEY: &str = "modelSelection";
/// TS `MAX_PROFILE_NAME_LENGTH` (profile-core) and the Input `maxLength`.
pub const MAX_PROFILE_NAME_LENGTH: usize = 80;

/// The three current-Main onboarding steps, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Welcome,
    Provider,
    Finish,
}

impl Step {
    pub const ALL: &'static [Step] = &[Step::Welcome, Step::Provider, Step::Finish];

    #[allow(dead_code)] // renderer-contract helper; the view tracks the machine step directly
    pub fn index(self) -> usize {
        match self {
            Step::Welcome => 0,
            Step::Provider => 1,
            Step::Finish => 2,
        }
    }

    pub fn from_index(index: usize) -> Step {
        Self::ALL.get(index).copied().unwrap_or(Step::Finish)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Step::Welcome => "Your profile",
            Step::Provider => "Model provider",
            Step::Finish => "Ready to go",
        }
    }
}

/// The six compact provider choices from current Main. Pi's remaining release-
/// pinned providers are rendered separately behind "Choose from more" so the
/// first view, copy, and ordering stay identical to the Electron flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderChoice {
    Openai,
    ChatGpt,
    Anthropic,
    LmStudio,
    Ollama,
    Tailscale,
    // Kept as hidden factory variants for compatibility with existing
    // provider-state tests and stored drafts; these are surfaced through the
    // release-pinned Pi catalog rather than the compact primary grid.
    Google,
    DeepSeek,
    Moonshot,
}

impl ProviderChoice {
    pub const ALL: &'static [ProviderChoice] = &[
        ProviderChoice::Openai,
        ProviderChoice::ChatGpt,
        ProviderChoice::Anthropic,
        ProviderChoice::LmStudio,
        ProviderChoice::Ollama,
        ProviderChoice::Tailscale,
    ];

    pub fn title(self) -> &'static str {
        match self {
            ProviderChoice::Openai => "OpenAI API key",
            ProviderChoice::ChatGpt => "ChatGPT sign in",
            ProviderChoice::Anthropic => "Anthropic API key",
            ProviderChoice::LmStudio => "LM Studio",
            ProviderChoice::Ollama => "Ollama",
            ProviderChoice::Tailscale => "Tailscale custom model",
            ProviderChoice::Google => "Google Gemini",
            ProviderChoice::DeepSeek => "DeepSeek",
            ProviderChoice::Moonshot => "Moonshot (Kimi)",
        }
    }

    pub const fn icon_provider_id(self) -> &'static str {
        match self {
            ProviderChoice::Openai => "openai",
            ProviderChoice::ChatGpt => "openai-codex",
            ProviderChoice::Anthropic => "anthropic",
            ProviderChoice::LmStudio => "custom:lmstudio",
            ProviderChoice::Ollama => "custom:ollama",
            ProviderChoice::Tailscale => "",
            ProviderChoice::Google => "google",
            ProviderChoice::DeepSeek => "deepseek",
            ProviderChoice::Moonshot => "moonshotai",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ProviderChoice::Openai => "Connect with your own API key.",
            ProviderChoice::ChatGpt => "Connect through browser sign-in.",
            ProviderChoice::Anthropic => "Connect with your Anthropic API key.",
            ProviderChoice::LmStudio => "Use models running in LM Studio.",
            ProviderChoice::Ollama => "Use models running in Ollama.",
            ProviderChoice::Tailscale => "Connect to a private model on your tailnet.",
            ProviderChoice::Google => "Use Gemini with your Google AI Studio or Gemini API key.",
            ProviderChoice::DeepSeek => "Use DeepSeek's hosted models with your own API key.",
            ProviderChoice::Moonshot => {
                "Use Moonshot AI's hosted Kimi models with your own API key."
            }
        }
    }

    pub fn footnote(self) -> &'static str {
        match self {
            ProviderChoice::Openai => "The key stays on this Mac.",
            ProviderChoice::ChatGpt => "OAuth tokens stay encrypted in this Mac's Keychain.",
            ProviderChoice::Anthropic | ProviderChoice::Google => {
                "The key stays on this Mac and can be rotated later in Settings."
            }
            ProviderChoice::DeepSeek | ProviderChoice::Moonshot => {
                "Key is saved through Aiden's local secret storage."
            }
            ProviderChoice::LmStudio => "Default URL: http://127.0.0.1:1234/v1",
            ProviderChoice::Ollama => "Default URL: http://127.0.0.1:11434/v1",
            ProviderChoice::Tailscale => "Connects only to the URL you provide.",
        }
    }

    /// TS `requiresKey`: every hosted provider needs a key; the local servers
    /// are keyless.
    pub fn requires_key(self) -> bool {
        matches!(
            self,
            ProviderChoice::Openai
                | ProviderChoice::Anthropic
                | ProviderChoice::Google
                | ProviderChoice::DeepSeek
                | ProviderChoice::Moonshot
        )
    }

    /// The Base URL input shows for every key-based provider (so users can
    /// point at a gateway/proxy); local servers always use their fixed default
    /// URL.
    pub fn shows_base_url(self) -> bool {
        self.requires_key() || matches!(self, ProviderChoice::Tailscale)
    }

    #[allow(dead_code)] // renderer-contract helper; the view uses the machine defaults
    pub fn base_url_placeholder(self) -> &'static str {
        match self {
            ProviderChoice::ChatGpt => "Managed by secure ChatGPT sign-in",
            ProviderChoice::LmStudio => "http://127.0.0.1:1234/v1",
            ProviderChoice::Ollama => "http://127.0.0.1:11434/v1",
            ProviderChoice::Tailscale => "https://model.tailnet.ts.net/v1",
            _ => "Default provider URL",
        }
    }
}

/// Port of `makeOnboardingProvider`'s return type — the TS
/// `Omit<Provider, "hasKey">`. `kind`/`deployment` serialize with the same
/// wire strings as the TS values.
#[derive(Debug, Clone, PartialEq)]
pub struct OnboardingProvider {
    pub id: String,
    pub kind: ProviderKind,
    pub label: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub default_model: Option<String>,
    pub needs_key: bool,
    pub deployment: ProviderDeployment,
}

/// What the provider step must persist. Every real provider choice produces a
/// record; the `None` provider arm is kept so the writer can no-op when no
/// record is ready (e.g. a future OAuth/sign-in path). The key is `Some` only
/// for choices that require one.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingProviderSave {
    pub provider: Option<OnboardingProvider>,
    pub api_key: Option<String>,
}

/// Build the provider record for a compact choice. The three hosted/private
/// records intentionally preserve current Main's exact onboarding IDs,
/// endpoints, model defaults, and deployment flags.
pub fn make_onboarding_provider(
    choice: ProviderChoice,
    base_url: &str,
) -> Option<OnboardingProvider> {
    let (id, kind, label, models, default_model, needs_key, deployment) = match choice {
        ProviderChoice::ChatGpt => return None,
        ProviderChoice::Openai => (
            "custom:onboarding-openai".to_string(),
            ProviderKind::Openai,
            "OpenAI".to_string(),
            vec!["gpt-4.1".to_string(), "gpt-4.1-mini".to_string()],
            Some("gpt-4.1-mini".to_string()),
            true,
            ProviderDeployment::Hosted,
        ),
        ProviderChoice::Anthropic => (
            "custom:onboarding-anthropic".to_string(),
            ProviderKind::Anthropic,
            "Anthropic".to_string(),
            vec![
                "claude-sonnet-4-5".to_string(),
                "claude-haiku-4-5".to_string(),
            ],
            Some("claude-sonnet-4-5".to_string()),
            true,
            ProviderDeployment::Hosted,
        ),
        // Google talks to the generativelanguage API but is wired as an
        // OpenAI-compatible provider (`kind: "openai"` in the TS config).
        ProviderChoice::Google => (
            "google".to_string(),
            ProviderKind::Openai,
            "Google Gemini".to_string(),
            vec![
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-flash-lite".to_string(),
                "gemini-2.5-pro".to_string(),
            ],
            Some("gemini-2.5-flash".to_string()),
            true,
            ProviderDeployment::Hosted,
        ),
        ProviderChoice::DeepSeek => (
            "deepseek".to_string(),
            ProviderKind::Openai,
            "DeepSeek".to_string(),
            vec!["deepseek-chat".to_string(), "deepseek-reasoner".to_string()],
            Some("deepseek-chat".to_string()),
            true,
            ProviderDeployment::Hosted,
        ),
        ProviderChoice::Moonshot => (
            "moonshotai".to_string(),
            ProviderKind::Openai,
            "Moonshot (Kimi)".to_string(),
            vec![
                "kimi-k2-0711-preview".to_string(),
                "moonshot-v1-128k".to_string(),
                "moonshot-v1-32k".to_string(),
            ],
            Some("kimi-k2-0711-preview".to_string()),
            true,
            ProviderDeployment::Hosted,
        ),
        ProviderChoice::LmStudio => (
            "custom:lmstudio".to_string(),
            ProviderKind::Openai,
            "LM Studio (local)".to_string(),
            Vec::new(),
            None,
            false,
            ProviderDeployment::Local,
        ),
        ProviderChoice::Ollama => (
            "custom:ollama".to_string(),
            ProviderKind::Openai,
            "Ollama (local)".to_string(),
            Vec::new(),
            None,
            false,
            ProviderDeployment::Local,
        ),
        ProviderChoice::Tailscale => (
            "custom:onboarding-tailscale".to_string(),
            ProviderKind::Openai,
            "Tailscale model".to_string(),
            Vec::new(),
            None,
            false,
            ProviderDeployment::Local,
        ),
    };
    let base_url = if base_url.trim().is_empty() {
        match choice {
            ProviderChoice::ChatGpt => return None,
            ProviderChoice::Openai => "https://api.openai.com/v1",
            ProviderChoice::Anthropic => "https://api.anthropic.com/v1",
            ProviderChoice::Google => "https://generativelanguage.googleapis.com/v1beta",
            ProviderChoice::DeepSeek => "https://api.deepseek.com/v1",
            ProviderChoice::Moonshot => "https://api.moonshot.ai/v1",
            ProviderChoice::LmStudio => "http://127.0.0.1:1234/v1",
            ProviderChoice::Ollama => "http://127.0.0.1:11434/v1",
            ProviderChoice::Tailscale => "",
        }
    } else {
        base_url.trim()
    };
    Some(OnboardingProvider {
        id,
        kind,
        label,
        base_url: base_url.to_string(),
        models,
        default_model,
        needs_key,
        deployment,
    })
}

/// Whether onboarding should run. Mirrors `shouldShowOnboarding`:
/// `localStorage.getItem(STORAGE_KEY) !== "true"` — i.e. show whenever the
/// marker is missing or not exactly `"true"`.
pub fn should_show_onboarding(settings: &serde_json::Map<String, serde_json::Value>) -> bool {
    settings
        .get(ONBOARDING_COMPLETE_KEY)
        .and_then(|value| value.as_str())
        != Some("true")
}

/// Outcome of a transition. The view performs per-step persistence when it
/// sees `Advanced`, and writes the marker + emits the completion event on
/// `Completed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextOutcome {
    Advanced,
    Completed,
    /// Never constructed by the wired view (it validates before advancing);
    /// kept so the state machine is exhaustive.
    #[allow(dead_code)]
    Blocked,
}

/// The pure step-state machine. No IO, no GPUI: every transition and the
/// validation copy are unit-tested below (mirroring `onboarding-flow.test.tsx`).
#[derive(Debug, Clone)]
pub struct OnboardingMachine {
    step_index: usize,
    pub name: String,
    pub choice: ProviderChoice,
    pub api_key: String,
    pub defer_pi_setup: bool,
    pub base_url: String,
    /// Kept outside the visible step model so animation follows the persisted
    /// accessibility preference without adding a Rust-only Appearance step.
    pub reduce_motion: ReduceMotion,
    pub saved_provider: Option<OnboardingProvider>,
    pub codex_configured: bool,
    completed: bool,
    pub error: Option<&'static str>,
}

impl Default for OnboardingMachine {
    fn default() -> Self {
        Self {
            step_index: 0,
            name: String::new(),
            // Current Main opens the provider step on ChatGPT sign in.
            choice: ProviderChoice::ChatGpt,
            api_key: String::new(),
            defer_pi_setup: false,
            base_url: String::new(),
            reduce_motion: ReduceMotion::System,
            saved_provider: None,
            codex_configured: false,
            completed: false,
            error: None,
        }
    }
}

impl OnboardingMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> Step {
        Step::from_index(self.step_index)
    }

    pub fn step_index(&self) -> usize {
        self.step_index
    }

    pub fn total_steps(&self) -> usize {
        Step::ALL.len()
    }

    /// Dev visual-QA entrypoint. Production never calls this; the index is
    /// clamped through the same source-of-truth list as normal navigation.
    pub fn start_at_step_for_visual_qa(&mut self, step_index: usize) {
        self.step_index = step_index.min(Step::ALL.len() - 1);
        self.error = None;
    }

    #[allow(dead_code)] // renderer-contract helper; the view emits the completion event
    pub fn is_complete(&self) -> bool {
        self.completed
    }

    /// The provider persisted by the provider step, if any. The view uses
    /// this only to offer an explicit handoff to the app-owned setup modal.
    pub fn saved_provider(&self) -> Option<&OnboardingProvider> {
        self.saved_provider.as_ref()
    }

    pub fn defer_pi_provider_setup(&mut self) {
        self.defer_pi_setup = self.choice.requires_key();
        self.api_key.clear();
        self.error = None;
    }

    /// The blocking validation error for the current step, if any. Copy is
    /// taken from the TS toasts / profile-core messages.
    pub fn validate(&self) -> Option<&'static str> {
        match self.current() {
            Step::Welcome => {
                if self.name.trim().is_empty() {
                    return Some("Enter the name you want shown on your profile.");
                }
                if self.name.chars().count() > MAX_PROFILE_NAME_LENGTH {
                    return Some("Profile names can be up to 80 characters.");
                }
                None
            }
            Step::Provider => {
                if self.choice.requires_key()
                    && !self.defer_pi_setup
                    && self.api_key.trim().is_empty()
                {
                    return Some("Paste an API key or choose a sign-in/local option.");
                }
                if self.choice == ProviderChoice::Tailscale && self.base_url.trim().is_empty() {
                    return Some("Enter the Tailscale model server URL before continuing.");
                }
                None
            }
            Step::Finish => None,
        }
    }

    /// Whether the Next button is enabled (TS `canContinue`).
    pub fn can_continue(&self) -> bool {
        match self.current() {
            Step::Welcome => self.validate().is_none(),
            // Current Main enables Next as soon as a provider card is selected,
            // then surfaces field/setup errors after activation.
            Step::Provider | Step::Finish => true,
        }
    }

    /// Move to the next step without validation (the view calls this only
    /// after persisting side effects). Completes on the last step.
    pub fn advance(&mut self) -> NextOutcome {
        if self.current() == Step::Finish {
            self.completed = true;
            return NextOutcome::Completed;
        }
        self.step_index += 1;
        self.error = None;
        NextOutcome::Advanced
    }

    /// Validate + advance in one call. The view uses `validate()` /
    /// `pending_provider_save()` + `advance()` directly for the async provider
    /// step; `next()` covers everything else and all tests.
    #[allow(dead_code)] // renderer-contract helper; the wired view drives steps explicitly
    pub fn next(&mut self) -> NextOutcome {
        if let Some(message) = self.validate() {
            self.error = Some(message);
            return NextOutcome::Blocked;
        }
        self.error = None;
        self.advance()
    }

    /// Go back one step (TS Back button); never below Welcome.
    pub fn back(&mut self) {
        if self.step_index > 0 {
            self.step_index -= 1;
        }
        self.error = None;
    }

    /// Skip the whole flow (TS "Skip"): marks complete so the view writes the
    /// marker and closes.
    pub fn skip(&mut self) -> NextOutcome {
        self.completed = true;
        NextOutcome::Completed
    }

    /// What the provider step should persist for the current choices.
    pub fn pending_provider_save(&self) -> PendingProviderSave {
        let provider = make_onboarding_provider(self.choice, &self.base_url);
        let api_key = if self.choice.requires_key() {
            let key = self.api_key.trim().to_string();
            (!key.is_empty()).then_some(key)
        } else {
            None
        };
        PendingProviderSave { provider, api_key }
    }

    /// Record a successful provider save (config + keychain write). Current
    /// Main has no separate model step, so the provider's default is persisted
    /// immediately by the view.
    pub fn record_provider_saved(&mut self, provider: OnboardingProvider) {
        self.saved_provider = Some(provider);
        self.error = None;
    }

    pub fn record_codex_configured(&mut self) {
        let snapshot = aiden_providers::list::bundled_codex_provider_snapshot(true);
        let models = snapshot.models.into_iter().map(|model| model.id).collect();
        self.codex_configured = true;
        self.record_provider_saved(OnboardingProvider {
            id: "openai-codex".to_string(),
            kind: ProviderKind::Openai,
            label: "ChatGPT / Codex".to_string(),
            base_url: "https://chatgpt.com/backend-api".to_string(),
            models,
            default_model: Some("gpt-5.4".to_string()),
            needs_key: true,
            deployment: ProviderDeployment::Hosted,
        });
    }

    /// The implicit `(providerId, defaultModel)` pair current Main selects as
    /// part of provider setup.
    pub fn default_selection(&self) -> Option<(String, String)> {
        let provider = self.saved_provider.as_ref()?;
        let model = provider.default_model.clone()?;
        Some((provider.id.clone(), model))
    }

    pub fn set_reduce_motion(&mut self, reduce_motion: ReduceMotion) {
        self.reduce_motion = reduce_motion;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(
        key: &str,
        value: serde_json::Value,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut settings = serde_json::Map::new();
        settings.insert(key.to_string(), value);
        settings
    }

    // -----------------------------------------------------------------------
    // onboarding-flow.test.tsx — "onboarding appears only until it is completed"
    // -----------------------------------------------------------------------

    #[test]
    fn onboarding_appears_only_until_completed() {
        assert!(
            should_show_onboarding(&serde_json::Map::new()),
            "no marker yet → show"
        );
        assert!(
            should_show_onboarding(&settings_with(
                ONBOARDING_COMPLETE_KEY,
                serde_json::Value::String("yes".into())
            )),
            "any value other than exactly \"true\" still shows (TS !== \"true\")"
        );
        let completed = settings_with(
            ONBOARDING_COMPLETE_KEY,
            serde_json::Value::String("true".into()),
        );
        // The marker key matches the TS STORAGE_KEY exactly.
        assert_eq!(ONBOARDING_COMPLETE_KEY, "aiden:onboarding:v1:complete");
        assert!(
            !should_show_onboarding(&completed),
            "marker \"true\" hides the flow"
        );
    }

    #[test]
    fn skip_marks_the_flow_complete() {
        let mut machine = OnboardingMachine::new();
        assert_eq!(machine.skip(), NextOutcome::Completed);
        assert!(machine.is_complete());
    }

    // -----------------------------------------------------------------------
    // onboarding-provider.test.ts — exact compact connection records
    // -----------------------------------------------------------------------

    #[test]
    fn provider_factory_emits_real_provider_ids_models_and_defaults() {
        assert_eq!(
            make_onboarding_provider(ProviderChoice::Anthropic, ""),
            Some(OnboardingProvider {
                id: "custom:onboarding-anthropic".into(),
                kind: ProviderKind::Anthropic,
                label: "Anthropic".into(),
                base_url: "https://api.anthropic.com/v1".into(),
                models: vec!["claude-sonnet-4-5".into(), "claude-haiku-4-5".into()],
                default_model: Some("claude-sonnet-4-5".into()),
                needs_key: true,
                deployment: ProviderDeployment::Hosted,
            })
        );

        assert_eq!(
            make_onboarding_provider(ProviderChoice::Openai, "https://gateway.example/v1"),
            Some(OnboardingProvider {
                id: "custom:onboarding-openai".into(),
                kind: ProviderKind::Openai,
                label: "OpenAI".into(),
                base_url: "https://gateway.example/v1".into(),
                models: vec!["gpt-4.1".into(), "gpt-4.1-mini".into()],
                default_model: Some("gpt-4.1-mini".into()),
                needs_key: true,
                deployment: ProviderDeployment::Hosted,
            })
        );

        assert_eq!(
            make_onboarding_provider(ProviderChoice::LmStudio, ""),
            Some(OnboardingProvider {
                id: "custom:lmstudio".into(),
                kind: ProviderKind::Openai,
                label: "LM Studio (local)".into(),
                base_url: "http://127.0.0.1:1234/v1".into(),
                models: vec![],
                default_model: None,
                needs_key: false,
                deployment: ProviderDeployment::Local,
            })
        );

        assert_eq!(
            make_onboarding_provider(ProviderChoice::Ollama, ""),
            Some(OnboardingProvider {
                id: "custom:ollama".into(),
                kind: ProviderKind::Openai,
                label: "Ollama (local)".into(),
                base_url: "http://127.0.0.1:11434/v1".into(),
                models: vec![],
                default_model: None,
                needs_key: false,
                deployment: ProviderDeployment::Local,
            })
        );

        assert_eq!(
            make_onboarding_provider(ProviderChoice::Tailscale, "https://model.tailnet.ts.net/v1"),
            Some(OnboardingProvider {
                id: "custom:onboarding-tailscale".into(),
                kind: ProviderKind::Openai,
                label: "Tailscale model".into(),
                base_url: "https://model.tailnet.ts.net/v1".into(),
                models: vec![],
                default_model: None,
                needs_key: false,
                deployment: ProviderDeployment::Local,
            })
        );
    }

    /// Compact parity lock: source order, copy, and icon identities must stay
    /// identical to `providerChoices` in the Electron flow.
    #[test]
    fn compact_provider_choices_match_current_main() {
        let expected = [
            (
                ProviderChoice::Openai,
                "OpenAI API key",
                "Connect with your own API key.",
                "openai",
            ),
            (
                ProviderChoice::ChatGpt,
                "ChatGPT sign in",
                "Connect through browser sign-in.",
                "openai-codex",
            ),
            (
                ProviderChoice::Anthropic,
                "Anthropic API key",
                "Connect with your Anthropic API key.",
                "anthropic",
            ),
            (
                ProviderChoice::LmStudio,
                "LM Studio",
                "Use models running in LM Studio.",
                "custom:lmstudio",
            ),
            (
                ProviderChoice::Ollama,
                "Ollama",
                "Use models running in Ollama.",
                "custom:ollama",
            ),
            (
                ProviderChoice::Tailscale,
                "Tailscale custom model",
                "Connect to a private model on your tailnet.",
                "",
            ),
        ];
        assert_eq!(ProviderChoice::ALL.len(), expected.len());
        for (actual, (choice, title, description, icon)) in
            ProviderChoice::ALL.iter().copied().zip(expected)
        {
            assert_eq!(actual, choice);
            assert_eq!(actual.title(), title);
            assert_eq!(actual.description(), description);
            assert_eq!(actual.icon_provider_id(), icon);
        }
    }

    #[test]
    fn factory_trims_custom_base_urls_and_defaults_local_urls() {
        let provider =
            make_onboarding_provider(ProviderChoice::Anthropic, "  https://gateway.example/v1  ")
                .expect("anthropic provider");
        assert_eq!(provider.base_url, "https://gateway.example/v1");
        // Local choices fall back to their fixed default URL when left blank.
        assert_eq!(
            make_onboarding_provider(ProviderChoice::LmStudio, "")
                .expect("lm studio provider")
                .base_url,
            "http://127.0.0.1:1234/v1"
        );
        assert_eq!(
            make_onboarding_provider(ProviderChoice::Ollama, "")
                .expect("ollama provider")
                .base_url,
            "http://127.0.0.1:11434/v1"
        );
    }

    // -----------------------------------------------------------------------
    // Machine transitions
    // -----------------------------------------------------------------------

    #[test]
    fn welcome_step_requires_a_name() {
        let mut machine = OnboardingMachine::new();
        assert_eq!(machine.current(), Step::Welcome);
        assert!(!machine.can_continue());
        assert_eq!(
            machine.next(),
            NextOutcome::Blocked,
            "empty name blocks advance"
        );
        assert_eq!(
            machine.error,
            Some("Enter the name you want shown on your profile.")
        );

        machine.name = "Ada".into();
        assert!(machine.can_continue());
        assert_eq!(machine.next(), NextOutcome::Advanced);
        assert_eq!(machine.current(), Step::Provider);
    }

    #[test]
    fn name_length_is_capped_at_eighty() {
        let mut machine = OnboardingMachine::new();
        machine.name = "a".repeat(81);
        assert_eq!(
            machine.validate(),
            Some("Profile names can be up to 80 characters.")
        );
        machine.name = "a".repeat(80);
        assert_eq!(machine.validate(), None);
    }

    #[test]
    fn provider_step_validation_matches_ts() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".into();
        machine.next();

        // Every key-requiring choice demands a key.
        for choice in [ProviderChoice::Anthropic, ProviderChoice::Openai] {
            machine.choice = choice;
            assert_eq!(
                machine.validate(),
                Some("Paste an API key or choose a sign-in/local option."),
                "{choice:?} requires a key"
            );
            machine.api_key = "  sk-ant-abcdef  ".into();
            assert_eq!(
                machine.validate(),
                None,
                "a non-empty (untrimmed) key passes TS too ({choice:?})"
            );
            machine.api_key.clear();
        }

        // Local choices never require a key.
        for choice in [ProviderChoice::LmStudio, ProviderChoice::Ollama] {
            machine.choice = choice;
            assert_eq!(machine.validate(), None, "{choice:?} is keyless");
        }
        machine.choice = ProviderChoice::Tailscale;
        assert_eq!(
            machine.validate(),
            Some("Enter the Tailscale model server URL before continuing.")
        );
        machine.base_url = "https://model.tailnet.ts.net/v1".into();
        assert_eq!(machine.validate(), None);
    }

    #[test]
    fn explicit_pi_setup_handoff_allows_progress_without_portable_key_material() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".into();
        assert_eq!(machine.next(), NextOutcome::Advanced);
        machine.choice = ProviderChoice::Anthropic;
        assert!(machine.validate().is_some());
        machine.defer_pi_provider_setup();
        assert_eq!(machine.validate(), None);
        let pending = machine.pending_provider_save();
        assert_eq!(pending.api_key, None);
        assert_eq!(
            pending.provider.map(|provider| provider.id),
            Some("custom:onboarding-anthropic".to_string())
        );
    }

    #[test]
    fn pending_provider_save_carries_the_trimmed_key() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".into();
        machine.next();
        machine.choice = ProviderChoice::Anthropic;
        machine.api_key = "  sk-ant-abcdef  ".into();
        machine.base_url = "".into();

        let pending = machine.pending_provider_save();
        assert_eq!(pending.api_key.as_deref(), Some("sk-ant-abcdef"));
        assert_eq!(
            pending
                .provider
                .as_ref()
                .map(|provider| provider.id.as_str()),
            Some("custom:onboarding-anthropic")
        );

        machine.choice = ProviderChoice::LmStudio;
        let pending = machine.pending_provider_save();
        assert_eq!(pending.api_key, None);
        assert_eq!(
            pending.provider.map(|provider| provider.id),
            Some("custom:lmstudio".to_string()),
            "local choices still produce a provider record"
        );
    }

    #[test]
    fn provider_step_resolves_the_implicit_default_model_selection() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".into();
        machine.next();
        machine.choice = ProviderChoice::Anthropic;
        machine.api_key = "sk-ant-abcdef".into();
        let provider = machine
            .pending_provider_save()
            .provider
            .expect("anthropic provider");
        machine.record_provider_saved(provider);
        assert_eq!(
            machine
                .saved_provider()
                .map(|provider| provider.id.as_str()),
            Some("custom:onboarding-anthropic")
        );
        assert_eq!(machine.current(), Step::Provider);
        assert_eq!(machine.next(), NextOutcome::Advanced);
        assert_eq!(machine.current(), Step::Finish);
        assert_eq!(
            machine.default_selection(),
            Some((
                "custom:onboarding-anthropic".to_string(),
                "claude-sonnet-4-5".to_string()
            ))
        );
    }

    #[test]
    fn exact_current_main_step_contract_is_three_steps() {
        assert_eq!(Step::ALL, &[Step::Welcome, Step::Provider, Step::Finish]);
        assert_eq!(
            Step::ALL
                .iter()
                .map(|step| step.label())
                .collect::<Vec<_>>(),
            vec!["Your profile", "Model provider", "Ready to go"]
        );
    }

    #[test]
    fn finish_marks_the_flow_complete() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".into();
        machine.choice = ProviderChoice::Anthropic;
        machine.api_key = "sk-ant-abcdef".into();
        // Walk the exact three-step flow.
        for _ in 0..2 {
            let outcome = machine.next();
            assert_eq!(outcome, NextOutcome::Advanced);
        }
        assert_eq!(machine.current(), Step::Finish);
        assert!(!machine.is_complete());
        assert_eq!(machine.next(), NextOutcome::Completed);
        assert!(machine.is_complete());
    }

    #[test]
    fn back_never_goes_below_welcome() {
        let mut machine = OnboardingMachine::new();
        machine.back();
        assert_eq!(machine.current(), Step::Welcome);
        machine.name = "Ada".into();
        machine.next();
        machine.back();
        assert_eq!(machine.current(), Step::Welcome);
    }

    #[test]
    fn blocked_advance_sets_the_error_and_clears_resets_it() {
        let mut machine = OnboardingMachine::new();
        assert_eq!(machine.next(), NextOutcome::Blocked);
        assert!(machine.error.is_some());
        machine.name = "Ada".into();
        assert_eq!(machine.next(), NextOutcome::Advanced);
        assert_eq!(machine.error, None);
    }

    // The wired view calls `advance()` (not `next()`) after a passing
    // `validate()`, and treats the `Blocked` arm as a defensive no-op instead
    // of panicking (ObjC callbacks cannot unwind). This locks in the invariant
    // that makes that no-op correct: `advance()` only ever yields `Advanced`
    // or `Completed`, never `Blocked`.
    #[test]
    fn advance_never_yields_blocked_across_the_whole_flow() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".into();
        loop {
            let outcome = machine.advance();
            assert!(
                matches!(outcome, NextOutcome::Advanced | NextOutcome::Completed),
                "advance() must never return Blocked, got {outcome:?}"
            );
            if outcome == NextOutcome::Completed {
                break;
            }
        }
        assert!(machine.is_complete());
        // Advancing past completion keeps yielding Completed (never Blocked).
        assert_eq!(machine.advance(), NextOutcome::Completed);
    }

    #[test]
    fn back_and_reenter_preserves_entered_step_data() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".into();
        machine.next(); // → Provider
        machine.choice = ProviderChoice::Anthropic;
        machine.api_key = "  sk-ant-abcdef  ".into();
        machine.base_url = "https://gateway.example/v1".into();
        machine.back(); // → Welcome
        assert_eq!(machine.current(), Step::Welcome);
        machine.next(); // → Provider again
                        // The machine keeps every field the user entered; the inputs mirror
                        // it, so re-advancing never requires re-entering data.
        assert_eq!(machine.choice, ProviderChoice::Anthropic);
        assert_eq!(machine.api_key, "  sk-ant-abcdef  ");
        assert_eq!(machine.base_url, "https://gateway.example/v1");
        // The provider is saved again on re-advance; the save is an upsert by
        // id (config_store::save_provider), so this is idempotent.
        assert_eq!(
            machine.pending_provider_save().api_key.as_deref(),
            Some("sk-ant-abcdef")
        );
    }

    #[test]
    fn advance_is_bounded_by_the_finish_step() {
        let mut machine = OnboardingMachine::new();
        machine.name = "Ada".into();
        machine.choice = ProviderChoice::Anthropic;
        machine.api_key = "sk-ant-abcdef".into();
        // Walk the flow, then hammer Next well past the end.
        for _ in 0..10 {
            let _ = machine.next();
        }
        assert_eq!(machine.current(), Step::Finish);
        assert!(machine.is_complete());
        // The step index never escapes the step list.
        assert_eq!(machine.step_index(), Step::Finish.index());
        assert_eq!(machine.current(), Step::from_index(machine.step_index()));
    }

    #[test]
    fn should_show_onboarding_when_the_marker_is_missing_or_malformed() {
        // Non-string marker values (corrupt / legacy) still show the flow —
        // the TS contract is `localStorage.getItem(key) !== "true"`.
        assert!(should_show_onboarding(&settings_with(
            ONBOARDING_COMPLETE_KEY,
            serde_json::json!(true)
        )));
        assert!(should_show_onboarding(&settings_with(
            ONBOARDING_COMPLETE_KEY,
            serde_json::json!(1)
        )));
        assert!(should_show_onboarding(&settings_with(
            ONBOARDING_COMPLETE_KEY,
            serde_json::Value::Null
        )));
        // The exact TS string hides it.
        assert!(!should_show_onboarding(&settings_with(
            ONBOARDING_COMPLETE_KEY,
            serde_json::json!("true")
        )));
    }

    #[test]
    fn skip_then_advance_never_reenters_editing() {
        let mut machine = OnboardingMachine::new();
        machine.skip();
        assert!(machine.is_complete());
        // The machine stays at Welcome (index 0) but reports complete, so the
        // view's completion guard prevents any further navigation. Calling
        // `next()` here cannot advance (the empty name fails `validate()`), so
        // it returns `Blocked` and the machine remains complete + parked.
        assert_eq!(machine.step_index(), 0);
        assert_eq!(machine.next(), NextOutcome::Blocked);
        assert!(machine.is_complete());
        assert_eq!(machine.step_index(), 0);
    }

    #[test]
    fn choosing_chatgpt_defers_network_setup_to_the_view_activation_path() {
        let mut machine = OnboardingMachine::new();
        machine.choice = ProviderChoice::ChatGpt;

        let pending = machine.pending_provider_save();

        assert_eq!(pending.provider, None);
        assert_eq!(pending.api_key, None);
        machine.step_index = Step::Provider.index();
        assert_eq!(machine.validate(), None);
        assert!(machine.can_continue());
    }

    #[test]
    fn configured_chatgpt_exposes_bundled_models_and_exact_default_selection() {
        let mut machine = OnboardingMachine::new();
        machine.choice = ProviderChoice::ChatGpt;

        machine.record_codex_configured();

        assert!(machine.codex_configured);
        assert!(machine
            .saved_provider()
            .expect("configured Codex provider")
            .models
            .iter()
            .any(|model| model == "gpt-5.6-sol"));
        assert_eq!(
            machine.default_selection(),
            Some(("openai-codex".to_string(), "gpt-5.4".to_string()))
        );
    }
}
