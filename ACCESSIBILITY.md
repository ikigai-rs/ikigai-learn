# Accessibility

How this repository approaches accessibility: what it emits that people read, the standard we
aim at, the evidence that exists today and what it does not cover, the barriers we know about,
and how to report one. It is a working agreement for contributors, maintainers and coding agents.
It is not a conformance claim, an Accessibility Conformance Report or a VPAT, and it is not legal
advice.

Adapted from the [ACCESSIBILITY.md template](https://github.com/mgifford/ACCESSIBILITY.md)
(MIT).

## Project information

| Field | Value |
|---|---|
| Project | ikigai-learn: a Rust library serving self-paced courses as ikigai resources |
| Published at | not published: a scaffold |
| Owner | the maintainers of this repository |
| Report a barrier | [a GitHub issue](https://github.com/ikigai-rs/ikigai-learn/issues/new) |
| Private channel | [private vulnerability reporting](https://github.com/ikigai-rs/ikigai-learn/security/advisories/new), for anything security-sensitive |
| Target | WCAG 2.2 Level AA, for any page or terminal that presents this output |
| Status | Evaluation in progress. No conformance claim. |
| Last reviewed | 2026-11-26 |
| Next review | when the crate first emits a unit form or terminal text |

## Commitment

A course is for everyone who wants to learn from it, including people who listen to it through a
screen reader, read it magnified, or work from a keyboard alone. A course that carries its meaning
only in a slide's layout, a diagram or a color keeps those learners out of the lesson itself. So
every unit is planned in a `transcript` form (headings, prose, speaker notes as prose, diagrams as
their accessible description) beside its slides, a verdict is a word (`incorrect`, `pending`)
rather than only a color or a mark, every sentence this crate emits will be plain words pinned by tests,
and this file will say what has not been tested.

## Scope

### In scope

- **The release refusals** (`crates/ikigai-learn/src/release.rs`): one sentence each, naming what
  was wrong and what to do, for a scope that is not one or is never released, an expiry that is
  not a UTC date-time or has passed, a release to the learner themselves, a caller who is not the
  learner (issuing, revoking or listing releases), a release that does not share a view or has expired, a reader holding no release, and a
  rewrite that would widen a release. Pinned word for word in `crates/ikigai-learn/tests/release.rs`.
- **The revocation answer**, `revoked {release}`, plain text, pinned in the same file.

As the crate adds output people read, each is listed here with its form and its test. The planned
ones are a unit's four forms (`slides`, `page`, `transcript`, `handout`), question and exercise
text, check results and verdicts.

### Not ours to fix, still ours to report

The course content authors write. A unit's form can only carry the alternative text and structure
its author wrote; the forms will make missing alternative text visible rather than hide it.

## The rules that output follows

When output arrives, it follows the rules ikigai's other libraries follow: one sentence per
refusal that names what was wrong and what to do; no meaning carried by color, symbols or
alignment alone; no terminal control characters from untrusted input; American English.

## Conformance status

No conformance claim. The only output today is the release's refusal sentences and its plain-text
and Turtle answers; no page presents them yet.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Refusal sentences pinned by tests | Pass | the release refusals, word for word, in `crates/ikigai-learn/tests/release.rs` |
| Unit forms checked in a browser and by an automated engine | Blocked | no unit form exists yet |
| Unit forms read by a screen reader | Blocked | no unit form exists yet; no screen-reader session has been run |

**No screen reader or other assistive technology has been used to test this repository.**

## Known limitations

- A refusal names a party or learner by its principal IRI, which a person may find hard to read
  aloud; the page that presents it should show the principal's display name beside it.

## Contributor requirements

A pull request that adds output people read lists it under "In scope" above, pins it in a test,
and fills the pull request's `## Accessibility` section with a result for every check: Pass, Fail
or Blocked, with its evidence.

## Reporting a barrier

[Open an issue](https://github.com/ikigai-rs/ikigai-learn/issues/new) and put "accessibility"
in its title. Useful details: what you were trying to do, what happened and what you expected,
whether you found a way around it, and your tools and any assistive technology, if you are
comfortable sharing them. You never need to disclose a disability. We aim to acknowledge within
7 days.

Anything security-sensitive, or anything you would rather not post publicly, goes through
[private vulnerability reporting](https://github.com/ikigai-rs/ikigai-learn/security/advisories/new).
If neither route works for you, say so in whichever you can reach and we will find another.

## Ownership and review

The maintainers of this repository own this file and review it whenever the crate first emits
output people read.

### Change history

- 2026-11-26: first version, for the scaffold.
- 2026-11-26: the release refusals and the revocation answer, pinned by tests.
- 2026-11-26: the release set's refusal to anyone but the learner, pinned by tests.

## References

- [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- [ikigai-cli's ACCESSIBILITY.md](https://github.com/ikigai-rs/ikigai-cli/blob/main/ACCESSIBILITY.md), the model this file follows
