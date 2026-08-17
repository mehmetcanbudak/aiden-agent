//! The UI type scale, mirrored from Electron's semantic tokens in
//! `renderer/styles.css`.
//!
//! Every token there is a `calc()` offset from `--ui-font-size`:
//!
//! ```text
//! --text-heading1:     calc(var(--ui-font-size) + 10px)
//! --text-heading2:     calc(var(--ui-font-size) +  4px)
//! --text-large-strong: calc(var(--ui-font-size) +  2px)
//! --text-regular:      var(--ui-font-size)
//! --text-small:        calc(var(--ui-font-size) -  1px)
//! --text-mini:         calc(var(--ui-font-size) -  3px)
//! ```
//!
//! so the whole scale follows the user's UI font-size setting. `theme.font_size`
//! is that same setting on this side — `services::appearance` assigns it from
//! `ui_font_size`, and gpui-component feeds it to `rem_size` — so the tokens
//! here are offsets applied to it.
//!
//! Prefer these over gpui's `text_xs()`/`text_sm()`/`text_lg()`. Those are
//! *multiplicative* against `rem_size` (0.75/0.875/1.125) where Electron's scale
//! is additive, so they land between tokens and drift further as the user raises
//! the font size: `text_xs()` is 10.5px against `--text-mini`'s 11px at the 14pt
//! default, and 13.5px against 15px at 18pt. Only `text_base()` (1.0 rem) lines
//! up with a token, and that token is [`regular`].
//!
//! Weight is a separate axis, as it is in Electron's `Text` component: the
//! `-strong` variants pair a size here with `FontWeight::MEDIUM`
//! (`--text-strong`, `--text-small-strong`) or `FontWeight::SEMIBOLD`
//! (`--text-heading1`). Set it alongside the size at the call site.

use gpui::{px, Pixels};
use gpui_component::theme::Theme;

/// `--text-heading1`.
pub const HEADING1_OFFSET_PX: f32 = 10.0;
/// `--text-heading2`.
pub const HEADING2_OFFSET_PX: f32 = 4.0;
/// `--text-large-strong`.
pub const LARGE_STRONG_OFFSET_PX: f32 = 2.0;
/// `--text-regular` and `--text-strong`.
pub const REGULAR_OFFSET_PX: f32 = 0.0;
/// `--text-small` and `--text-small-strong`.
pub const SMALL_OFFSET_PX: f32 = -1.0;
/// `--text-mini`.
pub const MINI_OFFSET_PX: f32 = -3.0;

/// Resolve one token against a UI font size. Split out from the `Theme`-taking
/// helpers so the scale can be exercised without building a theme.
fn resolve(ui_font_size: Pixels, offset_px: f32) -> Pixels {
    ui_font_size + px(offset_px)
}

/// `--text-heading1`. Pairs with `FontWeight::SEMIBOLD`.
pub fn heading1(theme: &Theme) -> Pixels {
    resolve(theme.font_size, HEADING1_OFFSET_PX)
}

/// `--text-heading2`.
pub fn heading2(theme: &Theme) -> Pixels {
    resolve(theme.font_size, HEADING2_OFFSET_PX)
}

/// `--text-large-strong`.
pub fn large_strong(theme: &Theme) -> Pixels {
    resolve(theme.font_size, LARGE_STRONG_OFFSET_PX)
}

/// `--text-regular`. `--text-strong` is this size with `FontWeight::MEDIUM`.
pub fn regular(theme: &Theme) -> Pixels {
    resolve(theme.font_size, REGULAR_OFFSET_PX)
}

/// `--text-small`. `--text-small-strong` is this size with `FontWeight::MEDIUM`.
pub fn small(theme: &Theme) -> Pixels {
    resolve(theme.font_size, SMALL_OFFSET_PX)
}

/// `--text-mini`.
pub fn mini(theme: &Theme) -> Pixels {
    resolve(theme.font_size, MINI_OFFSET_PX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The appearance settings clamp `ui_font_size` to this range, so the scale
    /// has to stay correct across all of it — not just at the 14pt default.
    const UI_FONT_SIZE_RANGE: [f32; 3] = [12.0, 14.0, 18.0];

    #[test]
    fn offsets_match_the_electron_tokens() {
        assert_eq!(HEADING1_OFFSET_PX, 10.0);
        assert_eq!(HEADING2_OFFSET_PX, 4.0);
        assert_eq!(LARGE_STRONG_OFFSET_PX, 2.0);
        assert_eq!(REGULAR_OFFSET_PX, 0.0);
        assert_eq!(SMALL_OFFSET_PX, -1.0);
        assert_eq!(MINI_OFFSET_PX, -3.0);
    }

    #[test]
    fn the_scale_tracks_the_ui_font_size() {
        // At the 14pt default these are Electron's 24/18/16/14/13/11px.
        assert_eq!(resolve(px(14.0), HEADING1_OFFSET_PX), px(24.0));
        assert_eq!(resolve(px(14.0), HEADING2_OFFSET_PX), px(18.0));
        assert_eq!(resolve(px(14.0), LARGE_STRONG_OFFSET_PX), px(16.0));
        assert_eq!(resolve(px(14.0), REGULAR_OFFSET_PX), px(14.0));
        assert_eq!(resolve(px(14.0), SMALL_OFFSET_PX), px(13.0));
        assert_eq!(resolve(px(14.0), MINI_OFFSET_PX), px(11.0));

        // And they keep their spacing as the setting moves, which is the whole
        // reason these are offsets rather than pinned pixels.
        for ui_font_size in UI_FONT_SIZE_RANGE {
            let base = px(ui_font_size);
            assert_eq!(resolve(base, REGULAR_OFFSET_PX), base);
            assert_eq!(resolve(base, SMALL_OFFSET_PX), base - px(1.0));
            assert_eq!(resolve(base, MINI_OFFSET_PX), base - px(3.0));
        }
    }

    #[test]
    fn the_scale_is_strictly_ordered_at_every_supported_font_size() {
        for ui_font_size in UI_FONT_SIZE_RANGE {
            let base = px(ui_font_size);
            let steps = [
                resolve(base, MINI_OFFSET_PX),
                resolve(base, SMALL_OFFSET_PX),
                resolve(base, REGULAR_OFFSET_PX),
                resolve(base, LARGE_STRONG_OFFSET_PX),
                resolve(base, HEADING2_OFFSET_PX),
                resolve(base, HEADING1_OFFSET_PX),
            ];
            for pair in steps.windows(2) {
                assert!(pair[0] < pair[1], "scale must ascend at {ui_font_size}pt");
            }
        }
    }

    /// gpui's rem helpers are multiplicative against `rem_size`, which is
    /// `theme.font_size`. This pins the divergence the module exists to avoid,
    /// so nobody "simplifies" a token back into `text_xs()`.
    #[test]
    fn gpui_rem_helpers_do_not_land_on_the_electron_scale() {
        for ui_font_size in UI_FONT_SIZE_RANGE {
            let base = px(ui_font_size);
            // text_xs() is 0.75 rem; --text-mini is base - 3px.
            let text_xs = base * 0.75;
            // text_sm() is 0.875 rem; --text-small is base - 1px.
            let text_sm = base * 0.875;
            assert_ne!(text_sm, resolve(base, SMALL_OFFSET_PX));
            if ui_font_size != 12.0 {
                // The two curves cross at exactly 12pt and separate from there.
                assert_ne!(text_xs, resolve(base, MINI_OFFSET_PX));
            } else {
                assert_eq!(text_xs, resolve(base, MINI_OFFSET_PX));
            }
            // text_base() is the one helper that is always a real token.
            assert_eq!(base * 1.0, resolve(base, REGULAR_OFFSET_PX));
        }
    }
}
