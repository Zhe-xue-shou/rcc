// Valid: 'a', 'b', 'c', and implicit '\0'
char s1[] = "abc";

// Valid: 'a', 'b', 'c'. No null terminator added because size is explicit.
char s2[4] = "abc";

// Error: String too long for the array
char s3[2] = "abc";

char many_chars[][3] = {"abc", "de", "f"}; // Valid: 3 rows, 3 columns

// Valid: Flattened (equivalent to {{1, 2}, {3, 0}})
int a[2][2] = {1, 2, 3};

// Valid: Partially braced
int b[2][2] = {{1}, [1][1] = 2, 3};

// Valid: Excess braces (should usually trigger a warning)
int c[2] = {{1}, {2}};

// Array size will be 101 (indices 0 to 100)
int wide[] = {[100] = 1};

// Array size will be 11
int mixed[] = {1, 2, [10] = 3};

int err1[2] = {1, 2, 3};  // Warning: Excess elements
int err2[5] = {[10] = 1}; // Error: Designator index outside array bounds
int err3[5] = {[-1] = 1}; // Error: Negative index
int getchar();
void y() {
  int x = getchar();
  int y[][] = {1, 2, 3, 4};
}