use ::std::fmt::Display;

use super::{
  Array, ArraySize, Enum, FunctionProto, FunctionSpecifier, Pointer,
  QualifiedType, Qualifiers, Record, Type, Union,
};
impl Display for Qualifiers {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.into_static_str())
  }
}
impl Display for FunctionSpecifier {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.into_static_str())
  }
}
impl Display for QualifiedType<'_> {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.qualifiers.is_empty() {
      write!(f, "{}", self.unqualified_type)
    } else {
      write!(f, "{} {}", self.qualifiers, self.unqualified_type)
    }
  }
}
impl Display for Array<'_> {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}[", self.element_type)?;
    match &self.size {
      ArraySize::Constant(sz) => write!(f, "{}", sz)?,
      ArraySize::Incomplete => write!(f, "")?,
      ArraySize::Variable(_id) => todo!(), // ignore for now
    }
    write!(f, "]")
  }
}

impl Display for FunctionProto<'_> {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}(", self.return_type)?;
    for (i, param) in self.parameter_types.iter().enumerate() {
      if i > 0 {
        write!(f, ", ")?;
      }
      write!(f, "{}", param)?;
    }
    write!(f, ")")
  }
}

impl Display for Pointer<'_> {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "*{}", self.pointee)
  }
}
impl Display for Enum<'_> {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<enum {}>", self.name.unwrap_or("<unnamed>"))
  }
}
impl Display for Record<'_> {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<struct {}>", self.name.unwrap_or("<unnamed>"))
  }
}
impl Display for Union<'_> {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<union {}>", self.name.unwrap_or("<unnamed>"))
  }
}

// impl Display for Type<'_> {
//   fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
//     ::rcc_utils::static_dispatch!(
//       self,
//       |variant| variant.fmt(f) =>
//       Primitive FunctionProto Pointer Array Enum Record Union
//     )
//   }
// }
impl Type<'_> {
  /// reverse-back the type for printing. TODO: function pointers still incorrect.
  fn pretty_print(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // base
    self.print_base_type(f)?;

    // (abstract) declarator
    self.print_declarator(f)
  }

  fn print_base_type(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    use Type::*;
    match self {
      Array(arr) => arr.element_type.unqualified_type.print_base_type(f),
      Pointer(ptr) => ptr.pointee.unqualified_type.print_base_type(f),
      FunctionProto(func) =>
        func.return_type.unqualified_type.print_base_type(f),
      Primitive(p) => write!(f, "{}", p),
      Record(r) => write!(f, "<struct {}>", r.name.unwrap_or("<unnamed>")),
      Enum(e) => write!(f, "<enum {}>", e.name.unwrap_or("<unnamed>")),
      Union(u) => write!(f, "<union {}>", u.name.unwrap_or("<unnamed>")),
    }
  }

  fn print_declarator(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    use Type::*;
    match self {
      Array(arr) => {
        // print from the outside in
        write!(f, "[")?;
        match &arr.size {
          ArraySize::Constant(sz) => write!(f, "{}", sz)?,
          ArraySize::Incomplete => (),
          ArraySize::Variable(_id) => (), // ignore for now
        }
        write!(f, "]")?;
        arr.element_type.unqualified_type.print_declarator(f)
      },
      Pointer(ptr)
        if matches!(
          ptr.pointee.unqualified_type,
          Array(_) | FunctionProto(_)
        ) =>
      {
        // if the pointee is an array or function, parentheses is needed, e.g., `(*)[10]`

        write!(f, "(*)")?;

        ptr.pointee.qualifiers.fmt(f)?;
        ptr.pointee.unqualified_type.print_declarator(f)?;
        Ok(())
      },
      Pointer(ptr) => {
        write!(f, "*")?;
        ptr.pointee.qualifiers.fmt(f)?;
        ptr.pointee.unqualified_type.print_declarator(f)?;
        Ok(())
      },
      FunctionProto(func) => {
        write!(f, "(")?;
        if func.is_variadic {
          if !func.parameter_types.is_empty() {
            write!(f, ", ")?;
          }
          write!(f, "...")?;
        } else if func.parameter_types.is_empty() {
          write!(f, "void")?;
        } else {
          for (i, param) in func.parameter_types.iter().enumerate() {
            if i > 0 {
              write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
          }
        }
        write!(f, ")")?;
        func.return_type.unqualified_type.print_declarator(f)
      },
      // the rest of types is considered `base type` here.
      _ => Ok(()),
    }
  }
}

impl Display for Type<'_> {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.pretty_print(f)
  }
}
