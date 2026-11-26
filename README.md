# ikigai-learn

Self-paced courses as resources for [ikigai](https://github.com/ikigai-rs): a course is a list of
names, a learner's work is a [gonk](https://github.com/ikigai-rs/ikigai-gonk) ledger, review is a
tuplespace, and progress is a question the kernel answers, never stored state.

Everything a course is made of has a name under `urn:learn:`. A unit is an authored Markdown file
(a [Lectern](https://github.com/bsletten/lectern-slides) deck or a Book chapter) that an author
writes in git and a learner never writes. A release pins a course to a git tag, so everything in it
resolves the same way for years and caches for good. A learner's answers, submissions and notes are
their own atoms, read over the shared names of the release they enrolled in, and their capability
decides which graphs exist, so there is nothing to filter afterward.

Status: **nothing is built yet.** This repository is a scaffold; the map below is the plan, tracked
as ledger #4142 and its children.

## What the design encodes

Each of these is a consequence in the resource map, not a preference:

- **Retrieval, not rereading.** Questions sit inside every unit, and nothing anywhere marks a page
  "read": practice testing and spaced practice are the techniques with strong evidence behind them
  (Roediger and Karpicke 2006; Dunlosky et al. 2013).
- **Spaced review.** A review queue whose schedule is data (Leitner boxes first), and mastery that
  needs two sessions at least a day apart (Cepeda et al. 2006).
- **Mastery before progression.** `next` offers the first unmastered unit, not the next one in
  order (Bloom 1984; Kulik, Kulik and Bangert-Drowns 1990).
- **Short units**, about six minutes each (Guo, Kim and Rubin 2014), and early exercises that are
  worked examples whose support fades (Sweller, van Merriënboer and Paas 2019).
- **A wrong answer is a value**, a verdict of `incorrect`, never a refusal, so reviews stay
  cacheable. A "not yet" ("I don't understand this yet") is excluded from every evaluation by
  construction.
- **No model verdict.** A verdict is a person's, then the deterministic checks', then `pending`. A
  model may draft feedback, which is shown as a draft in its own graph, never as a grade.

## The resource map (planned)

Names are singular, the type first, and every partition is by a named key.

### Content: authored, and immutable at a release

| Name | What it is |
|---|---|
| `urn:learn:unit:name:{u}` | a unit: an authored Lectern deck or Book chapter, read through ikigai-fs or ikigai-repo |
| `urn:learn:unit:name:{u}:release:{r}` | the unit at a release's git tag; cacheable forever |
| `urn:learn:unit:name:{u}:form:{f}` | the unit as `slides` (HTML), `page` (an article), `transcript` (prose, notes and diagram descriptions, for screen readers and audio) or `handout` (PDF) |
| `urn:learn:slide:unit:{u}:number:{n}` | one slide, citable by feedback and the tutor |
| `urn:learn:slide:unit:{u}:range:{spec}` | a slide range in Lectern's `#1-3,14` grammar |
| `urn:learn:lift:unit:{u}` | the unit's structure as RDF: ikigai-markdown's lift plus a Lectern mapping |
| `urn:learn:question:unit:{u}:id:{q}` | a question authored inline (choice, exact, short answer, or a check script) |
| `urn:learn:key:question:{q}`, `urn:learn:key:exercise:{e}` | an answer key; a reviewed exercise's key is sealed, readable only under `urn:cap:learn:key:read:course:{c}` |
| `urn:learn:exercise:name:{e}` | an exercise: the task, the prediction to write first, whether an agent may help (`allowed`, `draft-only`, `none`), the evidence that counts as done, the rubric |
| `urn:learn:check:exercise:{e}:name:{k}` | a check, as an ikigai-script resource |
| `urn:learn:course:name:{c}:release:{r}` | a course release: its manifest at a git tag, the list of names below it |
| `urn:learn:rule:course:{c}:name:{n}` | a mastery rule, as data |
| `urn:learn:schedule:name:{s}` | a review schedule, as data |

### The learner's space

| Name | What it is |
|---|---|
| `urn:learn:learner:id:{l}` | a learner: a principal (a passkey for a person, a certificate for their agent) |
| `urn:learn:enroll:course:{c}` | the enroll Sink: writes an enrollment, creates a named ledger with an opaque name, and seeds one item per exercise |
| `urn:learn:enrollment:id:{n}` | an enrollment atom |
| `urn:learn:response:enrollment:{n}:question:{q}:id:{r}` | a response, append-only |
| `urn:learn:result:submission:{s}:check:{k}` | a check's result, cached by submission digest and check version |
| `urn:learn:verdict:submission:{s}` | the verdict: a person's, then the checks', then `pending` |
| `urn:learn:review:enrollment:{n}:date:{d}` | the review queue for a day: a composition |
| `urn:learn:next:enrollment:{n}` | the next thing to work on: a composition |

Mastery and progress are compositions too: stored SPARQL or s-expression programs over one
learner's atoms, so an answer recomputes one learner's progress and nobody else's.

### Review

Grading is a tuplespace layer: one offer per submission carrying the enrollment's pseudonym, never
the learner's name (blind grading), with a lease so a grader who wanders off releases it. Checks
run in a sandboxed worker, and the report is mirrored onto the learner's ledger item.

## Errors as contract

Denied (no capability, a sealed name), Conflict (a second enrollment in one release, a taken item),
NotFound (an unknown course, unit or release), InvalidArgument (a malformed answer).

## Privacy is a property, not a setting

A learner's record is theirs and exports losslessly (Turtle and JSON-LD, same names). Anyone else
sees part of it only through a release grant the learner issues and can revoke. Counts about people
are floored (`fewer than 5`). There are no per-learner rows, no time on page, no per-person funnels,
no cookies and no stored addresses anywhere in this module.

## Related

- [ikigai-credential](https://github.com/ikigai-rs/ikigai-credential): verifiable credentials for
  a completed course release.
- [ikigai-markdown](https://github.com/ikigai-rs/ikigai-markdown): the structural lift units use.
- [The ikigai Book](https://ikigai-rs.github.io/ikigai-tutorial/): the first free courses are built
  on it.

## Accessibility

See [ACCESSIBILITY.md](ACCESSIBILITY.md): the target (WCAG 2.2 AA), the evidence that exists, what
has not been tested, and how to report a barrier.

## License

MIT OR Apache-2.0.
