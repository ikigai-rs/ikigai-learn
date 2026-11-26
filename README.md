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

Status: **the release grant is built** (`ikigai_learn::release`, ledger #4151), and **stage W1
is built** (`ikigai_learn::course`, `tools/build_release.py`, ledger #4458): a course release
built by Lectern at a git tag, served as the course, unit, form and slide names marked below.
Its builder is blocked on one thing, said below: Lectern's reveal pages load reveal.js from a CDN.
Everything else in the map below is the plan, tracked as ledger #4142 and its children.

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
| `urn:learn:unit:name:{u}:release:{r}` | **built.** The unit at a release's git tag, with its declared source kind (`lectern-deck`; `mdbook-chapter` planned), the Lectern commit that built it, its forms and its slides; cacheable forever |
| `urn:learn:unit:name:{u}:release:{r}:form:{f}` | **built.** A face of the unit's build at a release: `slides` (Lectern's reveal HTML), `outline` and `transcript` (Lectern's outline) |
| `urn:learn:unit:name:{u}:form:{f}` | the unit as `slides` (HTML), `page` (an article), `transcript` (prose, notes and diagram descriptions, for screen readers and audio) or `handout` (PDF) |
| `urn:learn:slide:unit:{u}:number:{n}:release:{r}` | **built.** One slide at a release, 1-based, citable by feedback and the tutor, naming its source file and slide number; its HTML face is the slides form plus `#/{n-1}` |
| `urn:learn:slide:unit:{u}:range:{spec}` | a slide range in Lectern's `#1-3,14` grammar |
| `urn:learn:lift:unit:{u}` | the unit's structure as RDF: ikigai-markdown's lift plus a Lectern mapping |
| `urn:learn:question:unit:{u}:id:{q}` | a question authored inline (choice, exact, short answer, or a check script) |
| `urn:learn:key:question:{q}`, `urn:learn:key:exercise:{e}` | an answer key; a reviewed exercise's key is sealed, readable only under `urn:cap:learn:key:read:course:{c}` |
| `urn:learn:exercise:name:{e}` | an exercise: the task, the prediction to write first, whether an agent may help (`allowed`, `draft-only`, `none`), the evidence that counts as done, the rubric |
| `urn:learn:check:exercise:{e}:name:{k}` | a check, as an ikigai-script resource |
| `urn:learn:course:name:{c}:release:{r}` | **built.** A course release: its manifest at a git tag, its modules in order, each an ordered list of unit names at the release |
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
| `urn:learn:release:enrollment:{n}` | **built.** The release set: for the learner alone, a Turtle listing of each release on the enrollment (its party, scopes and expiry), which a devices-and-grants page lists and revokes from; never cached |
| `urn:learn:release:enrollment:{n}:party:{p}` | **built.** A release: part of the learner's record shared with one party (`{p}` is the bare-hex BLAKE3 digest of the party's principal), with its scopes and expiry. An atom only the learner writes (Sink), which a rewrite may only narrow; Delete revokes it |
| `urn:learn:credential:enrollment:{n}` | **built.** The `credential` view: the issued credential |
| `urn:learn:completion:enrollment:{n}` | **built.** The `completion` view: complete or not, and when |
| `urn:learn:progress:enrollment:{n}` | **built.** The `progress` view: mastery per unit and the verdict per exercise |
| `urn:learn:submission:enrollment:{n}:exercise:{e}` | **built.** The `submission` view: the work submitted for one exercise |

Mastery and progress are compositions too: stored SPARQL or s-expression programs over one
learner's atoms, so an answer recomputes one learner's progress and nobody else's.

### Review

Grading is a tuplespace layer: one offer per submission carrying the enrollment's pseudonym, never
the learner's name (blind grading), with a lease so a grader who wanders off releases it. Checks
run in a sandboxed worker, and the report is mirrored onto the learner's ledger item.

## Errors as contract

Denied (no capability, a sealed name), Conflict (a second enrollment in one release, a taken item),
NotFound (an unknown course, unit or release), InvalidArgument (a malformed answer).

## Course releases: built by Lectern at a git tag

A course is authored in git: a manifest, `course.toml` (this module's own format,
`format = "urn:learn:manifest:v1"`, never Lectern's `deck.toml`), naming the modules in order,
each an ordered list of units, and each unit's source kind and directory. TOML because the
builder reads it with Python's standard `tomllib`, the crate with the wasm-clean `toml` parser,
and authors already write TOML for Lectern.

`tools/build_release.py` runs in CI at the release's tag, with Lectern installed by
`tools/install_lectern.sh` at a pinned commit and its dependencies from a hash-locked file
(`tools/lectern-requirements.txt`). For each unit it runs `lectern assemble` (the `@from` map)
and `lectern build` (the reveal slides and the outline), and writes a self-contained release
directory: `{r}/course/{c}/manifest.toml` and `{r}/unit/{u}/` (`unit.toml`, `assembled.md`,
`slides/`, `outline.md`). Lectern runs only there, never inside an endpoint, so the crate stays
wasm-clean and Python stays out of the kernel. The PDF needs Playwright and Chromium, so it is
built only with `--pdf` and never in CI. The builder refuses, with one sentence saying what to
change, a URL `asset_base`, a source kind it does not know, a unit with no `deck.toml`, a
`LECTERN_COMMIT` that is unset or is not the installed commit, and a built page that loads
anything from another host.

**Blocked today:** Lectern at the pinned commit (`7531175f`, its current head) writes reveal.js
into every reveal page as a load from jsDelivr, a constant with no local mode, so the builder
refuses every reveal deck, the sample's included, until Lectern can copy reveal.js into the
release. A viewer's request to a CDN is a request nobody here controls, so no release ships one.
The committed `sample/release` is what the builder writes at the pin with that one check left
out: the endpoints and their tests read it, and it is not a release to publish.

`ikigai_learn::course::space(store)` serves the names over the release directory a host mounts
as `urn:file:{store}/` (ikigai-fs, or ikigai-repo at the tag). Each is a composition over the
release's files read through the kernel: cacheable, hanging from those files' golden threads,
storing nothing. A slide is named from the assembled deck's `---` boundaries (outside code
fences, as Lectern splits them) and the first `<!-- @from PATH slide=N -->` in it.

**A departure, said plainly:** the form and slide names carry the release,
`urn:learn:unit:name:{u}:release:{r}:form:{f}` and `urn:learn:slide:unit:{u}:number:{n}:release:{r}`,
because without the release key a slide cannot resolve, or cache, for good.

| Capability | What it allows |
|---|---|
| `urn:cap:learn:course:read` | reading a course release's names: course content, not anyone's record |
| `urn:cap:fs:read:{store}` | reading the release directory; every course door declares it too, and narrows its reads to it |

The published grant is `course::reader_grant(store)`: exactly those two. The kernel refuses a
caller missing either before a door runs, and `urn:cap:learn:release:read`, which reads a
learner's shared record, does not read a course.

## Releases: sharing part of a record

A learner shares part of their own record with another party by issuing a release
(`urn:learn:release:enrollment:{n}:party:{p}`) with one or more scopes and an expiry:

| Scope | What the party reads |
|---|---|
| `credential` | the credential only |
| `completion` | complete or not, and when |
| `progress` | mastery per unit and the verdict per exercise |
| `submission` | the work itself |

Responses, attempts, time and drafts are in no scope: a release naming one is refused, and no view
reads them. A view answers the enrollment's learner, and anyone else only while a release naming
them shares that view and has not expired. That decision is made on every call from the release
atom, read through the kernel: a session caches who you are, never what you may read. A cached view
hangs from the release atom's golden thread and expires with the release, so the Delete that
revokes a release denies the very next read.

Only the enrollment's learner issues, narrows or revokes a release. A rewrite may narrow it (fewer
scopes, an earlier expiry) and never widen it: widening is a Delete and a new release, so the
learner sees the change they are making.

That rule is enforced at the door, not by the file: the learner's grant writes
`enrollment/{n}/release` on the state root, so the learner's own `urn:file:` Sink there can
rewrite a release atom directly, wider included. That is harmless, since only the learner holds
that write and the learner may always delete and reissue; no party's grant writes there.

The learner lists their releases on an enrollment at `urn:learn:release:enrollment:{n}`: Turtle
naming each release as a member of the set, with its party, scopes and expiry. A host's
devices-and-grants page shows that listing beside the principal's other grants and revokes a
release with a Delete of its name, so it goes through the door rather than reading
`enrollment/{n}/release/` around it. Only the learner reads the set; a party is Denied it.

### Capabilities and the published grants

| Capability | What it allows |
|---|---|
| `urn:cap:learn:release:read` | reading a release (the door still answers only its learner and its party) |
| `urn:cap:learn:release:write` | issuing, narrowing and revoking a release (the door still answers only its learner) |
| `urn:cap:learn:{scope}:read` | reading the `{scope}` view (the door still checks the release) |

The published grants are functions of the crate, so a host mints exactly them and a test runs under
exactly them:

- `release::learner_grant(n, learner)`: both release capabilities, all four views, read on
  `enrollment/{n}` under the state root, write and delete on `enrollment/{n}/release` only, and the
  learner's principal.
- `release::party_grant(release)`: reading the release, and for each scope it shares that view
  and the one record file it reads (`enrollment/{n}/completion.ttl` for `completion`), read on the
  party's own release atom, and the party's principal. Nothing else on the state root: not the
  enrollment, not the responses or drafts. A host derives it from the live release on every call,
  which is how a release sits on the principal's devices-and-grants page beside their other grants.

So a view holds three floors: the kernel enforces its declared capability, the door enforces the
release, and ikigai-fs's path ACL enforces the grant on the state root.

### Where the atoms live

Under the state root a host mounts as `urn:file:` (ikigai-fs), one directory per enrollment:
`enrollment/{n}/enrollment.ttl` (its learner), `enrollment/{n}/release/{p}.ttl`, and one record
file per view (`credential.json`, `completion.ttl`, `progress.ttl`, `submission/{e}.ttl`). The atoms
use the vocabulary `urn:learn:term:` (`Release`, `enrollment`, `party`, `scope`, `expires`,
`learner`, `complete`), which this module owns.

**A departure, said plainly:** until the mastery and completion compositions are built, the
`completion` and `progress` views read a record file per view rather than computing over the
learner's atoms. One file per scope is what lets a party's grant on the state root be exactly as
narrow as the release. When the compositions land, each view computes, and the file it reads
becomes the composition's own name.

### Not built yet

Anything that leaves for another system (an LMS grade passback, xAPI statements) will leave only
under a release, and a relay subscription will carry the release's narrowed grant, rechecked at
every delivery (ikigai-events-relay), so revoking the release stops delivery. Ledger #4155 and
ledger #4157 build those; the devices-and-grants page is the host's.

## Privacy is a property, not a setting

A learner's record is theirs and exports losslessly (Turtle and JSON-LD, same names). Anyone else
sees part of it only through a release grant the learner issues and can revoke. Counts about people
are floored (`fewer than 5`). There are no per-learner rows, no time on page, no per-person funnels,
no cookies and no stored addresses anywhere in this module.

## Related

- [ikigai-credential](https://github.com/ikigai-rs/ikigai-credential): verifiable credentials for
  a completed course release.
- [ikigai-markdown](https://github.com/ikigai-rs/ikigai-markdown): the structural lift units use.
- [Wrap Lectern, or build a native course module](docs/lectern-wrap-or-native.md): the design
  note behind the plan to wrap Lectern first.
- [The ikigai Book](https://ikigai-rs.github.io/ikigai-tutorial/): the first free courses are built
  on it.

## Accessibility

See [ACCESSIBILITY.md](ACCESSIBILITY.md): the target (WCAG 2.2 AA), the evidence that exists, what
has not been tested, and how to report a barrier.

## License

MIT OR Apache-2.0.
