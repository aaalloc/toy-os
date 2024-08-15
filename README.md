# toy-os
Toy project around an kernel written in Rust/RISC-V

## Feature
- [x] Booting
- [x] Memory management
- [x] Process management
- [x] User space
- [x] Shell
- [x] File system
- Drivers (thanks to virtio)
  - [x] UART
  - [x] Block device
  - [ ] Network


https://github.com/user-attachments/assets/c0c72448-4394-4103-9d39-1057909a5819

## Run the os
You should have QEMU and rust tools (rustup, cargo, ...) installed before trying to compile/run anything.

```bash
$ git clone git@github.com:aaalloc/toy-os.git
$ cd toy-os/os
$ make run
# with log
$ LOG=INFO make run
```


## Ressources
Mainly used https://github.com/rcore-os/rCore-Tutorial-v3

and
- https://os.phil-opp.com/
- https://osblog.stephenmarz.com
