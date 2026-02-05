use ::rc_utils::static_dispatch;
use ::std::fmt::Display;

use super::{
  Array, ArraySize, Constant, Enum, FunctionProto, FunctionSpecifier, Pointer,
  QualifiedType, Qualifiers, Record, Type, Union,
};

impl Display for Qualifiers {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut qualifiers = Vec::new();
    if self.contains(Qualifiers::Const) {
      qualifiers.push("const");
    }
    if self.contains(Qualifiers::Volatile) {
      qualifiers.push("volatile");
    }
    if self.contains(Qualifiers::Restrict) {
      qualifiers.push("restrict");
    }
    write!(f, "{}", qualifiers.join(" "))
  }
}
impl Display for FunctionSpecifier {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut specifiers = Vec::new();
    if self.contains(FunctionSpecifier::Inline) {
      specifiers.push("inline");
    }
    if self.contains(FunctionSpecifier::Noreturn) {
      specifiers.push("_Noreturn");
    }
    write!(f, "{}", specifiers.join(" "))
  }
}
impl Display for QualifiedType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.qualifiers().is_empty() {
      write!(f, "{}", self.unqualified_type())
    } else {
      write!(f, "{} {}", self.qualifiers(), self.unqualified_type())
    }
  }
}
impl Display for Array {
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

impl Display for FunctionProto {
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

impl Display for Type {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    static_dispatch!(
      self.fmt(f),
      Primitive FunctionProto Pointer Array Enum Record Union
    )
  }
}

impl Display for Pointer {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "*{}", self.pointee)
  }
}
impl Display for Enum {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<enum {}>", self.name.as_deref().unwrap_or("<unnamed>"))
  }
}
impl Display for Record {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "<struct {}>",
      self.name.as_deref().unwrap_or("<unnamed>")
    )
  }
}
impl Display for Union {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<union {}>", self.name.as_deref().unwrap_or("<unnamed>"))
  }
}

impl Display for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use Constant::*;
    match self {
      Integral(i) => write!(f, "{i}"),
      Floating(d) => write!(f, "{d}"),
      String(s) => write!(f, "\"{}\"", s),
      Nullptr(_) => write!(f, "nullptr"),
    }
  }
}
