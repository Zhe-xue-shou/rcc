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
#[derive(Debug)]
struct ArrayWrite<'c> {
  path: Vec<usize>,
  expression: pe::Expression<'c>,
  kind: Kind,
}

impl<'c> ArrayWrite<'c> {
  fn new(path: Vec<usize>, expression: pe::Expression<'c>, kind: Kind) -> Self {
    Self {
      path,
      expression,
      kind,
    }
  }
}

#[derive(Debug, Default)]
struct ArrayInitState<'c> {
  writes: Vec<ArrayWrite<'c>>,
  seen_paths: HashMap<Vec<usize>, usize>,
  max_top_index: Option<usize>,
}

impl<'c> ArrayInitState<'c> {
  // fn new(
  //   writes: Vec<ArrayWrite<'c>>,
  //   seen_paths: HashMap<Vec<usize>, usize>,
  //   max_top_index: Option<usize>,
  // ) -> Self {
  //   Self {
  //     writes,
  //     seen_paths,
  //     max_top_index,
  //   }
  // }
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
    (scalar.into(), *scalar.qualified_type())
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

    let mut state = ArrayInitState::default();
    self.consume_array_initializer_list(
      entries,
      incomplete_array_type,
      Vec::new(),
      &mut state,
    );

    let inferred_size = state.max_top_index.map_or(0, |m| m + 1);
    let qualified_type = ast::Type::Array(ast::Array::new(
      incomplete_array_type.as_array_unchecked().element_type,
      ast::ArraySize::Constant(inferred_size),
    ))
    .lookup(self.context())
    .into();
    let entries = self.materialize_array_entries(state.writes, qualified_type);

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

  /// struct/union unimplemented
  fn rel_scalar_path_from_flat(
    &self,
    mut object_type: QualifiedType<'c>,
    mut flat_index: usize,
  ) -> (Vec<usize>, QualifiedType<'c>) {
    let mut path = Vec::new();

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
    state: &mut ArrayInitState<'c>,
    kind: Kind,
  ) {
    match target_type.unqualified_type {
      ast::Type::Array(_) => match initializer {
        pd::Initializer::InitializerList(list) => {
          self.consume_array_initializer_list(
            list.entries,
            target_type,
            object_path,
            state,
          );
        },
        pd::Initializer::Expression(expression) => {
          // scalar-to-aggregate brace elision: initialize the first scalar leaf.
          let (mut rel_path, _) =
            self.rel_scalar_path_from_flat(target_type, 0);
          let mut full_path = object_path;
          full_path.append(&mut rel_path);

          self.record_array_write(state, full_path, expression, kind);
        },
      },
      ast::Type::Record(_) | ast::Type::Union(_) => {
        self.add_error(
          UnsupportedFeature(
            "struct/union initializer not implemented yet".to_string(),
          ),
          initializer.span(),
        );
      },
      _ => {
        if let Some(expression) =
          self.extract_scalar_expression(initializer, target_type)
        {
          self.record_array_write(state, object_path, expression, kind);
        }
      },
    }
  }

  /// FIXME: this function works but not ideal...
  fn extract_scalar_expression(
    &self,
    initializer: pd::Initializer<'c>,
    target_type: QualifiedType<'c>,
  ) -> Option<pe::Expression<'c>> {
    match initializer {
      pd::Initializer::Expression(expression) => Some(expression),
      pd::Initializer::InitializerList(list) => {
        let pd::InitializerList { mut entries, span } = list;
        if entries.is_empty() {
          // self.add_error(
          //   Custom(
          //     "empty initializer list cannot initialize a scalar".to_string(),
          //   ),
          //   span,
          // );
          // return None;
          todo!(
            "not an error, empty init here shall mean all zeroinit/default \
             init"
          )
        }

        if entries.len() > 1 {
          self.add_warning(ExcessElemInInitializer, span);
        }

        let first = entries.remove(0);
        match first {
          pd::InitializerListEntry::Initializer(initializer) =>
            self.extract_scalar_expression(initializer, target_type),
          pd::InitializerListEntry::Designated(designated) => {
            self.add_error(
              DesignatorForNonAggregate(target_type.to_string()),
              designated.span,
            );
            self.extract_scalar_expression(designated.initializer, target_type)
          },
        }
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
    let mut state = ArrayInitState::default();
    self.consume_array_initializer_list(
      entries,
      array_type,
      Vec::new(),
      &mut state,
    );

    self.materialize_array_entries(state.writes, array_type)
  }

  fn resolve_array_subobject_type(
    &self,
    mut array_type: QualifiedType<'c>,
    path: &[usize],
  ) -> QualifiedType<'c> {
    for _ in path {
      match array_type.unqualified_type {
        ast::Type::Array(array) => {
          array_type = array.element_type;
        },
        _ => unreachable!(),
      }
    }

    array_type
  }

  fn resolve_array_designator_path(
    &self,
    designators: Vec<pd::Designator<'c>>,
    mut target_type: QualifiedType<'c>,
    span: SourceSpan,
  ) -> (Vec<usize>, QualifiedType<'c>) {
    let mut resolved = Vec::new();

    for designator in designators {
      match designator {
        pd::Designator::Index(expression) => {
          let index = self.try_fold_to_usize(expression, span);

          match target_type.unqualified_type {
            ast::Type::Array(array) => {
              if let ast::ArraySize::Constant(bound) = array.size
                && let Some(index) = index
                && index >= bound
              {
                self.add_error(DesignatorIndexOutOfBound(index, bound), span);
              }
              resolved.push(index.unwrap_or(npos));
              target_type = array.element_type;
            },
            _ => {
              if let Some(index) = index {
                self.add_error(
                  Custom(format!(
                    "designator [{}] cannot be applied to non-array type '{}'",
                    index, target_type
                  )),
                  span,
                );
              }
              break;
            },
          }
        },
        pd::Designator::Field(field) => {
          self.add_error(
            UnsupportedFeature(format!(
              "field designator '.{}' is not implemented yet (array-only path)",
              field
            )),
            span,
          );
          break;
        },
      }
    }

    (resolved, target_type)
  }

  fn consume_array_initializer_list(
    &self,
    entries: Vec<pd::InitializerListEntry<'c>>,
    array_type: QualifiedType<'c>,
    prefix: Vec<usize>,
    state: &mut ArrayInitState<'c>,
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

          let Some(&first_index) = rel_path.first() else {
            // self.add_error(Custom("empty designator list".to_string()), span);
            // continue;
            todo!("not an error...")
          };

          if let Some(bound) = array_type.as_array_unchecked().size.size_opt()
            && first_index >= bound
          {
            self.add_error(DesignatorIndexOutOfBound(first_index, bound), span);
            continue;
          }

          cursor_flat = first_index.saturating_mul(element_scalar_width);

          let mut object_path = prefix.clone();
          object_path.extend(rel_path);
          self.consume_object_initializer(
            initializer,
            designated_type,
            object_path,
            state,
            Explicit,
          );

          cursor_flat = first_index
            .saturating_add(1)
            .saturating_mul(element_scalar_width);
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
    state: &mut ArrayInitState<'c>,
  ) {
    let array_bound = array_type.as_array_unchecked().size.size_opt();
    let total_scalars =
      array_bound.map(|bound| bound.saturating_mul(element_scalar_width));

    match initializer {
      pd::Initializer::InitializerList(ref list) => {
        let object_index = *cursor_flat / element_scalar_width;
        if let Some(bound) = array_bound
          && object_index >= bound
        {
          self.add_warning(ExcessElemInInitializer, list.span);
          // return;
        }

        let mut object_path = prefix.to_vec();
        object_path.push(object_index);
        self.consume_object_initializer(
          initializer,
          array_type.as_array_unchecked().element_type,
          object_path,
          state,
          Implicit,
        );

        *cursor_flat = object_index
          .saturating_add(1)
          .saturating_mul(element_scalar_width);
      },
      pd::Initializer::Expression(expression) => {
        if let Some(total) = total_scalars
          && *cursor_flat >= total
        {
          self.add_warning(ExcessElemInInitializer, expression.span());
          // return;
        }

        if array_type.as_array_unchecked().element_type.is_array() {
          let (mut rel_path, _) =
            self.rel_scalar_path_from_flat(array_type, *cursor_flat);
          let mut full_path = prefix.to_vec();
          full_path.append(&mut rel_path);
          self.record_array_write(state, full_path, expression, Implicit);
        } else {
          let object_index = *cursor_flat;

          if let Some(bound) = array_bound
            && object_index >= bound
          {
            self.add_warning(ExcessElemInInitializer, expression.span());
            // return;
          }
          let mut full_path = prefix.to_vec();
          full_path.push(object_index);
          self.record_array_write(state, full_path, expression, Implicit);
        }

        *cursor_flat += 1;
      },
    }
  }

  fn record_array_write(
    &self,
    state: &mut ArrayInitState<'c>,
    path: Vec<usize>,
    expression: pe::Expression<'c>,
    kind: Kind,
  ) {
    if let Some(previous_index) = state.seen_paths.get(&path).copied() {
      self.add_warning(
        DuplicateInitializer(Self::render_array_path(&path)),
        expression.span(),
      );
      state.writes[previous_index] =
        ArrayWrite::new(path.clone(), expression, kind);
    } else {
      let slot = state.writes.len();
      state.seen_paths.insert(path.clone(), slot);
      state
        .writes
        .push(ArrayWrite::new(path.clone(), expression, kind));
    }

    if let Some(&top) = path.first() {
      state.max_top_index = Some(match state.max_top_index {
        Some(prev) => prev.max(top),
        None => top,
      });
    }
  }

  fn materialize_array_entries(
    &self,
    writes: Vec<ArrayWrite<'c>>,
    array_type: QualifiedType<'c>,
  ) -> &'c [sd::InitializerListEntry<'c>] {
    writes
      .into_iter()
      .map(|write| {
        let target_type =
          self.resolve_array_subobject_type(array_type, &write.path);

        let designators = write
          .path
          .iter()
          .copied()
          .map(sd::Designator::Array)
          .collect_in::<ArenaVec<_>>(self.context().arena())
          .into_bump_slice();
        let is_implicit = match write.kind {
          Implicit => true,
          Explicit => false,
        };
        sd::InitializerListEntry::new(
          designators,
          self.scalar(write.expression, target_type).into(),
          is_implicit,
        )
      })
      .collect_in::<ArenaVec<_>>(self.context().arena())
      .into_bump_slice()
  }
}
/// helpers
impl<'i, 'c> Initialization<'i, 'c> {
  fn assign_cvt_if_eligible(
    &self,
    target_type: QualifiedType<'c>,
    expr: se::ExprRef<'c>,
  ) -> se::ExprRef<'c> {
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
        .inspect_error(|e| {
          self.add_error(
            ExprNotConstant(format!(
              "Expression {e} cannot be evaluated to a constant value"
            )),
            e.span(),
          );
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

  fn render_array_path(path: &[usize]) -> String {
    if path.is_empty() {
      return String::from("<root>");
    }

    let mut rendered = String::with_capacity(path.len() * 4);

    use ::std::fmt::Write;
    _ = path
      .iter()
      .try_for_each(|index| write!(rendered, "[{index}]"));
    rendered
  }
}
