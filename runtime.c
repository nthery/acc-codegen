// Entry point that calls function generated by compiler and print out its
// output.

#include <stdio.h>

extern int evaluate(void);

int main(void) {
	int n = evaluate();
	printf("%d\n", n);
	return 0;
}
