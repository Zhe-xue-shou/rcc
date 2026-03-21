use ::bimap::BiHashMap;
use ::bumpalo::Bump;
use ::slotmap::{DenseSlotMap, SlotMap};
use ::std::{cell::RefCell, collections::HashSet, ops::Deref};

use crate::{common::StrRef, ir, types};

type Interner<T> = RefCell<HashSet<T>>;
#[derive(Debug)]
pub struct Storage<'c> {
  pub ast_arena: &'c Arena,
  pub ir_arena: RefCell<SlotMap<ir::ValueID, ir::Value<'c>>>,
  pub ir_def_use: RefCell<DenseSlotMap<ir::ValueID, ir::Value<'c>>>,

  pub ast_type_interner: Interner<types::TypeRef<'c>>,
  pub str_interner: Interner<StrRef<'c>>,
  pub ir_type_interner: Interner<ir::TypeRef<'c>>,
  /// currently only for ir stage. use it in previous stage could cause unprecedented catastrophe. see the git stash.
  pub constant_interner: RefCell<BiHashMap<ir::ValueID, types::Constant<'c>>>,
}

impl<'c> Storage<'c> {
  pub fn new(arena: &'c Arena) -> Self {
    Self {
      ast_arena: arena,
      ir_arena: Default::default(),
      ir_def_use: Default::default(),
      ast_type_interner: Default::default(),
      ir_type_interner: Default::default(),
      str_interner: Default::default(),
      constant_interner: Default::default(),
    }
  }
}

pub type StorageRef<'c> = &'c Storage<'c>;

type DropFn = unsafe fn(*mut u8);
#[derive(Debug, Default)]
pub struct Arena {
  bump: Bump,
  registry: RefCell<Vec<(*mut u8, DropFn)>>,
  #[cfg(debug_assertions)]
  counter: RefCell<usize>,
}
impl Deref for Arena {
  type Target = Bump;

  fn deref(&self) -> &Self::Target {
    &self.bump
  }
}
impl Arena {
  pub fn alloc<T>(&self, val: T) -> &mut T {
    let ptr = self.bump.alloc(val);

    if const { ::std::mem::needs_drop::<T>() } {
      if const { cfg!(debug_assertions) } {
        static THRESHOLD: usize = 16;
        *self.counter.borrow_mut() += 1;
        if *self.counter.borrow() >= THRESHOLD {
          eprintln!(
            "Error: registered too much needs_drop elems into the bump; \
             perhaps you bumped the wrong type? {}",
            self.counter.borrow()
          );
        }
      }

      unsafe fn drop_fn<T>(ptr: *mut u8) {
        unsafe { ::std::ptr::drop_in_place(ptr as *mut T) };
      }

      self
        .registry
        .borrow_mut()
        .push((&raw mut *ptr as *mut u8, drop_fn::<T>));
    }

    ptr
  }
}
impl Drop for Arena {
  fn drop(&mut self) {
    self
      .registry
      .borrow_mut()
      .iter()
      .rev()
      .for_each(|(ptr, drop_fn)| unsafe {
        drop_fn(*ptr);
      });
  }
}
