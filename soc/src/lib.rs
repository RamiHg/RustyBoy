#![allow(unused_imports)]
#![allow(unused_doc_comments)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![deny(clippy::all)]
#![feature(trait_alias)]

#[macro_use]
mod io_registers;

// TODO: Fix the public API. Don't expose so many internals.
pub mod cart;
pub mod error;
pub mod gpu;
pub mod log;
pub mod system;

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
extern crate shrinkwraprs;
