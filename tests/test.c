
void f(void) {
  int p = {};
  int a[3] = {
      0,
      1,
      2,

  };
  int b[3][2] = {{0, 1}, {2, 3}, 4, 5};
  // int b[] = {[0] = p, [4] = 8, 3, [1] = 23};
  // int c[][2] = {[0][1] = 10, [0] = {[0] = 0, [1] = 2}, {20, 30, 30}, 20, 30,
  //               .f = "123"};
}
// struct Foo {
//   int x;
//   int y;
// };

// struct Bar {
//   struct Foo f;
//   int z;
// };

// struct Bar b = {.f.x = 1, .f = {.y = 9}, 2};

// struct A {
//   int b;
//   int c;
// };
// struct A d = {.b = 10, 10};