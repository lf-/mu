#![no_std]
/*!
This crate is themed around bringing type safety to integers as commonly used in
FFI or other low level constructs.

# Enums with integers
We implement two macros that allow you to declare an enum with integer
options, and automatically implement [`From`] or [`TryFrom`] for them.

Two options are provided, [`int_enum!`] and [`int_enum_only!`]

[`int_enum!`] is intended for storing values that can be anything in a `uN`
integer, and its [`From`] opportunistically turns inputs into actual enum
variants if they match, otherwise generating an `Other(uN)`.

[`int_enum_only!`] is intended for cases where there are a limited number of
variants and it is unexpected that an input is found to not match. It implements
[`TryFrom`], failing if a variant is not matched.

This is useful for doing FFI and other low-level work where it is desirable
to make some integer value from FFI into a type-safe enumeration to leverage
exhaustivity checking, for example.

This library is `no_std`.

[`From`]: [core::convert::From]
[`TryFrom`]: [core::convert::TryFrom]
*/

/**
Creates an enum which has all of the given variants and an Other variant.

Automatically generates [From](core::convert::From) implementations for
converting between the inner type and the enum.

Note that due to a bug, this requires enums have at least one variant. Sorry.

# Examples

Create a u64 containing enum.
```rust
use typesafe_ints::int_enum;
int_enum!(
    #[derive(Eq, PartialEq, Clone, Copy, Debug)]
    pub(crate) enum A(u64) {
        B = 1,
        C = 2,
    }
);
let b = A::B;
let c = A::C;
let d = A::Other(5);
let b_: u64 = b.into();
let c_: u64 = c.into();
let d_: u64 = d.into();
assert_eq!(b_, 1u64);
assert_eq!(c_, 2u64);
assert_eq!(d_, 5u64);

let v_b: A = 1.into();
let v_c: A = 2.into();
let v_d: A = 5.into();
assert_eq!(v_b, b);
assert_eq!(v_c, c);
assert_eq!(v_d, d);
```
*/
#[macro_export]
macro_rules! int_enum {
    ($(#[$meta:meta])* $vis:vis enum $ident:ident($ty:ty) {
        $($(#[$varmeta:meta])* $variant:ident = $num:expr),* $(,)*
    }) => {
        $(#[$meta])*
        $vis enum $ident {
            $($(#[$varmeta:meta])* $variant),*
            , Other($ty)
        }

        impl ::core::convert::From<$ty> for $ident {
            fn from(t: $ty) -> $ident {
                match t {
                    $(
                        $num => $ident::$variant
                    ),*
                    , o => $ident::Other(o)
                }
            }
        }

        impl ::core::convert::From<$ident> for $ty {
            fn from(t: $ident) -> $ty {
                match t {
                    $(
                        $ident::$variant => $num
                    ),*
                    , $ident::Other(o) => o
                }
            }
        }
    }
}

/**
Creates an enum which has all of the given variants and a
[TryInto](core::convert::TryInto) implementation for coming from its backing
type. This one is `#[repr(uN)]` and is thus the size of `uN`.

Note that due to a bug, this requires enums have at least one variant. Sorry.

# Examples

Create a u64 enum.
```rust
use typesafe_ints::int_enum_only;
use core::convert::TryInto;
int_enum_only!(
    #[derive(Eq, PartialEq, Clone, Copy, Debug)]
    pub(crate) enum A(u64) {
        B = 1,
        C = 2,
    }
);
let b = A::B;
let c = A::C;
let b_: u64 = b as _;
let c_: u64 = c as _;
assert_eq!(b_, 1u64);
assert_eq!(c_, 2u64);

let v_b: Result<A, ()> = 1.try_into();
let v_c: Result<A, ()> = 2.try_into();
let v_d: Result<A, ()> = 5.try_into();
assert_eq!(v_b, Ok(b));
assert_eq!(v_c, Ok(c));
assert_eq!(v_d, Err(()));
```
*/
#[macro_export]
macro_rules! int_enum_only {
    ($(#[$meta:meta])* $vis:vis enum $ident:ident($ty:ty) {
        $($(#[$varmeta:meta])* $variant:ident = $num:expr),* $(,)*
    }) => {
        $(#[$meta])*
        $vis enum $ident {
            $($(#[$varmeta:meta])* $variant = $num),*
        }

        impl ::core::convert::TryFrom<$ty> for $ident {
            type Error = ();
            fn try_from(t: $ty) -> Result<$ident, Self::Error> {
                match t {
                    $(
                        $num => Ok($ident::$variant)
                    ),*
                    , _ => Err(())
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        int_enum!(
            enum A(u64) {
                A = 1
            }
        );
    }
}
