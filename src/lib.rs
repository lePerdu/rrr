#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
mod std {
    pub use ::alloc::*;
    pub use ::core::*;
}

#[cfg(feature = "std")]
mod std {
    pub use std::*;
}

extern crate num_traits;

// TODO Make modules public, or re-export everything?

mod basic;
mod event;
mod operators;
mod sf;
mod task;

pub use basic::*;
pub use event::*;
pub use operators::*;
pub use sf::*;
pub use task::*;
