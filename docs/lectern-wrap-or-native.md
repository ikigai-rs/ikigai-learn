# Design note: wrap Lectern, or build a native course module

Status: **design only, nothing built.** Ledger item #4154, 2026-11-26. This note tests the current
plan (wrap Lectern first, then move the assembly into names, keep the renderer) against the
alternative of an ikigai-native course module. It changes no name in the README's resource map.
It proposes two additions to the map, the unit's source kind and the public names, for a later
pull request to land with the code that uses them.

## The two paths

- **Wrap (W).** [Lectern](https://github.com/bsletten/lectern-slides) stays the authoring tool and
  the renderer.
  - *Stage W1:* a release is built by running `lectern` in CI at the release's git tag. Its outputs
    (reveal HTML, the outline, the PDF) and its assembled provenance become `urn:learn:*`
    resources.
  - *Stage W2:* the assembly moves into names. Units, slides, ranges and partials resolve through
    the kernel as `urn:learn:*`, and Lectern's renderer stays, run by a render worker.
- **Native (N).** A Rust module, either a new `ikigai-lectern` repository or a part of this crate,
  that parses Lectern Markdown, assembles in the kernel and renders reveal HTML itself.

## Evidence

All of this was measured on one machine (16 cores, Linux, Python 3.12.3) against Lectern at commit
`7531175f245fafb90d06c49abd97c0638b66a104` (2026-10-22, package version 0.2.0). Lectern was used
read-only: cloned into a scratch directory and installed into a virtual environment with
`pip install .`, without the `pdf` extra.

| What | Result |
|---|---|
| Size of `src/lectern` | 5,913 lines of Python: CLI, config and server 1,615; renderers 1,241; PDF pipeline 1,249; assembly (preprocess, ranges, slides, source map, `Source`) 635; audits, outline, themes, assets and the rest 1,173. Tests: 3,413 lines in 23 files |
| `lectern build` of the sample deck (22 slides) to HTML, cold process | 0.15 to 0.17 s over three runs. Importing `lectern.cli` alone takes 0.13 s |
| In-process rebuild, the work `lectern watch` repeats on every save | median 9.7 ms (min 9.6, max 10.3) over 20 rebuilds, measured with a short script that calls `LiveReloadServer.build()` |
| `lectern build -f outline` and `lectern check` | 0.14 s each |
| `lectern assemble` | writes `<!-- @from PATH slide=N -->` before every contributed slide. An include inside a slide adds a second `@from` within that slide, so the sample deck shows 23 comments for 22 slides. Slide boundaries are the `---` separators, not the comments |
| `lectern watch` | rebuilds the whole deck on any change under the watched paths. It keeps no dependency graph |
| An HTML-comment directive in a slide (`<!-- question: id=q1 kind=choice -->`) | copied verbatim into `dist/index.html`, dropped from `outline.md`, and not reported by `lectern check` |
| PDF build | **not measured.** It needs the `pdf` extra (Playwright and Chromium), which was not installed |

Everything in the Source seam section comes from reading `source.py`, `assets.py`,
`theming.py`, `config.py` and `preprocess.py` at that commit.

## 1. The authoring process

In **both paths** the author writes Markdown in git and a learner never writes a unit. Slides are
Lectern Markdown. Retrieval questions and exercises are directives inside the unit, and an
exercise's task, rubric and agent policy sit beside the deck as data.

**Wrap.**

- *Install:* Python 3.11 or later and `lectern-slides`. Playwright and Chromium are needed only
  for the PDF.
- *Edit-to-preview loop:* `lectern watch` rebuilds in about 10 ms and reloads the browser over
  server-sent events. On a deck this size, the browser reload costs more than the build.
- *Questions and exercises:* Lectern has no question or exercise syntax. A directive written as an
  HTML comment is invisible in the preview, while the prose around it shows. So the author
  previews slides in Lectern and previews questions through ikigai (the lift of the unit, plus a
  check run). That means two tools, until the question renders as a slide fragment. A small
  Lectern feature could add that (a directive Lectern renders as a visible card), or stage W2
  could add it in the kernel.
- *Sealed answer keys:* the comment test above settles where keys live. **An answer key must never
  be written into a deck,** not even as a comment, because the comment ships in the published
  HTML. Keys go in a separate file per unit, outside the deck's paths and outside every path
  Lectern reads, served only under `urn:cap:learn:key:read:course:{c}`. A key in a deck should
  fail the release build. That is a deterministic check this module owns, not a Lectern change.

**Native.**

- *Install:* one Rust binary (the `ikigai` CLI with this module). That is the one real authoring
  gain: no Python environment and no second tool.
- *Edit-to-preview loop:* unmeasured, since nothing exists. It cannot beat 10 ms by a margin an
  author would notice.
- *The cost:* the preview has to reproduce Lectern's layout, themes and fragments to show the
  author what the learner sees. Until it does, the author previews something other than what
  ships.

## 2. Course composition

Composition is the same model in both paths, and it belongs to this module, not to Lectern:

- A **course release** (`urn:learn:course:name:{c}:release:{r}`) is a manifest at a git tag. It
  holds an ordered list of modules, and each module holds an ordered list of unit names.
  `deck.toml`'s `slides = [...]` already has this shape one level down: a unit is a list of
  slide sources.
- A **unit needs a declared source kind**, either a Lectern deck or an mdBook chapter. The first
  free courses rest on the ikigai and gonk Books in `ikigai-tutorial`, which are mdBook books
  (`books/*/book.toml`), not Lectern decks. The question and exercise directives must be one
  syntax shared by both kinds. That rules out making them a Lectern feature alone.
- **Reuse.** A unit is reused across courses by naming it in more than one manifest. A slide is
  reused within a deck through Lectern's `file#1-3,14` include, and across courses through its
  name, `urn:learn:slide:unit:{u}:number:{n}`. The number is 1-based, matching Lectern's range
  grammar.
- **Release by git tag.** ikigai-repo reads the unit sources at the tag, so a release resolves
  the same way for good and caches forever.

What golden-thread rebuilds cost and buy:

| | Wrap W1 | Wrap W2 | Native |
|---|---|---|---|
| Granularity | the deck. Lectern reads its files outside the kernel, so the kernel sees only "this deck was built at this tag" | the file. Every slide source, partial, theme and asset read goes through the kernel | the file |
| How an edited partial finds its decks | every Markdown file a deck included is in its `@from` comments, so a CI step can rebuild exactly the decks that cite the file. Themes and assets are not in `@from` | the golden thread does it | the golden thread does it |
| Cost | none: a whole release rebuilds at about 0.15 s per deck (about 15 s for 100 decks), and a tag build is a clean build anyway | the Source seam fix (section 5) | the rewrite |
| What it buys | correct caching of the served release | precise invalidation of drafts between releases, plus citation of a partial's slides wherever they appear | the same as W2 |

**Moving the assembly into names buys reuse and citation, not speed.** The build is already
faster than the reload. Precise invalidation matters between releases (draft previews served by
the kernel), not within one.

## 3. Publishing

The same in both paths:

1. A git tag on the course repository triggers CI, which builds every unit at the tag.
2. The outputs are stored as resources: `…:release:{r}`, the `form:{f}` faces (`slides`, `page`,
   `transcript`, `handout`), and one `slide:unit:{u}:number:{n}` per slide, named from the
   assembled `---` boundaries and their first `@from`.
3. A slide's HTML face is the unit's reveal page plus a fragment. reveal.js counts from zero by
   default (Lectern sets `hash: true` and leaves `hashOneBasedIndex` unset), so slide `n` is
   `#/{n-1}`.

`urn:learn:` names are kernel-local. They do not resolve on the open web. People and credentials
reach a release through persistent names that ikigai-name resolves, both claimed in its registry
with a `redirect` strategy whose target is the current host:

- an IRI on iriref.org, for example `https://iriref.org/learn/course/{c}/release/{r}`
- a URN through urnid.org, for example `urn:urnid:learn:course:{c}:release:{r}`

The exact prefixes are proposals for Brian to claim. A credential from ikigai-credential cites the
persistent name, never a host URL.

**What keeps resolving if the host changes:** everything named through iriref.org or urnid.org.
Moving hosts is an edit to one rule in the registry, with no rebuild, and `as-of` still reports
the old answer for earlier dates. **What does not:** a host URL baked into built HTML. Lectern
writes a URL `asset_base` into every root-absolute asset reference when it builds. So a release
must be built with local assets (Lectern then copies them, named by content hash, into a relative
`assets/` folder, and the release is self-contained), or with an `asset_base` that is itself a
persistent IRI. The release build should refuse any other URL `asset_base`.

## 4. What each path keeps or loses

| | Wrap (W1, W2) | Native |
|---|---|---|
| reveal.js renderer (plus remark, Marp and Quarto adapters) | kept | reimplemented, reveal only at first |
| Eleven themes plus `base.css`, with the AA contrast audit | kept | the CSS can be reused as files, the audit reimplemented |
| Accessibility audit (slide names, iframe titles, raw `img` alt, Mermaid `accTitle`/`accDescr`, icon names) | kept, source-cited to `file:line` | reimplemented |
| Tagged PDF and imposition (1,249 lines, Playwright plus Chromium) | kept | reimplemented, and it would still need a headless browser |
| Per-slide citation | W1: from `@from` plus slide boundaries, no Lectern change. W2: native names | native names |
| Golden-thread rebuilds | W1: per deck. W2: per file | per file |
| Transcript form | kept: `-f outline` is already a heading-structured transcript, with notes as prose and diagrams as their accessible description. It fits `form:transcript` as is | reimplemented |
| mdBook chapters as units | neither path gets them from Lectern. This module lifts them through ikigai-markdown in both | the same |

Native's losses are concrete and tested code. Its gains (one binary, wasm-clean assembly) are real
but internal: neither changes what a learner sees.

## 5. The Source seam finding

The finding holds, and it is slightly wider than reported:

- `Source` (`source.py`) is a protocol of `read`, `exists` and `list`, keyed by `pathlib.Path`.
  Only `preprocess.py` uses it, and only for slide and partial Markdown.
- `assets.py` reads the disk directly (`is_file`, `read_bytes`) for relative and local-root
  assets.
- `theming.py` reads deck-local and `theme_paths` themes from disk. Bundled themes come from the
  installed package through `importlib.resources`.
- `config.py` reads `deck.toml` and the user config with `open()` and probes directories with
  `is_dir()`. **The manifest itself does not go through `Source` either.**

How much of the wrap path depends on fixing it:

- **Stage W1 needs none of it.** CI checks out the tag and Lectern reads a real directory.
- **Stage W2 needs IRI keying for Markdown first.** Without it there is no kernel-side assembly.
  Assets and themes come next, and they matter less:
  - bundled themes and root-absolute assets under a URL `asset_base` already work without disk
  - deck-local themes and relative assets need the routing fix
  - the manifest needs it only if W2 reads `deck.toml` through Lectern; the course manifest is
    this module's own, so it need not
- **Native does not depend on it.** That is the one thing native gains here, and it is not a
  reason a learner or an author sees.

The seam fix is an upstream change to `bsletten/lectern-slides`, which is Brian's repository and
outside the ikigai-rs org. This note proposes nothing there and files nothing there.

## 6. Recommendation

**Wrap, and keep the work inside `ikigai-learn`. No `ikigai-lectern` repository.**

- **Why wrap.** The plan holds. Lectern already has what a learner sees (the renderer, the themes,
  contrast and accessibility audits, the transcript, the PDF). Its build is faster than the
  browser reload. Its `@from` provenance already supports per-slide citation. A rewrite would
  replace about 5,900 lines of tested Python, with no difference a learner or an author would see.
- **Why no new repository.** The wrap leaves nothing that is "Lectern in Rust". What remains is the
  course model, the unit source kinds, the question and exercise directives, sealed keys,
  publishing and the render worker. All of it is this module's resource map.
- **Where the pieces go.**
  - The Lectern Markdown mapping goes in ikigai-markdown, where the structural lift already lives.
  - The seam fix belongs upstream in Lectern, at Brian's discretion.
  - Lectern runs at build time in CI (W1), or in a render worker that takes tuplespace offers
    (W2), never inside an endpoint. That keeps this crate wasm-clean and Python out of the
    kernel.

| Reused as is | New here (not reimplementations) | Reimplemented |
|---|---|---|
| Lectern: assembly (W1), renderer, themes, audits, outline, PDF. ikigai-markdown: the lift. ikigai-repo: reads at a tag. ikigai-name: persistent names. ikigai-credential | course and module manifest, unit source kind, question and exercise directives shared by decks and Book chapters, the key-in-deck check, the release builder, the slide naming from `@from` | nothing in W1. In W2, assembly as kernel resolution, *if* the seam fix is not taken upstream |

**What would justify revisiting native:** authoring or rendering that has to run where Python
cannot (in the browser or under wasm), or the seam fix being declined upstream once W2 is due.

## Open questions for Brian

1. May a unit be an mdBook chapter? This note assumes yes, since the first courses are Books.
2. Where do answer keys live for free public courses? A private sibling repository, or a sealed
   path in the course repository served only under the key capability?
3. Should questions render in Lectern's preview, through a small upstream feature, or only
   through ikigai?
4. Which iriref.org and urnid.org prefixes should the course, unit and credential names claim?

## Reproducing the measurements

```sh
git clone https://github.com/bsletten/lectern-slides && cd lectern-slides
git checkout 7531175f245fafb90d06c49abd97c0638b66a104
python3 -m venv /tmp/venv && /tmp/venv/bin/pip install .
cp -r examples/sample-deck /tmp/deck
time /tmp/venv/bin/lectern build /tmp/deck            # HTML
/tmp/venv/bin/lectern assemble /tmp/deck | grep -c '@from'
```

The in-process figure came from calling `lectern.serve.LiveReloadServer(deck, deck / "dist",
open_browser=False).build()` 20 times after a warm-up build and taking the median.
