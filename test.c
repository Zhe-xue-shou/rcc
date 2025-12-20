// // int f();     // FunctionNoProto in clang AST
// // int g(void); // FunctionProto in clang AST
// inline volatile int *h(const volatile int *x) { return x; }
// int main(void) {
//   int a = 12345;
//   int i;

//   typedef volatile int *(*p)(const volatile int *);

//   for (i = 5; i >= 0; i = i - 1)
//     a = a / 3;
//   h(&a);
//   return a;
// }
int foo(int a, ) { return a + 1; }
int main(void) { //
  int f(int, int);
  return f(2, 3);
}

int f(int i, int j) {
  int k = i + j;
  return k;
}