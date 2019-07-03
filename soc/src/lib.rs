#![allow(unused_doc_comments)]
#![allow(unused_variables)]
#![warn(clippy::all)]
// #![deny(warnings)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::cast_lossless)]

#[macro_use]
mod io_registers;

// TODO: Fix the public API. Don't expose so many internals.
pub mod cart;
pub mod error;
pub mod gpu;
pub mod joypad;
pub mod log;
pub mod system;

#[cfg(feature = "audio")]
mod apu;

mod cpu;
mod dma;
mod mmu;
mod serial;
mod timer;
mod util;

#[cfg(test)]
mod test;

#[macro_use]
extern crate log as logging;
#[macro_use]
extern crate more_asserts;
#[macro_use]
extern crate serde;
