#![no_std]
//#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(uncommon_codepoints)]
//#![feature(generic_const_exprs)]
//#![feature(const_fn_floating_point_arithmetic)]
//#![feature(associated_const_equality)]

pub mod protocol;

#[cfg(feature = "std")]
pub mod std;

#[cfg(feature = "ch32")]
mod ch32;
#[cfg(feature = "ch32")]
pub use ch32::*;

pub mod arduino;

pub mod common;

mod traits;
pub use traits::*;

mod impl_traits;

pub mod prelude;
