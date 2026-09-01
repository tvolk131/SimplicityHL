//! Shared machinery for the SimplicityHL benchmark suite: a deterministic
//! corpus of generated programs, and harnesses for driving individual compiler
//! stages in isolation.
//!
//! See `README.md` in this directory for the workflows built on top of it.

pub mod corpus;
pub mod harness;
pub mod rng;
