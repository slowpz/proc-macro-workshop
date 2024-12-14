pub unsafe trait IsJustAErrorMessage {
    
}

pub unsafe  trait DoNotImpThisTrait: IsJustAErrorMessage {
    
}

pub trait TotalSizeIsMultipleOfEightBits: DoNotImpThisTrait {
    
}

pub trait SizeMustBeMultipleOfEightBits: TotalSizeIsMultipleOfEightBits {
    
}