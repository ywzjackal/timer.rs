#![allow(dead_code)]
extern crate libc;
extern crate event;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;