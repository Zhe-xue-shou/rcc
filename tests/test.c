void f(void (*(*p)())());
void p(void p, void t);
void g(void);
int main() { const auto Y = sizeof(f); }