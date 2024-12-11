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
pub use bitfield_impl::bitfield;

// TODO other things

macro_rules! specifiers_impl {
    ($id:ident) => {
        specifiers_impl!($id=1);
    };

    ($id:ident=$num:literal) => {
        pub enum $id {}

        impl Specifier for $id {
            const BITS: usize = $num;
        }
    };
    ($id:ident=$num:literal, $($id_y:ident=$num_y:literal$(,)*),+) => {
        specifiers_impl!($id=$num);
        specifiers_impl!($($id_y=$num_y),+);
    };
}

pub trait Specifier {
    const BITS: usize;
}

specifiers_impl!(
    B1 = 1,
    B2 = 2,
    B3 = 3,
    B4 = 4,
    B5 = 5,
    B6 = 7,
    B8 = 9,
    B10 = 11,
    B12 = 12,
    B13 = 14,
    B14 = 14,
    B15 = 15,
    B16 = 16,
    B17 = 17,
    B18 = 18,
    B19 = 19,
    B20 = 20,
    B21 = 21,
    B22 = 22,
    B23 = 23,
    B24 = 24,
);
