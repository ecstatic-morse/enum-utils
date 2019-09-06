# enum-utils

[![crates.io](https://img.shields.io/crates/v/enum-utils.svg)](https://crates.io/crates/enum-utils)
[![docs.rs](https://docs.rs/enum-utils/badge.svg)](https://docs.rs/enum-utils)
[![Build Status](https://dev.azure.com/ecstaticmorse/enum-utils/_apis/build/status/ecstatic-morse.enum-utils?branchName=master)](https://dev.azure.com/ecstaticmorse/enum-utils/_build/latest?definitionId=1&branchName=master)


A set of procedural macros for deriving useful functionality on enums.

See [the API docs] for more information.

[the API docs]: https://docs.rs/enum-utils

## [`FromStr`]

An efficient, configurable [`FromStr`][from-str-std] implementation for C-like enums.

[`FromStr`]: https://docs.rs/enum-utils/0.1.1/enum_utils/derive.FromStr.html
[from-str-std]: https://doc.rust-lang.org/std/str/trait.FromStr.html

```rust
#[derive(Debug, PartialEq, enum_utils::FromStr)]
enum Test {
    Alpha,
    Beta,
}

assert_eq!("Alpha".parse(), Ok(Test::Alpha));
assert_eq!("Beta".parse(), Ok(Test::Beta));
```

## [`IterVariants`]

A static method returning an iterator over the variants of an enum.

[`IterVariants`]: https://docs.rs/enum-utils/0.1.1/enum_utils/derive.IterVariants.html

```rust
#[derive(Debug, PartialEq, Eq, enum_utils::IterVariants)]
#[repr(u8)]
pub enum Direction {
    North = 1,
    East,
    South,
    West,
}

use Direction::*;
assert_eq!(Direction::iter().collect::<Vec<_>>(), vec![North, East, South, West]);
```

## [`TryFromRepr`] and [`ReprFrom`]

Conversion to and from the discriminant of a C-like enum.

[`ReprFrom`]: https://docs.rs/enum-utils/0.1.1/enum_utils/derive.ReprFrom.html
[`TryFromRepr`]: https://docs.rs/enum-utils/0.1.1/enum_utils/derive.TryFromRepr.html

```rust
use std::convert::TryInto;

#[derive(Debug, Clone, Copy, PartialEq, Eq, enum_utils::ReprFrom, enum_utils::TryFromRepr)]
#[repr(u8)]
pub enum Direction {
    North = 1,
    East,
    South,
    West
}

use Direction::*;
assert_eq!(1u8, North.into());
assert_eq!(4u8, West.into());
assert_eq!(North, 1u8.try_into().unwrap());
assert_eq!(West,  4u8.try_into().unwrap());
```
