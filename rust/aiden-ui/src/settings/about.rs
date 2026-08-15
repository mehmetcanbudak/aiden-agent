//! About settings, matched to `renderer/components/settings/about-settings.tsx`.

use std::sync::Arc;

use aiden_core::app_update::AppUpdateSnapshot;
use gpui::{
    div, img, prelude::FluentBuilder as _, AppContext as _, Context, FontWeight, Image,
    ImageFormat, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable as _, WindowExt as _,
};
use gpui_tokio_bridge::Tokio;

use super::{settings_field, settings_fieldset, SettingsView, SETTINGS_SECTION_TITLE_PX};

pub const APP_NAME: &str = "Aiden Agent";
pub const REPOSITORY_URL: &str = "https://github.com/sambitcreate/aiden-agent";

const PACKAGE_JSON: &str = include_str!("../../../../package.json");
const APP_ICON_PNG: &[u8] = include_bytes!("../../../../resources/app-icon.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEnvironment {
    Development,
    Release,
}

impl RuntimeEnvironment {
    pub fn current() -> Self {
        if aiden_data::is_dev_mode() {
            Self::Development
        } else {
            Self::Release
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Development => "Development build",
            Self::Release => "Production build",
        }
    }
}

pub fn product_version_from(package_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(package_json)
        .ok()?
        .get("version")?
        .as_str()
        .filter(|version| !version.trim().is_empty())
        .map(ToOwned::to_owned)
}

pub fn product_version() -> String {
    option_env!("AIDEN_BUILD_VERSION")
        .filter(|version| !version.trim().is_empty())
        .map(str::to_string)
        .or_else(|| product_version_from(PACKAGE_JSON))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn launch_repository_with(launch: impl FnOnce(&str) -> Result<(), String>) -> Result<(), String> {
    launch(REPOSITORY_URL)
}

fn update_description(snapshot: &AppUpdateSnapshot) -> String {
    match snapshot {
        AppUpdateSnapshot::Idle { .. } => {
            "Aiden checks automatically and downloads signed updates without interrupting your work."
                .to_string()
        }
        AppUpdateSnapshot::Ready { version } => {
            format!("Aiden Agent {version} is ready. Restart to finish installing it.")
        }
    }
}

impl SettingsView {
    pub(crate) fn about_section(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let version = product_version();
        let environment = RuntimeEnvironment::current().label();
        let snapshot = self.services.app_updates.snapshot();
        let update_ready = matches!(snapshot, AppUpdateSnapshot::Ready { .. });
        let well = crate::services::appearance::well_surface(cx);

        let header = h_flex()
            .w_full()
            .items_center()
            .gap_4()
            .p_4()
            .child(
                div().size(gpui::px(64.)).flex_shrink_0().child(
                    img(Arc::new(Image::from_bytes(
                        ImageFormat::Png,
                        APP_ICON_PNG.to_vec(),
                    )))
                    .size_full(),
                ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w(gpui::px(0.))
                    .child(
                        div()
                            .text_size(gpui::px(SETTINGS_SECTION_TITLE_PX))
                            .font_weight(FontWeight::MEDIUM)
                            .child(APP_NAME),
                    )
                    .child(
                        div()
                            .mt(gpui::px(2.))
                            .text_size(gpui::px(13.))
                            .text_color(theme.secondary_foreground)
                            .child(format!("Version {version} · Beta · {environment}")),
                    )
                    .child(
                        h_flex().mt_3().child(
                            Button::new("about-github")
                                .small()
                                .icon(IconName::GitHub)
                                .label("GitHub")
                                .on_click(cx.listener(|this, _event, _window, cx| {
                                    this.open_repository(cx);
                                })),
                        ),
                    ),
            )
            .into_any_element();

        let update_button = Button::new("about-check-update")
            .small()
            .icon(Icon::default().path("native-icons/refresh-cw.svg"))
            .when(update_ready, |button| button.primary())
            .label(if update_ready {
                "Update and restart"
            } else {
                "Check for updates"
            })
            .on_click(cx.listener(|this, _event, window, cx| {
                this.check_for_updates(window, cx);
            }));

        let update = settings_field(
            "about-software-update",
            "Software update",
            update_description(&snapshot),
            update_button,
            true,
            &theme,
        );
        let reset = settings_field(
            "about-reset-onboarding",
            "Reset onboarding",
            "Clear this profile’s setup and preferences, restart Aiden, and return to the first onboarding step.",
            Button::new("about-reset-onboarding-button")
                .small()
                .icon(IconName::Undo2)
                .label("Reset onboarding…")
                .on_click(cx.listener(|_this, _event, _window, cx| {
                    cx.emit(super::SettingsEvent::OnboardingResetRequested);
                })),
            false,
            &theme,
        );

        v_flex()
            .id("about-section")
            .w_full()
            .child(settings_fieldset(
                "About",
                vec![header, update, reset],
                well,
            ))
            .when_some(self.error.clone(), |view, message| {
                view.child(
                    div()
                        .w_full()
                        .px_3()
                        .py_2()
                        .rounded(gpui::px(12.))
                        .bg(theme.danger.opacity(0.12))
                        .text_size(gpui::px(13.))
                        .text_color(theme.danger)
                        .child(message),
                )
            })
    }

    fn check_for_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let authority = self.services.app_updates.clone();
        let task = Tokio::spawn(cx, async move { authority.check_now(true).await });
        cx.spawn(async move |this, cx| {
            let _ = task.await;
            let _ = this.update(cx, |_this, cx| cx.notify());
        })
        .detach();
        window.push_notification("Checking for Aiden updates…", cx);
    }

    fn open_repository(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    launch_repository_with(|url| {
                        std::process::Command::new("/usr/bin/open")
                            .arg(url)
                            .spawn()
                            .map(|_| ())
                            .map_err(|error| error.to_string())
                    })
                })
                .await;
            this.update(cx, |this, cx| {
                if let Err(error) = result {
                    this.error = Some(format!("Could not open the GitHub repository: {error}"));
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_version_comes_from_root_package_metadata() {
        assert_eq!(
            product_version_from(PACKAGE_JSON).as_deref(),
            Some("0.28.0")
        );
        assert_eq!(
            product_version_from(r#"{"version":"1.2.3"}"#).as_deref(),
            Some("1.2.3")
        );
        assert_eq!(product_version_from("{}"), None);
    }

    #[test]
    fn runtime_environment_uses_the_aiden_dev_profile() {
        assert_eq!(
            RuntimeEnvironment::current(),
            if aiden_data::is_dev_mode() {
                RuntimeEnvironment::Development
            } else {
                RuntimeEnvironment::Release
            }
        );
    }

    #[test]
    fn repository_opener_receives_only_the_fixed_url_and_surfaces_failure() {
        let mut observed = None;
        let result = launch_repository_with(|url| {
            observed = Some(url.to_string());
            Err("launcher unavailable".to_string())
        });
        assert_eq!(observed.as_deref(), Some(REPOSITORY_URL));
        assert_eq!(result.unwrap_err(), "launcher unavailable");
    }

    #[test]
    fn update_copy_matches_the_electron_default_and_ready_states() {
        assert!(
            update_description(&AppUpdateSnapshot::Idle { version: None })
                .starts_with("Aiden checks automatically")
        );
        assert_eq!(
            update_description(&AppUpdateSnapshot::Ready {
                version: "1.2.3".into()
            }),
            "Aiden Agent 1.2.3 is ready. Restart to finish installing it."
        );
    }
}
