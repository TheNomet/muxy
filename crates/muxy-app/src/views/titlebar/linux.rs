use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, ClickEvent, Decorations, InteractiveElement, IntoElement,
    MouseButton, ParentElement, SharedString, StatefulInteractiveElement, Styled, Window,
    WindowDecorations, div,
};
use muxy_ui::components::{IconGlyph, Tooltip};
use muxy_ui::icon::Icon;
use muxy_ui::theme::{Metrics, Theme};
use muxy_ui::tr;

/// Leaves the window's frame to the system, which shows `title` in it. Where
/// the system draws none, the window keeps its own buttons so it can still be
/// minimized and closed.
pub(crate) fn adopt_system_frame(window: &mut Window, title: &str) {
    window.set_window_title(title);
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if gpui::guess_compositor() == "Wayland" && leaves_windows_unframed(&desktop) {
        window.request_decorations(WindowDecorations::Client);
    }
}

/// GNOME's Wayland compositor draws no window frame and has no protocol to
/// ask it for one.
fn leaves_windows_unframed(desktop: &str) -> bool {
    desktop
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("GNOME"))
}

/// The buttons of a window the system left unframed.
pub(in crate::views) fn controls(
    window: &Window,
    theme: &Theme,
    metrics: Metrics,
    close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Option<AnyElement> {
    if !matches!(window.window_decorations(), Decorations::Client { .. }) {
        return None;
    }
    let capabilities = window.window_controls();
    Some(
        div()
            .flex()
            .items_center()
            .gap(metrics.spacing1())
            .when(capabilities.minimize, |controls| {
                controls.child(button(
                    "window-minimize",
                    tr!("Minimize"),
                    div()
                        .w(metrics.scaled(8.0))
                        .h(metrics.scaled(1.0))
                        .bg(theme.fg_muted)
                        .into_any_element(),
                    theme,
                    metrics,
                    |_, window, _| window.minimize_window(),
                ))
            })
            .when(capabilities.maximize, |controls| {
                controls.child(button(
                    "window-maximize",
                    if window.is_maximized() {
                        tr!("Restore Window")
                    } else {
                        tr!("Maximize")
                    },
                    IconGlyph::new(Icon::Maximize, metrics.icon_sm(), theme.fg_muted)
                        .into_any_element(),
                    theme,
                    metrics,
                    |_, window, _| window.zoom_window(),
                ))
            })
            .child(button(
                "window-close",
                tr!("Close Window"),
                IconGlyph::new(Icon::X, metrics.icon_sm(), theme.fg_muted).into_any_element(),
                theme,
                metrics,
                close,
            ))
            .into_any_element(),
    )
}

fn button(
    id: &'static str,
    label: SharedString,
    glyph: AnyElement,
    theme: &Theme,
    metrics: Metrics,
    action: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let (background, foreground, border, theme_background) =
        (theme.raised(), theme.fg, theme.border, theme.bg);
    div()
        .id(id)
        .debug_selector(move || id.into())
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(metrics.scaled(22.0))
        .rounded(metrics.radius_sm())
        .hover(|style| style.bg(theme.hover))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            action(event, window, cx);
        })
        .tooltip(move |_, cx| {
            cx.new(|_| {
                Tooltip::new(
                    label.clone(),
                    background,
                    foreground,
                    border,
                    theme_background,
                )
            })
            .into()
        })
        .child(glyph)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::leaves_windows_unframed;

    #[test]
    fn only_gnome_sessions_keep_their_own_window_buttons() {
        for desktop in ["GNOME", "ubuntu:GNOME", "GNOME-Classic:gnome"] {
            assert!(leaves_windows_unframed(desktop), "{desktop}");
        }
        for desktop in ["", "KDE", "sway", "X-Cinnamon"] {
            assert!(!leaves_windows_unframed(desktop), "{desktop}");
        }
    }
}
