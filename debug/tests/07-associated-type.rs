// This test case covers one more heuristic that is often worth incorporating
// into derive macros that infer trait bounds. Here we look for the use of an
// associated type of a type parameter.
//
// The generated impl will need to look like:
//
//     impl<T: Trait> Debug for Field<T>
//     where
//         T::Value: Debug,
//     {...}
//
// You can identify associated types as any syn::TypePath in which the first
// path segment is one of the type parameters and there is more than one
// segment.
//
//
// Resources:
//
//   - The relevant types in the input will be represented in this syntax tree
//     node: https://docs.rs/syn/1.0/syn/struct.TypePath.html

use derive_debug::CustomDebug;
use std::fmt::{self, Debug};

pub trait Trait {
    type Value;
}

//我要自动推导，如果T实现了Debug，我也要自动实现Debug。或者只有当T实现了Debug的时候，我才能实现为Field实现Debug
#[derive(CustomDebug)]
struct A;

pub struct Field<T: Trait> {
    values: Vec<T::Value>,
}

impl <T: Trait> Debug for Field<T> where T::Value: Debug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field").field("values", &self.values).finish()
    }
}

fn assert_debug<F: Debug>() {}

fn main() {
    // Does not implement Debug, but its associated type does.
    struct Id;

    impl Trait for Id {
        type Value = u8;
    }

    assert_debug::<Field<Id>>();
}
