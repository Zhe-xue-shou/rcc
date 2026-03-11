/// IR Type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'context> {
  Void,
  Label,
  IEEE32Float,
  IEEE64Float,

  Pointer,
  Integer(u8),
  Array(Array<'context>),
  Function(Function<'context>),
  // TODO: complete it later, placeholder now vvv
  Struct,
}

pub type TypeRef<'context> = &'context Type<'context>;
pub type TypeRefMut<'context> = &'context mut Type<'context>;

#[derive(Debug, Clone, PartialEq, Eq)]

pub struct Array<'context> {
  element_type: TypeRef<'context>,
  length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function<'context> {
  result_type: TypeRef<'context>,
  params: &'context [TypeRef<'context>],
  is_variadic: bool,
}
