
extern int j;
static int j = 0;
extern int j;
int j;

//// this is also ok???
// extern int k[10];
// int k[];
// extern int k[10];
inline static static int foo(int a) { return a + 1; }
int main(void) { //
  // goto label;
  {
    // label:;
    int k = foo(j);
  }
  int f(int, int);
  // typedef int INT;
  // INT x = 5;
  // static int y = 1;
  // switch (x) {
  // case 0:
  // case 1:
  //   y = y + 1;
  //   x = x + 1;
  //   break;
  // default:
  //   y = y + 2;
  // }
  return f(2, 3);
}

int f(int i, int j) {
  // label:;
  int k = i + j;
  return k;
}