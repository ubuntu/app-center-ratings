//! Contains autogenerated protobuf implementations from [`prost`].

#![allow(rustdoc::broken_intra_doc_links)]
#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]

pub mod app {
    //! Contains protobufs relating to the app features
    include!("../proto/ratings.features.app.rs");
}

pub mod common {
    //! Contains common protobufs
    include!("../proto/ratings.features.common.rs");
}

pub mod user {
    //! Contains user protobufs
    include!("../proto/ratings.features.user.rs");
}

pub mod chart {
    //! Contains chart protobufs
    include!("../proto/ratings.features.chart.rs");
}