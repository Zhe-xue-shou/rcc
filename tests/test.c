constexpr auto P = 1;
void f(void) {
  int p = 0;
  int a[2] = {
      1,
      2,
      3,
  };
  int b[*] = {[0] = p, [4] = 8, [P] = 3, 23};
}

// struct A {
//   int b;
//   int c;
// };
// struct A d = {.b = 10, 10};