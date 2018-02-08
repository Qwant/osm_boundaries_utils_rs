# osm_boundaries_utils_rs
misc utilities for OpenStreetMap boundary reading in rust

This library provides mainly a method to compute the boundary of an OSM relation (as a geo::MultiPolygon).

It also provides as osm_builder utility to create osm datasets, mainly to write osm tests easily.

# Build

`cargo build`

# Test

`cargo test`

This crate is not yet on crates.io, for the moment you need to put 
`osm_boundaries_utils = { git = "https://github.com/QwantResearch/osm_boundaries_utils_rs" }`
in your own Cargo.toml to use it.
