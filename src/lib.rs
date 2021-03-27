//! Implementation based on the algorithms described in
//! The paper [Response Time Analysis for Fixed Priority Servers][paper]
//! by Hamann et al
//!
//! [paper]: https://doi.org/10.1145/3273905.3273927

#![warn(missing_debug_implementations)]
#![allow(private_intra_doc_links)]
#![warn(unused)]
//
#![warn(clippy::cargo)]
//
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)] // modules named after structs they define
#![allow(clippy::redundant_else)] // may make code less clear
//
#![warn(clippy::nursery)]
#![allow(clippy::use_self)] // too many false positives and fails to be overridden locally
#![allow(clippy::redundant_pub_crate)] // prevents accidents when changing the visibility of the containing modul
//
#![warn(clippy::missing_const_for_fn)]
#![warn(clippy::missing_docs_in_private_items)]
#![warn(clippy::missing_errors_doc)]
#![warn(clippy::unimplemented)]
#![warn(clippy::unwrap_in_result)]
#![warn(clippy::unwrap_used)]

pub mod time;

pub mod curve;
pub(crate) mod seal;
pub mod server;
pub mod task;
pub mod window;

pub mod paper;
