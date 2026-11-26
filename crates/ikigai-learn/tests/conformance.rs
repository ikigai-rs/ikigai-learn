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

/// The course doors: a host-named space (it reads the store it was handed), walked over the
/// committed sample release under exactly the published reader grant.
const COURSE: [&str; 4] = ["learn-course", "learn-unit", "learn-form", "learn-slide"];

fn course_walk() -> ikigai_conformance::Report {
    use ikigai_core::{Capability, Fallback, Kernel};
    use ikigai_learn::course;
    use std::sync::Arc;

    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sample");
    let doors = Arc::new(course::space("release"));
    let files = Arc::new(ikigai_fs::cacheable_space(sample));
    let whole = Arc::new(Fallback::new(vec![doors.clone(), files]));
    let kernel = Kernel::with_meta_renderer(whole.clone(), Arc::new(ikigai_vocab::TurtleRenderer));
    let reader = Capability::root().attenuate(course::reader_grant("release"));
    let mut suite = Suite::new()
        .host_named_space("course::space(store)", doors)
        .body_space(whole);
    for id in COURSE {
        suite = suite.own(id).cacheable(id);
    }
    suite = suite
        .fixture(
            Fixture::new("learn-course", Verb::Source)
                .capability(reader.clone())
                .binding("c", "sample")
                .binding("r", "v1"),
        )
        .fixture(
            Fixture::new("learn-unit", Verb::Source)
                .capability(reader.clone())
                .binding("u", "recall")
                .binding("r", "v1"),
        )
        .fixture(
            Fixture::new("learn-form", Verb::Source)
                .capability(reader.clone())
                .binding("u", "recall")
                .binding("r", "v1")
                .binding("f", "outline"),
        )
        .fixture(
            Fixture::new("learn-slide", Verb::Source)
                .capability(reader)
                .binding("u", "recall")
                .binding("n", "2")
                .binding("r", "v1"),
        );
    suite.run_blocking(&kernel)
}

#[test]
fn the_course_doors_conform() {
    let report = course_walk();
    println!("{report}");
    report.assert_clean_with(&Expected::new().nothing_unprobed());
}

#[test]
fn the_course_walk_reaches_every_course_door() {
    let report = course_walk();
    let mut walked: Vec<&str> = report.walked.iter().map(String::as_str).collect();
    walked.sort_unstable();
    let mut expected = COURSE.to_vec();
    expected.sort_unstable();
    assert_eq!(walked, expected);
}
