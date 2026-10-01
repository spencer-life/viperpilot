//! Protocol, state, planning, storage, and device-control core for the Viper V4 Pro.
//!
//! The core modules are platform-independent. Windows HID and observer support is
//! compiled only for the Windows target so packet and safety behavior can be
//! tested in WSL without access to the mouse.

pub mod capability;
#[cfg(feature = "egui-preview")]
pub mod draft_editor;
pub mod engine;
pub mod gui_logic;
pub mod model;
pub mod planning;
pub mod profile_intent;
pub mod profile_library;
pub mod protocol;
pub mod storage;
#[cfg(any(windows, test))]
mod tray_logic;

#[cfg(windows)]
pub mod tray;
#[cfg(windows)]
pub mod windows;
