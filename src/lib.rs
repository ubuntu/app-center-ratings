//! The `ratings` library represents the code necessary for the Ubuntu app center's
//! ratings backend.

/* Normally we'd use #! for these attributes, but it breaks the protobuf files and I can't find
a good way to allow the warnings from `build.rs` */

#[deny(rustdoc::broken_intra_doc_links)]
#[warn(missing_docs)]
#[warn(clippy::missing_docs_in_private_items)]
pub mod app;

#[deny(rustdoc::broken_intra_doc_links)]
#[warn(missing_docs)]
#[warn(clippy::missing_docs_in_private_items)]
pub mod features;

#[deny(rustdoc::broken_intra_doc_links)]
#[warn(missing_docs)]
#[warn(clippy::missing_docs_in_private_items)]
pub mod utils;
