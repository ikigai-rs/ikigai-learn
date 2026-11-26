//! Self-paced courses as ikigai resources.
//!
//! Mostly a scaffold. Two parts are built: [`release`], the grant a learner issues over their
//! own record, and [`course`], stage W1: a course release built by Lectern at a git tag, served
//! under its own capability.
//! A course is a list of names, a learner's work is a ledger, review is a tuplespace, and progress is a question the kernel answers, never stored state. Every name
//! lives under [`NAMESPACE`], singular and type first, partitioned by named keys
//! (`urn:learn:unit:name:{u}`, `urn:learn:course:name:{c}:release:{r}`). The repository's
//! README holds the planned resource map.
#![forbid(unsafe_code)]

/// The prefix every name in this module starts with.
///
/// ```
/// assert_eq!(ikigai_learn::NAMESPACE, "urn:learn:");
/// assert!("urn:learn:unit:name:intro".starts_with(ikigai_learn::NAMESPACE));
/// ```
pub const NAMESPACE: &str = "urn:learn:";

pub mod course;
pub mod release;

pub use release::{space, Release, Scope};
