//! Controller discovery.
//!
//! Wine's macOS driver (winemac.drv) and its HID/SDL backends do the actual
//! XInput/DirectInput translation. This crate only reports what the host
//! sees so users can attach it to bug reports (`stormgate input list`).
//!
//! The macOS implementation will use GameController.framework through a
//! small Objective-C bridge (tracked in docs/roadmap.md, issue "Input
//! diagnostics"). Until then [`list_controllers`] reports `Unsupported`
//! instead of guessing.

use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Connection {
    Usb,
    Bluetooth,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Controller {
    pub index: usize,
    pub vendor: String,
    pub model: String,
    pub connection: Connection,
    /// Whether Wine exposes it through XInput.
    pub xinput: bool,
}

impl fmt::Display for Controller {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Controller {}", self.index)?;
        writeln!(f, "  Vendor: {}", self.vendor)?;
        writeln!(f, "  Model: {}", self.model)?;
        writeln!(f, "  Connection: {:?}", self.connection)?;
        write!(
            f,
            "  XInput mapping: {}",
            if self.xinput { "active" } else { "inactive" }
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("controller discovery is not implemented yet on this platform ({0})")]
    Unsupported(&'static str),
}

/// Lists connected controllers.
pub fn list_controllers() -> Result<Vec<Controller>, InputError> {
    Err(InputError::Unsupported(std::env::consts::OS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_documented_format() {
        let c = Controller {
            index: 0,
            vendor: "Sony".into(),
            model: "DualSense".into(),
            connection: Connection::Bluetooth,
            xinput: true,
        };
        assert_eq!(
            c.to_string(),
            "Controller 0\n  Vendor: Sony\n  Model: DualSense\n  Connection: Bluetooth\n  XInput mapping: active"
        );
        assert!(list_controllers().is_err());
    }
}
