use ::std::fmt::Display;

use super::{
  Array, ArraySize, Constant, Enum, FunctionProto, FunctionSpecifier, Pointer,
  Primitive, QualifiedType, Qualifiers, Record, Type, Union,
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
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Type::Primitive(builtin) => <Primitive as Display>::fmt(builtin, f),
      Type::FunctionProto(proto) => <FunctionProto as Display>::fmt(proto, f),
      Type::Pointer(ptr) => <Pointer as Display>::fmt(ptr, f),
      Type::Array(array_type) => <Array as Display>::fmt(array_type, f),
      Type::Enum(enum_type) => <Enum as Display>::fmt(enum_type, f),
      Type::Record(record_type) => <Record as Display>::fmt(record_type, f),
      Type::Union(variant_type) => <Union as Display>::fmt(variant_type, f),
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
    use Constant::*;
    match self {
      Char(i) => write!(f, "{}", i),
      Short(i) => write!(f, "{}", i),
      Int(i) => write!(f, "{}", i),
      LongLong(i) => write!(f, "{}", i),
      UChar(u) => write!(f, "{}", u),
      UShort(u) => write!(f, "{}", u),
      UInt(u) => write!(f, "{}", u),
      ULongLong(u) => write!(f, "{}", u),
      Float(fl) => write!(f, "{}", fl),
      Double(fl) => write!(f, "{}", fl),
      Bool(b) => write!(f, "{}", b),
      StringLiteral(s) => write!(f, "\"{}\"", s),
      Nullptr(_) => write!(f, "nullptr"),
    }
  }
}
