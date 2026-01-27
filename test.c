int *p;
int const *ptr;
// func ptr test
// int (*func_ptr)(int, int);
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
inline static static int foo(int a) { return a + 1; }
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
  for (int i = 0; i < 10; i = i + 1) { // my parser can't handle += and ++ yet
    y = y + i;
    continue;
  }
  return f(2, 3);
}

int f(int i, int j) {
label:;
  int k = i + j;
  float a = 1.0;
  goto label;
  return k;
}