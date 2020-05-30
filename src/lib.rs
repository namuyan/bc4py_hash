#![feature(link_args)]
extern crate num_cpus;

// proof of capacity
#[cfg(feature = "poc")]
mod poc;
#[cfg(feature = "poc")]
pub use poc::*;

// general hash functions
#[cfg(feature = "hashs")]
mod hashs;
#[cfg(feature = "hashs")]
pub use hashs::*;
