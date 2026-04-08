void f(void (*(*p)())());
void g(void);
int main() {
  void (*(*p)(void))(void);
  void (*q)();
  const auto Y = sizeof(void (*(*)(void))(void));
  int b[2][2] = {{1}, [1][1] = 2};
  int c[100] = {};
}