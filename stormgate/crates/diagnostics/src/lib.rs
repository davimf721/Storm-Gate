//! Diagnostics: `doctor`, per-launch log sessions and sanitized bundles.

pub mod doctor;
pub mod logs;
pub mod redact;
pub mod system;

pub use doctor::{run_doctor, Check, CheckStatus, DoctorReport, Section};
pub use logs::{bundle, last_session, LogSession};
pub use redact::Redactor;
pub use system::{HostProbe, SystemInfo, SystemProbe};
