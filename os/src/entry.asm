# .section .data
# float_val: .float 23.1    
    .attribute arch, "rv64gc"
    .section .text.entry
    .globl _start
_start:
    # do little arithmetic with floating point numbers
    # la a0, float_val  # Load address of float_val into a0
    # flw ft0, 0(a0)    # Load the floating-point value into ft0

    la sp, boot_stack_top
    call kmain

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top: