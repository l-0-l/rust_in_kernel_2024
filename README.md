# Meetup materials from a Rust for Linux kernel 2024
## All the files I've added and edited in Linux kernel v6.11
- helpers.c
  - Location: rust/
  - Added custom wrappers for kfifo macros and Input subsystem inline functions
- bindigs_helper.h
  - Location: rust/bindings/
  - Added headers needed for the commands and structs I used for TTY and Input devices handling
- Kconfig
  - Location: samples/rust/
  - Since I am building the module in-tree, added the relevant entry for it to Kconfig
- Makefile
  - Location: samples/rust/
  - Also must add it to the Makefile so that it gets built
- rust_leon.rs
  - Location: samples/rust/
  - The module which communicates with the micro:bit with line discipline and input device
## Relevant slides with the module structure outline
- ![slide19.png](https://github.com/l-0-l/rust_in_kernel_2024/blob/main/slide19.png)
- ![slide20.png](https://github.com/l-0-l/rust_in_kernel_2024/blob/main/slide20.png)
## The code for the `micro:bit`.
- micro_bit_code.rs
  - Use my repo from the previous meetup, the ex4 one [rust-examples-microbit-2024](https://github.com/l-0-l/rust-examples-microbit-2024/tree/main/ex4_accelerometer/src)
  - Copy the code from this file to the main.rs you have there, I made a few modifications.
## My workflow for the meetup
- workflow.md
  - Mostly the commands I used to set everything up. Remember, YMMV! 
