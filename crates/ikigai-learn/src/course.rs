//! Stage W1: a course release built by Lectern at a git tag, served as `urn:learn:` names.
//!
//! `tools/build_release.py` runs Lectern in CI at the tag and writes a self-contained release
//! directory. Every name here is a composition over that directory's files, read through the
//! kernel (`urn:file:`, from ikigai-fs or ikigai-repo at the tag) and narrowed to read the
//! release store ([`fs_read`]), so each answer is cacheable and hangs from the golden threads
//! of the files it read. Nothing here stores.
//!
//! This is the course's content, not a learner's record: its capability,
//! [`CAP_COURSE_READ`], is its own, and the sharing capability of [`crate::release`] does
//! not read it.
//!
//! | Name | File read | Answer |
//! |---|---|---|
//! | `urn:learn:course:name:{c}:release:{r}` | `{r}/course/{c}/manifest.toml` | JSON: modules, each an ordered list of unit names |
//! | `urn:learn:unit:name:{u}:release:{r}` | `{r}/unit/{u}/unit.toml`, `assembled.md` | JSON: the source kind, its forms and slides |
//! | `urn:learn:unit:name:{u}:release:{r}:form:{f}` | `slides/index.html`, `outline.md` | the face itself |
//! | `urn:learn:slide:unit:{u}:number:{n}:release:{r}` | `{r}/unit/{u}/assembled.md` | JSON: the source file and slide number, and the HTML face |
use ikigai_core::{
    ArgSpec, AsyncFnEndpoint, Description, EndpointSpace, Error, FailureClass, Invocation, Iri,
    ReprType, Representation, Result, UriTemplate, Verb,
};
use serde::Deserialize;
use serde_json::json;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const JSON: &str = "application/json";

/// The capability a caller holds to read a course release's names. Course content, so it is
/// not [`crate::release::CAP_RELEASE_READ`], which reads a learner's shared record.
pub const CAP_COURSE_READ: &str = "urn:cap:learn:course:read";

/// The course release template.
pub const COURSE_TEMPLATE: &str = "urn:learn:course:name:{c}:release:{r}";
/// The unit-at-a-release template.
pub const UNIT_TEMPLATE: &str = "urn:learn:unit:name:{u}:release:{r}";
/// A face of a unit's build.
pub const FORM_TEMPLATE: &str = "urn:learn:unit:name:{u}:release:{r}:form:{f}";
/// One slide of a unit at a release.
pub const SLIDE_TEMPLATE: &str = "urn:learn:slide:unit:{u}:number:{n}:release:{r}";

/// The manifest format this module reads (`course.toml`'s `format` line).
pub const MANIFEST_FORMAT: &str = "urn:learn:manifest:v1";

/// A unit's declared source kind.
///
/// Only [`SourceKind::LecternDeck`] exists in this stage. An `mdbook-chapter` kind attaches
/// here as a second variant, with its own forms in [`SourceKind::forms`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// A Lectern deck, built by `lectern build` and `lectern assemble`.
    LecternDeck,
}

impl SourceKind {
    /// Parse a kind as the builder writes it in `unit.toml`.
    pub fn parse(kind: &str) -> Option<Self> {
        match kind {
            "lectern-deck" => Some(SourceKind::LecternDeck),
            _ => None,
        }
    }

    /// The kind's name.
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::LecternDeck => "lectern-deck",
        }
    }

    /// The forms this kind builds: `(form, file under the unit's directory, media type)`.
    /// A `handout` (the PDF) is optional and never built in CI, so it is not listed.
    pub fn forms(self) -> &'static [(&'static str, &'static str, &'static str)] {
        match self {
            SourceKind::LecternDeck => &[
                ("slides", "slides/index.html", "text/html"),
                ("outline", "outline.md", "text/markdown"),
                // Lectern's outline is already a heading-structured transcript (notes as
                // prose, diagrams as their description): the design note's finding.
                ("transcript", "outline.md", "text/markdown"),
            ],
        }
    }
}

/// One slide of an assembled deck: its 1-based number and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    /// 1-based, matching Lectern's range grammar.
    pub number: usize,
    /// The source file, relative to the deck, from the slide's first `@from`.
    pub source: Option<String>,
    /// The slide's number within its source file.
    pub source_slide: Option<usize>,
}

/// Split `lectern assemble` output into slides, as Lectern does: on lines that are exactly
/// `---` once trimmed, outside fenced code. Each slide's provenance is its first
/// `<!-- @from PATH slide=N -->` comment; an include inside a slide adds a later one, which
/// does not name the slide.
///
/// ```
/// let slides = ikigai_learn::course::slides(
///     "<!-- @from a.md slide=1 -->\n# A\n---\n<!-- @from b.md slide=2 -->\n```\n---\n```\n",
/// );
/// assert_eq!(slides.len(), 2);
/// assert_eq!(slides[1].source.as_deref(), Some("b.md"));
/// assert_eq!(slides[1].source_slide, Some(2));
/// ```
pub fn slides(assembled: &str) -> Vec<Slide> {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    let mut current: Option<(String, usize)> = None;
    let push = |current: &mut Option<(String, usize)>, out: &mut Vec<Slide>| {
        let (source, source_slide) = match current.take() {
            Some((s, n)) => (Some(s), Some(n)),
            None => (None, None),
        };
        out.push(Slide {
            number: out.len() + 1,
            source,
            source_slide,
        });
    };
    for line in assembled.split('\n') {
        match fence {
            None => {
                if let Some(marker) = fence_marker(line) {
                    fence = Some((marker.0, marker.1));
                } else if line.trim() == "---" {
                    push(&mut current, &mut out);
                    continue;
                } else if current.is_none() {
                    current = from_comment(line);
                }
            }
            Some((ch, len)) => {
                if let Some((c, l, info)) = fence_marker(line) {
                    if c == ch && l >= len && info.trim().is_empty() {
                        fence = None;
                    }
                }
            }
        }
    }
    push(&mut current, &mut out);
    out
}

/// A fence marker: up to three spaces, then three or more backticks or tildes.
fn fence_marker(line: &str) -> Option<(char, usize, &str)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run = rest.len() - rest.trim_start_matches(ch).len();
    (run >= 3).then(|| (ch, run, &rest[run..]))
}

/// `<!-- @from PATH slide=N -->`, as `lectern assemble` writes it.
fn from_comment(line: &str) -> Option<(String, usize)> {
    let inner = line
        .trim()
        .strip_prefix("<!--")?
        .strip_suffix("-->")?
        .trim();
    let rest = inner.strip_prefix("@from ")?;
    let (path, slide) = rest.rsplit_once(" slide=")?;
    Some((path.trim().to_string(), slide.trim().parse().ok()?))
}

/// The file scope a reader holds over the store the releases are mounted under: read, under
/// `store`, and nothing else. Every door declares it and every file read is narrowed to it.
///
/// ```
/// assert_eq!(ikigai_learn::course::fs_read("releases"), "urn:cap:fs:read:releases");
/// ```
pub fn fs_read(store: &str) -> String {
    format!("urn:cap:fs:read:{}", store.trim_end_matches('/'))
}

/// The published grant of a course reader over the releases under `urn:file:{store}/`:
/// [`CAP_COURSE_READ`] and [`fs_read`]`(store)`, nothing else. A host mints exactly this, and
/// the tests run under exactly this.
///
/// ```
/// assert_eq!(
///     ikigai_learn::course::reader_grant("release"),
///     ["urn:cap:learn:course:read", "urn:cap:fs:read:release"],
/// );
/// ```
pub fn reader_grant(store: &str) -> Vec<String> {
    vec![CAP_COURSE_READ.to_string(), fs_read(store)]
}

/// The course release names, reading built releases under `urn:file:{store}/`.
///
/// The host mounts a file space holding the builder's output at `store` and names this
/// space itself (it depends on `store`). A reader holds [`reader_grant`]`(store)`; the
/// kernel refuses a caller missing either scope before a door runs.
pub fn space(store: &str) -> EndpointSpace {
    let store = store.trim_end_matches('/').to_string();
    let template = |t: &str| UriTemplate::parse(t).expect("the templates are valid");
    EndpointSpace::new()
        .bind(
            template(COURSE_TEMPLATE),
            endpoint(
                "learn-course",
                "A course release: its manifest at a git tag",
                "JSON: the course's modules in order, each an ordered list of unit names at \
                 the release. NotFound when the course or the release is not built.",
                &[("c", "the course's name"), ("r", "the release: a git tag")],
                &store,
                course,
            ),
        )
        .bind(
            template(FORM_TEMPLATE),
            endpoint(
                "learn-form",
                "A face of a unit's build at a release",
                "The face itself: `slides` (Lectern's reveal HTML), `outline` or `transcript` \
                 (Markdown). NotFound for a form the unit's kind does not build.",
                &[
                    ("u", "the unit's name"),
                    ("r", "the release: a git tag"),
                    ("f", "the form: slides, outline or transcript"),
                ],
                &store,
                form,
            ),
        )
        .bind(
            template(UNIT_TEMPLATE),
            endpoint(
                "learn-unit",
                "A unit at a release",
                "JSON: the unit's source kind, the Lectern commit that built it, its forms and \
                 its slides in order. NotFound when the unit is not in the release.",
                &[("u", "the unit's name"), ("r", "the release: a git tag")],
                &store,
                unit,
            ),
        )
        .bind(
            template(SLIDE_TEMPLATE),
            endpoint(
                "learn-slide",
                "One slide of a unit at a release",
                "JSON: the slide's source file and its number in that file (from the first \
                 `@from` in the slide), and its HTML face: the slides form plus `#/{n-1}`. \
                 NotFound past the last slide.",
                &[
                    ("u", "the unit's name"),
                    ("n", "the slide's number, from 1"),
                    ("r", "the release: a git tag"),
                ],
                &store,
                slide,
            ),
        )
}

type Handler = for<'a> fn(
    &'a Invocation<'_>,
    &'a str,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Representation>> + Send + 'a>,
>;

fn binding(name: &str, summary: &str) -> ArgSpec {
    let class = if name == "n" {
        "positiveInteger"
    } else {
        "string"
    };
    ArgSpec::new(name)
        .summary(summary)
        .class(format!("{XSD}{class}"))
        .binding()
}

fn endpoint(
    name: &'static str,
    title: &str,
    summary: &str,
    inputs: &[(&str, &str)],
    store: &str,
    handler: Handler,
) -> AsyncFnEndpoint {
    let mut d = Description::new(name)
        .title(title)
        .summary(summary)
        .verb(Verb::Source)
        .verb(Verb::Meta)
        .requires(CAP_COURSE_READ)
        .requires(fs_read(store))
        .fails(FailureClass::Permanent)
        .fails(FailureClass::Unknown);
    for (n, s) in inputs {
        d = d.input(binding(n, s));
    }
    d = if name == "learn-form" {
        d.output("text/html").output("text/markdown")
    } else {
        d.output(JSON)
    };
    let store = store.to_string();
    AsyncFnEndpoint::new(name, move |inv| {
        let store = store.clone();
        Box::pin(async move { handler(inv, &store).await })
    })
    .with_description(d)
}

/// A key bound from the request: a name that is also one path segment of the store.
fn key<'a>(inv: &'a Invocation<'_>, var: &str) -> Result<&'a str> {
    let value = inv
        .bindings
        .get(var)
        .ok_or_else(|| Error::MissingArgument(var.into()))?;
    let ok = !value.is_empty()
        && !value.contains("..")
        && value
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if ok {
        Ok(value)
    } else {
        Err(Error::InvalidArgument {
            name: var.to_string(),
            detail: format!(
                "`{value}` is not a name this module accepts: use letters, digits, dots, \
                 hyphens and underscores, starting with a letter or digit, with no `..`"
            ),
        })
    }
}

/// Read one file of a release through the kernel, narrowed to read under the store and
/// nothing else.
async fn read(inv: &Invocation<'_>, store: &str, path: &str) -> Result<String> {
    let iri = Iri::parse(format!("urn:file:{store}/{path}"))
        .map_err(|e| Error::Endpoint(format!("a release path is not an IRI: {e}")))?;
    let repr = inv
        .source_attenuated(&iri, [fs_read(store)])
        .await
        .map_err(|e| match e {
            Error::NotFound(_) => Error::NotFound(format!(
                "`{}` is not in any built release",
                inv.request.target
            )),
            other => other,
        })?;
    String::from_utf8(repr.bytes)
        .map_err(|_| Error::Endpoint(format!("`{path}` in the release is not UTF-8")))
}

fn json_repr(value: serde_json::Value) -> Representation {
    Representation::new(ReprType::new(JSON), value.to_string().into_bytes()).cacheable()
}

fn unit_iri(u: &str, r: &str) -> String {
    format!("urn:learn:unit:name:{u}:release:{r}")
}

#[derive(Deserialize)]
struct Manifest {
    format: String,
    name: String,
    title: Option<String>,
    #[serde(default)]
    module: Vec<Module>,
}

#[derive(Deserialize)]
struct Module {
    name: String,
    title: Option<String>,
    #[serde(default)]
    units: Vec<String>,
}

#[derive(Deserialize)]
struct UnitFile {
    kind: String,
    lectern: Option<String>,
}

fn malformed(what: &str, e: impl std::fmt::Display) -> Error {
    Error::Endpoint(format!("the release's {what} is malformed: {e}"))
}

fn course<'a>(
    inv: &'a Invocation<'_>,
    store: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Representation>> + Send + 'a>> {
    Box::pin(async move {
        let (c, r) = (key(inv, "c")?, key(inv, "r")?);
        let text = read(inv, store, &format!("{r}/course/{c}/manifest.toml")).await?;
        let manifest: Manifest = toml::from_str(&text).map_err(|e| malformed("manifest", e))?;
        if manifest.format != MANIFEST_FORMAT || manifest.name != c {
            return Err(malformed("manifest", "its format or name does not match"));
        }
        let modules: Vec<_> = manifest
            .module
            .iter()
            .map(|m| {
                json!({
                    "name": m.name,
                    "title": m.title,
                    "units": m.units.iter().map(|u| unit_iri(u, r)).collect::<Vec<_>>(),
                })
            })
            .collect();
        Ok(json_repr(json!({
            "@id": inv.request.target.as_str(),
            "course": c,
            "release": r,
            "title": manifest.title,
            "modules": modules,
        })))
    })
}

async fn unit_kind(
    inv: &Invocation<'_>,
    store: &str,
    u: &str,
    r: &str,
) -> Result<(SourceKind, UnitFile)> {
    let text = read(inv, store, &format!("{r}/unit/{u}/unit.toml")).await?;
    let file: UnitFile = toml::from_str(&text).map_err(|e| malformed("unit.toml", e))?;
    let kind = SourceKind::parse(&file.kind)
        .ok_or_else(|| malformed("unit.toml", format!("unknown source kind `{}`", file.kind)))?;
    Ok((kind, file))
}

fn unit<'a>(
    inv: &'a Invocation<'_>,
    store: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Representation>> + Send + 'a>> {
    Box::pin(async move {
        let (u, r) = (key(inv, "u")?, key(inv, "r")?);
        let (kind, file) = unit_kind(inv, store, u, r).await?;
        let assembled = read(inv, store, &format!("{r}/unit/{u}/assembled.md")).await?;
        let base = unit_iri(u, r);
        let forms: serde_json::Map<_, _> = kind
            .forms()
            .iter()
            .map(|(f, _, _)| (f.to_string(), json!(format!("{base}:form:{f}"))))
            .collect();
        let slides: Vec<_> = slides(&assembled)
            .iter()
            .map(|s| format!("urn:learn:slide:unit:{u}:number:{}:release:{r}", s.number))
            .collect();
        Ok(json_repr(json!({
            "@id": base,
            "unit": u,
            "release": r,
            "kind": kind.as_str(),
            "lectern": file.lectern,
            "forms": forms,
            "slides": slides,
        })))
    })
}

fn form<'a>(
    inv: &'a Invocation<'_>,
    store: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Representation>> + Send + 'a>> {
    Box::pin(async move {
        let (u, r, f) = (key(inv, "u")?, key(inv, "r")?, key(inv, "f")?);
        let (kind, _) = unit_kind(inv, store, u, r).await?;
        let Some((_, path, media)) = kind.forms().iter().find(|(name, _, _)| *name == f) else {
            return Err(Error::NotFound(format!(
                "a {} unit has no form `{f}`: ask for slides, outline or transcript",
                kind.as_str()
            )));
        };
        let body = read(inv, store, &format!("{r}/unit/{u}/{path}")).await?;
        Ok(Representation::new(
            ReprType::new(*media).with_param("charset", "utf-8"),
            body.into_bytes(),
        )
        .cacheable())
    })
}

fn slide<'a>(
    inv: &'a Invocation<'_>,
    store: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Representation>> + Send + 'a>> {
    Box::pin(async move {
        let (u, r) = (key(inv, "u")?, key(inv, "r")?);
        let raw = inv.bindings.get("n").unwrap_or("");
        let n: usize = raw
            .parse()
            .ok()
            .filter(|n| *n >= 1 && raw.bytes().all(|b| b.is_ascii_digit()) && !raw.starts_with('0'))
            .ok_or_else(|| Error::InvalidArgument {
                name: "n".into(),
                detail: format!("`{raw}` is not a slide number: slides are numbered from 1"),
            })?;
        let assembled = read(inv, store, &format!("{r}/unit/{u}/assembled.md")).await?;
        let all = slides(&assembled);
        let Some(s) = all.get(n - 1) else {
            return Err(Error::NotFound(format!(
                "unit `{u}` at release `{r}` has {} slides, so there is no slide {n}",
                all.len()
            )));
        };
        Ok(json_repr(json!({
            "@id": inv.request.target.as_str(),
            "unit": unit_iri(u, r),
            "number": n,
            "source": s.source,
            "sourceSlide": s.source_slide,
            "html": format!("{}:form:slides#/{}", unit_iri(u, r), n - 1),
        })))
    })
}
