# enum-utils

A set of procedural macros for deriving useful functionality on enums.

See [the API docs] for more information.

[the API docs]: https://docs.rs/enum-utils

## `FromStr`

An efficient, configurable [`FromStr`] implementation for C-like enums.

[`FromStr`]: https://doc.rust-lang.org/std/str/trait.FromStr.html

```rust
#[derive(Debug, PartialEq, enum_utils::FromStr)]
enum Test {
    Alpha,
    Beta,
}

assert_eq!("Alpha".parse(), Ok(Test::Alpha));
assert_eq!("Beta".parse(), Ok(Test::Beta));
```

## `IterVariants`

A static method returning an iterator over the variants of an enum.

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

## `TryFromRepr` and `ReprFrom`

Conversion to and from the discriminant of a C-like enum.


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
