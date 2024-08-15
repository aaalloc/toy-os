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
