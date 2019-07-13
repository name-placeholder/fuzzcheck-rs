#![feature(drain_filter)]
#![feature(never_type)]
#![feature(thread_spawn_unchecked)]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

extern crate structopt;

pub mod artifact;
mod code_coverage_sensor;
pub mod command_line;
pub mod fuzzer;
pub mod generators;
mod hooks;
pub mod input;
mod input_pool;
mod signals_handler;
mod weighted_index;
pub mod world;
