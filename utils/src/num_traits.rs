//! Traits and implementations for built-in numeric types conversion.
//!
//! do **NOT** add any new `impl` to arbitrary types here. Only built-in numeric types are allowed.

mod private {
  pub const trait Sealed {
    /* nothing. */
  }
}

macro_rules! traits {
  ($($variant:ty)*) => {
    ::paste::paste! {
        $(
            impl const private::Sealed for $variant {}

            pub const trait [< To $variant:camel >] : private::Sealed {
                #[must_use]
                fn [< to_ $variant:lower >](self) -> $variant;
            }
        )*
    }
  };
}
macro_rules! impl_it {
    ($($t:ty)* : $variant:ty) => {
      ::paste::paste! {
          $(
              impl const [<To $variant:camel>] for $t {
                  #[inline(always)]
                  fn [< to_ $variant:lower>](self) -> $variant {
                      self as $variant
                  }
              }
          )*
      }
    };
}

macro_rules! impl_all {
    ($($t:ty)*) => {
        impl_it!($($t)* : i8);
        impl_it!($($t)* : i16);
        impl_it!($($t)* : i32);
        impl_it!($($t)* : i64);
        impl_it!($($t)* : i128);
        impl_it!($($t)* : u8);
        impl_it!($($t)* : u16);
        impl_it!($($t)* : u32);
        impl_it!($($t)* : u64);
        impl_it!($($t)* : u128);
        impl_it!($($t)* : isize);
        impl_it!($($t)* : usize);
    };
}

traits!(bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize f32 f64);
impl_all!(bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize f32 f64);

macro_rules! impl_all_float {
    ($($t:ty)*) => {
        impl_it!($($t)* : f32);
        impl_it!($($t)* : f64);
    };
}

impl_all_float!(f32 f64);

pub const trait NumTo<T>: private::Sealed {
  #[must_use]
  fn to(self) -> T;
}

pub const trait NumFrom<T>: private::Sealed {
  #[must_use]
  fn from(value: T) -> Self;
}

// automatially impl NumTo for types that implement NumFrom
impl<T, U> const NumTo<U> for T
where
  T: [const] private::Sealed,
  U: [const] NumFrom<T>,
{
  #[inline(always)]
  fn to(self) -> U {
    U::from(self)
  }
}

macro_rules! from {
  ($from:ty => $to:ty) => {
    ::paste::paste! {
        impl const NumFrom<$from> for $to {
            #[inline(always)]
            fn from(value: $from) -> Self {
                value as $to
            }
        }
    }
  };
}

macro_rules! from_all {
  ($($t:ty)*) => {
      $(
          from!($t => i8);
          from!($t => i16);
          from!($t => i32);
          from!($t => i64);
          from!($t => i128);
          from!($t => u8);
          from!($t => u16);
          from!($t => u32);
          from!($t => u64);
          from!($t => u128);
          from!($t => isize);
          from!($t => usize);
      )*
  };
}
from_all!(bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize f32 f64);

#[macro_export]
macro_rules! underlying_type_of {
  (f32) => {
    u32
  };
  (f64) => {
    u64
  };
  (f128) => {
    u128
  };
  ($t:ty) => {
    compile_error!("unsupported type");
  };
}
#[macro_export]
macro_rules! signed_type_of {
  (u8) => {
    i8
  };
  (u16) => {
    i16
  };
  (u32) => {
    i32
  };
  (u64) => {
    i64
  };
  (u128) => {
    i128
  };
  (usize) => {
    isize
  };
  ($t:ty) => {
    compile_error!("unsupported type");
  };
}
#[macro_export]
macro_rules! unsigned_type_of {
  (i8) => {
    u8
  };
  (i16) => {
    u16
  };
  (i32) => {
    u32
  };
  (i64) => {
    u64
  };
  (i128) => {
    u128
  };
  (isize) => {
    usize
  };
  ($t:ty) => {
    compile_error!("unsupported type");
  };
}
macro_rules! generate_tag{
  ($($t:ty)* => $tag:ident) => {
    pub const trait $tag {
      /* nothing. */
    }
    $(
      impl const $tag for $t {}
    )*
  }
}

macro_rules! mark_neg_tag{
  ($($t:ty)* => $tag:ident) => {
    $(
      impl const !$tag for $t {}
    )*
  }
}

generate_tag!(f32 f64 => BuiltinFloat);
mark_neg_tag!(bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize => BuiltinFloat);
generate_tag!(i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize => BuiltinInteger);
mark_neg_tag!(bool f32 f64 => BuiltinInteger);
generate_tag!(i8 i16 i32 i64 i128 isize => BuiltinSignedInteger);
mark_neg_tag!(bool u8 u16 u32 u64 u128 usize f32 f64 => BuiltinSignedInteger);
generate_tag!(u8 u16 u32 u64 u128 usize => BuiltinUnsignedInteger);
mark_neg_tag!(bool i8 i16 i32 i64 i128 isize f32 f64 => BuiltinUnsignedInteger);
generate_tag!(f32 f64 i8 i16 i32 i64 i128 isize => BuiltinSignedNumeric);
mark_neg_tag!(bool u8 u16 u32 u64 u128 usize => BuiltinSignedNumeric);
generate_tag!(f32 f64 i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize => BuiltinNumeric);
mark_neg_tag!(bool => BuiltinNumeric);

generate_tag!(bool => BuiltinBoolean);
mark_neg_tag!(i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize f32 f64 => BuiltinBoolean);
generate_tag!(bool u8 u16 u32 u64 u128 usize => BuiltinUnsignedOrBoolean);
mark_neg_tag!(i8 i16 i32 i64 i128 isize f32 f64 => BuiltinUnsignedOrBoolean);
generate_tag!(bool i8 i16 i32 i64 i128 isize => BuiltinSignedIntegerOrBoolean);
mark_neg_tag!(u8 u16 u32 u64 u128 usize f32 f64 => BuiltinSignedIntegerOrBoolean);
generate_tag!(bool i8 i16 i32 i64 i128 isize f32 f64 => BuiltinSignedNumericOrBoolean);
mark_neg_tag!(u8 u16 u32 u64 u128 usize => BuiltinSignedNumericOrBoolean);
generate_tag!(bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize => BuiltinIntegerOrBoolean);
mark_neg_tag!(f32 f64 => BuiltinIntegerOrBoolean);
generate_tag!(f32 f64 bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 isize usize => BuiltinNumericOrBoolean);
mark_neg_tag!(/* nothing */ => BuiltinNumericOrBoolean);
