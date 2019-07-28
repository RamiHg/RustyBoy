#![allow(unused_doc_comments)]
#![warn(clippy::all)]
// #![deny(warnings)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::cast_lossless)]
// #![cfg_attr(feature = "strict_assert", allow(unreachable_code))]

#[macro_use]
mod io_registers;

#[macro_use]
mod util;

// TODO: Fix the public API. Don't expose so many internals.
pub mod cart;
pub mod cpu;
pub mod error;
pub mod gpu;
pub mod joypad;
pub mod log;
pub mod system;

#[cfg(feature = "audio")]
mod apu;

mod dma;
mod mmu;
mod serial;
mod timer;

#[cfg(test)]
mod test;

#[macro_use]
extern crate log as logging;
#[macro_use]
extern crate more_asserts;
#[macro_use]
extern crate serde;
