//! The release grant, under the narrow published grants: a party holding a `completion`
//! release reads completion and nothing else, the learner alone issues, narrows and revokes,
//! and revocation (or expiry) denies the very next read, cached or not.

mod common;

use common::*;
use ikigai_core::{Capability, Error, Verb};
use ikigai_learn::release::{self, Release, Scope};

fn completion_release() -> Release {
    Release {
        enrollment: N.into(),
        party: EMPLOYER.into(),
        scopes: [Scope::Completion].into(),
        expires: 1_814_313_600_000, // EXPIRES
    }
}

fn denied(r: ikigai_core::Result<ikigai_core::Representation>) -> String {
    match r {
        Err(Error::Denied(d)) => d.to_string(),
        other => panic!("expected Denied, got {other:?}"),
    }
}

#[test]
fn the_clock_constant_is_the_date_it_names() {
    assert_eq!(release::date_time(NOW), "2026-11-26T12:00:00Z");
    assert_eq!(
        release::millis_of(EXPIRES),
        Some(completion_release().expires)
    );
}

/// ★ Done when: a party holding a `completion` release reads completion and nothing else.
#[test]
fn a_completion_release_reads_completion_and_nothing_else() {
    let h = Host::new();
    let written = h.release(EMPLOYER, "completion").unwrap();
    let stored = String::from_utf8(written.bytes).unwrap();
    assert_eq!(stored, completion_release().turtle());
    let party = h.party(&completion_release());

    let completion = h.text_as(&party, Verb::Source, &view("completion"), &[]);
    assert!(
        completion.contains("<urn:learn:term:complete> true"),
        "{completion}"
    );

    // The other views: the kernel refuses each, since the published grant holds none of their
    // capabilities.
    for scope in ["credential", "progress", "submission"] {
        let e = h.issue_as(&party, Verb::Source, &view(scope), &[]);
        assert!(matches!(e, Err(Error::Denied(_))), "{scope}: {e:?}");
    }
    // And the state root holds nothing else for it: not the record files, not the responses
    // or drafts no scope names, not the enrollment.
    for path in [
        "enrollment/e1/credential.json",
        "enrollment/e1/progress.ttl",
        "enrollment/e1/submission/first.ttl",
        "enrollment/e1/response/q1.ttl",
        "enrollment/e1/draft/first.ttl",
        "enrollment/e1/enrollment.ttl",
    ] {
        let e = h.issue_as(&party, Verb::Source, &format!("urn:file:{path}"), &[]);
        assert!(matches!(e, Err(Error::Denied(_))), "{path}: {e:?}");
    }
    // It reads its own release, and cannot write it.
    let own = h.text_as(&party, Verb::Source, &release_iri(EMPLOYER), &[]);
    assert_eq!(own, completion_release().turtle());
    let e = h.issue_as(
        &party,
        Verb::Sink,
        &release_iri(EMPLOYER),
        &[
            ("party", EMPLOYER),
            ("scope", "completion,progress"),
            ("expires", EXPIRES),
        ],
    );
    assert!(matches!(e, Err(Error::Denied(_))), "{e:?}");
}

/// The door, not only the grant: a party whose grant is wider than its release (every view and
/// every record file) still reads only what the release shares.
#[test]
fn the_release_bounds_a_grant_wider_than_itself() {
    let h = Host::new();
    h.release(EMPLOYER, "completion").unwrap();
    let mut wide = completion_release();
    wide.scopes = Scope::ALL.into();
    let party = h.party(&wide);
    h.text_as(&party, Verb::Source, &view("completion"), &[]);
    for scope in ["credential", "progress", "submission"] {
        let sentence = denied(h.issue_as(&party, Verb::Source, &view(scope), &[]));
        assert_eq!(
            sentence,
            format!("the release to {EMPLOYER} on enrollment e1 shares completion, not {scope}")
        );
    }
}

/// ★ Done when: revocation denies the next read, even of an answer the kernel had cached.
#[test]
fn revoking_denies_the_next_read() {
    let h = Host::new();
    h.release(EMPLOYER, "completion").unwrap();
    // The grant a party already holds outlives the release: the door alone refuses it.
    let party = h.party(&completion_release());
    let first = h.text_as(&party, Verb::Source, &view("completion"), &[]);
    let again = h.text_as(&party, Verb::Source, &view("completion"), &[]);
    assert_eq!(first, again);

    let revoked = h.text_as(&h.learner(), Verb::Delete, &release_iri(EMPLOYER), &[]);
    assert_eq!(revoked, format!("revoked {}\n", release_iri(EMPLOYER)));
    let sentence = denied(h.issue_as(&party, Verb::Source, &view("completion"), &[]));
    assert_eq!(
        sentence,
        format!(
            "{EMPLOYER} holds no release of the completion of enrollment e1; only its learner, \
             or a party the learner released it to, may read it"
        )
    );
    // Gone for the learner too, and a second revoke has nothing to revoke.
    let e = h.issue_as(&h.learner(), Verb::Source, &release_iri(EMPLOYER), &[]);
    assert!(matches!(e, Err(Error::NotFound(_))), "{e:?}");
    let e = h.issue_as(&h.learner(), Verb::Delete, &release_iri(EMPLOYER), &[]);
    assert!(matches!(e, Err(Error::NotFound(_))), "{e:?}");
}

#[test]
fn an_expired_release_denies_the_next_read() {
    let h = Host::new();
    h.release_until(EMPLOYER, "completion", "2026-12-01T00:00:00Z")
        .unwrap();
    let mut r = completion_release();
    r.expires = release::millis_of("2026-12-01T00:00:00Z").unwrap();
    let party = h.party(&r);
    h.text_as(&party, Verb::Source, &view("completion"), &[]);
    h.clock.set(r.expires);
    let sentence = denied(h.issue_as(&party, Verb::Source, &view("completion"), &[]));
    assert_eq!(
        sentence,
        format!("the release to {EMPLOYER} on enrollment e1 expired at 2026-12-01T00:00:00Z")
    );
}

#[test]
fn only_the_learner_issues_or_revokes_a_release() {
    let h = Host::new();
    // A stranger holding the learner's whole published grant but not their principal.
    let mut grant = release::learner_grant(N, LEARNER);
    grant.retain(|s| !s.starts_with("urn:cap:principal:"));
    grant.push(ikigai_core::principal_scope(STRANGER).unwrap());
    let stranger = Capability::scoped(grant);
    let sentence = denied(h.issue_as(
        &stranger,
        Verb::Sink,
        &release_iri(STRANGER),
        &[
            ("party", STRANGER),
            ("scope", "submission"),
            ("expires", EXPIRES),
        ],
    ));
    assert_eq!(
        sentence,
        format!(
            "only the learner of enrollment e1 may issue its releases, and {STRANGER} is not \
             that learner"
        )
    );
    h.release(EMPLOYER, "completion").unwrap();
    let sentence = denied(h.issue_as(&stranger, Verb::Delete, &release_iri(EMPLOYER), &[]));
    assert!(
        sentence.starts_with("only the learner of enrollment e1 may revoke"),
        "{sentence}"
    );
    // Still there.
    h.text_as(&h.learner(), Verb::Source, &release_iri(EMPLOYER), &[]);
}

#[test]
fn a_release_narrows_and_never_widens() {
    let h = Host::new();
    h.release(EMPLOYER, "completion,progress").unwrap();
    // Narrower: fewer scopes, earlier expiry.
    h.release_until(EMPLOYER, "completion", "2027-01-01T00:00:00Z")
        .unwrap();
    // Wider in scope, or later in time: refused, and the stored release is unchanged.
    for (scope, expires) in [
        ("completion,progress", "2027-01-01T00:00:00Z"),
        ("completion", "2027-02-01T00:00:00Z"),
    ] {
        let e = h.release_until(EMPLOYER, scope, expires);
        match e {
            Err(Error::Conflict(s)) => assert_eq!(
                s,
                format!(
                    "a release only narrows: the one to {EMPLOYER} shares completion until \
                     2027-01-01T00:00:00Z; delete it and issue a new one to widen it"
                )
            ),
            other => panic!("{scope} until {expires}: {other:?}"),
        }
    }
    // Delete, then a new one may be wider.
    h.text_as(&h.learner(), Verb::Delete, &release_iri(EMPLOYER), &[]);
    h.release(EMPLOYER, "completion,progress").unwrap();
}

#[test]
fn what_no_scope_names_is_refused_with_a_sentence() {
    let h = Host::new();
    for (scope, expires, arg, sentence) in [
        (
            "response",
            EXPIRES,
            "scope",
            "`response` is never released: a release shares only credential, completion, \
             progress or submission",
        ),
        (
            "drafts",
            EXPIRES,
            "scope",
            "`drafts` is never released: a release shares only credential, completion, \
             progress or submission",
        ),
        (
            "grades",
            EXPIRES,
            "scope",
            "`grades` is not a scope: use credential, completion, progress or submission",
        ),
        (
            "completion",
            "2026-11-01T00:00:00Z",
            "expires",
            "2026-11-01T00:00:00Z has already passed",
        ),
        (
            "completion",
            "next year",
            "expires",
            "`next year` is not a UTC date-time such as 2027-06-30T00:00:00Z",
        ),
    ] {
        match h.release_until(EMPLOYER, scope, expires) {
            Err(Error::InvalidArgument { name, detail }) => {
                assert_eq!((name.as_str(), detail.as_str()), (arg, sentence))
            }
            other => panic!("{scope} {expires}: {other:?}"),
        }
    }
    match h.release(LEARNER, "completion") {
        Err(Error::InvalidArgument { name, detail }) => {
            assert_eq!(name, "party");
            assert_eq!(
                detail,
                "a learner already reads their own record: a release names another party"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_learner_reads_every_view_and_a_stranger_none() {
    let h = Host::new();
    for scope in ["credential", "completion", "progress", "submission"] {
        h.text_as(&h.learner(), Verb::Source, &view(scope), &[]);
    }
    // A stranger holding a party's whole grant shape for every scope, but no release.
    let mut r = completion_release();
    r.party = STRANGER.into();
    r.scopes = Scope::ALL.into();
    let stranger = h.party(&r);
    let sentence = denied(h.issue_as(&stranger, Verb::Source, &view("progress"), &[]));
    assert_eq!(
        sentence,
        format!(
            "{STRANGER} holds no release of the progress of enrollment e1; only its learner, or \
             a party the learner released it to, may read it"
        )
    );
}

#[test]
fn completion_before_completing_is_a_value() {
    let h = Host::new();
    h.issue_as(
        &Capability::root(),
        Verb::Delete,
        "urn:file:enrollment/e1/completion.ttl",
        &[],
    )
    .unwrap();
    h.release(EMPLOYER, "completion").unwrap();
    let party = h.party(&completion_release());
    let answer = h.text_as(&party, Verb::Source, &view("completion"), &[]);
    assert_eq!(
        answer,
        "<urn:learn:completion:enrollment:e1> <urn:learn:term:complete> false .\n"
    );
}
