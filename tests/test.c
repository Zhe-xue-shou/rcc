int y = 10;

int g(int);
int main() {
  const auto d = g;
  return d(y) & y;
}