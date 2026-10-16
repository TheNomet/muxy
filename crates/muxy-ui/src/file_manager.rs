//! The desktop's file manager, asked over the session bus.

use std::io;
use std::path::Path;

const SERVICE: &str = "org.freedesktop.FileManager1";

/// Opens the folder holding `file`, with the file selected. Fails when no
/// file manager on this desktop offers that.
pub fn select(file: &Path) -> io::Result<()> {
    let uri = ashpd::url::Url::from_file_path(file).map_err(|()| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "a file manager shows absolute paths only",
        )
    })?;
    crate::dbus::session()
        .and_then(|connection| {
            connection.call_method(
                Some(SERVICE),
                "/org/freedesktop/FileManager1",
                Some(SERVICE),
                "ShowItems",
                &(vec![uri.as_str()], ""),
            )
        })
        .map(drop)
        .map_err(io::Error::other)
}
