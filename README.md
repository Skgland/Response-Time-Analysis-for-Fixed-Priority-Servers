Response Time Analysis for Fixed Priority Servers implemented in Rust
=====================================================================

This Project is an implementation of the paper [Response Time Analysis for Fixed Priority Servers][1] by Hamann et al.
written in [Rust].

The Project consists of three parts
1. `rta-for-fps-lib` containing a library with the paper implementation
1. `rta-for-fps-latex-lib` a library to help with generation latex diagrams from the output of the main library
1. `rta-for-fps-latex-gen` an example usage of both libraries recreating a few of the papers figures

The goal behind this project is a better understanding of said paper as preparation for writing a seminar paper.

The implementation tries to reference the paper where practical and improve on it with type safety.
The examples in the paper are incorporated as tests where possible.

| crate            | crates.io | docs.rs |
| :--------------- | :-------: | :-----: |
| rta-for-fps-lib  | [![crates.io version badge](https://img.shields.io/crates/v/rta-for-fps-lib?style=flat-square)][crates.io-lib]       | [![docs.rs](https://img.shields.io/docsrs/rta-for-fps-lib?style=flat-square)][docs.rs-lib] |
| rta-for-fps-latex-lib  | [![crates.io version badge](https://img.shields.io/crates/v/rta-for-fps-lib?style=flat-square)][crates.io-latex-lib] | [![docs.rs](https://img.shields.io/docsrs/rta-for-fps-lib?style=flat-square)][docs.rs-latex-lib] |
| rta-for-fps-latex-gen  | [![crates.io version badge](https://img.shields.io/crates/v/rta-for-fps-lib?style=flat-square)][crates.io-latex-gen] | not a library                                                               |

[crates.io-lib]: https://crates.io/crates/rta-for-fps-lib
[docs.rs-lib]: https://docs.rs/rta-for-fps-lib
[crates.io-latex-lib]: https://crates.io/crates/rta-for-fps-latex-lib
[docs.rs-latex-lib]: https://docs.rs/rta-for-fps-latex-lib
[crates.io-latex-gen]: https://crates.io/crates/rta-for-fps-latex-gen

[Rust]: https://www.rust-lang.org/
[1]: https://doi.org/10.1145/3273905.3273927
