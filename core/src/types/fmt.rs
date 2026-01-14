use ::std::fmt::Display;

use super::{
  Array, ArraySize, Constant, Enum, FunctionProto, Pointer, QualifiedType,
  Qualifiers, Record, Type, Union,
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
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Type::Primitive(builtin) => write!(f, "{}", builtin),
      Type::FunctionProto(proto) => write!(f, "{}", proto),
      Type::Pointer(ptr) => write!(f, "*{}", ptr),
      Type::Array(array_type) => write!(f, "{}", array_type),
      Type::Enum(enum_type) => write!(f, "enum {}", enum_type),
      Type::Record(record_type) => write!(f, "struct {}", record_type),
      Type::Union(variant_type) => write!(f, "union {}", variant_type),
    }
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
    match self {
      Constant::Char(i) => write!(f, "{}", i),
      Constant::Short(i) => write!(f, "{}", i),
      Constant::Int(i) => write!(f, "{}", i),
      Constant::LongLong(i) => write!(f, "{}", i),
      Constant::UChar(u) => write!(f, "{}", u),
      Constant::UShort(u) => write!(f, "{}", u),
      Constant::UInt(u) => write!(f, "{}", u),
      Constant::ULongLong(u) => write!(f, "{}", u),
      Constant::Float(fl) => write!(f, "{}", fl),
      Constant::Double(fl) => write!(f, "{}", fl),
      Constant::Bool(b) => write!(f, "{}", b),
      Constant::String(s) => write!(f, "\"{}\"", s),
    }
  }
}
