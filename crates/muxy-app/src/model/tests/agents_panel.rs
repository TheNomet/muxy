use super::worktrees::{Sent, connected, pruning_fixture};
use super::*;
use crate::model::agents_panel::{AgentStatus, Entry, EntryKey, Placement};
use gpui::Modifiers;
use muxy_app_core::settings::{AgentsPanelGrouping, AppLayout};
use muxy_core::shortcuts::ShortcutId;
use muxy_protocol::{
    ActivityEvent, ActivityKind, ActivitySnapshot, AgentActivity, AgentProvider, AgentState,
};

struct Fixture {
    root: ProjectId,
    worktree: ProjectId,
    panes: [PaneId; 4],
    _sent: Sent,
}

fn agents(cx: &mut TestAppContext) -> (Entity<AppModel>, &mut VisualTestContext, Fixture) {
    let (mut state, root, [worktree, ..], _directory) = pruning_fixture();
    let home = state.home().id;
    let first_pane = |state: &AppState, project: ProjectId, tab: TabId| {
        state
            .project(project)
            .and_then(|project| project.tabs.iter().find(|entry| entry.id == tab))
            .map(|tab| tab.panes[0].id)
            .expect("pane")
    };
    let root_tab = state.open_terminal_tab(root).expect("tab");
    let root_pane = first_pane(&state, root, root_tab);
    let mirror = state
        .split_pane(root_pane, Direction::Right)
        .expect("split");
    let worktree_tab = state.open_terminal_tab(worktree).expect("tab");
    let worktree_pane = first_pane(&state, worktree, worktree_tab);
    let home_tab = state.open_terminal_tab(home).expect("tab");
    let home_pane = first_pane(&state, home, home_tab);
    state.select_project(root).expect("root");
    state.select_tab(root, root_tab).expect("tab");
    let session = |id| SessionId::new(id).expect("session");
    let agent = |id, state| AgentActivity {
        session: session(id),
        project: root,
        provider: AgentProvider::Claude,
        state,
    };
    let finished = |id, session, read| ActivityEvent {
        id,
        session,
        project: home,
        provider: AgentProvider::Claude,
        timestamp: 0,
        kind: ActivityKind::Completed,
        read,
    };
    let (view, cx, sent) = connected(state, cx);
    view.update(cx, |model, cx| {
        let local = ServerId::local();
        model.receive_attached(local, root_pane, session(1), attachment(), true, cx);
        model.receive_attached(local, mirror, session(1), attachment(), false, cx);
        model.receive_attached(local, worktree_pane, session(2), attachment(), false, cx);
        model.receive_attached(local, home_pane, session(3), attachment(), false, cx);
        model.servers.local.activity.loaded = true;
        model.receive_activity(
            local,
            Ok(ActivitySnapshot {
                agents: vec![
                    agent(1, AgentState::Working),
                    agent(2, AgentState::Blocked),
                    agent(3, AgentState::Idle),
                    agent(4, AgentState::Blocked),
                ],
                events: vec![
                    finished(1, session(1), false),
                    finished(2, session(3), false),
                ],
                ..ActivitySnapshot::default()
            }),
            cx,
        );
    });
    (
        view,
        cx,
        Fixture {
            root,
            worktree,
            panes: [root_pane, mirror, worktree_pane, home_pane],
            _sent: sent,
        },
    )
}

#[gpui::test]
fn the_panel_lists_each_shown_session_once_most_urgent_first(cx: &mut TestAppContext) {
    let (view, cx, fixture) = agents(cx);
    let [root_pane, _, worktree_pane, home_pane] = fixture.panes;
    view.read_with(cx, |model, _| {
        let agents: Vec<_> = model
            .running_agents()
            .into_iter()
            .map(|agent| (agent.pane, agent.status))
            .collect();
        assert_eq!(
            agents,
            [
                (worktree_pane, AgentStatus::Blocked),
                (root_pane, AgentStatus::Working),
                (home_pane, AgentStatus::Completed),
            ],
            "a session in two panes counts once, a working agent's old finish is not \
             Completed, and an agent no pane shows is left out"
        );
    });
    view.update(cx, |model, _| {
        model.appearance.agents_panel_grouping = AgentsPanelGrouping::Worktrees;
        let entries = model.agents_panel_entries();
        assert!(matches!(
            entries.first(),
            Some(Entry::Worktree { project, counts: [1, 0, 0, 0] }) if *project == fixture.worktree
        ));
        assert!(entries.iter().any(|entry| matches!(
            entry,
            Entry::Worktree { project, counts: [0, 1, 0, 0] } if *project == fixture.root
        )));
        assert_eq!(entries.len(), 3);
    });
}

#[gpui::test]
fn the_shortcut_shows_and_focuses_then_hides_and_keys_open_only_live_panes(
    cx: &mut TestAppContext,
) {
    let (view, cx, fixture) = agents(cx);
    let [root_pane, _, worktree_pane, _] = fixture.panes;
    cx.update(|_, cx| {
        crate::views::workspace::bind_keys(&muxy_app_core::settings::Keymap::default(), cx);
    });
    view.update(cx, |model, cx| {
        model.appearance.sidebar_expanded = true;
        model.appearance.agents_panel_visible = false;
        model.focus_requested = true;
        cx.notify();
    });
    cx.run_until_parked();
    let focused = |view: &Entity<AppModel>, cx: &mut VisualTestContext| {
        view.update_in(cx, |model, window, cx| {
            model.agents_panel.focus.contains_focused(window, cx)
        })
    };

    cx.simulate_keystrokes(ShortcutId::ToggleAgentsPanel.definition().keys[0]);
    view.read_with(cx, |model, _| {
        assert_eq!(model.agents_panel_placement(), Placement::Docked);
        assert!(model.agents_panel_shown());
    });
    assert!(focused(&view, cx), "a hidden panel shows with focus");
    cx.run_until_parked();
    assert!(
        focused(&view, cx),
        "focus is not handed back on the next frame"
    );

    cx.simulate_keystrokes("down enter");
    view.read_with(cx, |model, _| {
        assert_eq!(model.state.current_project().id, fixture.root);
        assert_eq!(model.active_pane(), Some(root_pane));
    });
    assert!(!focused(&view, cx), "opening an agent focuses its pane");

    cx.simulate_keystrokes(ShortcutId::ToggleAgentsPanel.definition().keys[0]);
    assert!(focused(&view, cx), "a shown panel takes focus first");
    view.read_with(cx, |model, _| assert!(model.agents_panel_shown()));
    cx.simulate_keystrokes(ShortcutId::ToggleAgentsPanel.definition().keys[0]);
    view.read_with(cx, |model, _| {
        assert!(!model.agents_panel_shown(), "a focused panel hides");
        assert!(!model.appearance.agents_panel_visible);
    });

    view.update(cx, |model, cx| {
        model.select_project(fixture.root, cx);
        model.detach_terminal(worktree_pane, cx);
    });
    view.update_in(cx, |model, window, cx| {
        model.open_agents_entry(
            EntryKey::Agent(ServerId::local(), SessionId::new(2).expect("session")),
            window,
            cx,
        );
        assert_eq!(
            model.state.current_project().id,
            fixture.root,
            "an agent whose pane is gone opens nothing"
        );
    });
}

#[gpui::test]
fn the_layout_menu_shows_and_hides_the_panel_without_changing_layout(cx: &mut TestAppContext) {
    let (view, cx, _fixture) = agents(cx);
    let click = |selector: &'static str, cx: &mut VisualTestContext| {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        let bounds = cx.debug_bounds(selector).expect(selector);
        cx.simulate_click(bounds.center(), Modifiers::none());
        cx.run_until_parked();
    };
    for detached in [false, true] {
        view.update_in(cx, |model, window, cx| {
            model.appearance.layout = AppLayout::TabFocused;
            model.appearance.sidebar_expanded = true;
            model.appearance.agents_panel_visible = false;
            model.appearance.agents_panel_detached = detached;
            model.focus_active(window, cx);
            cx.notify();
        });
        click("layout-menu", cx);
        view.read_with(cx, |model, _| {
            assert_eq!(
                model.menu_outline()[0].last().map(String::as_str),
                Some("Show agents panel")
            );
        });
        click("menu-item-3", cx);
        view.update_in(cx, |model, window, cx| {
            assert!(model.overlay.is_none());
            assert!(model.agents_panel_shown());
            assert!(model.settings.appearance.agents_panel_visible);
            assert!(model.agents_panel.focus.contains_focused(window, cx));
            assert_eq!(model.appearance.layout, AppLayout::TabFocused);
            model.focus_active(window, cx);
        });

        click("layout-menu", cx);
        view.read_with(cx, |model, _| {
            assert_eq!(
                model.menu_outline()[0].last().map(String::as_str),
                Some("Hide agents panel")
            );
        });
        click("menu-item-3", cx);
        view.update_in(cx, |model, window, cx| {
            assert!(model.overlay.is_none());
            assert!(!model.agents_panel_shown());
            assert!(!model.settings.appearance.agents_panel_visible);
            assert!(!model.agents_panel.focus.contains_focused(window, cx));
            assert_eq!(model.appearance.layout, AppLayout::TabFocused);
        });
    }
}

#[gpui::test]
fn the_panel_keeps_keyboard_focus_over_an_empty_project(cx: &mut TestAppContext) {
    let (view, cx, fixture) = agents(cx);
    cx.update(|_, cx| {
        crate::views::workspace::bind_keys(&muxy_app_core::settings::Keymap::default(), cx);
    });
    for detached in [false, true] {
        view.update(cx, |model, cx| {
            let empty = model
                .state
                .projects()
                .iter()
                .find(|project| project.parent_id == Some(fixture.root) && project.tabs.is_empty())
                .expect("empty worktree")
                .id;
            model.appearance.sidebar_expanded = true;
            model.appearance.agents_panel_detached = detached;
            model.appearance.agents_panel_visible = false;
            model.select_project(empty, cx);
            assert!(model.active_pane().is_none());
        });
        cx.run_until_parked();
        cx.simulate_keystrokes(ShortcutId::ToggleAgentsPanel.definition().keys[0]);
        cx.run_until_parked();
        view.update_in(cx, |model, window, cx| {
            assert!(model.agents_panel.focus.contains_focused(window, cx));
        });
        cx.simulate_keystrokes(ShortcutId::ToggleAgentsPanel.definition().keys[0]);
        cx.run_until_parked();
        view.read_with(cx, |model, _| assert!(!model.agents_panel_shown()));

        cx.simulate_keystrokes(ShortcutId::ToggleAgentsPanel.definition().keys[0]);
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |model, _| {
            assert_eq!(model.active_pane(), Some(fixture.panes[2]));
            assert_eq!(model.state.current_project().id, fixture.worktree);
        });
        view.update_in(cx, |model, window, cx| {
            model.focus_agents_panel(window);
            model.focus_requested = true;
            cx.notify();
        });
        cx.run_until_parked();
        view.update_in(cx, |model, window, cx| {
            assert!(!model.agents_panel.focus.contains_focused(window, cx));
        });
    }
}

#[gpui::test]
fn a_collapsed_sidebar_opens_the_panel_as_a_popup_that_escape_closes(cx: &mut TestAppContext) {
    let (view, cx, _fixture) = agents(cx);
    cx.update(|_, cx| {
        crate::views::workspace::bind_keys(&muxy_app_core::settings::Keymap::default(), cx);
    });
    view.update(cx, |model, cx| {
        model.appearance.sidebar_expanded = false;
        cx.notify();
    });
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("agents-panel-button").is_none(),
        "the button waits until the panel is shown"
    );
    cx.simulate_keystrokes(ShortcutId::ToggleAgentsPanel.definition().keys[0]);
    view.read_with(cx, |model, _| {
        assert_eq!(model.agents_panel_placement(), Placement::Popup);
        assert!(matches!(model.overlay, Some(Overlay::Agents)));
        assert!(
            model.appearance.agents_panel_visible,
            "showing the popup keeps its button until the panel is hidden"
        );
    });
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |model, _| assert!(model.overlay.is_none()));

    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("agents-panel-button").is_some(),
        "escape keeps the button after the first popup opening"
    );
    cx.simulate_keystrokes(ShortcutId::ToggleAgentsPanel.definition().keys[0]);
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |model, _| {
        assert!(model.overlay.is_none());
        assert!(
            model.appearance.agents_panel_visible,
            "escape closes the popup but keeps the button"
        );
    });
    view.update_in(cx, AppModel::hide_agents_panel);
    view.read_with(cx, |model, _| {
        assert!(
            !model.appearance.agents_panel_visible,
            "hiding the panel removes the button"
        );
    });
}

#[gpui::test]
fn keyboard_navigation_uses_the_highlighted_row_after_selection_changes(cx: &mut TestAppContext) {
    let (view, cx, fixture) = agents(cx);
    cx.update(|_, cx| {
        crate::views::workspace::bind_keys(&muxy_app_core::settings::Keymap::default(), cx);
    });
    view.update_in(cx, |model, window, cx| {
        model.appearance.sidebar_expanded = true;
        model.appearance.agents_panel_visible = true;
        model.focus_agents_panel(window);
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |model, _| {
        assert_eq!(model.active_pane(), Some(fixture.panes[2]));
        assert_eq!(model.state.current_project().id, fixture.worktree);
    });

    view.update_in(cx, |model, window, cx| {
        model
            .servers
            .local
            .activity
            .snapshot
            .agents
            .retain(|agent| agent.session != SessionId::new(2).expect("session"));
        model.focus_agents_panel(window);
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |model, _| {
        assert_eq!(model.active_pane(), Some(fixture.panes[0]));
        assert_eq!(model.state.current_project().id, fixture.root);
    });

    view.update_in(cx, |model, window, cx| {
        model.appearance.agents_panel_grouping = AgentsPanelGrouping::Worktrees;
        model.focus_agents_panel(window);
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("down enter");
    view.read_with(cx, |model, _| {
        assert_eq!(model.state.current_project().id, model.state.home().id);
    });
}

#[gpui::test]
fn a_saved_floating_frame_stays_inside_a_smaller_window(cx: &mut TestAppContext) {
    let (view, cx, _fixture) = agents(cx);
    cx.simulate_resize(size(px(500.0), px(400.0)));
    view.update_in(cx, |model, window, cx| {
        model.set_agents_panel_detached(true, window, cx);
        assert_eq!(model.agents_panel_placement(), Placement::Floating);
        model.appearance.agents_panel_frame = Some([2000.0, -50.0, 5000.0, 10.0]);
        let [x, y, width, height] = model.agents_panel_frame(window.viewport_size());
        assert!(x >= 0.0 && y >= 0.0);
        assert!(x + width <= 500.0 && y + height <= 400.0);
        assert!(height >= crate::model::agents_panel::MIN_HEIGHT);

        model.appearance.agents_panel_frame = Some([f32::NAN, 0.0, f32::NAN, f32::INFINITY]);
        let frame = model.agents_panel_frame(window.viewport_size());
        assert!(
            frame.iter().all(|value| value.is_finite()),
            "a hand-edited nan in settings.toml falls back instead of crashing: {frame:?}"
        );
        model.appearance.agents_panel_frame = Some([2000.0, -50.0, 5000.0, 10.0]);

        let origin = gpui::point(px(100.0), px(100.0));
        model.begin_agents_panel_drag(None, origin, window, cx);
        model.move_agents_panel_drag(gpui::point(px(-900.0), px(900.0)), window, cx);
        model.finish_agents_panel_drag(cx);
        let [x, y, _, height] = model.agents_panel_frame(window.viewport_size());
        assert!(x.abs() < 0.01, "dragged past the left edge, it stops there");
        assert!(
            (y + height - 400.0).abs() < 0.01,
            "dragged past the edge, it stops there"
        );
        assert_eq!(
            model.settings.appearance.agents_panel_frame, model.appearance.agents_panel_frame,
            "the dropped frame is saved"
        );

        model.set_agents_panel_detached(false, window, cx);
        assert!(!model.settings.appearance.agents_panel_detached);
    });
}

#[gpui::test]
fn the_floating_panel_moves_by_its_header_and_resizes_by_its_corner(cx: &mut TestAppContext) {
    let (view, cx, _fixture) = agents(cx);
    cx.simulate_resize(size(px(1200.0), px(800.0)));
    view.update_in(cx, |model, window, cx| {
        model.set_agents_panel_detached(true, window, cx);
        model.appearance.agents_panel_frame = Some([100.0, 100.0, 300.0, 300.0]);
    });
    let drag =
        |selector: &'static str, by: gpui::Point<gpui::Pixels>, cx: &mut VisualTestContext| {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            let bounds = cx.debug_bounds(selector).expect(selector);
            let start = bounds.center();
            cx.simulate_mouse_down(start, gpui::MouseButton::Left, Modifiers::none());
            cx.simulate_mouse_move(start + by, Some(gpui::MouseButton::Left), Modifiers::none());
            cx.simulate_mouse_up(start + by, gpui::MouseButton::Left, Modifiers::none());
            cx.run_until_parked();
        };
    let frame = |view: &Entity<AppModel>, cx: &mut VisualTestContext| {
        view.read_with(cx, |model, _| model.settings.appearance.agents_panel_frame)
            .expect("saved frame")
    };
    drag("agents-panel-resize", gpui::point(px(80.0), px(40.0)), cx);
    let [x, y, width, height] = frame(&view, cx);
    assert!(
        (x - 100.0).abs() < 0.5 && (y - 100.0).abs() < 0.5,
        "resizing keeps the origin"
    );
    assert!(
        (width - 380.0).abs() < 0.5 && (height - 340.0).abs() < 0.5,
        "the corner resizes: {width}x{height}"
    );
    drag("agents-panel-header", gpui::point(px(50.0), px(20.0)), cx);
    let [x, y, width, _] = frame(&view, cx);
    assert!(
        (x - 150.0).abs() < 0.5 && (y - 120.0).abs() < 0.5,
        "the header moves it: {x},{y}"
    );
    assert!((width - 380.0).abs() < 0.5, "moving keeps the size");
}

#[gpui::test]
fn the_docked_panel_resizes_by_its_top_edge_and_scrolls_by_its_thumb(cx: &mut TestAppContext) {
    let (view, cx, _fixture) = agents(cx);
    cx.simulate_resize(size(px(1200.0), px(800.0)));
    let drag = |selector: &'static str, by: gpui::Pixels, cx: &mut VisualTestContext| {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        let start = cx.debug_bounds(selector).expect(selector).center();
        let end = start + gpui::point(px(0.0), by);
        cx.simulate_mouse_down(start, gpui::MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Modifiers::none());
        cx.simulate_mouse_up(end, gpui::MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
    };

    let scrolled = |view: &Entity<AppModel>, cx: &mut VisualTestContext| {
        view.read_with(cx, |model, _| -model.agents_panel.scroll.offset().y)
    };
    for layout in [
        AppLayout::ProjectFocused,
        AppLayout::TabFocused,
        AppLayout::AgentsFocused,
    ] {
        view.update(cx, |model, cx| {
            model.appearance.layout = layout;
            model.appearance.sidebar_expanded = true;
            model.appearance.agents_panel_visible = true;
            model.appearance.agents_panel_docked_height =
                Some(crate::model::agents_panel::MIN_HEIGHT);
            model
                .agents_panel
                .scroll
                .set_offset(gpui::point(px(0.0), px(0.0)));
            cx.notify();
        });
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        assert!(
            view.read_with(cx, |model, _| model.agents_panel.scroll.max_offset().height) > px(0.0),
            "the short panel overflows in {layout:?}"
        );
        drag("agents-panel-scrollbar", px(40.0), cx);
        assert!(
            scrolled(&view, cx) > px(0.0),
            "the thumb scrolls the list in {layout:?}"
        );

        drag("agents-panel-docked-resize", px(-60.0), cx);
        let height = view.read_with(cx, |model, _| {
            model.settings.appearance.agents_panel_docked_height
        });
        let expected = crate::model::agents_panel::MIN_HEIGHT + 60.0;
        assert!(
            height.is_some_and(|height| (height - expected).abs() < 1.0),
            "dragging the top edge up makes it taller and is saved in {layout:?}: {height:?}"
        );
    }
}
