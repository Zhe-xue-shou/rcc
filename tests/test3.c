int getchar();
int getchar(void);

int main() {
  // __auto_type ptr = getchar;
  {
    int *a;
    int x;
    x = getchar();
    int b = a[2];
    int c = *(a + 3);

    // int d = *c;
    {
      if (a)
        0;
      int a = 0;
      a += 3;
      float b = 0;
      if (a >= 3)
        b += c;
    }
  }

  short c = 1;
  int a[10][100][1000];
  auto ptr = a;
  auto b = a[1][2][3];
  auto d = b && c;
}
