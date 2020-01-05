#![warn(clippy::all)]

pub mod micro_code;
pub mod register;

#[cfg(feature = "build")]
pub mod csv_loader;
#[cfg(feature = "build")]
pub mod pla;

#[cfg(feature = "build")]
mod asm;
#[cfg(feature = "build")]
mod compiler;
#[cfg(feature = "build")]
mod op_map;
#[cfg(feature = "build")]
mod parser;
