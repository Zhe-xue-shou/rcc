use ::std::{cell::RefCell, ops::Deref};

use crate::Bump;

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
  pub fn alloc<T: ::std::fmt::Debug>(&self, val: T) -> &mut T {
    fn _print_meta<T: ::std::fmt::Debug>(val: &T) {
      println!("Alloc for {}:  {:?}", ::std::any::type_name::<T>(), val);
    }

    // _print_meta(&val);

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
