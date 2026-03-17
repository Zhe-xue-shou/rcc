use ::rcc_utils::ensure_is_pod;

use super::{
  Array, ArraySize, Constant, Context, Enum, FunctionProto, Pointer, Primitive,
  Record, TypeInfo, Union,
};
use crate::common::{FloatFormat, Floating, Integral, RefEq, Signedness};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Type<'context> {
  Primitive(Primitive),
  Pointer(Pointer<'context>),
  Array(Array<'context>),
  FunctionProto(FunctionProto<'context>),
  Enum(Enum<'context>),
  Record(Record<'context>),
  Union(Union<'context>),
}
/// Indicates a reference to [`Type`] which stores in the `'context`.
/// Call [`Type::ref_eq`] to check two [`Type`] are equal or not -- dont use [`Eq`]/`==`.
pub type TypeRef<'context> = &'context Type<'context>;
pub type TypeRefMut<'context> = &'context mut Type<'context>;

ensure_is_pod!(Type);
ensure_is_pod!(TypeRef);

impl<'context> Type<'context> {
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

  pub fn lookup(self, context: &Context<'context>) -> TypeRef<'context> {
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
impl Integral {
  pub fn unqualified_type<'context>(
    &self,
    context: &'context Context,
  ) -> TypeRef<'context> {
    if self.signedness() == Signedness::Signed {
      match self.width() {
        Self::WIDTH_CHAR => Context::char_type(context),
        Self::WIDTH_SHORT => Context::short_type(context),
        Self::WIDTH_INT => Context::int_type(context),
        // Self::WIDTH_LONG => Type::Primitive(Primitive::Long),
        Self::WIDTH_LONG_LONG => Context::long_long_type(context),
        _ => Context::int_type(context), // default
      }
    } else {
      match self.width() {
        Self::WIDTH_CHAR => Context::uchar_type(context),
        Self::WIDTH_SHORT => Context::ushort_type(context),
        Self::WIDTH_INT => Context::uint_type(context),
        // Self::WIDTH_LONG => Type::Primitive(Primitive::ULong),
        Self::WIDTH_LONG_LONG => Context::ulong_long_type(context),
        _ => Context::uint_type(context), // default
      }
    }
  }
}

impl Floating {
  pub fn unqualified_type<'context>(
    &self,
    context: &'context Context,
  ) -> TypeRef<'context> {
    use FloatFormat::*;
    match self.format() {
      IEEE32 => Context::float_type(context),
      IEEE64 => Context::double_type(context),
    }
  }
}

impl<'context> Constant<'context> {
  pub fn unqualified_type(
    &self,
    context: &'context Context,
  ) -> TypeRef<'context> {
    match self {
      Self::Integral(integral) => integral.unqualified_type(context),
      Self::Floating(floating) => floating.unqualified_type(context),
      Self::String(str) => Context::make_array(
        context,
        context.char_type().into(),
        ArraySize::Constant(str.len()),
      ),
      Self::Nullptr(_) => Context::nullptr_type(context),
      Self::Address(_) => Context::voidptr_type(context),
    }
  }
}
