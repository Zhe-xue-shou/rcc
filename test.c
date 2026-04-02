int getchar();
int getchar(void);

int main() {
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
  __auto_type ptr = a;
  __auto_type b = a[1][2][3];
  __auto_type d = b && c;
}
