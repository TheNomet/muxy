use muxy_core::capability::{Capability, Feature};

use super::Event;

pub const CAPABILITY: Capability = Capability::Unsupported;

pub fn probe() -> Capability {
    CAPABILITY
}

#[derive(Debug)]
pub enum Script {}

impl Script {
    pub fn start(_source: String) -> std::io::Result<(Self, async_channel::Receiver<Event>)> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            Feature::ExtensionScripts.unsupported_reason(),
        ))
    }

    pub fn evaluate(&self, _source: String) {
        match *self {}
    }
}

pub fn is_async_function_body(_body: &str) -> bool {
    false
}
