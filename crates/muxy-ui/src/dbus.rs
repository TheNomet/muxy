//! The desktop's session bus, which the Linux backends reach its services through.

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use zbus::blocking::{Connection, connection::Builder, fdo::DBusProxy};

/// The session bus, connected on first use. A failed attempt is tried again
/// on the next call.
pub(crate) fn session() -> zbus::Result<Connection> {
    static SESSION: Mutex<Option<Connection>> = Mutex::new(None);
    let mut session = SESSION.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(connection) = session.as_ref() {
        return Ok(connection.clone());
    }
    let connection = Builder::session()?
        .method_timeout(Duration::from_secs(10))
        .build()?;
    *session = Some(connection.clone());
    Ok(connection)
}

/// Whether a service owns `name`, or the bus can start one for it.
pub(crate) fn offers(connection: &Connection, name: &str) -> bool {
    let Ok(bus) = DBusProxy::new(connection) else {
        return false;
    };
    let owned = zbus::names::BusName::try_from(name)
        .ok()
        .and_then(|name| bus.name_has_owner(name).ok())
        .unwrap_or(false);
    owned
        || bus
            .list_activatable_names()
            .is_ok_and(|names| names.iter().any(|activatable| activatable.as_str() == name))
}
