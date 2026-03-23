int getchar();
int main() {
  int i = getchar();
  // int j;
  // if (i) {
  //   if (i == 10) {
  //     j = 20;
  //   } else {
  //     j = 40;
  //   }
  //   i = getchar();
  // }
  // int i;
  // i = 10;
  do {
    int j = getchar();
    if (j == 10) {
      break;
    }
    continue;
    i = i - 1;
  } while (i);
  return 0;

  // for (int j = getchar(); j < 10; j = j + 1) {
  //   i = i + 1;
  // }
  // return i;
}
