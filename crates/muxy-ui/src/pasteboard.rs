use std::path::PathBuf;
#[derive(Debug)]
pub enum Content {
    Files(Vec<PathBuf>),
    Image(Vec<u8>),
    Text,
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod portable;
#[cfg(target_os = "macos")]
pub use macos::{IMAGE_SUBMISSION, Lease, read_content};
#[cfg(not(target_os = "macos"))]
pub use portable::{IMAGE_SUBMISSION, Lease, read_content};
