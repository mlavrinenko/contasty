#include <stdio.h>
#include <string.h>

/* A small calculator. */
typedef struct {
    int total;
} Calculator;

static const int SEEDS[3] = { 1, 2, 3 };
static const char *BANNER = "Calculator";

// Add two integers.
int calc_add(int a, int b) {
    return a + b;
}

int calc_total(const Calculator *c) {
    return c->total;
}

void test_add(void) {
    if (calc_add(1, 2) != 3) {
        printf("bad\n");
    }
}
