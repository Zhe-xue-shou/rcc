
// this is also... weird but valid
extern int k[10];
int k[];
extern int k[10];
typedef int INT;
typedef int const CONST_INT;
int (*func_ptr)(INT, CONST_INT);
int func(INT a, CONST_INT b) { return a + b; }
inline static int foo(int a) { return a + 1; }
int p = 0 ? 1, 0 : 2;
int f(int i, int j) {
  // k[0] = i + j;
  p++;
  p += 9;
  // 1 ++;
  func_ptr &&func_ptr;
  int k = i + j;
  k = foo(0);

label:
  k = k + 1;
  int *(ptr_to_k) = &k;
  // ptr_to_k = ptr_to_k + 1;
  float a = 1.0;
  unsigned int u1 = 10U;
  unsigned int u2 = 20U;
  unsigned int res = u1 - u2;
  typedef int (*FUNC_PTR)(int, int);
  FUNC_PTR p = &func;
  k = p(2, 3);
  goto label;
  return k;
}
// void ff(double (*restrict a)[5]);
// void ff(double a[restrict][5]);
// void ff(double a[restrict 3][5]);
// void ff(double a[restrict static 3][5]);
// int p(int a[*]);
// int p(int a[static 10]) { return 0; }

// // Error: Second dimension mismatch
// void f2(int a[][5]);
// void f2(int a[][10]); // ERROR: 'int(*)[5]' vs 'int(*)[10]'