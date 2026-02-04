int *normal_ptr;
const int *ptr_to_const;
int const *ptr_to_const2;
int *const const_ptr;
const int *const const_ptr_to_const;

int **ptr_to_ptr;
int *const *ptr_to_const_ptr;
int **const const_ptr_to_ptr;
const int **ptr_to_ptr_to_const;
int *const *const const_ptr_to_const_ptr;
const int **const const_ptr_to_ptr_to_const;
const int *const *ptr_to_const_ptr_to_const;
const int *const *const const_ptr_to_const_ptr_to_const;

// well, if this passed parsing, it might be... ok ig
static const volatile int **const *const
    *volatile volatile_ptr_to_very_const_ptr_to_very_const_ptr;
// func ptr test
extern int j;
static int j = 0;
extern int j;
int j;

// this is also... weird but valid
extern int k[10];
int k[];
extern int k[10];
typedef int INT;
typedef int const CONST_INT;
int (*func_ptr)(INT, CONST_INT);
inline static int foo(int a) { return a + 1; }
int main(int argc, char **argv) { //
  goto label;
  {
  label:;
    int k = foo(0);
  }
  int f(int, int);
  typedef int const CONST_INT;
  INT x = sizeof(char);
  typedef int const CONST_INT;
  int foo;
  CONST_INT(INT) = (10);
  static int y = sizeof x;
  switch (x) {
  case 3.0 / 5.0:
  case 2147483647 + 1:
    y = y + 1;
    x = x + 1;
    break;
  default:
    y = y + 2;
  }
  for (int i = 0; i < 10; i = i + 1) { // my parser can't handle += and ++
    y = y + i;
    continue;
  }
  const int a = 2.0 / 3;
  return f(2, 3);
}

int f(int i, int j) {
  int k = i + j;
label:
  k = k + 1;
  int *(ptr_to_k) = &k;
  // ptr_to_k = ptr_to_k + 1;
  float a = 1.0;
  unsigned int u1 = 10U;
  unsigned int u2 = 20U;
  unsigned int res = u1 - u2;
  typedef int (*FUNC_PTR)(int, int);
  goto label;
  return k;
}
void ff(double (*restrict a)[5]);
void ff(double a[restrict][5]);
void ff(double a[restrict 3][5]);
void ff(double a[restrict static 3][5]);
int p(int a[*]);
int p(int a[static 10]) { return 0; }

// Error: Second dimension mismatch
void f2(int a[][5]);
void f2(int a[][5]); // ERROR: 'int(*)[5]' vs 'int(*)[10]'