// Bitfield enums with any discriminant (implicit or explicit) outside of the
// range 0..2^BITS should fail to compile.

use bitfield::*;

const F: isize = 1;

#[derive(BitfieldSpecifier)]
pub enum DeliveryMode {
    Fixed = F,
    Lowest,
    SMI,
    RemoteRead,
    NMI,
    Init,
    Startup,
    External,
}

fn main() {
    let a = 8usize;
    println!("{}", 3f32.log2());
    println!("{}", DeliveryMode::External as u8);
    println!("{}", DeliveryMode::External as u8);
}
