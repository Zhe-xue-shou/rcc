//! this file would be furthur split into multiple files when more impls are added.

use ::rcc_utils::StrRef;

use super::{Primitive, QualifiedType, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pointer<'c> {
  pub pointee: QualifiedType<'c>,
}
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpressionId {
  id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ::strum_macros::Display)]
pub enum ArraySize {
  Constant(usize),
  /// unspecified size
  Incomplete,
  /// unsupported dynamic size, but i kept it here for the `full` type category
  ///
  /// TODO: if this holds an expression -- it's a cyclic reference of mod `type` and mod `analyzer::expression`. may use `ExpressionId` as a workaround.
  Variable(ExpressionId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Array<'c> {
  /// Array itself cannot have qualifiers,
  /// hence the QualifiedType::qualifiers of the whole array should be empty,
  /// the actual element type's qualifiers are stored here.
  pub element_type: QualifiedType<'c>,
  pub size: ArraySize,
  // These are not elem's, but the arraysize's. static is a hint for optimization,etc. dont care it for now.
  // pub qualifiers: Qualifiers,
  // pub is_static: bool,
}

/// function types themselves don't have qualifiers, but pointers to them can.
/// so the functionproto's qualifiers must be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionProto<'c> {
  pub return_type: QualifiedType<'c>,
  pub parameter_types: &'c [QualifiedType<'c>],
  pub is_variadic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Field<'c> {
  pub name: StrRef<'c>,
  pub field_type: QualifiedType<'c>,
}

// ignore unnamed/anonymous structs/unions for now
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Record<'c> {
  pub name: Option<StrRef<'c>>,
  pub fields: &'c [Field<'c>],
}

// seems not so much difference between struct and union here, but for convenience we keep them separate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Union<'c> {
  pub name: Option<StrRef<'c>>,
  pub fields: &'c [Field<'c>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumConstant<'c> {
  pub name: StrRef<'c>,
  pub value: Option<isize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Enum<'c> {
  pub name: Option<StrRef<'c>>,
  pub constants: &'c [EnumConstant<'c>],
  pub underlying_type: Primitive, // must be integer type
}

impl<'c> Pointer<'c> {
  pub fn new(pointee: QualifiedType<'c>) -> Self {
    Self { pointee }
  }
}

impl<'c> Array<'c> {
  pub fn new(element_type: QualifiedType<'c>, size: ArraySize) -> Self {
    Self { element_type, size }
  }
}
impl ArraySize {
  pub fn size(self) -> usize {
    match self {
      Self::Constant(c) => c,
      Self::Incomplete => 0,
      Self::Variable(_) => 0,
    }
  }
}
impl<'c> FunctionProto<'c> {
  pub fn new(
    return_type: QualifiedType<'c>,
    parameter_types: &'c [QualifiedType<'c>],
    is_variadic: bool,
  ) -> Self {
    Self {
      return_type,
      parameter_types,
      is_variadic,
    }
  }
}
impl<'c> Enum<'c> {
  pub fn new(
    name: Option<StrRef<'c>>,
    constants: &'c [EnumConstant<'c>],
    underlying_type: Primitive,
  ) -> Self {
    assert!(underlying_type.is_integer());
    Self {
      name,
      constants,
      underlying_type,
    }
  }

  pub fn into_underlying_type(self) -> Primitive {
    self.underlying_type
  }
}

// macro_rules! to_qualified_type {
//   ($ty:ty) => {
//     impl<'c> From<$ty> for QualifiedType<'c> {
//       fn from(value: $ty) -> Self {
//         QualifiedType::new_unqualified(Type::from(value).into())
//       }
//     }

//     impl<'c> From<$ty> for Box<QualifiedType<'c>> {
//       fn from(value: $ty) -> Self {
//         Box::new(QualifiedType::from(value))
//       }
//     }
//   };
// }

// to_qualified_type!(Primitive);
// to_qualified_type!(Array<'c>);
// to_qualified_type!(Pointer<'c>);
// to_qualified_type!(FunctionProto<'c>);
// to_qualified_type!(Enum<'c>);
// to_qualified_type!(Record<'c>);
// to_qualified_type!(Union<'c>);

::rcc_utils::interconvert!(Primitive, Type<'c>);
::rcc_utils::interconvert!(Array, Type, 'c);
::rcc_utils::interconvert!(Pointer, Type, 'c);
::rcc_utils::interconvert!(FunctionProto, Type, 'c);
::rcc_utils::interconvert!(Enum, Type, 'c);
::rcc_utils::interconvert!(Record, Type, 'c);
::rcc_utils::interconvert!(Union, Type, 'c);

::rcc_utils::make_trio_for!(Primitive, Type<'c>);
::rcc_utils::make_trio_for!(Array, Type, 'c);
::rcc_utils::make_trio_for!(Pointer, Type, 'c);
::rcc_utils::make_trio_for!(FunctionProto, Type, 'c);
::rcc_utils::make_trio_for!(Enum, Type, 'c);
::rcc_utils::make_trio_for!(Record, Type, 'c);
::rcc_utils::make_trio_for!(Union, Type, 'c);
