use std::sync::{
    Arc, Mutex, PoisonError,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as backend;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as backend;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod unsupported;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use unsupported as backend;

pub use backend::{CAPABILITY, LOCAL_MODEL, languages};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Language {
    pub id: String,
    pub name: String,
    pub preferred: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Phase {
    #[default]
    Starting,
    Recording,
    Paused,
    Transcribing,
    Finished,
    Failed,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub phase: Phase,
    pub transcript: String,
    pub elapsed: Duration,
    pub level: f32,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
struct Shared {
    snapshot: Mutex<Snapshot>,
    cancelled: AtomicBool,
    paused: AtomicBool,
    finishing: AtomicBool,
    #[cfg(target_os = "macos")]
    device_changed: AtomicBool,
}

#[derive(Debug)]
pub struct Recorder {
    shared: Arc<Shared>,
}

impl Recorder {
    pub fn start(language: String, model: String) -> Result<Self, String> {
        backend::start(language, model)
    }

    pub fn snapshot(&self) -> Snapshot {
        self.shared
            .snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn pause(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Release);
    }

    pub fn finish(&self) {
        self.shared.finishing.store(true, Ordering::Release);
    }

    pub fn finished(&self) -> Option<String> {
        if !self.shared.finishing.load(Ordering::Acquire) {
            return None;
        }
        let snapshot = self
            .shared
            .snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        (snapshot.phase == Phase::Finished).then(|| snapshot.transcript.trim().to_owned())
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.shared.cancelled.store(true, Ordering::Release);
    }
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Debug)]
pub struct SimulatedRecorder(Arc<Shared>);

#[cfg(any(test, feature = "test-support"))]
impl SimulatedRecorder {
    pub fn new(snapshot: Snapshot) -> (Recorder, Self) {
        let shared = Arc::new(Shared {
            snapshot: Mutex::new(snapshot),
            ..Shared::default()
        });
        (
            Recorder {
                shared: shared.clone(),
            },
            Self(shared),
        )
    }

    pub fn cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
    pub fn paused(&self) -> bool {
        self.0.paused.load(Ordering::Acquire)
    }
    pub fn finishing(&self) -> bool {
        self.0.finishing.load(Ordering::Acquire)
    }
    pub fn update(&self, snapshot: Snapshot) {
        *self
            .0
            .snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = snapshot;
    }
}
