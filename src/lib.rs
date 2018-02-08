extern crate geo;
#[macro_use]
extern crate log;
extern crate osmpbfreader;

pub mod osm_builder;
mod boundaries;

pub use boundaries::build_boundary;
