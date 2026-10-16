use super::{Release, Result};
use std::path::Path;

pub(crate) const CAPABILITY: muxy_core::capability::Capability =
    muxy_core::capability::Capability::Unsupported;

#[derive(Clone, Debug)]
pub(crate) enum Installation {}

#[derive(Clone, Debug)]
pub(crate) struct PreparedUpdate {
    pub(crate) version: String,
    pub(crate) build: Option<muxy_protocol::BuildInfo>,
    installation: Installation,
}

fn unavailable<T>() -> Result<T> {
    Err(muxy_core::capability::Feature::Updates
        .unsupported_reason()
        .into())
}

impl Installation {
    pub(crate) fn detect() -> Result<Self> {
        unavailable()
    }
    pub(crate) fn latest() -> Result<Option<Release>> {
        unavailable()
    }
    pub(crate) fn prepare(&self, _release: Release) -> Result<PreparedUpdate> {
        match *self {}
    }
    pub(crate) fn restore(&self, _path: &Path) -> Result<Option<(PreparedUpdate, bool)>> {
        match *self {}
    }
    pub(crate) fn cleanup(&self, _socket: &Path) -> Result<()> {
        match *self {}
    }
}

impl PreparedUpdate {
    pub(crate) fn discard(&self, _socket: &Path) -> Result<()> {
        match self.installation {}
    }
    pub(crate) fn compatible_with(&self, server: &muxy_protocol::ServerInfo) -> bool {
        self.build
            .as_ref()
            .is_some_and(|build| build.shares_protocol_with(&server.build))
    }
    pub(crate) fn validate(&self) -> Result<()> {
        match self.installation {}
    }
    pub(crate) fn persist(&self, _path: &Path, _scheduled: bool) -> Result<()> {
        match self.installation {}
    }
    pub(crate) fn install(
        &self,
        _lock: muxy_core::file_lock::FileLock,
        _server: &muxy_protocol::ServerInfo,
    ) -> Result<()> {
        match self.installation {}
    }
}
