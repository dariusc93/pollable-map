#![no_main]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
pub mod futures;

#[cfg(feature = "alloc")]
pub mod stream;

pub(crate) mod common;
pub mod optional;
