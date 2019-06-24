use enum_utils::IterVariants;

#[derive(Debug, IterVariants, PartialEq, Eq)]
#[repr(u32)]
enum LargeDiscriminant {
    MaxMinusOne = 0xffff_fffe,
    Max = 0xffff_ffff,
}

#[test]
fn large_discriminant() {
    use self::LargeDiscriminant::*;

    assert_eq!(vec![MaxMinusOne, Max],
               LargeDiscriminant::iter().collect::<Vec<_>>());
}

#[derive(Debug, Clone, IterVariants, PartialEq, Eq)]
#[repr(u8)]
enum Zst {
    Singleton,
}

#[test]
fn zst() {
    assert_eq!(vec![Zst::Singleton],
               Zst::iter().collect::<Vec<_>>());
}

#[derive(Debug, IterVariants, PartialEq, Eq)]
enum Empty {}

#[test]
fn empty() {
    assert_eq!(Vec::<Empty>::new(),
               Empty::iter().collect::<Vec<Empty>>());
}
