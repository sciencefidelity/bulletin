#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]
pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;

pub use startup::run;
