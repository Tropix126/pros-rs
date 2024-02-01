//! Helpers for terminal I/O functionality.

pub mod print_impl;

pub use no_std_io::io::*;

pub use crate::{print, println, eprint, eprintln, dbg};
