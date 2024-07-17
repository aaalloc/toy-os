constexpr int SYSCALL_READ = 63;
constexpr int SYSCALL_WRITE = 64;
constexpr int SYSCALL_EXIT = 93;
constexpr int SYSCALL_YIELD = 124;

int syscall(int id, int arg1, int arg2, int arg3)
{
    int ret;

    /*
    asm!(
        "ecall",
        inlateout("x10") args[0] => ret,
        in("x11") args[1],
        in("x12") args[2],
        in("x17") id
    );
    */
    // clang-format off
    asm volatile("ecall"
                 : "+r"(ret)
                 : "r"(arg1), "r"(arg2), "r"(arg3), "r"(id)
                 : "memory");
    // clang-format on
    return ret;
}

int read(int fd, void *buf, int count)
{
    //
    return syscall(SYSCALL_READ, fd, (int)buf, count);
}

int write(int fd, const void *buf, int count)
{
    //
    return syscall(SYSCALL_WRITE, fd, (int)buf, count);
}

void exit(int status)
{
    //
    syscall(SYSCALL_EXIT, status, 0, 0);
}

void yield()
{
    //
    syscall(SYSCALL_YIELD, 0, 0, 0);
}