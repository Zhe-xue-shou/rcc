use ::rcc_adt::{FloatFormat, Floating, Integral, Signedness};
use ::rcc_shared::Constant;
use ::rcc_utils::{RefEq, ensure_is_pod};

use super::{
  Array, ArraySize, Enum, FunctionProto, Pointer, Primitive, Record, TypeInfo,
  Union,
};
use crate::context::Context;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Type<'c> {
  Primitive(Primitive),
  Pointer(Pointer<'c>),
  Array(Array<'c>),
  FunctionProto(FunctionProto<'c>),
  Enum(Enum<'c>),
  Record(Record<'c>),
  Union(Union<'c>),
}
/// Indicates a reference to [`Type`] which stores in the `'c`.
/// Call [`Type::ref_eq`] to check two [`Type`] are equal or not -- dont use [`Eq`]/`==`.
pub type TypeRef<'c> = &'c Type<'c>;
pub type TypeRefMut<'c> = &'c mut Type<'c>;

ensure_is_pod!(Type);
ensure_is_pod!(TypeRef);
impl<'c> Type<'c> {
  fn is_unsigned(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_unsigned(),
      Type::Pointer(_) => true,
      Type::Enum(e) => e.underlying_type.is_unsigned(),
      _ => false,
    }
  }

  fn is_signed(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_signed(),
      Type::Enum(e) => e.underlying_type.is_signed(),
      _ => false,
    }
  }

  pub fn signedness(&self) -> Option<Signedness> {
    use Signedness::*;
    if self.is_signed() {
      Some(Signed)
    } else if self.is_unsigned() {
      Some(Unsigned)
    } else {
      None
    }
  }
}
impl<'c> Type<'c> {
  pub fn is_modifiable(&self) -> bool {
    if self.size() == 0 {
      false
    } else {
      match self {
        Type::Array(_) => false,
        _ => true, // todo: struct/union with const member
      }
    }
  }

  pub fn is_void(&self) -> bool {
    matches!(self, Type::Primitive(Primitive::Void))
  }

  pub fn is_integer(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_integer(),
      _ => false,
    }
  }

  pub fn is_floating_point(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_floating_point(),
      _ => false,
    }
  }

  pub fn is_arithmetic(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_arithmetic(),
      _ => false,
    }
  }

  pub fn lookup(self, context: &Context<'c>) -> TypeRef<'c> {
    context.intern(self)
  }
}
impl RefEq for TypeRef<'_> {
  fn ref_eq(lhs: Self, rhs: Self) -> bool
  where
    Self: PartialEq + Sized,
  {
    {
      let ref_eq = ::std::ptr::eq(lhs, rhs);
      if const { cfg!(debug_assertions) } {
        let actual_eq = lhs == rhs;
        if ref_eq != actual_eq {
          eprintln!(
            "INTERNAL ERROR: comparing by pointer address result did not \
             match 
          the actual result: {:p}: {:?} and {:p}: {:?}
        ",
            lhs, lhs, rhs, rhs
          );
        }
        return actual_eq;
      }
      ref_eq
    }
  }
}
mod private {
  use super::{Constant, Floating, Integral};
  pub trait Sealed {}
  impl Sealed for Integral {}
  impl Sealed for Floating {}
  impl Sealed for Constant<'_> {}
}
pub trait UnqualExt<'c>: private::Sealed {
  fn unqualified_type(&self, context: &'c Context) -> TypeRef<'c>;
}

impl<'c> UnqualExt<'c> for Integral {
  fn unqualified_type(&self, context: &'c Context) -> TypeRef<'c> {
    use Signedness::*;
    match self.signedness() {
      Signed => {
        match self.width() {
          Self::WIDTH_CHAR => Context::char_type(context),
          Self::WIDTH_SHORT => Context::short_type(context),
          Self::WIDTH_INT => Context::int_type(context),
          // Self::WIDTH_LONG => Type::Primitive(Primitive::Long),
          Self::WIDTH_LONG_LONG => Context::long_long_type(context),
          _ => Context::int_type(context), // default
        }
      },
      Unsigned => {
        match self.width() {
          Self::WIDTH_CHAR => Context::uchar_type(context),
          Self::WIDTH_SHORT => Context::ushort_type(context),
          Self::WIDTH_INT => Context::uint_type(context),
          // Self::WIDTH_LONG => Type::Primitive(Primitive::ULong),
          Self::WIDTH_LONG_LONG => Context::ulong_long_type(context),
          _ => Context::uint_type(context), // default
        }
      },
    }
  }
}

impl<'c> UnqualExt<'c> for Floating {
  fn unqualified_type(&self, context: &'c Context) -> TypeRef<'c> {
    use FloatFormat::*;
    match self.format() {
      IEEE32 => Context::float32_type(context),
      IEEE64 => Context::float64_type(context),
    }
  }
}

impl<'c> UnqualExt<'c> for Constant<'c> {
  fn unqualified_type(&self, context: &'c Context) -> TypeRef<'c> {
    match self {
      Self::Integral(integral) => integral.unqualified_type(context),
      Self::Floating(floating) => floating.unqualified_type(context),
      Self::String(str) => Context::make_array(
        context,
        context.char_type().into(),
        ArraySize::Constant(str.len()),
      ),
      Self::Nullptr() => Context::nullptr_type(context),
      Self::Address(_) => Context::voidptr_type(context),
    }
  }
}
