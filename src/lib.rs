
#[macro_use]
extern crate log;
extern crate osmpbfreader;
extern crate geo;

pub mod osm_builder;
mod boundaries;

pub use boundaries::build_boundary;