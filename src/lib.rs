//! Tray indicators for Windows: Bluetooth battery levels, CPU temperature and CPU package power.

mod app;
mod autostart_request;
mod render;
mod run;
mod sensors;
mod tray;
mod win;

pub use run::run;
