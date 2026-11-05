use super::AppModel;
use crate::boot::Work;
use crate::views::voice::{VoiceEvent, VoicePanel};
use gpui::{
    AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Styled, Subscription,
    Window, div,
};
use muxy_app_core::{
    PaneId, ProjectId, ProjectKind, ProjectStatus, ServerId, composer::DraftId,
    dictionary::Dictionary,
};
use muxy_core::capability::Feature;
use muxy_protocol::{ChannelId, Modes};
use muxy_ui::tr;
use std::collections::HashMap;
use std::sync::Arc;

struct Target {
    project: DraftId,
    pane: PaneId,
    attachment: (ServerId, ChannelId),
    generation: u64,
    modes: Modes,
}

#[derive(Default)]
pub(crate) struct VoiceRuntime {
    pub(crate) view: Option<Entity<VoicePanel>>,
    pub(crate) closing: Option<Entity<VoicePanel>>,
    exit: Option<gpui::Task<()>>,
    target: Option<Target>,
    subscription: Option<Subscription>,
    /// Each project's `.muxy/dictionary.txt`, as last read.
    dictionaries: HashMap<ProjectId, Arc<Dictionary>>,
    /// The project whose dictionary was last requested on activation.
    dictionary_project: Option<ProjectId>,
}

impl AppModel {
    /// The current project's dictation dictionary.
    pub(crate) fn dictionary(&self) -> Arc<Dictionary> {
        self.voice
            .dictionaries
            .get(&self.state.current_project().id)
            .cloned()
            .unwrap_or_default()
    }

    pub(super) fn sync_dictionary(&mut self, cx: &mut Context<Self>) {
        let server = self.state.current_project().server_id;
        if !self.ready(server)
            || !self.state.project_intents(server).is_empty()
            || self
                .servers
                .get(server)
                .is_none_or(|runtime| runtime.catalog.restore.is_some())
        {
            return;
        }
        self.voice
            .dictionaries
            .retain(|id, _| self.state.project(*id).is_some());
        let project = self.state.current_project().id;
        if self.voice.dictionary_project != Some(project) && self.request_dictionary(cx) {
            self.voice.dictionary_project = Some(project);
        }
    }

    /// Reads the current project's dictionary again, so edits apply to the
    /// next dictation. A worktree without one uses its project's.
    fn request_dictionary(&mut self, cx: &mut Context<Self>) -> bool {
        let project = self.state.current_project();
        if project.status() != ProjectStatus::Available {
            return false;
        }
        let work = Work::ProjectDictionary {
            project: project.id,
            fallback: project
                .parent_id
                .filter(|_| project.kind == Some(ProjectKind::Worktree)),
        };
        self.send(project.server_id, work, cx)
    }

    pub(super) fn receive_dictionary(
        &mut self,
        project: ProjectId,
        result: Result<Dictionary, String>,
        cx: &mut Context<Self>,
    ) {
        let dictionary = match result {
            Ok(dictionary) => Arc::new(dictionary),
            Err(error) => {
                self.set_banner_error(Some(
                    tr!(
                        "Could not load %@: %@",
                        muxy_app_core::dictionary::PATH,
                        &error
                    )
                    .to_string(),
                ));
                cx.notify();
                return;
            }
        };
        if self.state.project(project).is_none() {
            return;
        }
        if project == self.state.current_project().id
            && let Some(view) = &self.composer.view
        {
            let dictionary = dictionary.clone();
            view.update(cx, |view, _| view.dictionary = dictionary);
        }
        self.voice.dictionaries.insert(project, dictionary);
    }

    /// Forgets which project's dictionary was read, so a new connection to
    /// `server` reads it again.
    pub(super) fn forget_server_dictionary(&mut self, server: ServerId) {
        if self
            .voice
            .dictionary_project
            .is_some_and(|project| self.state.project_server(project) == Some(server))
        {
            self.voice.dictionary_project = None;
        }
    }
    pub(crate) fn toggle_voice(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_prompt.is_some() {
            return;
        }
        if self.voice.view.is_some() {
            self.close_voice(cx);
            return;
        }
        if !self.require(Feature::Voice, cx) {
            return;
        }
        self.request_dictionary(cx);
        if let Some(view) = &self.composer.view {
            view.update(cx, crate::views::composer::Composer::toggle_voice);
            view.focus_handle(cx).focus(window);
            return;
        }
        let target = self.active_pane().and_then(|pane| {
            let view = self.grids.get(&pane)?.view.read(cx);
            let attachment = view.attachment()?;
            Some(Target {
                project: self.composer_key(),
                pane,
                attachment,
                generation: self.generation(attachment.0),
                modes: view.grid.as_ref()?.modes,
            })
        });
        let Some(target) = target else {
            self.set_banner_error(Some(
                tr!("Open a live terminal before starting dictation.").to_string(),
            ));
            cx.notify();
            return;
        };
        self.dismiss_overlay(cx);
        self.voice.closing = None;
        self.voice.exit = None;
        let vocabulary = self.dictionary().terms().to_vec();
        let view = cx.new(|cx| {
            VoicePanel::new(
                self.settings.composer.language.clone(),
                self.settings.composer.voice_model.clone(),
                vocabulary,
                self.settings.composer.voice_auto_send,
                self.theme.clone(),
                self.metrics,
                cx,
            )
        });
        self.voice.subscription = Some(cx.subscribe(&view, |model, _, event, cx| match event {
            VoiceEvent::Cancel => model.close_voice(cx),
            VoiceEvent::Finish(text) => model.finish_voice(text, cx),
        }));
        view.focus_handle(cx).focus(window);
        self.voice.view = Some(view);
        self.voice.target = Some(target);
        self.focus_requested = false;
        cx.notify();
    }

    pub(super) fn close_voice(&mut self, cx: &mut Context<Self>) {
        if let Some(view) = self.voice.view.take() {
            view.update(cx, VoicePanel::dismiss);
            self.voice.closing = Some(view);
            self.voice.exit = Some(cx.spawn(async move |model, cx| {
                cx.background_executor()
                    .timer(crate::views::voice::TRANSITION)
                    .await;
                let _ = model.update(cx, |model, cx| {
                    model.voice.closing = None;
                    cx.notify();
                });
            }));
        }
        self.voice.target = None;
        self.voice.subscription = None;
        self.focus_requested = true;
        cx.notify();
    }

    pub(super) fn sync_voice(&mut self, cx: &mut Context<Self>) {
        let Some(target) = &self.voice.target else {
            return;
        };
        if !self.voice_target_current(target, cx) || self.active_pane() != Some(target.pane) {
            self.close_voice(cx);
        }
    }

    fn voice_target_current(&self, target: &Target, cx: &gpui::App) -> bool {
        self.generation(target.attachment.0) == target.generation
            && self.composer_key() == target.project
            && self
                .grids
                .get(&target.pane)
                .is_some_and(|pane| pane.view.read(cx).attachment() == Some(target.attachment))
    }

    fn finish_voice(&mut self, text: &str, cx: &mut Context<Self>) {
        let Some(target) = self.voice.target.take() else {
            return;
        };
        if !self.voice_target_current(&target, cx) {
            self.close_voice(cx);
            return;
        }
        let text = self.dictionary().correct(text);
        let mut bytes = crate::views::terminal::clipboard::paste(&text, target.modes);
        if self.settings.composer.voice_auto_send {
            bytes.push(b'\r');
        }
        self.close_voice(cx);
        let (server, channel) = target.attachment;
        let Some(worker) = self
            .servers
            .get(server)
            .and_then(|runtime| runtime.work.clone())
        else {
            return;
        };
        cx.spawn(async move |model, cx| {
            if let Err(error) =
                super::composer::deliver(&worker, target.generation, channel, bytes).await
            {
                let _ = model.update(cx, |model, cx| {
                    model.set_banner_error(Some(
                        tr!("Dictation delivery failed: %@", &error).to_string(),
                    ));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(crate) fn voice_panel(&self) -> gpui::AnyElement {
        let Some(view) = self.voice.view.as_ref().or(self.voice.closing.as_ref()) else {
            return div().into_any_element();
        };
        div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(
                div()
                    .relative()
                    .child(view.clone())
                    .child(self.webview_occlusion()),
            )
            .into_any_element()
    }
}
