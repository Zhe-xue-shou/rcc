pub struct StaticStr<const N: usize>(pub [u8; N]);

impl<const N: usize> StaticStr<N> {
  #[inline(always)]
  pub const fn as_str(&self) -> &str {
    // SAFETY: safe.
    unsafe { str::from_utf8_unchecked(&self.0) }
  }
}

#[macro_export]
macro_rules! concat_static_str {
  () => { "" };

  ($($strs:expr),+ $(,)?) => {{

    const TOTAL_LEN: usize = 0 $(+ $strs.len())+;

    #[allow(unused_assignments, reason = "when there are only 1 or 2 str passed, the offset would be marked as unused.")]
    const RESULT: $crate::StaticStr<TOTAL_LEN> = {
        let mut buf = [0u8; TOTAL_LEN];
        let mut offset = 0;
        $(
            let s_bytes = $strs.as_bytes();
            let mut i = 0;
            while i < s_bytes.len() {
                buf[offset + i] = s_bytes[i];
                i += 1;
            }
            offset += s_bytes.len();
        )+

        $crate::StaticStr(buf)
    };
    RESULT.as_str()
  }};
}

#[cfg(test)]
mod tests {
  use crate::static_assert_eq;
  #[test]
  fn basic_concat() {
    const PREFIX: &str = "Hello, ";
    const SUFFIX: &str = "World!";
    const FULL_KEY: &str = concat_static_str!(PREFIX, SUFFIX);
    static_assert_eq!(FULL_KEY, "Hello, World!");
  }

  #[test]
  fn multiple_concat() {
    const PART1: &str = "The quick ";
    const PART2: &str = "brown fox ";
    const PART3: &str = "jumps over ";
    const PART4: &str = "the lazy dog.";
    const FULL_SENTENCE: &str = concat_static_str!(PART1, PART2, PART3, PART4);
    static_assert_eq!(
      FULL_SENTENCE,
      "The quick brown fox jumps over the lazy dog."
    );
  }
}
