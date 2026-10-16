use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, DispatchPhase, FontWeight, Hsla, InteractiveElement, IntoElement,
    MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, canvas, div, px,
};
use muxy_ui::components::{IconButton, IconGlyph};
use muxy_ui::icon::Icon;
use muxy_ui::picker::{Confirm, Dismiss, SelectNext, SelectPrevious};
use muxy_ui::theme::contrasting_foreground;
use muxy_ui::tr;

use crate::model::AppModel;
use crate::model::agents_panel::{
    AgentStatus, Drag, Entry, EntryKey, MIN_HEIGHT, MIN_WIDTH, Placement, RunningAgent,
};

const DOCKED_MAX_HEIGHT: f32 = 260.0;
const POPUP_WIDTH: f32 = 300.0;
const POPUP_MAX_HEIGHT: f32 = 380.0;
const RESIZE_GRIP: f32 = 18.0;
const RESIZE_EDGE: f32 = 6.0;
const FLOATING_GROUP: &str = "agents-panel-floating";

fn status_label(status: AgentStatus) -> SharedString {
    match status {
        AgentStatus::Blocked => tr!("Needs input"),
        AgentStatus::Working => tr!("Working"),
        AgentStatus::Completed => tr!("Completed"),
        AgentStatus::Idle => tr!("Idle"),
    }
}

fn status_color(status: AgentStatus, model: &AppModel) -> Hsla {
    match status {
        AgentStatus::Blocked => model.theme.warning,
        AgentStatus::Working | AgentStatus::Completed => model.theme.accent,
        AgentStatus::Idle => model.theme.fg_dim,
    }
}

fn status_glyph(
    key: EntryKey,
    status: AgentStatus,
    owner: gpui::EntityId,
    model: &AppModel,
) -> AnyElement {
    let m = model.metrics;
    let glyph = if status == AgentStatus::Working {
        super::tab_activity::spinner_in(format!("agents-panel-{key}"), m.icon_sm(), owner, model)
    } else {
        let color = status_color(status, model);
        div()
            .size(m.scaled(7.0))
            .rounded_full()
            .when(status == AgentStatus::Idle, |dot| {
                dot.border_1().border_color(color)
            })
            .when(status != AgentStatus::Idle, |dot| dot.bg(color))
            .into_any_element()
    };
    div()
        .flex_none()
        .w(m.icon_md())
        .flex()
        .items_center()
        .justify_center()
        .child(glyph)
        .into_any_element()
}

impl AppModel {
    fn agents_location(&self, project: muxy_app_core::ProjectId) -> String {
        let Some(project) = self.state.project(project) else {
            return String::new();
        };
        match project
            .parent_id
            .and_then(|parent| self.state.project(parent))
        {
            Some(parent) => format!("{} › {}", parent.name, project.name),
            None => self.project_title(project),
        }
    }
}

fn trailing(entry: &Entry, model: &AppModel) -> AnyElement {
    let m = model.metrics;
    match entry {
        Entry::Agent(agent) => div()
            .flex_none()
            .text_size(m.font_footnote())
            .text_color(status_color(agent.status, model))
            .child(status_label(agent.status))
            .into_any_element(),
        Entry::Worktree { counts, .. } => div()
            .flex()
            .flex_none()
            .gap(m.spacing2())
            .children(
                AgentStatus::ALL
                    .into_iter()
                    .zip(counts)
                    .filter(|(_, count)| **count > 0)
                    .map(|(status, count)| {
                        div()
                            .flex()
                            .items_center()
                            .gap(m.spacing1())
                            .text_size(m.font_footnote())
                            .text_color(status_color(status, model))
                            .child(
                                div()
                                    .size(m.scaled(6.0))
                                    .rounded_full()
                                    .bg(status_color(status, model)),
                            )
                            .child(count.to_string())
                    }),
            )
            .into_any_element(),
    }
}

fn leading(entry: &Entry, owner: gpui::EntityId, model: &AppModel) -> AnyElement {
    let m = model.metrics;
    let key = entry.key();
    match entry {
        Entry::Agent(agent) => div()
            .flex_none()
            .flex()
            .items_center()
            .gap(m.spacing2())
            .child(status_glyph(key, agent.status, owner, model))
            .child(super::tab_activity::provider_icon(
                agent.provider,
                m.icon_sm(),
                model,
            ))
            .into_any_element(),
        Entry::Worktree { .. } => status_glyph(key, entry.status(), owner, model),
    }
}

fn entry_row(
    entry: &Entry,
    selected: bool,
    focused: bool,
    model: &AppModel,
    cx: &mut Context<AppModel>,
) -> AnyElement {
    let m = model.metrics;
    let theme = &model.theme;
    let key = entry.key();
    let (title, detail) = match entry {
        Entry::Agent(agent) => {
            let title = model
                .state
                .project(agent.project)
                .and_then(|project| project.tabs.iter().find(|tab| tab.id == agent.tab))
                .map(|tab| tab.title(Some(agent.pane)).trim().to_owned())
                .filter(|title| !title.is_empty())
                .unwrap_or_else(|| agent.provider.name().to_owned());
            (title, Some(model.agents_location(agent.project)))
        }
        Entry::Worktree { project, .. } => (model.agents_location(*project), None),
    };
    let trailing = trailing(entry, model);
    let leading = leading(entry, model.agents_panel.owner.entity_id(), model);
    div()
        .id(SharedString::from(format!("agents-panel-row-{key}")))
        .debug_selector(move || format!("agents-panel-row-{key}"))
        .flex()
        .flex_none()
        .items_center()
        .gap(m.spacing3())
        .px(m.spacing3())
        .py(m.spacing2())
        .rounded(m.radius_md())
        .cursor_pointer()
        .when(selected, |row| {
            row.bg(if focused { theme.hover } else { theme.surface })
        })
        .hover(|style| style.bg(theme.hover))
        .on_click(cx.listener(move |model, _, window, cx| {
            model.open_agents_entry(key, window, cx);
        }))
        .child(leading)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_size(m.font_body())
                        .text_color(theme.fg)
                        .child(title),
                )
                .children(detail.filter(|detail| !detail.is_empty()).map(|detail| {
                    div()
                        .truncate()
                        .text_size(m.font_footnote())
                        .text_color(theme.fg_muted)
                        .child(detail)
                })),
        )
        .child(trailing)
        .into_any_element()
}

fn rows(
    entries: &[Entry],
    model: &AppModel,
    cx: &mut Context<AppModel>,
    window: &Window,
) -> AnyElement {
    let m = model.metrics;
    let theme = &model.theme;
    let focused = model.agents_panel.focus.contains_focused(window, cx);
    let selected = model.selected_agents_entry(entries);
    let mut list = div()
        .id("agents-panel-rows")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .gap(m.scaled(1.0))
        .px(m.spacing2())
        .pb(m.spacing2())
        .overflow_y_scroll()
        .track_scroll(&model.agents_panel.scroll);
    if entries.is_empty() {
        return list
            .child(
                div()
                    .px(m.spacing3())
                    .py(m.spacing3())
                    .text_size(m.font_footnote())
                    .text_color(theme.fg_muted)
                    .child(tr!("No agents running")),
            )
            .into_any_element();
    }
    let mut previous = None;
    for entry in entries {
        let status = entry.status();
        if previous != Some(status) {
            list = list.child(
                div()
                    .px(m.spacing3())
                    .pt(m.spacing2())
                    .text_size(m.font_micro())
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.fg_dim)
                    .child(status_label(status).to_uppercase()),
            );
        }
        previous = Some(status);
        let is_selected = selected == Some(entry.key());
        list = list.child(entry_row(entry, is_selected, focused, model, cx));
    }
    div()
        .relative()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .child(list)
        .child(scrollbar(model, cx))
        .into_any_element()
}

fn header_buttons(detached: bool, model: &AppModel, cx: &mut Context<AppModel>) -> [IconButton; 2] {
    let m = model.metrics;
    let theme = &model.theme;
    let button = |id: &'static str, icon: Icon, tooltip: SharedString| {
        IconButton::new(
            id,
            icon,
            m.icon_xs(),
            m.control_small(),
            theme.fg_muted,
            theme.fg,
        )
        .tooltip(tooltip, theme.raised(), theme.fg, theme.border, theme.bg)
    };
    let dock = button(
        if detached {
            "agents-panel-reattach"
        } else {
            "agents-panel-detach"
        },
        if detached {
            Icon::PanelLeft
        } else {
            Icon::Maximize
        },
        if detached {
            tr!("Reattach to Sidebar")
        } else {
            tr!("Detach")
        },
    )
    .on_click(cx.listener(move |model, _, window, cx| {
        cx.stop_propagation();
        model.set_agents_panel_detached(!detached, window, cx);
    }));
    let close = button("agents-panel-hide", Icon::X, tr!("Hide")).on_click(cx.listener(
        |model, _, window, cx| {
            cx.stop_propagation();
            model.hide_agents_panel(window, cx);
        },
    ));
    [dock, close]
}

fn header(
    placement: Placement,
    agents: &[RunningAgent],
    model: &AppModel,
    cx: &mut Context<AppModel>,
) -> AnyElement {
    let m = model.metrics;
    let theme = &model.theme;
    let count = |status| agents.iter().filter(|agent| agent.status == status).count();
    let summary = [
        (
            AgentStatus::Blocked,
            tr!("%lld need input", count(AgentStatus::Blocked)),
        ),
        (
            AgentStatus::Working,
            tr!("%lld working", count(AgentStatus::Working)),
        ),
    ]
    .into_iter()
    .filter(|(status, _)| count(*status) > 0)
    .map(|(_, label)| label.to_string())
    .collect::<Vec<_>>()
    .join(" · ");
    let detached = placement == Placement::Floating;
    div()
        .id("agents-panel-header")
        .debug_selector(|| "agents-panel-header".into())
        .flex()
        .flex_none()
        .items_center()
        .gap(m.spacing2())
        .pl(m.spacing4())
        .pr(m.spacing2())
        .h(m.control_large())
        .when(detached, |header| {
            header.cursor(gpui::CursorStyle::OpenHand).on_mouse_down(
                MouseButton::Left,
                cx.listener(|model, event: &gpui::MouseDownEvent, window, cx| {
                    model.begin_agents_panel_drag(None, event.position, window, cx);
                    model.focus_agents_panel(window);
                    window.prevent_default();
                    cx.stop_propagation();
                }),
            )
        })
        .child(
            div()
                .flex_none()
                .text_size(m.font_caption())
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.fg_muted)
                .child(tr!("Agents")),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .text_size(m.font_footnote())
                .text_color(theme.fg_dim)
                .child(summary),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(header_buttons(detached, model, cx)),
        )
        .into_any_element()
}

fn body(
    placement: Placement,
    model: &AppModel,
    window: &Window,
    cx: &mut Context<AppModel>,
) -> gpui::Stateful<gpui::Div> {
    let agents = model.running_agents();
    let entries = model.agents_panel_entries_from(agents.clone());
    div()
        .id("agents-panel")
        .debug_selector(|| "agents-panel".into())
        .track_focus(&model.agents_panel.focus)
        .key_context("Picker")
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|model, _, window, cx| {
                model.focus_agents_panel(window);
                cx.notify();
            }),
        )
        .on_action(cx.listener(|model, _: &SelectNext, _, cx| {
            model.move_agents_selection(1, cx);
        }))
        .on_action(cx.listener(|model, _: &SelectPrevious, _, cx| {
            model.move_agents_selection(-1, cx);
        }))
        .on_action(cx.listener(|model, _: &Confirm, window, cx| {
            model.open_agents_selection(window, cx);
        }))
        .on_action(cx.listener(move |model, _: &Dismiss, window, cx| {
            if placement == Placement::Popup {
                model.close_agents_popup(window, cx);
            } else {
                model.focus_active(window, cx);
                cx.notify();
            }
        }))
        .relative()
        .flex()
        .flex_col()
        .min_h(px(0.0))
        .child(header(placement, &agents, model, cx))
        .child(rows(&entries, model, cx, window))
        .child(drag_tracker(cx))
}

pub(crate) fn docked(
    model: &AppModel,
    window: &Window,
    cx: &mut Context<AppModel>,
) -> Option<AnyElement> {
    if model.agents_panel_placement() != Placement::Docked || !model.appearance.agents_panel_visible
    {
        return None;
    }
    let m = model.metrics;
    let height = model
        .appearance
        .agents_panel_docked_height
        .filter(|height| height.is_finite())
        .map(|height| model.clamp_docked_height(height, window.viewport_size()));
    let measured = model.agents_panel.docked_height.clone();
    let start = measured.clone();
    let active = matches!(model.agents_panel.drag, Some(Drag::Docked { .. }));
    let background = model.sidebar_background();
    let separator = Hsla::from(contrasting_foreground(background.into())).opacity(0.08);
    let panel = div()
        .relative()
        .flex()
        .flex_col()
        .flex_none()
        .mx(m.spacing3())
        .mb(m.spacing3())
        .map(|panel| match height {
            Some(height) => panel.h(m.scaled(height)),
            None => panel.max_h(m.scaled(DOCKED_MAX_HEIGHT)),
        })
        .child(
            canvas(
                move |bounds, _, _| measured.set(bounds.size.height),
                |_, (), _, _| {},
            )
            .absolute()
            .size_full(),
        )
        .child(body(Placement::Docked, model, window, cx).flex_1())
        .child(
            div()
                .id("agents-panel-docked-resize")
                .debug_selector(|| "agents-panel-docked-resize".into())
                .group("agents-panel-docked-resize")
                .absolute()
                .top(-m.scaled(RESIZE_EDGE / 2.0))
                .left(-m.spacing3())
                .right(-m.spacing3())
                .h(m.scaled(RESIZE_EDGE))
                .cursor(gpui::CursorStyle::ResizeUpDown)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |model, event: &gpui::MouseDownEvent, window, cx| {
                        model.begin_agents_panel_docked_drag(event.position.y, start.get(), cx);
                        window.prevent_default();
                        cx.stop_propagation();
                    }),
                )
                .child(
                    div()
                        .absolute()
                        .top(m.scaled(RESIZE_EDGE / 2.0) - px(1.0))
                        .w_full()
                        .h(px(1.0))
                        .group_hover("agents-panel-docked-resize", |line| {
                            line.bg(model.theme.accent)
                        }),
                ),
        );
    Some(
        div()
            .flex_none()
            .bg(background)
            .border_t_1()
            .border_color(separator)
            .when(active, |panel| panel.border_color(model.theme.accent))
            .child(panel)
            .into_any_element(),
    )
}

fn scrollbar(model: &AppModel, cx: &mut Context<AppModel>) -> AnyElement {
    use muxy_ui::scrollbar::{MINIMUM_THUMB_LENGTH, ThumbGeometry, WIDTH};
    let m = model.metrics;
    let color = model.theme.fg_dim.opacity(0.5);
    let handle = model.agents_panel.scroll.clone();
    let press = handle.clone();
    let thumb = |handle: &gpui::ScrollHandle, track: gpui::Pixels| {
        let visible = f64::from(f32::from(handle.bounds().size.height));
        let max = f64::from(f32::from(handle.max_offset().height));
        ThumbGeometry::from_lengths(
            visible + max,
            visible,
            f64::from(f32::from(-handle.offset().y)),
            f64::from(f32::from(track)),
            MINIMUM_THUMB_LENGTH,
        )
    };
    let width = m.scaled(WIDTH);
    div()
        .id("agents-panel-scrollbar")
        .debug_selector(|| "agents-panel-scrollbar".into())
        .absolute()
        .top_0()
        .bottom(m.spacing2())
        .right(m.scaled(2.0))
        .w(width)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |model, event: &gpui::MouseDownEvent, _, cx| {
                let track = press.bounds();
                let Some(geometry) = thumb(&press, track.size.height) else {
                    return;
                };
                cx.stop_propagation();
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Thumb lengths are a window's pixels"
                )]
                let (origin, length) = (geometry.origin as f32, geometry.length as f32);
                let travel = (f32::from(track.size.height) - length).max(1.0);
                let ratio = f32::from(press.max_offset().height) / travel;
                let y = f32::from(event.position.y - track.top());
                if y < origin || y > origin + length {
                    let target = ((y - length / 2.0) * ratio)
                        .clamp(0.0, f32::from(press.max_offset().height));
                    press.set_offset(gpui::point(press.offset().x, -px(target)));
                }
                model.begin_agents_panel_scroll_drag(event.position.y, ratio, cx);
            }),
        )
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, (), window, _| {
                    let Some(geometry) = thumb(&handle, bounds.size.height) else {
                        return;
                    };
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "Thumb lengths are a window's pixels"
                    )]
                    let (origin, length) = (geometry.origin as f32, geometry.length as f32);
                    window.paint_quad(
                        gpui::fill(
                            gpui::Bounds::new(
                                gpui::point(bounds.left(), bounds.top() + px(origin)),
                                gpui::size(bounds.size.width, px(length)),
                            ),
                            color,
                        )
                        .corner_radii(width / 2.0),
                    );
                },
            )
            .size_full(),
        )
        .into_any_element()
}

pub(crate) fn collapsed_button(model: &AppModel, cx: &mut Context<AppModel>) -> Option<AnyElement> {
    let open = matches!(model.overlay, Some(super::overlays::Overlay::Agents));
    if model.agents_panel_placement() != Placement::Popup
        || model.appearance.sidebar_expanded
        || !(open || model.appearance.agents_panel_visible)
    {
        return None;
    }
    let m = model.metrics;
    let theme = &model.theme;
    let urgent = model
        .running_agents()
        .first()
        .map(|agent| agent.status)
        .filter(|status| *status != AgentStatus::Idle);
    Some(
        div()
            .id("agents-panel-button")
            .debug_selector(|| "agents-panel-button".into())
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .mx_auto()
            .mb(m.spacing2())
            .size(m.scaled(34.0))
            .rounded(m.radius_xl())
            .bg(if open { theme.hover } else { theme.surface })
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .tooltip(super::status_bar::tooltip(tr!("Agents").to_string(), model))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|model, _, window, cx| {
                if matches!(model.overlay, Some(super::overlays::Overlay::Agents)) {
                    model.close_agents_popup(window, cx);
                } else {
                    model.toggle_agents_panel(window, cx);
                }
            }))
            .child(IconGlyph::new(
                Icon::TerminalStack,
                m.font_body(),
                theme.fg_muted,
            ))
            .children(urgent.map(|status| {
                div()
                    .absolute()
                    .top(m.scaled(5.0))
                    .right(m.scaled(5.0))
                    .size(m.scaled(7.0))
                    .rounded_full()
                    .bg(status_color(status, model))
            }))
            .into_any_element(),
    )
}

pub(crate) fn popup(model: &AppModel, window: &Window, cx: &mut Context<AppModel>) -> AnyElement {
    let m = model.metrics;
    let theme = &model.theme;
    let bottom = if model.appearance.status_bar_visible {
        m.scaled(36.0)
    } else {
        m.spacing4()
    };
    div()
        .absolute()
        .left(px(model.sidebar_width()) + m.spacing4())
        .bottom(bottom)
        .w(m.scaled(POPUP_WIDTH))
        .max_h(m.scaled(POPUP_MAX_HEIGHT))
        .flex()
        .flex_col()
        .rounded(m.radius_xl())
        .border_1()
        .border_color(theme.border)
        .bg(theme.raised())
        .shadow_lg()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(body(Placement::Popup, model, window, cx).flex_1())
        .child(model.webview_occlusion_except(Some(model.agents_panel.owner.entity_id())))
        .into_any_element()
}

fn resize_handle(
    id: &'static str,
    axes: [bool; 2],
    model: &AppModel,
    cx: &mut Context<AppModel>,
) -> AnyElement {
    let m = model.metrics;
    let theme = &model.theme;
    div()
        .id(id)
        .debug_selector(move || id.into())
        .absolute()
        .map(|handle| match axes {
            [true, false] => handle
                .top_0()
                .bottom(m.scaled(RESIZE_GRIP))
                .right_0()
                .w(m.scaled(RESIZE_EDGE))
                .cursor(gpui::CursorStyle::ResizeLeftRight),
            [false, true] => handle
                .left_0()
                .right(m.scaled(RESIZE_GRIP))
                .bottom_0()
                .h(m.scaled(RESIZE_EDGE))
                .cursor(gpui::CursorStyle::ResizeUpDown),
            _ => handle
                .right_0()
                .bottom_0()
                .size(m.scaled(RESIZE_GRIP))
                .cursor(gpui::CursorStyle::ResizeUpLeftDownRight)
                .child(
                    div()
                        .invisible()
                        .group_hover(FLOATING_GROUP, Styled::visible)
                        .absolute()
                        .right(m.scaled(4.0))
                        .bottom(m.scaled(4.0))
                        .size(m.scaled(7.0))
                        .border_r_2()
                        .border_b_2()
                        .rounded_br(m.scaled(2.0))
                        .border_color(theme.fg_dim),
                ),
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |model, event: &gpui::MouseDownEvent, window, cx| {
                model.begin_agents_panel_drag(Some(axes), event.position, window, cx);
                window.prevent_default();
                cx.stop_propagation();
            }),
        )
        .into_any_element()
}

fn drag_tracker(cx: &mut Context<AppModel>) -> AnyElement {
    let weak = cx.weak_entity();
    canvas(
        |_, _, _| (),
        move |_, (), window, _| {
            let moving = weak.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                let _ = moving.update(cx, |model, cx| {
                    if model.agents_panel.drag.is_none() {
                        return;
                    }
                    if event.pressed_button == Some(MouseButton::Left) {
                        model.move_agents_panel_drag(event.position, window, cx);
                    } else {
                        model.finish_agents_panel_drag(cx);
                    }
                    cx.stop_propagation();
                });
            });
            let ending = weak.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                    return;
                }
                let _ = ending.update(cx, |model, cx| {
                    if model.agents_panel.drag.is_some() {
                        model.move_agents_panel_drag(event.position, window, cx);
                        model.finish_agents_panel_drag(cx);
                        cx.stop_propagation();
                    }
                });
            });
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}

pub(crate) fn floating(
    model: &AppModel,
    window: &Window,
    cx: &mut Context<AppModel>,
) -> Option<AnyElement> {
    if model.agents_panel_placement() != Placement::Floating
        || !model.appearance.agents_panel_visible
    {
        return None;
    }
    let m = model.metrics;
    let theme = &model.theme;
    let [x, y, width, height] = model.agents_panel_frame(window.viewport_size());
    let dragging = model.agents_panel.drag.is_some();
    let panel = div()
        .group(FLOATING_GROUP)
        .absolute()
        .left(m.scaled(x))
        .top(m.scaled(y))
        .w(m.scaled(width.max(MIN_WIDTH)))
        .h(m.scaled(height.max(MIN_HEIGHT)))
        .flex()
        .flex_col()
        .rounded(m.radius_xl())
        .border_1()
        .border_color(theme.border)
        .bg(theme.raised())
        .shadow_lg()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(body(Placement::Floating, model, window, cx).flex_1())
        .child(resize_handle(
            "agents-panel-resize-right",
            [true, false],
            model,
            cx,
        ))
        .child(resize_handle(
            "agents-panel-resize-bottom",
            [false, true],
            model,
            cx,
        ))
        .child(resize_handle(
            "agents-panel-resize",
            [true, true],
            model,
            cx,
        ))
        .child(model.webview_occlusion_except(Some(model.agents_panel.owner.entity_id())));
    Some(
        div()
            .absolute()
            .inset_0()
            .when(dragging, |layer| {
                layer.child(div().absolute().inset_0().occlude())
            })
            .child(panel)
            .into_any_element(),
    )
}
