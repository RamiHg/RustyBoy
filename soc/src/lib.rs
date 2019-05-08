#![allow(unused_imports)]
#![allow(unused_doc_comments)]
#![allow(dead_code)]
#![allow(unused_variables)]

#![deny(clippy::all)]

#![feature(trait_alias)]

#[macro_use]
pub mod io_registers;

pub mod cart;
mod cpu;
mod dma;
pub mod error;
pub mod gpu;
pub mod log;
mod mmu;
mod serial;
pub mod system;
pub mod timer;
mod util;

#[cfg(test)]
mod test;

#[macro_use]
extern crate log as logging;
#[macro_use]
extern crate more_asserts;
#[macro_use]
extern crate shrinkwraprs;
