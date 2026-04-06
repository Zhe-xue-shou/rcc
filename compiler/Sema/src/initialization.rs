//! Initialization of objects.
//!
//! This module is responsible for processing initializers in variable declarations and static assertions.
//! It handles both scalar and aggregate initializers, including array initializers with designators.
//!
//! ### Implementation Note:
//! - C99 designated initializers and brace elision is one of the **most complex parts** I've ever implemented!!!
//! - Struct and union initializers are not implemented yet.
//! - The implementation is currently focused on correctness and clarity rather than performance.
//!   I know there's a lot of room for optimization, but I want to get it working first.
//!

use ::rcc_ast::types::{self as ast, QualifiedType, TypeInfo};
use ::rcc_parse::{declaration as pd, expression as pe};
use ::rcc_shared::{ArenaVec, CollectIn, DiagData::*, SourceSpan};
use ::rcc_utils::RefEq;
use ::std::{collections::HashMap, ops::Deref};

use crate::{Sema, declaration as sd, expression as se, semantics::HandleWith};

#[allow(non_upper_case_globals)]
const npos: usize = sd::Designator::npos;
pub struct Initialization<'i, 'c>
where
  'c: 'i,
{
  sema: &'i Sema<'c>,
  /// Gloabl decl or local static decl
  requires_folding: bool,
}
impl<'i, 'c> Deref for Initialization<'i, 'c> {
  type Target = Sema<'c>;

  fn deref(&self) -> &'i Self::Target {
    self.sema
  }
}
#[derive(Debug)]
enum Kind {
  Implicit,
  Explicit,
}
use Kind::*;

impl Kind {
  #[inline(always)]
  fn is_implicit(&self) -> bool {
    matches!(self, Implicit)
  }
}

#[derive(Debug)]
enum InitNode<'c> {
  Scalar(pe::Expression<'c>),
  List(ArrayTree<'c>),
}

impl InitNode<'_> {
  fn span(&self) -> SourceSpan {
    match self {
      InitNode::Scalar(expr) => expr.span(),
      InitNode::List(tree) => tree.span,
    }
  }
}

#[derive(Debug)]
struct TreeEntry<'c> {
  node: InitNode<'c>,
  index: usize,
  is_implicit: bool,
}

impl<'c> TreeEntry<'c> {
  fn new(index: usize, node: InitNode<'c>, is_implicit: bool) -> Self {
    Self {
      index,
      node,
      is_implicit,
    }
  }
}

#[derive(Debug, Default)]
struct ArrayTree<'c> {
  entries: Vec<TreeEntry<'c>>,
  positions: HashMap<usize, usize>,
  max_index: usize,
  span: SourceSpan,
}

impl<'c> ArrayTree<'c> {
  fn new(span: SourceSpan) -> Self {
    Self {
      span,
      ..Default::default()
    }
  }

  #[inline]
  fn note_index(&mut self, index: usize) {
    if index == npos {
      return;
    }
    self.max_index = Ord::max(self.max_index, index);
  }

  fn insert_leaf(
    &mut self,
    index: usize,
    node: InitNode<'c>,
    is_implicit: bool,
  ) -> bool {
    self.note_index(index);
    if let Some(&pos) = self.positions.get(&index) {
      let entry = &mut self.entries[pos];
      entry.node = node;
      entry.is_implicit = is_implicit;
      true
    } else {
      let pos = self.entries.len();
      self.positions.insert(index, pos);
      self.entries.push(TreeEntry::new(index, node, is_implicit));
      false
    }
  }

  fn insert_path(
    &mut self,
    path: &[usize],
    node: InitNode<'c>,
    is_implicit: bool,
  ) -> bool {
    if path.is_empty() {
      return false;
    }

    let index = path[0];
    if path.len() == 1 {
      return self.insert_leaf(index, node, is_implicit);
    }

    self.note_index(index);

    let span = node.span();
    let pos = if let Some(&pos) = self.positions.get(&index) {
      pos
    } else {
      let pos = self.entries.len();
      self.positions.insert(index, pos);
      self.entries.push(TreeEntry::new(
        index,
        InitNode::List(ArrayTree::new(span)),
        is_implicit,
      ));
      pos
    };

    let entry = &mut self.entries[pos];
    entry.is_implicit = is_implicit;

    let is_overridden = if !matches!(entry.node, InitNode::List(_)) {
      entry.node = InitNode::List(ArrayTree::new(span));
      true
    } else {
      false
    };

    let deeper = match &mut entry.node {
      InitNode::List(tree) => tree.insert_path(&path[1..], node, is_implicit),
      InitNode::Scalar(_) => unreachable!(),
    };

    is_overridden || deeper
  }
}

/// Wrappers.
impl<'i, 'c> Initialization<'i, 'c> {
  pub fn new(sema: &'i Sema<'c>, requires_folding: bool) -> Self {
    Self {
      sema,
      requires_folding,
    }
  }

  pub fn doit(
    self,
    initializer: pd::Initializer<'c>,
    target_type: Option<QualifiedType<'c>>,
  ) -> (sd::Initializer<'c>, QualifiedType<'c>) {
    match initializer {
      pd::Initializer::Expression(expr) => self.top_scalar(expr, target_type),
      pd::Initializer::InitializerList(list) =>
        self.top_list(list, target_type),
    }
  }

  fn top_scalar(
    &self,
    expr: pe::Expression<'c>,
    target_type: Option<QualifiedType<'c>>,
  ) -> (sd::Initializer<'c>, QualifiedType<'c>) {
    let scalar =
      self.scalar(expr, target_type.unwrap_or(self.ast().void_type().into()));
    let qualified_type = match target_type {
      None => *scalar.qualified_type(),
      Some(target_type) =>
        self.complete_array_type_from_string_if_eligible(target_type, scalar),
    };

    (scalar.into(), qualified_type)
  }

  fn top_list(
    &self,
    list: pd::InitializerList<'c>,
    target_type: Option<QualifiedType<'c>>,
  ) -> (sd::Initializer<'c>, QualifiedType<'c>) {
    match target_type {
      Some(target_type) => match target_type.unqualified_type {
        ast::Type::Array(ast::Array {
          size: ast::ArraySize::Incomplete,
          ..
        }) => self.top_incomplete_dimension(list, target_type),
        _ => (self.list(list, target_type).into(), target_type),
      },
      None => {
        self.add_error(DeducedTypeWithBracedInitializer, list.span);
        (self.__empty_expr.into(), self.ast().void_type().into())
      },
    }
  }

  fn top_incomplete_dimension(
    &self,
    list: pd::InitializerList<'c>,
    incomplete_array_type: QualifiedType<'c>,
  ) -> (sd::Initializer<'c>, QualifiedType<'c>) {
    let pd::InitializerList { entries, span } = list;

    let mut state = ArrayTree::new(span);
    self.consume_array_ilist(
      entries,
      incomplete_array_type,
      Vec::default(),
      &mut state,
    );

    let inferred_size = state.max_index.saturating_add(1);
    let qualified_type = ast::Type::Array(ast::Array::new(
      incomplete_array_type.as_array_unchecked().element_type,
      ast::ArraySize::Constant(inferred_size),
    ))
    .lookup(self.context())
    .into();
    let entries = self.materialize_array_entries(state, qualified_type);

    (
      sd::Initializer::List(sd::InitializerList::new(entries, span)),
      qualified_type,
    )
  }
}
/// Commons.
impl<'i, 'c> Initialization<'i, 'c> {
  fn list(
    &self,
    list: pd::InitializerList<'c>,
    target_type: QualifiedType<'c>,
  ) -> sd::InitializerList<'c> {
    let pd::InitializerList { entries, span } = list;

    let entries = match target_type.unqualified_type {
      init_type if init_type.is_scalar() => self.array(
        entries,
        ast::Type::Array(ast::Array::new(
          target_type,
          ast::ArraySize::Constant(1),
        ))
        .lookup(self.context())
        .into(),
      ),
      ast::Type::Array(_) => self.array(entries, target_type),
      _ => {
        todo!("struct/union initializer")
      },
    };
    sd::InitializerList::new(entries, span)
  }

  fn scalar(
    &self,
    expression: pe::Expression<'c>,
    target_type: QualifiedType<'c>,
  ) -> se::ExprRef<'c> {
    self
      .expression(expression)
      .map(|expr| self.assign_cvt_if_eligible(target_type, expr))
      .map(|expr| self.fold_if_eligible(expr))
      .handle_with(self, self.__empty_expr)
  }

  fn scalar_leaf_width(&self, target_type: QualifiedType<'c>) -> usize {
    match target_type.unqualified_type {
      ast::Type::Array(array) => match array.size {
        ast::ArraySize::Constant(size) =>
          size.saturating_mul(self.scalar_leaf_width(array.element_type)),
        ast::ArraySize::Incomplete | ast::ArraySize::Variable(_) => 0,
      },
      _ => 1,
    }
  }

  fn make_singleton_array_type(
    &self,
    element_type: QualifiedType<'c>,
  ) -> QualifiedType<'c> {
    ast::Type::Array(ast::Array::new(element_type, ast::ArraySize::Constant(1)))
      .lookup(self.context())
      .into()
  }

  fn rel_scalar_path_from_flat(
    &self,
    mut object_type: QualifiedType<'c>,
    mut flat_index: usize,
  ) -> (Vec<usize>, QualifiedType<'c>) {
    let mut path = Vec::default();

    // struct/union unimplemented
    while let ast::Type::Array(array) = object_type.unqualified_type {
      let stride = self.scalar_leaf_width(array.element_type).max(1);
      let index = flat_index / stride;
      path.push(index);
      flat_index %= stride;
      object_type = array.element_type;
    }

    (path, object_type)
  }

  fn consume_object_initializer(
    &self,
    initializer: pd::Initializer<'c>,
    target_type: QualifiedType<'c>,
    object_path: Vec<usize>,
    state: &mut ArrayTree<'c>,
    kind: Kind,
  ) {
    match target_type.unqualified_type {
      ast::Type::Array(_) => match initializer {
        pd::Initializer::InitializerList(list) => {
          let pd::InitializerList { entries, span } = list;
          let mut local_state = ArrayTree::new(span);
          self.consume_array_ilist(
            entries,
            target_type,
            Vec::default(),
            &mut local_state,
          );
          self.record_array_node(
            state,
            object_path,
            InitNode::List(local_state),
            kind,
          );
        },
        pd::Initializer::Expression(expression) => {
          // scalar-to-aggregate brace elision: initialize the first scalar leaf.
          let (mut rel_path, _) =
            self.rel_scalar_path_from_flat(target_type, 0);
          let mut full_path = object_path;
          full_path.append(&mut rel_path);
          self.record_array_node(
            state,
            full_path,
            InitNode::Scalar(expression),
            kind,
          );
        },
      },
      ast::Type::Record(_) | ast::Type::Union(_) => {
        self.add_error(
          UnsupportedFeature(
            "struct/union initializer not implemented yet".to_string(),
          ),
          initializer.span(),
        );

        match initializer {
          pd::Initializer::Expression(expression) => self.record_array_node(
            state,
            object_path,
            InitNode::Scalar(expression),
            kind,
          ),
          pd::Initializer::InitializerList(list) => {
            self.record_array_node(
              state,
              object_path,
              InitNode::List(ArrayTree::new(list.span)),
              kind,
            );
          },
        }
      },
      _ => match initializer {
        pd::Initializer::Expression(expression) => self.record_array_node(
          state,
          object_path,
          InitNode::Scalar(expression),
          kind,
        ),
        pd::Initializer::InitializerList(list) => {
          self.add_warning(ExcessBraceAroundScalarInitializer, list.span);

          let mut local_state = ArrayTree::new(list.span);
          let pseudo_scalar = self.make_singleton_array_type(target_type);

          self.consume_array_ilist(
            list.entries,
            pseudo_scalar,
            Vec::default(),
            &mut local_state,
          );

          self.record_array_node(
            state,
            object_path,
            InitNode::List(local_state),
            kind,
          );
        },
      },
    }
  }
}
/// Arrays.
impl<'i, 'c> Initialization<'i, 'c> {
  fn array(
    &self,
    entries: Vec<pd::InitializerListEntry<'c>>,
    array_type: QualifiedType<'c>,
  ) -> &'c [sd::InitializerListEntry<'c>] {
    let mut state = ArrayTree::new(Default::default());
    self.consume_array_ilist(entries, array_type, Vec::default(), &mut state);

    self.materialize_array_entries(state, array_type)
  }

  fn resolve_array_designator_path(
    &self,
    designators: Vec<pd::Designator<'c>>,
    mut target_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> (Vec<usize>, QualifiedType<'c>) {
    let mut resolved = Vec::default();

    for designator in designators {
      match designator {
        pd::Designator::Index(expression) => {
          let mut index =
            self.try_fold_to_usize(expression, span).unwrap_or(npos);

          match target_type.unqualified_type {
            ast::Type::Array(array) => {
              if let ast::ArraySize::Constant(bound) = array.size
                && index != npos
                && index >= bound
              {
                self.add_error(DesignatorIndexOutOfBound(index, bound), span);
                index = npos;
              }
              resolved.push(index);
              target_type = array.element_type;
            },
            _ => {
              if index != npos {
                self.add_error(
                  Custom(format!(
                    "designator [{}] cannot be applied to non-array type '{}'",
                    index, target_type
                  )),
                  span,
                );
              }
              resolved.push(npos);
              break;
            },
          }
        },
        pd::Designator::Field(field) => {
          self.add_error(InvalidDesignator(true, field.to_string()), span);
          break;
        },
      }
    }

    (resolved, target_type)
  }

  fn consume_array_ilist(
    &self,
    entries: Vec<pd::InitializerListEntry<'c>>,
    array_type: QualifiedType<'c>,
    prefix: Vec<usize>,
    state: &mut ArrayTree<'c>,
  ) {
    let mut cursor_flat: usize = 0;
    let element_scalar_width = self
      .scalar_leaf_width(array_type.as_array_unchecked().element_type)
      .max(1);

    for entry in entries {
      match entry {
        pd::InitializerListEntry::Designated(designated) => {
          let pd::Designated {
            designators,
            initializer,
            span,
          } = designated;

          let (rel_path, designated_type) =
            self.resolve_array_designator_path(designators, array_type, span);

          let mut rel_path = rel_path;
          if rel_path.is_empty() {
            rel_path.push(npos);
          }
          if rel_path.contains(&npos) {
            continue;
          }

          let first_index = rel_path[0];
          if first_index != npos {
            cursor_flat = first_index.saturating_mul(element_scalar_width);
          }

          let mut object_path = prefix.clone();
          object_path.extend(rel_path);
          self.consume_object_initializer(
            initializer,
            designated_type,
            object_path,
            state,
            Explicit,
          );

          if first_index != npos {
            cursor_flat = first_index
              .saturating_add(1)
              .saturating_mul(element_scalar_width);
          }
        },
        pd::InitializerListEntry::Initializer(initializer) => {
          self.consume_anonymous_array_entry(
            initializer,
            array_type,
            &prefix,
            &mut cursor_flat,
            element_scalar_width,
            state,
          );
        },
      }
    }
  }

  fn consume_anonymous_array_entry(
    &self,
    initializer: pd::Initializer<'c>,
    array_type: QualifiedType<'c>,
    prefix: &[usize],
    cursor_flat: &mut usize,
    element_scalar_width: usize,
    state: &mut ArrayTree<'c>,
  ) {
    let element_type = array_type.as_array_unchecked().element_type;

    let array_bound = array_type.as_array_unchecked().size_opt();
    let total_scalars =
      array_bound.map(|bound| bound.saturating_mul(element_scalar_width));

    match initializer {
      pd::Initializer::InitializerList(ref list) => {
        let object_index = *cursor_flat / element_scalar_width;
        if let Some(bound) = array_bound
          && object_index >= bound
        {
          self.add_warning(ExcessElemInInitializer, list.span);
          *cursor_flat = cursor_flat.saturating_add(element_scalar_width);
          return;
        }

        let mut object_path = prefix.to_vec();
        object_path.push(object_index);
        self.consume_object_initializer(
          initializer,
          element_type,
          object_path,
          state,
          Implicit,
        );

        let next_object =
          (*cursor_flat / element_scalar_width).saturating_add(1);
        *cursor_flat = next_object.saturating_mul(element_scalar_width);
      },
      pd::Initializer::Expression(expression) => {
        if let Some(total) = total_scalars
          && *cursor_flat >= total
        {
          self.add_warning(ExcessElemInInitializer, expression.span());
          *cursor_flat = cursor_flat.saturating_add(1);
          return;
        }

        if element_type.is_array() {
          let mut rel_path =
            self.rel_scalar_path_from_flat(array_type, *cursor_flat).0;
          let mut full_path = prefix.to_vec();
          full_path.append(&mut rel_path);
          self.record_array_node(
            state,
            full_path,
            InitNode::Scalar(expression),
            Implicit,
          );
        } else {
          let object_index = *cursor_flat;
          let mut full_path = prefix.to_vec();
          full_path.push(object_index);
          self.record_array_node(
            state,
            full_path,
            InitNode::Scalar(expression),
            Implicit,
          );
        }

        *cursor_flat = cursor_flat.saturating_add(1)
      },
    }
  }

  fn record_array_node(
    &self,
    state: &mut ArrayTree<'c>,
    path: Vec<usize>,
    node: InitNode<'c>,
    kind: Kind,
  ) {
    if path.is_empty() || path.contains(&npos) {
      return;
    }

    let span = node.span();

    if state.insert_path(&path, node, kind.is_implicit()) {
      /// Diag helper. render a designator path like `[0][2][3]`.
      fn render_array_path(path: &[usize]) -> String {
        if path.is_empty() {
          return String::from("<root>");
        }

        let mut rendered = String::with_capacity(path.len() * 4);

        use ::std::fmt::Write;
        let _idc_if_it_failed = path
          .iter()
          .try_for_each(|index| write!(rendered, "[{index}]"));
        rendered
      }
      self.add_warning(DuplicateInitializer(render_array_path(&path)), span);
    }
  }

  fn materialize_array_entries(
    &self,
    tree: ArrayTree<'c>,
    array_type: QualifiedType<'c>,
  ) -> &'c [sd::InitializerListEntry<'c>] {
    tree
      .entries
      .into_iter()
      .map(|entry| {
        let designators = [sd::Designator::Array(entry.index)]
          .into_iter()
          .collect_in::<ArenaVec<_>>(self.context().arena())
          .into_bump_slice();

        let initializer = self.materialize_node(
          entry.node,
          array_type.as_array_unchecked().element_type,
        );

        sd::InitializerListEntry::new(
          designators,
          initializer,
          entry.is_implicit,
        )
      })
      .collect_in::<ArenaVec<_>>(self.context().arena())
      .into_bump_slice()
  }

  fn materialize_node(
    &self,
    node: InitNode<'c>,
    target_type: QualifiedType<'c>,
  ) -> sd::Initializer<'c> {
    match node {
      InitNode::Scalar(expression) =>
        self.scalar(expression, target_type).into(),
      InitNode::List(tree) => {
        let list_target_type = if target_type.is_array() {
          target_type
        } else {
          self.make_singleton_array_type(target_type)
        };
        let span = tree.span;
        sd::InitializerList::new(
          self.materialize_array_entries(tree, list_target_type),
          span,
        )
        .into()
      },
    }
  }
}
/// helpers
impl<'i, 'c> Initialization<'i, 'c> {
  fn complete_array_type_from_string_if_eligible(
    &self,
    target_type: QualifiedType<'c>,
    expr: se::ExprRef<'c>,
  ) -> QualifiedType<'c> {
    let ast::Type::Array(array) = target_type.unqualified_type else {
      return target_type;
    };

    if !matches!(array.size, ast::ArraySize::Incomplete)
      || !Self::is_character_type(array.element_type)
    {
      return target_type;
    }

    let Some(required_size) =
      Self::string_literal_len(expr).map(|len| len.saturating_add(1))
    else {
      return target_type;
    };

    ast::Type::Array(ast::Array::new(
      array.element_type,
      ast::ArraySize::Constant(required_size),
    ))
    .lookup(self.context())
    .into()
  }

  fn is_character_type(target_type: QualifiedType<'c>) -> bool {
    matches!(
      target_type.unqualified_type,
      ast::Type::Primitive(
        ast::Primitive::Char | ast::Primitive::SChar | ast::Primitive::UChar
      )
    )
  }

  fn string_literal_len(expr: se::ExprRef<'c>) -> Option<usize> {
    expr
      .raw_expr()
      .as_constant()
      .and_then(|constant| constant.as_string())
      .map(|&string| string.len())
  }

  fn assign_cvt_if_eligible(
    &self,
    target_type: QualifiedType<'c>,
    expr: se::ExprRef<'c>,
  ) -> se::ExprRef<'c> {
    // NASTY EXCEPTION: character arrays initialized with strings
    if let ast::Type::Array(array) = target_type.unqualified_type
      && Self::is_character_type(array.element_type)
      && let Some(string_len) = Self::string_literal_len(expr)
    {
      if let Some(bound) = array.size_opt()
        && bound < string_len
      {
        self.add_error(
          Custom(format!(
            "initializer string requires array size at least {}, but target \
             size is {}",
            string_len, bound
          )),
          expr.span(),
        );
      }
      return expr;
    }

    let expr = expr.lvalue_conversion(self.context()).decay(self.context());

    if RefEq::ref_eq(target_type.unqualified_type, self.context().void_type()) {
      expr
    } else {
      expr
        .assignment_conversion(self.context(), &target_type)
        .handle_with(self, self.__empty_expr)
    }
  }

  fn fold_if_eligible(&self, expr: se::ExprRef<'c>) -> se::ExprRef<'c> {
    if !self.requires_folding {
      expr
    } else {
      expr
        .fold(self.session)
        .inspect_error(|&e| {
          if !e.raw_expr().is_empty() {
            // empty is error node currently
            self.add_error(
              ExprNotConstant(format!(
                "Expression {e} cannot be evaluated to a constant value"
              )),
              e.span(),
            )
          };
        })
        .take()
    }
  }

  fn try_fold_to_usize(
    &self,
    expression: pe::Expression<'c>,
    span: SourceSpan,
  ) -> Option<usize> {
    let analyzed = self
      .expression(expression)
      .handle_with(self, self.__empty_expr);

    if !analyzed.qualified_type().unqualified_type.is_integer() {
      self.add_error(
        NonIntegerInArraySubscript(analyzed.to_string()),
        analyzed.span(),
      );
      None?
    }

    use super::folding::FoldingResult::*;
    match analyzed.fold(self.session) {
      Success(expr) => {
        if !expr.is_integer_constant() {
          self.add_error(
            ExprNotConstant(format!(
              "array designator index '{}' is not an integer constant \
               expression",
              expr
            )),
            expr.span(),
          );
          None?
        }

        match expr.raw_expr() {
          se::RawExpr::Constant(se::Constant::Integral(integral)) => {
            if integral.is_negative() {
              self.add_error(
                DesignatorIndexNegative(integral.to_builtin()),
                span,
              );
              None?
            } else {
              Some(integral.to_builtin())
            }
          },
          _ => {
            self.add_error(
              NonIntegerInArraySubscript(expr.to_string()),
              expr.span(),
            );
            None?
          },
        }
      },
      Failure(expr) => {
        self.add_error(
          ExprNotConstant(format!(
            "array designator index '{}' is not an integer constant expression",
            expr
          )),
          expr.span(),
        );
        None?
      },
    }
  }
}
