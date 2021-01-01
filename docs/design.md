# design

`mu` is as small of a microkernel as i can get away with. we aim to delegate
all blocking tasks to userspace if possible, and just run the main sections
of the kernel with interrupts disabled.

as such we definitely take inspiration from the L4 series of microkernels in the
outside world

our threading model in kernel is that we have the Courage™ (foolishness,
probably) to use small locks in-kernel on each structure. we do not share page
tables across threads, so accesses to those can be unsynchronized.

## goals

* i want to be able to write a web server serving files off the disk of this
  thing as a final goal -> fs driver and net stack
* figure out how to make a fully custom target for rust for our userland
* maybe design a cool ipc message format?
* it would be fun to have privilege separation
* it might be cool to prototype a fast snapshotting mechanism for processes that
  conceptually could maybe be ported to linux?

## non goals

* portability. i mean, it would be cool but i first need to prove i can write a
  kernel that works
* actually be useful,