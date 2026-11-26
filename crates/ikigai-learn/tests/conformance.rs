//! The module recipe as one test: `ikigai-conformance` walks every resource the learn space
//! binds (the state root the host composes is inherited and not walked) and reports every
//! violation at once.

mod common;

use common::*;
use ikigai_conformance::{Check, Expected, Fixture, Suite};
use ikigai_core::Verb;

const OURS: [&str; 5] = [
    "learn-release",
    "learn-credential",
    "learn-completion",
    "learn-progress",
    "learn-submission",
];

/// The release set: owned and walked, but never cached.
const SET: &str = "learn-release-set";

fn walk() -> ikigai_conformance::Report {
    let h = Host::new();
    h.release(EMPLOYER, "completion").unwrap();
    let key = ikigai_learn::release::party_key(EMPLOYER);
    let mut suite = Suite::new()
        .self_named_space("learn", ikigai_learn::space)
        .body_space(h.root_space.clone())
        .required_on_mutating()
        // The vocabulary the atoms and views use, documented in the README as this module's.
        .namespace(ikigai_learn::release::TERM);
    for id in OURS {
        suite = suite.own(id).cacheable(id);
    }
    // The release set is a listing over a directory, which ikigai-fs never caches, so it is
    // owned and declared live (uncacheable on purpose).
    suite = suite.own(SET).live(SET);
    // Every probe runs as the learner, under their published grant: the doors answer about
    // their caller, and root names no principal.
    let learner = h.learner();
    suite = suite
        .opt_out_check(
            "learn-release",
            Check::Pipeline,
            "a release is three named values (party, scope, expires), not a piped body",
        )
        .opt_out_check(
            SET,
            Check::Bodies,
            "a live listing: independent calls cannot agree by bytes; tests/release.rs pins \
             its body after each issue and revoke",
        )
        .opt_out(
            "learn-release",
            Some(Verb::Delete),
            "a revocation takes no content; tests/release.rs fires it and pins its answer",
        );
    suite = suite
        .fixture(
            Fixture::new("learn-release", Verb::Source)
                .capability(learner.clone())
                .binding("n", N)
                .binding("p", &key),
        )
        .fixture(
            Fixture::new("learn-release", Verb::Sink)
                .capability(learner.clone())
                .binding("n", N)
                .binding("p", &key)
                .arg("party", EMPLOYER)
                .arg("scope", "completion")
                .arg("expires", EXPIRES),
        )
        .fixture(
            Fixture::new(SET, Verb::Source)
                .capability(learner.clone())
                .binding("n", N),
        )
        .fixture(
            Fixture::new("learn-credential", Verb::Source)
                .capability(learner.clone())
                .binding("n", N),
        )
        .fixture(
            Fixture::new("learn-completion", Verb::Source)
                .capability(learner.clone())
                .binding("n", N),
        )
        .fixture(
            Fixture::new("learn-progress", Verb::Source)
                .capability(learner.clone())
                .binding("n", N),
        )
        .fixture(
            Fixture::new("learn-submission", Verb::Source)
                .capability(learner.clone())
                .binding("n", N)
                .binding("e", "first"),
        );
    suite.run_blocking(&h.kernel)
}

#[test]
fn conforms() {
    let report = walk();
    println!("{report}");
    report.assert_clean_with(&Expected::new().nothing_unprobed());
}

#[test]
fn the_walk_reaches_every_resource_this_crate_binds() {
    let report = walk();
    let mut walked: Vec<&str> = report.walked.iter().map(String::as_str).collect();
    walked.sort_unstable();
    let mut expected = OURS.to_vec();
    expected.push(SET);
    expected.sort_unstable();
    assert_eq!(walked, expected);
}
