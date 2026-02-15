//! this file would be furthur split into multiple files when more impls are added.

use ::rcc_utils::SmallString;

use super::{Primitive, QualifiedType, Type};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pointer {
  pub pointee: QualifiedType,
}
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpressionId {
  id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, ::strum_macros::Display)]
pub enum ArraySize {
  Constant(usize),
  /// unspecified size
  Incomplete,
  /// unsupported dynamic size, but i kept it here for the `full` type category
  ///
  /// TODO: if this holds an expression -- it's a cyclic reference of mod `type` and mod `analyzer::expression`. may use `ExpressionId` as a workaround.
  Variable(ExpressionId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Array {
  /// Array itself cannot have qualifiers,
  /// hence the QualifiedType::qualifiers of the whole array should be empty,
  /// the actual element type's qualifiers are stored here.
  pub element_type: QualifiedType,
  pub size: ArraySize,
  // These are not elem's, but the arraysize's. static is a hint for optimization,etc. dont care it for now.
  // pub qualifiers: Qualifiers,
  // pub is_static: bool,
}

/// function types themselves don't have qualifiers, but pointers to them can.
/// so the functionproto's qualifiers must be dropped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionProto {
  pub return_type: QualifiedType,
  pub parameter_types: Vec<QualifiedType>,
  pub is_variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
  pub name: SmallString,
  pub field_type: QualifiedType,
}

// ignore unnamed/anonymous structs/unions for now
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Record {
  pub name: Option<SmallString>,
  pub fields: Vec<Field>,
}

// seems not so much difference between struct and union here, but for convenience we keep them separate
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Union {
  pub name: Option<SmallString>,
  pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumConstant {
  pub name: SmallString,
  pub value: Option<isize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Enum {
  pub name: Option<SmallString>,
  pub constants: Vec<EnumConstant>,
  pub underlying_type: Primitive, // must be integer type
}

impl Pointer {
  pub fn new(pointee: QualifiedType) -> Self {
    Self { pointee }
  }
}

impl Array {
  pub fn new(element_type: QualifiedType, size: ArraySize) -> Self {
    Self { element_type, size }
  }
}
impl FunctionProto {
  pub fn new(
    return_type: QualifiedType,
    parameter_types: Vec<QualifiedType>,
    is_variadic: bool,
  ) -> Self {
    Self {
      return_type,
      parameter_types,
      is_variadic,
    }
  }
}
impl Enum {
  pub fn new(
    name: Option<SmallString>,
    constants: Vec<EnumConstant>,
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

macro_rules! to_qualified_type {
  ($ty:ty) => {
    impl From<$ty> for QualifiedType {
      fn from(value: $ty) -> Self {
        QualifiedType::new_unqualified(Type::from(value).into())
      }
    }

    impl From<$ty> for Box<QualifiedType> {
      fn from(value: $ty) -> Self {
        Box::new(QualifiedType::from(value))
      }
    }
  };
}

to_qualified_type!(Primitive);
to_qualified_type!(Array);
to_qualified_type!(Pointer);
to_qualified_type!(FunctionProto);
to_qualified_type!(Enum);
to_qualified_type!(Record);
to_qualified_type!(Union);

::rcc_utils::interconvert!(Primitive, Type);
::rcc_utils::interconvert!(Array, Type);
::rcc_utils::interconvert!(Pointer, Type);
::rcc_utils::interconvert!(FunctionProto, Type);
::rcc_utils::interconvert!(Enum, Type);
::rcc_utils::interconvert!(Record, Type);
::rcc_utils::interconvert!(Union, Type);

::rcc_utils::make_trio_for!(Primitive, Type);
::rcc_utils::make_trio_for!(Array, Type);
::rcc_utils::make_trio_for!(Pointer, Type);
::rcc_utils::make_trio_for!(FunctionProto, Type);
::rcc_utils::make_trio_for!(Enum, Type);
::rcc_utils::make_trio_for!(Record, Type);
::rcc_utils::make_trio_for!(Union, Type);
