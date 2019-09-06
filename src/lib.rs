//! A set of procedural macros for deriving useful functionality on enums.

extern crate proc_macro;

#[macro_use]
mod attr;
mod iter;
mod from_str;
mod conv;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

fn unwrap_errors<T>(res: Result<T, attr::ErrorList>) -> T {
    match res {
        Ok(x) => x,
        Err(list) => {
            // TODO: print error spans with proc_macro_diagnostic
            let desc: String = list.iter()
                .map(|e| format!("\n{}", e))
                .collect();

            panic!("enum_utils encountered one or more errors:{}", desc);
        }
    }
}

/// Derives [`FromStr`] for C-like enums.
///
/// The generated code will be more efficient than a simple `match` statement for most enums. It is
/// guaranteed to run in `O(m)` time (where `m` is the length of the input string) rather than the
/// `O(mn)` time (where `n` is the number of variants) which would be required by the naive
/// approach.
///
/// # Examples
///
/// [`FromStr`] can be derived for C-like enums by deriving `enum_utils::FromStr`.
///
/// ```
/// #[derive(Debug, PartialEq, enum_utils::FromStr)]
/// enum Test {
///     Alpha,
///     Beta,
/// }
///
/// assert_eq!("Alpha".parse(), Ok(Test::Alpha));
/// assert_eq!("Beta".parse(), Ok(Test::Beta));
/// ```
///
/// # Attributes
///
/// The implementation can be customized by attributes of the form `#[enumeration(...)]`. These
/// are based on the ones in [`serde`].
///
/// ## `#[enumeration(skip)]`
///
/// This attribute causes a single variant of the enum to be ignored when deserializing.
/// Variants which are skipped may have data fields.
///
/// ```
/// #[derive(Debug, PartialEq, enum_utils::FromStr)]
/// enum Skip {
///     #[enumeration(skip)]
///     Alpha(usize),
///     Beta,
/// }
///
/// assert_eq!("Alpha".parse::<Skip>(), Err(()));
/// assert_eq!("Beta".parse(), Ok(Skip::Beta));
/// ```
///
/// ## `#[enumeration(rename = "...")]`
///
/// This attribute renames a single variant of an enum. This replaces the name of the variant and
/// overrides [`rename_all`].
///
/// Only one [`rename`] attribute can appear for each enum variant.
///
/// ```
/// #[derive(Debug, PartialEq, enum_utils::FromStr)]
/// enum Rename {
///     #[enumeration(rename = "α")]
///     Alpha,
///     Beta,
/// }
///
/// assert_eq!("Alpha".parse::<Rename>(), Err(()));
/// assert_eq!("α".parse(), Ok(Rename::Alpha));
/// assert_eq!("Beta".parse(), Ok(Rename::Beta));
/// ```
///
/// ## `#[enumeration(alias = "...")]`
///
/// This attribute is similar to [`rename`], but it does not replace the name of the variant.
///
/// Unlike [`rename`], there is no limit to the number of `alias` attributes which can be applied.
/// This allows multiple strings to serialize to the same variant.
///
/// ```
/// #[derive(Debug, PartialEq, enum_utils::FromStr)]
/// enum Alias {
///     #[enumeration(alias = "A", alias = "α")]
///     Alpha,
///     Beta,
/// }
///
/// assert_eq!("Alpha".parse(), Ok(Alias::Alpha));
/// assert_eq!("A".parse(), Ok(Alias::Alpha));
/// assert_eq!("α".parse(), Ok(Alias::Alpha));
/// assert_eq!("Beta".parse(), Ok(Alias::Beta));
/// ```
///
/// ## `#[enumeration(rename_all = "...")]`
///
/// This attribute can be applied to an entire enum, and causes all fields to be renamed according
/// to the given [rename rule]. All rename rules defined in [`serde`] are supported.
///
/// ```
/// #[enumeration(rename_all = "snake_case")]
/// #[derive(Debug, PartialEq, enum_utils::FromStr)]
/// enum RenameAll {
///     FooBar,
///     BarFoo,
/// }
///
/// assert_eq!("foo_bar".parse(), Ok(RenameAll::FooBar));
/// assert_eq!("bar_foo".parse(), Ok(RenameAll::BarFoo));
/// ```
///
/// ## `#[enumeration(case_insensitive)]`
///
/// This attribute can be applied to an entire enum, it causes all variants to be parsed
/// case-insensitively.
///
/// ```
/// #[enumeration(case_insensitive)]
/// #[derive(Debug, PartialEq, enum_utils::FromStr)]
/// enum NoCase {
///     Alpha,
///     Beta,
/// }
///
/// assert_eq!("ALPHA".parse(), Ok(NoCase::Alpha));
/// assert_eq!("beta".parse(), Ok(NoCase::Beta));
/// ```
///
/// [`FromStr`]: https://doc.rust-lang.org/std/str/trait.FromStr.html
/// [`serde`]: https://serde.rs/attributes.html
/// [`rename`]: #enumerationrename--
/// [`rename_all`]: #enumerationrename_all--
/// [rename rule]: https://serde.rs/container-attrs.html#rename_all
#[proc_macro_derive(FromStr, attributes(enumeration))]
pub fn from_str_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    unwrap_errors(from_str::derive(&ast)).into()
}

/// Derives a static method, `iter()`, which iterates over the variants of an enum.
///
/// # Examples
///
/// Variants are yielded in the order they are defined. If the discriminants of the enum form a
/// single, increasing run and `#[repr(...)]` is specified as in the following example, a fast
/// implementation of `iter` can be generated which does not require the enum to implement `Clone`.
///
/// ```
/// /// The discriminants of this enum are `[1, 2, 3, 4]`.
/// #[derive(Debug, PartialEq, Eq, enum_utils::IterVariants)]
/// #[repr(u8)]
/// pub enum Direction {
///     North = 1,
///     East,
///     South,
///     West,
/// }
///
/// use Direction::*;
/// assert_eq!(Direction::iter().collect::<Vec<_>>(), vec![North, East, South, West]);
/// ```
///
/// If the preceding conditions are not met, `Clone` must be implemented to successfully derive
/// `IterVariants`. `enum_utils` will create a `const` array containing each variant and iterate
/// over that.
///
/// ```
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, enum_utils::IterVariants)]
/// #[repr(u16)]
/// pub enum Bitmask {
///     Empty = 0x0000,
///     Full = 0xffff,
/// }
///
/// use Bitmask::*;
/// assert_eq!(Bitmask::iter().collect::<Vec<_>>(), vec![Empty, Full]);
/// ```
///
/// Named constants or complex expressions (beyond an integer literal) are not evaluated when used
/// as a discriminant and will cause `IterVariants` to default to the `Clone`-based implementation.
///
/// ```compile_fail
/// #[derive(Debug, PartialEq, Eq, enum_utils::IterVariants)] // Missing `Clone` impl
/// #[repr(u8)]
/// pub enum Bitmask {
///     Bit1 = 1 << 0,
///     Bit2 = 1 << 1,
/// }
/// ```
///
/// # Attributes
///
/// ## `#[enumeration(skip)]`
///
/// Use `#[enumeration(skip)]` to avoid iterating over a variant. This can be useful when an enum
/// contains a "catch-all" variant.
///
/// ```
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, enum_utils::IterVariants)]
/// pub enum Http2FrameType {
///     Data,
///     Headers,
///
///     /* ... */
///
///     Continuation,
///
///     #[enumeration(skip)]
///     Unknown(u8),
/// }
///
/// use Http2FrameType::*;
/// assert_eq!(Http2FrameType::iter().collect::<Vec<_>>(),
///            vec![Data, Headers, /* ... */ Continuation]);
/// ```
#[proc_macro_derive(IterVariants, attributes(enumeration))]
pub fn iter_variants_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    unwrap_errors(iter::derive(&ast)).into()
}

/// Derives [`TryFrom<Repr>`] for an enum, where `Repr` is a [primitive representation] specified
/// in `#[repr(...)]`.
///
/// [`TryFrom<Repr>`]: https://doc.rust-lang.org/std/convert/trait.TryFrom.html
/// [primitive representation]: https://doc.rust-lang.org/reference/type-layout.html#primitive-representations
///
/// # Examples
///
/// ```
/// use std::convert::{TryFrom, TryInto};
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, enum_utils::TryFromRepr)]
/// #[repr(u8)]
/// pub enum Direction {
///     North = 1,
///     East,
///     South,
///     West
/// }
///
/// use Direction::*;
/// assert_eq!(North, 1u8.try_into().unwrap());
/// assert_eq!(West,  4u8.try_into().unwrap());
/// assert_eq!(Err(()), Direction::try_from(0u8));
/// assert_eq!(Err(()), Direction::try_from(5u8));
/// ```
#[proc_macro_derive(TryFromRepr, attributes(enumeration))]
pub fn try_from_repr_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    unwrap_errors(conv::derive_try_from_repr(&ast)).into()
}

/// Derives [`From<Enum>`] for the [primitive representation] specified in `#[repr(...)]`.
///
/// [`From<Enum>`]: https://doc.rust-lang.org/std/convert/trait.From.html
/// [primitive representation]: https://doc.rust-lang.org/reference/type-layout.html#primitive-representations
///
/// # Examples
///
/// ```
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, enum_utils::ReprFrom)]
/// #[repr(u8)]
/// pub enum Direction {
///     North = 1,
///     East,
///     South,
///     West
/// }
///
/// use Direction::*;
/// assert_eq!(1u8, North.into());
/// assert_eq!(4u8, West.into());
/// ```
#[proc_macro_derive(ReprFrom, attributes(enumeration))]
pub fn repr_from_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    unwrap_errors(conv::derive_repr_from(&ast)).into()
}
