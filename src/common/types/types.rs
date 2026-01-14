use ::once_cell::sync::Lazy;
use ::std::str::FromStr;

use crate::common::{
  error::Error,
  rawdecl::FunctionSpecifier,
  types::{
    Array, ArraySize, Compatibility, Enum, EnumConstant, FunctionProto,
    Pointer, Primitive, QualifiedType, Qualifiers, Record, Type, TypeInfo,
    Union,
  },
};

impl QualifiedType {
  pub fn new(qualifiers: Qualifiers, unqualified_type: Type) -> Self {
    Self {
      qualifiers,
      unqualified_type,
    }
  }

  pub fn new_unqualified(unqualified_type: Type) -> Self {
    Self {
      qualifiers: Qualifiers::empty(),
      unqualified_type,
    }
  }

  pub fn void() -> Self {
    Self::new_unqualified(Type::void())
  }

  pub fn bool() -> Self {
    Self::new_unqualified(Type::bool())
  }

  pub fn int() -> Self {
    Self::new_unqualified(Type::int())
  }
}
impl Pointer {
  pub fn new(pointee: Box<QualifiedType>) -> Self {
    Self { pointee }
  }
}

impl Primitive {
  pub fn new(str: String) -> Self {
    Self::maybe_new(str).unwrap()
  }

  pub fn maybe_new(str: String) -> Option<Self> {
    Primitive::from_str(&str).ok()
  }
}

impl FunctionProto {
  const MAIN_PROTO_ARGS: Lazy<FunctionProto> = Lazy::new(|| {
    FunctionProto::new(
      Box::new(QualifiedType::new(
        Qualifiers::empty(),
        Type::Primitive(Primitive::Int),
      )),
      vec![QualifiedType::new(
        Qualifiers::empty(),
        Type::Pointer(Pointer::new(Box::new(QualifiedType::new(
          Qualifiers::empty(),
          Type::Primitive(Primitive::Char),
        )))),
      )],
      false,
    )
  });
  const MAIN_PROTO_EMPTY: Lazy<FunctionProto> = Lazy::new(|| {
    FunctionProto::new(
      Box::new(QualifiedType::new(
        Qualifiers::empty(),
        Type::Primitive(Primitive::Int),
      )),
      vec![],
      false,
    )
  });

  pub fn new(
    return_type: Box<QualifiedType>,
    parameter_types: Vec<QualifiedType>,
    is_variadic: bool,
  ) -> Self {
    Self {
      return_type,
      parameter_types,
      is_variadic,
    }
  }

  pub fn main_proto_validate(
    &self,
    function_specifier: FunctionSpecifier,
  ) -> Result<(), Error> {
    if self.is_variadic {
      Err(())
    } else if function_specifier.contains(FunctionSpecifier::Inline) {
      Err(())
    } else if !self.compatible_with(&Self::MAIN_PROTO_EMPTY)
      && !self.compatible_with(&Self::MAIN_PROTO_ARGS)
    {
      Err(())
    } else {
      todo!()
    }
  }
}

impl Array {
  pub fn new(element_type: Box<QualifiedType>, size: ArraySize) -> Self {
    Self { element_type, size }
  }
}
impl Enum {
  pub fn new(
    name: Option<String>,
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

use paste::paste;

use crate::breakpoint;

macro_rules! make_trio_for {
  ($variant:ident) => {
    make_trio_for!($variant, $variant);
  };
  // We use :ident because we are working with names, not complex types
  ($variant:ident, $inner:ident) => {
    paste! {
        impl Type {
            #[inline]
            pub fn [<is_ $variant:lower>](&self) -> bool {
                matches!(self, Self::$variant(_))
            }

            #[inline]
            pub fn [<as_ $variant:lower>](&self) -> Option<&$inner> {
                match self {
                    Self::$variant(v) => Some(v),
                    _ => None,
                }
            }

            #[inline]
            pub fn [<as_ $variant:lower _unchecked>](&self) -> &$inner {
                match self {
                    Self::$variant(v) => v,
                    _ => {
                        breakpoint!();
                        unreachable!()
                    }
                }
            }

            #[inline]
            pub fn [<into_ $variant:lower>](self) -> Option<$inner> {
                match self {
                    Self::$variant(v) => Some(v),
                    _ => None,
                }
            }
        }
    }
  };
}
use crate::interconvert;

interconvert!(Primitive, Type);
interconvert!(Array, Type);
interconvert!(Pointer, Type);
interconvert!(FunctionProto, Type);
interconvert!(Enum, Type);
interconvert!(Record, Type);
interconvert!(Union, Type);

make_trio_for!(Primitive);
make_trio_for!(Array);
make_trio_for!(Pointer);
make_trio_for!(FunctionProto);
make_trio_for!(Enum);
make_trio_for!(Record);
make_trio_for!(Union);

impl Type {
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

  pub fn is_arithmetic(&self) -> bool {
    match self {
      Type::Primitive(p) => p.is_arithmetic(),
      _ => false,
    }
  }

  pub fn void() -> Self {
    Type::Primitive(Primitive::Void)
  }

  pub fn bool() -> Self {
    Type::Primitive(Primitive::Bool)
  }

  pub fn int() -> Self {
    Type::Primitive(Primitive::Int)
  }
}
impl QualifiedType {
  pub fn is_modifiable(&self) -> bool {
    self.unqualified_type.is_modifiable()
      && !self.qualifiers.contains(Qualifiers::Const)
  }

  pub fn is_void(&self) -> bool {
    self.unqualified_type.is_void()
  }
}
impl From<Type> for QualifiedType {
  fn from(value: Type) -> Self {
    QualifiedType::new_unqualified(value)
  }
}
