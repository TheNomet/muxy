pub mod panel;
pub mod platform;
pub mod shortcut_service;
pub mod view;
use async_channel::Receiver;
use muxy_core::quick_terminal::keys::KeyCombo;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortcutCapture {
    pub combo: KeyCombo,
    pub virtual_key_code: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortcutRecordingEvent {
    Captured(ShortcutCapture),
    Cancelled,
    Rejected(String),
}

#[derive(Debug)]
pub struct ShortcutRecording {
    native: platform::ShortcutRecorder,
}

impl ShortcutRecording {
    pub fn record(&self, keystroke: &gpui::Keystroke) -> bool {
        platform::record(&self.native, keystroke)
    }
}

#[derive(Debug)]
pub enum ShortcutRecordingAction {
    Ignore,
    Capture(ShortcutCapture),
    Cancel,
    Reject(String),
}

pub fn shortcut_recording_action(
    active_generation: u64,
    event_generation: u64,
    event: ShortcutRecordingEvent,
) -> ShortcutRecordingAction {
    if active_generation != event_generation {
        return ShortcutRecordingAction::Ignore;
    }
    match event {
        ShortcutRecordingEvent::Captured(capture) => ShortcutRecordingAction::Capture(capture),
        ShortcutRecordingEvent::Cancelled => ShortcutRecordingAction::Cancel,
        ShortcutRecordingEvent::Rejected(error) => ShortcutRecordingAction::Reject(error),
    }
}

pub fn start_shortcut_recording()
-> Result<(ShortcutRecording, Receiver<ShortcutRecordingEvent>), String> {
    let (sender, receiver) = async_channel::bounded(1);
    let native = platform::start_shortcut_recorder(sender)?;
    Ok((ShortcutRecording { native }, receiver))
}
