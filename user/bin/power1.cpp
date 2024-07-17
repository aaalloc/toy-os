// fast power algorithm
#include "src/syscall.h"
#include <cstdio>
int power(int x, int n)
{
    if (n == 0)
        return 1;
    if (n % 2 == 0)
        return power(x * x, n / 2);
    else
        return x * power(x * x, n / 2);
}

int main(void)
{
    int x = 2;
    int n = 10;
    int result = power(x, n);
    write(1, "2^10 = ", 7);
    write(1, &result, 1);
    write(1, "\n", 1);
    exit(0);
}
