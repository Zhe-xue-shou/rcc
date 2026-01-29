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

// this is also ok???
// extern int k[10];
// int k[];
// extern int k[10];
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
  CONST_INT INT = (10);
  static int y = sizeof x;
  switch (x) {
  case 0:
  case 1:
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
  return f(2, 3);
}

int f(int i, int j) {
label:;
  int k = i + j;
  int *(ptr_to_k) = &k;
  float a = 1.0;
  goto label;
  return k;
}