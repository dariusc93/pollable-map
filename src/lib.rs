#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
pub mod futures;

#[cfg(feature = "alloc")]
pub mod stream;

#[cfg(any(feature = "alloc", feature = "timeout"))]
pub(crate) mod common;

pub mod optional;

pub mod error;
