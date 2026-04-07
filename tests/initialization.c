// Valid: 'a', 'b', 'c', and implicit '\0'
char s1[] = "abc";

// Error: String too long for the array
char s2[2] = "abc";

// Valid: 'a', 'b', 'c'. No null terminator added because size is explicit.
char s3[3] = "abc";

// Lawfully valid :)
char s4[4] = "abc";

char many_chars[][3] = {"abc", "de", "f"}; // Valid: 3 rows, 3 columns

// Valid: Flattened (equivalent to {{1, 2}, {3, 0}})
int a[2][2] = {1, 2, 3};

// Valid: Partially braced
int b[2][2] = {{1}, [1][1] = 2, 3};

// Valid: Excess braces with excess elems
int c[2] = {{1, 2}, {2}};

// Array size will be 101 (indices 0 to 100)
int wide[] = {[100] = 1};

// Array size will be 11
int mixed[] = {1, 2, [10] = 3};

// Warning: Excess elements
int err1[2] = {1, 2, 3};

// Error: Designator index outside array bounds
int err2[5] = {[10] = 1};

// Error: Negative index
int err3[5] = {[-1] = 1};

int err4[3] = {{[0] = 0}, {{{{{{{{{{1}}}}}}}}}}, {{{{{{{}}}}}}}};