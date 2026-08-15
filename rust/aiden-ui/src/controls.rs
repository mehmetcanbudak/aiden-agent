//! Aiden-specific shared controls whose geometry or state colors differ from
//! gpui-component's generic defaults.

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder as _, px, App, ElementId, Hsla, InteractiveElement as _,
    IntoElement, ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window,
};
use gpui_component::{h_flex, tooltip::Tooltip, ActiveTheme as _, Disableable};

type SwitchClickHandler = dyn Fn(&bool, &mut Window, &mut App);

/// Exact native counterpart of `renderer/components/ui.tsx::Switch`.
///
/// The upstream GPUI primitive is 36×20 and uses one thumb color for both
/// states. Aiden's source control is 40×24 with a 20 px white off-thumb and an
/// accent-foreground on-thumb, so the stock primitive cannot represent it
/// faithfully in dark mode.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    disabled: bool,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
    on_click: Option<Rc<SwitchClickHandler>>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            label: None,
            tooltip: None,
            on_click: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Disableable for Switch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let disabled = self.disabled;
        let on_click = self.on_click;
        let track = if checked {
            cx.theme().primary
        } else {
            cx.theme().switch
        };
        let hover = if checked {
            cx.theme().primary_hover
        } else {
            cx.theme().secondary_active
        };
        let thumb = if checked {
            cx.theme().primary_foreground
        } else {
            Hsla::from(gpui::Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            })
        };

        h_flex()
            .gap_2()
            .items_center()
            .when(disabled, |this| this.opacity(0.45))
            .child(
                div()
                    .id(self.id)
                    .relative()
                    .w(px(40.0))
                    .h(px(24.0))
                    .rounded(px(12.0))
                    .bg(track)
                    .shadow_sm()
                    .when_some(self.tooltip, |this, tooltip| {
                        this.tooltip(move |window, cx| {
                            Tooltip::new(tooltip.clone()).build(window, cx)
                        })
                    })
                    .when(!disabled, |this| this.hover(move |style| style.bg(hover)))
                    .child(
                        div()
                            .absolute()
                            .top(px(2.0))
                            .left(if checked { px(18.0) } else { px(2.0) })
                            .size(px(20.0))
                            .rounded_full()
                            .bg(thumb)
                            .shadow_sm(),
                    )
                    .when_some(on_click.filter(|_| !disabled), |this, on_click| {
                        this.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            on_click(&!checked, window, cx);
                        })
                    }),
            )
            .when_some(self.label, |this, label| {
                this.child(div().line_height(px(24.0)).text_base().child(label))
            })
    }
}
