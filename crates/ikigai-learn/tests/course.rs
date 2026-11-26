//! Every stage W1 name, read through a real kernel over the sample release in
//! `sample/release` (what `tools/build_release.py` writes for `sample/course` at `v1` with
//! the pinned Lectern; the release job rebuilds it and fails if a byte differs, and today
//! refuses it for its CDN load, as the README says), under the published grant
//! [`course::reader_grant`].
use std::path::PathBuf;
use std::sync::Arc;

use futures::executor::block_on;
use ikigai_core::{Capability, Error, Fallback, Iri, Kernel, Request, Verb};
use ikigai_learn::course::{self, CAP_COURSE_READ};

fn kernel() -> Kernel {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sample");
    let space = Fallback::new(vec![
        Arc::new(course::space("release")),
        Arc::new(ikigai_fs::cacheable_space(root)),
    ]);
    Kernel::new(Arc::new(space))
}

/// The published grant, exactly.
fn reader() -> Capability {
    Capability::root().attenuate(course::reader_grant("release"))
}

fn get(kernel: &Kernel, iri: &str, cap: &Capability) -> Result<(String, String), Error> {
    let request = Request::new(Verb::Source, Iri::parse(iri).unwrap());
    let repr = block_on(kernel.issue(request, cap))?;
    Ok((
        repr.repr_type.to_string(),
        String::from_utf8(repr.bytes).unwrap(),
    ))
}

fn json(kernel: &Kernel, iri: &str) -> serde_json::Value {
    let (_, body) = get(kernel, iri, &reader()).unwrap_or_else(|e| panic!("{iri}: {e}"));
    serde_json::from_str(&body).unwrap()
}

#[test]
fn a_release_resolves_from_its_tag_to_its_slides() {
    let k = kernel();
    let course = json(&k, "urn:learn:course:name:sample:release:v1");
    assert_eq!(course["release"], "v1");
    let modules = course["modules"].as_array().unwrap();
    assert_eq!(modules.len(), 2);
    assert_eq!(
        modules[0]["units"][0],
        "urn:learn:unit:name:intro:release:v1"
    );
    assert_eq!(
        modules[1]["units"][0],
        "urn:learn:unit:name:recall:release:v1"
    );

    let unit = json(&k, "urn:learn:unit:name:intro:release:v1");
    assert_eq!(unit["kind"], "lectern-deck");
    assert_eq!(unit["lectern"], "7531175f245fafb90d06c49abd97c0638b66a104");
    assert_eq!(unit["slides"].as_array().unwrap().len(), 3);

    let (media, html) = get(&k, unit["forms"]["slides"].as_str().unwrap(), &reader()).unwrap();
    assert!(media.starts_with("text/html"), "{media}");
    assert!(html.contains("Welcome"));
    let (media, outline) = get(
        &k,
        "urn:learn:unit:name:intro:release:v1:form:outline",
        &reader(),
    )
    .unwrap();
    assert!(media.starts_with("text/markdown"), "{media}");
    assert!(outline.contains("# Welcome"));
    let (_, transcript) = get(
        &k,
        "urn:learn:unit:name:intro:release:v1:form:transcript",
        &reader(),
    )
    .unwrap();
    assert_eq!(transcript, outline);
}

#[test]
fn each_slide_names_its_source_file_and_number() {
    let k = kernel();
    let expect = [
        ("intro", 1, "slides/01-welcome.md", 1),
        ("intro", 2, "slides/02-plan.md", 1),
        ("intro", 3, "slides/02-plan.md", 2),
        ("recall", 1, "slides/01-why.md", 1),
        // The partial included by range `#2-3`: its slides keep their own numbers.
        ("recall", 2, "_partials/techniques.md", 2),
        ("recall", 3, "_partials/techniques.md", 3),
    ];
    for (u, n, file, at) in expect {
        let slide = json(
            &k,
            &format!("urn:learn:slide:unit:{u}:number:{n}:release:v1"),
        );
        assert_eq!(slide["source"], file, "{u} {n}");
        assert_eq!(slide["sourceSlide"], at, "{u} {n}");
        assert_eq!(
            slide["html"],
            format!("urn:learn:unit:name:{u}:release:v1:form:slides#/{}", n - 1)
        );
    }
}

#[test]
fn the_answers_are_cacheable() {
    let k = kernel();
    for iri in [
        "urn:learn:course:name:sample:release:v1",
        "urn:learn:unit:name:recall:release:v1",
        "urn:learn:unit:name:recall:release:v1:form:slides",
        "urn:learn:slide:unit:recall:number:2:release:v1",
    ] {
        get(&k, iri, &reader()).unwrap();
        let request = Request::new(Verb::Source, Iri::parse(iri).unwrap());
        assert!(k.is_cached(&request, &reader()), "{iri}");
    }
}

#[test]
fn the_caller_needs_both_scopes_the_doors_declare() {
    let k = kernel();
    let iris = [
        "urn:learn:course:name:sample:release:v1",
        "urn:learn:unit:name:intro:release:v1",
        "urn:learn:unit:name:intro:release:v1:form:slides",
        "urn:learn:slide:unit:intro:number:1:release:v1",
    ];
    let none = Capability::root().attenuate(["urn:cap:other"]);
    // The course read alone, without the file read on the store: refused.
    let course_only = Capability::root().attenuate([CAP_COURSE_READ]);
    // The file read alone does not read the names either.
    let files_only = Capability::root().attenuate([course::fs_read("release")]);
    // The sharing capability of a learner's record is not a course reader.
    let sharing = Capability::root().attenuate([
        ikigai_learn::release::CAP_RELEASE_READ.to_string(),
        course::fs_read("release"),
    ]);
    for iri in iris {
        for (who, cap) in [
            ("none", &none),
            ("course only", &course_only),
            ("files only", &files_only),
            ("sharing", &sharing),
        ] {
            assert!(
                matches!(get(&k, iri, cap), Err(Error::Denied(_))),
                "{who} read {iri}"
            );
        }
        // The published grant works.
        get(&k, iri, &reader()).unwrap_or_else(|e| panic!("{iri}: {e}"));
    }
}

#[test]
fn unknown_names_are_not_found_and_bad_keys_are_refused() {
    let k = kernel();
    for iri in [
        "urn:learn:course:name:nope:release:v1",
        "urn:learn:unit:name:intro:release:v9",
        "urn:learn:slide:unit:intro:number:4:release:v1",
        "urn:learn:unit:name:intro:release:v1:form:handout",
    ] {
        assert!(
            matches!(get(&k, iri, &reader()), Err(Error::NotFound(_))),
            "{iri}"
        );
    }
    for iri in [
        "urn:learn:slide:unit:intro:number:0:release:v1",
        "urn:learn:slide:unit:intro:number:01:release:v1",
        "urn:learn:unit:name:..:release:v1",
    ] {
        assert!(
            matches!(get(&k, iri, &reader()), Err(Error::InvalidArgument { .. })),
            "{iri}"
        );
    }
}
