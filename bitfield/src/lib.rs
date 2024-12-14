// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
use bitfield_impl::specifiers;
pub use bitfield_impl::{bitfield, BitfieldSpecifier};

// TODO other things

pub trait Specifier {
    const BITS: usize;
    type T;

    fn get(data: &[u8], bit_offset: usize) -> Self::T;

    fn set(data: &mut [u8], bit_offset: usize, val: Self::T);
}

impl Specifier for bool {
    const BITS: usize = 1;

    type T = bool;

    fn get(data: &[u8], bit_offset: usize) -> Self::T {
        let idx = bit_offset >> 3;
        data[idx] & 1u8.rotate_left(bit_offset as u32) != 0
    }

    fn set(data: &mut [u8], bit_offset: usize, val: Self::T) {
        let idx = bit_offset >> 3;
        if val {
            data[idx] |= 1u8.rotate_left(bit_offset as u32);
        } else {
            data[idx] &= !(1u8.rotate_left(bit_offset as u32));
        }
    }
}

specifiers!(1..=128);
