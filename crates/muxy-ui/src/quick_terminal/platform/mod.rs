#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod unsupported;

mod panel;
pub use panel::PanelAdapter;

use super::ShortcutRecordingEvent;
use super::shortcut_service::ShortcutBackendFactory;
use async_channel::Sender;

#[cfg(target_os = "linux")]
pub use linux::CAPABILITY;
#[cfg(target_os = "macos")]
pub use macos::CAPABILITY;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub use unsupported::CAPABILITY;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemMutation {
    Accessibility,
    KeyboardLayout,
    Screens,
}

#[cfg(target_os = "macos")]
pub type SystemObservers = macos::SystemObservers;
#[cfg(target_os = "linux")]
pub type SystemObservers = linux::SystemObservers;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub type SystemObservers = unsupported::SystemObservers;

#[cfg(target_os = "macos")]
pub type ShortcutRecorder = macos::ShortcutRecorder;
#[cfg(target_os = "linux")]
pub type ShortcutRecorder = linux::ShortcutRecorder;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub type ShortcutRecorder = unsupported::ShortcutRecorder;

#[cfg(target_os = "macos")]
pub fn factory(_cx: &gpui::App) -> Box<dyn ShortcutBackendFactory> {
    Box::new(macos::MacShortcutBackendFactory::new())
}

#[cfg(target_os = "linux")]
pub fn factory(cx: &gpui::App) -> Box<dyn ShortcutBackendFactory> {
    Box::new(linux::LinuxShortcutBackendFactory::new(
        cx.foreground_executor().clone(),
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn factory(_cx: &gpui::App) -> Box<dyn ShortcutBackendFactory> {
    Box::new(unsupported::UnsupportedShortcutBackendFactory)
}

pub fn canonicalize(
    shortcut: &muxy_core::quick_terminal::QuickTerminalShortcut,
) -> Option<muxy_core::quick_terminal::QuickTerminalShortcut> {
    #[cfg(target_os = "macos")]
    {
        shortcut.canonicalized(macos::resolve_key)
    }
    #[cfg(not(target_os = "macos"))]
    {
        shortcut.canonicalized(|_| shortcut.key_combo().map(|combo| combo.key.clone()))
    }
}

#[cfg(target_os = "macos")]
pub fn start_shortcut_recorder(
    sender: Sender<ShortcutRecordingEvent>,
) -> Result<ShortcutRecorder, String> {
    macos::ShortcutRecorder::start(sender)
}

#[cfg(not(target_os = "macos"))]
pub fn start_shortcut_recorder(
    sender: Sender<ShortcutRecordingEvent>,
) -> Result<ShortcutRecorder, String> {
    ShortcutRecorder::start(sender)
}

#[cfg(target_os = "macos")]
pub fn observe_system_mutations(sender: Sender<SystemMutation>) -> Result<SystemObservers, String> {
    macos::SystemObservers::start(sender)
}

#[cfg(target_os = "linux")]
pub fn observe_system_mutations(
    _sender: Sender<SystemMutation>,
) -> Result<SystemObservers, String> {
    Ok(linux::SystemObservers)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn observe_system_mutations(
    _sender: Sender<SystemMutation>,
) -> Result<SystemObservers, String> {
    Ok(unsupported::SystemObservers)
}

pub fn capability(window: &gpui::Window) -> muxy_core::capability::Capability {
    #[cfg(target_os = "linux")]
    {
        linux::capability(window)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = window;
        CAPABILITY
    }
}

pub(super) fn record(recorder: &ShortcutRecorder, keystroke: &gpui::Keystroke) -> bool {
    #[cfg(target_os = "linux")]
    {
        recorder.record(keystroke);
        true
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (recorder, keystroke);
        false
    }
}

pub fn accessibility_preferences() -> super::panel::AccessibilityPreferences {
    #[cfg(target_os = "macos")]
    {
        macos::accessibility_preferences()
    }
    #[cfg(not(target_os = "macos"))]
    {
        super::panel::AccessibilityPreferences::default()
    }
}
