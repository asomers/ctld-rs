# CTLD-rs

A rewrite of FreeBSD's ctld(8) in Rust.

## Overview

`ctld` (CAM Target Layer Daemon) is the FreeBSD daemon that primarily provides
iSCSI targets.  This project is an experimental rewrite from C to Rust.  The
rewrite's benefits include:

* Greatly simplified UCL parsing.
* Greatly simplified XML parsing.
* RAII for creating LUNs and Targets.
* Automatically fixes whole classes of memory handling bugs.  See FreeBSD bugs
  214874, 274380, 279699, 246596, and commit
  2909ddd17cb4d750852dc04128e584f93f8c5058.
* Generally all-around simplified code.
* Unit tests for the kernel XML and config file parsing.
* Unit tests for the kernel API.  Crucially, these tests are based on mock
  objects, so they don't require root to run, don't modify the OS state, run
  blisteringly fast, and don't require any special hardware.  The mocking
  technique is impossible in ctld's current implementation language: C.

## Status

ctld-rs served as a good learning exercise to help me understand ctld's config
file and XML parsing.  It also serves as a proof of concept for how a modern
language like Rust can be used to improve the FreeBSD base system.  The most
important part, IMHO, are the unit tests for the kernel API, since those cannot
possibly be written in C.

Alas, there is little appetite within the project to allow base system
components to require an external toolchain.  And since CTL's ioctl interface
is considered unstable, a program like ctld can't live outside of the tree.  So
I don't plan to do any more work on this project.

[✓] CLI
[✓] UCL config file parsing
[✓] Kernel XML parsing
[✓] LUN creation and destruction
[ ] Target creation and destruction
[ ] Handling client connections
[ ] isns
[ ] iSCSI discovery
[ ] Legacy config file parsing

## Bugs

ctld(8) assumes that it will operate alongside of ctladm(8).  The latter can
create and destroy targets and LUNs manually.  So when ctld starts up, it will
take note of which targets and LUNs already exist, and refuse to manage those.
This can lead to some unfortunate situations, such as if ctld crashes or gets
SIGKILL.  In that case, it will exit without tearing down its LUNs and targets,
and then when it restarts it won't begin to manage them again, leading to
clients being unable to reconnect.

Arguably that behavior is a bug, but it's ctld's current behavior.  ctld-rs,
OTOH, assumes that it will be the source of truth for which LUNs and Targets
ought to exist.  So ctladm cannot be used to create new LUNs or Targets while
ctld-rs is running.
