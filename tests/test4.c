void f(void (*(*p)())());
void execute(int decay4func2ptr(int[]), int decay4arr2ptr[]) {
  decay4func2ptr(decay4arr2ptr);
};
void h(int a[const]); // or `volatile`, `restrict`.
void h(int *const a); // adjusted.
// void h(const int a[]); // incompatible with above.
int main(int, char *[]) {
  void (*(*p)(void))(void);
  void (*q)();
  const auto Y = sizeof(void (*(*)(void))(void));
}