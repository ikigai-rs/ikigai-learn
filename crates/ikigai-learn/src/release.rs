//! The release grant: a learner shares one part of their own record with one party.
//!
//! `urn:learn:release:enrollment:{n}:party:{p}` is an ATOM only the enrollment's learner writes,
//! naming the party (its principal, whose bare-hex BLAKE3 digest is `{p}`), the scopes it
//! releases and when it expires. Deleting it revokes it. The four record views
//! (`urn:learn:{scope}:enrollment:{n}`, and per exercise for a submission) answer the learner
//! always, and anyone else only while a release naming them covers that view and has not
//! expired. The decision is made on every call from the release atom read through the kernel,
//! never remembered: a cached view hangs from the release atom's golden thread, so the Delete
//! that revokes a release cuts every answer it allowed, and a cached view expires with it.
//!
//! Two floors hold each view. The kernel enforces the view's declared capability
//! (`urn:cap:learn:{scope}:read`) before the door runs, and the door enforces the release. The
//! sub-requests run under the CALLER's capability, so ikigai-fs's path ACL on the state root is
//! a third: a party's published grant reads only its own release atom and the one record file
//! its scope names.
//!
//! The atoms live under the state root a host mounts as `urn:file:` (ikigai-fs), one directory
//! per enrollment:
//!
//! | File | What it holds | Who writes it |
//! |---|---|---|
//! | `enrollment/{n}/enrollment.ttl` | the enrollment: its learner's principal | the enroll Sink |
//! | `enrollment/{n}/release/{p}.ttl` | a release to the party whose key is `{p}` | the learner, through this door |
//! | `enrollment/{n}/credential.json` | the issued credential | ikigai-credential |
//! | `enrollment/{n}/completion.ttl` | complete or not, and when | the completion composition |
//! | `enrollment/{n}/progress.ttl` | mastery per unit, verdict per exercise | the progress composition |
//! | `enrollment/{n}/submission/{e}.ttl` | the work submitted for exercise `{e}` | the submit Sink |
//!
//! Responses, attempts, time and drafts have no view here and no scope names them: no release
//! can reach them.

use std::collections::BTreeSet;
use std::fmt;

use ikigai_core::{
    principal_scope, ActionSpec, ArgRef, ArgSpec, AsyncFnEndpoint, ContentId, Denial, Description,
    EndpointSpace, Error, FailureClass, Invocation, Iri, ReprType, Representation, Request, Result,
    UriTemplate, Verb,
};
use oxrdf::{NamedOrBlankNode, Term};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const TURTLE: &str = "text/turtle";
const TEXT: &str = "text/plain";

/// The vocabulary the release and enrollment atoms use.
pub const TERM: &str = "urn:learn:term:";

/// The capability that reads a release (the learner's own, or the party's).
pub const CAP_RELEASE_READ: &str = "urn:cap:learn:release:read";

/// The capability that issues, narrows and revokes a release. The door still refuses anyone
/// but the enrollment's learner.
pub const CAP_RELEASE_WRITE: &str = "urn:cap:learn:release:write";

/// What a release may share. Nothing else is ever released.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    /// The issued credential only.
    Credential,
    /// Complete or not, and when.
    Completion,
    /// Mastery per unit and the verdict per exercise.
    Progress,
    /// The submitted work itself.
    Submission,
}

impl Scope {
    /// Every scope, in the order they widen.
    pub const ALL: [Scope; 4] = [
        Scope::Credential,
        Scope::Completion,
        Scope::Progress,
        Scope::Submission,
    ];

    /// The scope's name, as a release and a view spell it.
    pub fn name(self) -> &'static str {
        match self {
            Scope::Credential => "credential",
            Scope::Completion => "completion",
            Scope::Progress => "progress",
            Scope::Submission => "submission",
        }
    }

    /// The capability a view of this scope declares: `urn:cap:learn:{scope}:read`.
    pub fn read_cap(self) -> String {
        format!("urn:cap:learn:{}:read", self.name())
    }

    /// The scope a name spells, or the sentence that refuses it.
    pub fn parse(name: &str) -> std::result::Result<Scope, String> {
        if let Some(s) = Scope::ALL.into_iter().find(|s| s.name() == name) {
            return Ok(s);
        }
        if NEVER_RELEASED.contains(&name) {
            return Err(format!(
                "`{name}` is never released: a release shares only credential, completion, \
                 progress or submission"
            ));
        }
        Err(format!(
            "`{name}` is not a scope: use credential, completion, progress or submission"
        ))
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The parts of a record no release ever reaches, named so a request for one is refused with
/// that reason rather than as an unknown word.
pub const NEVER_RELEASED: [&str; 5] = ["attempt", "response", "time", "draft", "drafts"];

/// The name of a release.
pub fn release_name(enrollment: &str, party_key: &str) -> String {
    format!("urn:learn:release:enrollment:{enrollment}:party:{party_key}")
}

/// The name of an enrollment.
pub fn enrollment_name(enrollment: &str) -> String {
    format!("urn:learn:enrollment:id:{enrollment}")
}

/// The `{p}` of a release to `principal`: the bare-hex BLAKE3 digest of its IRI.
pub fn party_key(principal: &str) -> String {
    let id = ContentId::of(principal.as_bytes()).to_string();
    id.strip_prefix("b3:").unwrap_or(&id).to_string()
}

/// The enrollment atom's path under the state root.
pub fn enrollment_path(enrollment: &str) -> String {
    format!("enrollment/{enrollment}/enrollment.ttl")
}

/// A release atom's path under the state root.
pub fn release_path(enrollment: &str, party_key: &str) -> String {
    format!("enrollment/{enrollment}/release/{party_key}.ttl")
}

/// The record file a view reads, under the state root. `exercise` is the submission's.
pub fn record_path(scope: Scope, enrollment: &str, exercise: Option<&str>) -> String {
    match scope {
        Scope::Credential => format!("enrollment/{enrollment}/credential.json"),
        Scope::Completion => format!("enrollment/{enrollment}/completion.ttl"),
        Scope::Progress => format!("enrollment/{enrollment}/progress.ttl"),
        Scope::Submission => format!(
            "enrollment/{enrollment}/submission/{}.ttl",
            exercise.unwrap_or_default()
        ),
    }
}

/// The Turtle of an enrollment atom: whose enrollment it is.
pub fn enrollment_turtle(enrollment: &str, learner: &str) -> String {
    format!(
        "<{}> <{TERM}learner> <{learner}> .\n",
        enrollment_name(enrollment)
    )
}

/// A stored release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// The enrollment it releases part of.
    pub enrollment: String,
    /// The party's principal.
    pub party: String,
    /// What it shares.
    pub scopes: BTreeSet<Scope>,
    /// When it stops sharing, in milliseconds since the epoch (UTC).
    pub expires: u64,
}

impl Release {
    /// The release's Turtle atom.
    pub fn turtle(&self) -> String {
        let name = release_name(&self.enrollment, &party_key(&self.party));
        let mut out = format!(
            "<{name}> a <{TERM}Release> ;\n  <{TERM}enrollment> <{}> ;\n  <{TERM}party> <{}> ;\n",
            enrollment_name(&self.enrollment),
            self.party
        );
        for scope in &self.scopes {
            out.push_str(&format!("  <{TERM}scope> \"{scope}\" ;\n"));
        }
        out.push_str(&format!(
            "  <{TERM}expires> \"{}\"^^<{XSD}dateTime> .\n",
            date_time(self.expires)
        ));
        out
    }

    /// The scopes, comma-separated.
    pub fn scope_list(&self) -> String {
        let names: Vec<&str> = self.scopes.iter().map(|s| s.name()).collect();
        names.join(",")
    }

    /// Whether `other` gives nothing this release does not: no scope it lacks, no later expiry.
    pub fn covers(&self, other: &Release) -> bool {
        other.scopes.is_subset(&self.scopes) && other.expires <= self.expires
    }
}

// --- reading atoms back -------------------------------------------------------------------

fn triples(text: &str, name: &str) -> Result<Vec<(String, String, Term)>> {
    let mut out = Vec::new();
    for triple in oxttl::TurtleParser::new().for_slice(text.as_bytes()) {
        let t = triple
            .map_err(|e| Error::Endpoint(format!("the atom of <{name}> does not parse: {e}")))?;
        let subject = match t.subject {
            NamedOrBlankNode::NamedNode(n) => n.into_string(),
            other => other.to_string(),
        };
        out.push((subject, t.predicate.into_string(), t.object));
    }
    Ok(out)
}

fn values<'a>(
    ts: &'a [(String, String, Term)],
    subject: &'a str,
    local: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let predicate = format!("{TERM}{local}");
    ts.iter()
        .filter(move |(s, p, _)| s == subject && *p == predicate)
        .map(|(_, _, o)| match o {
            Term::NamedNode(n) => n.as_str().to_string(),
            Term::Literal(l) => l.value().to_string(),
            other => other.to_string(),
        })
}

/// Read a release atom back. Anything malformed fails closed: an atom that does not say
/// exactly one party, one enrollment, one expiry and only known scopes releases nothing.
pub fn parse_release(text: &str, enrollment: &str, key: &str) -> Result<Release> {
    let name = release_name(enrollment, key);
    let ts = triples(text, &name)?;
    let bad = |what: &str| Error::Endpoint(format!("the atom of <{name}> {what}"));
    let one = |local: &str| -> Result<String> {
        let mut all: Vec<String> = values(&ts, &name, local).collect();
        match all.len() {
            1 => Ok(all.remove(0)),
            0 => Err(bad(&format!("has no {local}"))),
            _ => Err(bad(&format!("has more than one {local}"))),
        }
    };
    let party = one("party")?;
    if party_key(&party) != key {
        return Err(bad("names a party whose key is not the one in its name"));
    }
    if one("enrollment")? != enrollment_name(enrollment) {
        return Err(bad("names another enrollment"));
    }
    let expires = millis_of(&one("expires")?)
        .ok_or_else(|| bad("has an expiry that is not a UTC date-time"))?;
    let scopes = values(&ts, &name, "scope")
        .map(|s| Scope::parse(&s).map_err(|e| bad(&e)))
        .collect::<Result<BTreeSet<Scope>>>()?;
    if scopes.is_empty() {
        return Err(bad("releases no scope"));
    }
    Ok(Release {
        enrollment: enrollment.into(),
        party,
        scopes,
        expires,
    })
}

/// The learner an enrollment atom names.
pub fn parse_learner(text: &str, enrollment: &str) -> Result<String> {
    let name = enrollment_name(enrollment);
    let ts = triples(text, &name)?;
    let mut all: Vec<String> = values(&ts, &name, "learner").collect();
    if all.len() != 1 {
        return Err(Error::Endpoint(format!(
            "the atom of <{name}> does not name exactly one learner"
        )));
    }
    Ok(all.remove(0))
}

// --- time ---------------------------------------------------------------------------------

/// `YYYY-MM-DDThh:mm:ssZ` for milliseconds since the epoch (UTC), to the second.
pub fn date_time(millis: u64) -> String {
    let secs = millis / 1000;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Milliseconds since the epoch for a `YYYY-MM-DDThh:mm:ssZ` UTC date-time, to the second.
pub fn millis_of(text: &str) -> Option<u64> {
    let b = text.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    let n = |r: std::ops::Range<usize>| -> Option<i64> {
        let part = &text[r];
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    let (y, m, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (hh, mm, ss) = (n(11..13)?, n(14..16)?, n(17..19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 59 {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + hh * 3600 + mm * 60 + ss;
    let millis = u64::try_from(secs).ok()?.checked_mul(1000)?;
    // A date that does not exist (February 30) does not round-trip.
    (date_time(millis) == text).then_some(millis)
}

// --- names and arguments ------------------------------------------------------------------

/// A name usable as one name segment: lowercase ASCII letters, digits and single hyphens,
/// starting with a letter or digit, at most 64 characters.
pub fn is_segment(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_key(key: &str) -> bool {
    key.len() == 64 && key.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

fn invalid(name: &str, detail: impl Into<String>) -> Error {
    Error::InvalidArgument {
        name: name.into(),
        detail: detail.into(),
    }
}

fn segment(inv: &Invocation<'_>, name: &str) -> Result<String> {
    let value = inv
        .bindings
        .get(name)
        .ok_or_else(|| Error::MissingArgument(name.into()))?;
    if !is_segment(value) {
        return Err(invalid(
            name,
            format!("`{value}` is not a name segment (lowercase letters, digits and hyphens)"),
        ));
    }
    Ok(value.to_string())
}

fn key_binding(inv: &Invocation<'_>) -> Result<String> {
    let value = inv
        .bindings
        .get("p")
        .ok_or_else(|| Error::MissingArgument("p".into()))?;
    if !is_key(value) {
        return Err(invalid(
            "p",
            format!("`{value}` is not a party key (the bare-hex BLAKE3 digest of its principal)"),
        ));
    }
    Ok(value.to_string())
}

fn xsd(local: &str) -> String {
    format!("{XSD}{local}")
}

fn binding(name: &str, summary: &str) -> ArgSpec {
    ArgSpec::new(name)
        .summary(summary)
        .class(xsd("string"))
        .binding()
}

fn file(path: &str) -> Result<Iri> {
    Iri::parse(format!("urn:file:{path}"))
        .map_err(|e| Error::Endpoint(format!("<urn:file:{path}>: {e}")))
}

async fn read_atom(inv: &Invocation<'_>, path: &str) -> Result<Option<Representation>> {
    match inv.issue(Request::new(Verb::Source, file(path)?)).await {
        Ok(rep) => Ok(Some(rep)),
        Err(Error::NotFound(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

async fn read_text(inv: &Invocation<'_>, path: &str) -> Result<Option<String>> {
    match read_atom(inv, path).await? {
        None => Ok(None),
        Some(rep) => String::from_utf8(rep.bytes)
            .map(Some)
            .map_err(|_| Error::Endpoint(format!("<urn:file:{path}> is not UTF-8"))),
    }
}

fn caller<'a>(inv: &'a Invocation<'_>, what: &str) -> Result<&'a str> {
    inv.capability.principal().ok_or_else(|| {
        Error::Denied(Denial::from(format!(
            "{what} needs a caller who names a principal, and this capability names none"
        )))
    })
}

fn now(inv: &Invocation<'_>) -> Result<u64> {
    inv.now()
        .map(|t| t.as_millis())
        .ok_or_else(|| Error::Unsupported {
            verb: None,
            detail: "the host configured no clock, and a release's expiry is checked against one"
                .into(),
        })
}

/// The enrollment's learner, or `None` when the caller may not read the enrollment atom.
async fn learner_of(inv: &Invocation<'_>, n: &str) -> Result<Option<String>> {
    match read_text(inv, &enrollment_path(n)).await {
        Ok(Some(text)) => parse_learner(&text, n).map(Some),
        Ok(None) => Err(Error::NotFound(enrollment_name(n))),
        Err(Error::Denied(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// The release naming `principal` on enrollment `n`, as stored, or `None` when there is none
/// (or the caller may not read it).
async fn release_of(inv: &Invocation<'_>, n: &str, key: &str) -> Result<Option<Release>> {
    match read_text(inv, &release_path(n, key)).await {
        Ok(Some(text)) => parse_release(&text, n, key).map(Some),
        Ok(None) | Err(Error::Denied(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Who a view answers, decided on this call.
enum Admitted {
    Learner,
    /// A party, until the release's expiry.
    Party(u64),
}

async fn admit(inv: &Invocation<'_>, n: &str, scope: Scope) -> Result<Admitted> {
    let who = caller(inv, &format!("the {scope} of an enrollment"))?;
    if let Some(release) = release_of(inv, n, &party_key(who)).await? {
        if !release.scopes.contains(&scope) {
            return Err(Error::Denied(Denial::from(format!(
                "the release to {who} on enrollment {n} shares {}, not {scope}",
                release.scope_list()
            ))));
        }
        if now(inv)? >= release.expires {
            return Err(Error::Denied(Denial::from(format!(
                "the release to {who} on enrollment {n} expired at {}",
                date_time(release.expires)
            ))));
        }
        return Ok(Admitted::Party(release.expires));
    }
    if learner_of(inv, n).await?.as_deref() == Some(who) {
        return Ok(Admitted::Learner);
    }
    Err(Error::Denied(Denial::from(format!(
        "{who} holds no release of the {scope} of enrollment {n}; only its learner, or a party \
         the learner released it to, may read it"
    ))))
}

// --- the doors ----------------------------------------------------------------------------

fn release_door() -> AsyncFnEndpoint {
    let bindings = || {
        vec![
            binding("n", "the enrollment's id"),
            binding(
                "p",
                "the party's key: the bare-hex BLAKE3 digest of its principal",
            ),
        ]
    };
    let mut d = Description::new("learn-release")
        .title("A release: part of a learner's record, shared with one party")
        .summary(
            "Source: the release as Turtle (its party, scopes and expiry), for the learner or \
             the party it names; NotFound when there is none. Sink: the enrollment's LEARNER \
             issues it, naming the party (`party`, whose digest is `{p}`), the scopes \
             (`scope`, comma-separated: credential, completion, progress, submission) and the \
             expiry (`expires`, a UTC date-time); rewriting it may only narrow it (Conflict \
             otherwise: delete it and issue a new one). Delete: the learner revokes it, and \
             the next read under it is Denied. Anyone but the learner is Denied a Sink or \
             Delete.",
        )
        .verb(Verb::Source)
        .verb(Verb::Meta)
        .requires(CAP_RELEASE_READ)
        .fails(FailureClass::Permanent)
        .fails(FailureClass::Unknown)
        .output(TURTLE);
    for b in bindings() {
        d = d.input(b);
    }
    let mut sink = ActionSpec::new(Verb::Sink)
        .summary("the learner issues or narrows the release")
        .requires(CAP_RELEASE_WRITE)
        .input(ArgSpec::new("party").summary("the party's principal IRI").class(xsd("anyURI")))
        .input(
            ArgSpec::new("scope")
                .summary("what it shares: credential, completion, progress or submission, comma-separated")
                .class(xsd("string")),
        )
        .input(
            ArgSpec::new("expires")
                .summary("when it stops sharing: a UTC date-time, YYYY-MM-DDThh:mm:ssZ, in the future")
                .class(xsd("dateTime")),
        )
        .output(TURTLE)
        .fails(FailureClass::Permanent)
        .fails(FailureClass::Unknown);
    let mut delete = ActionSpec::new(Verb::Delete)
        .summary("the learner revokes the release")
        .requires(CAP_RELEASE_WRITE)
        .output(TEXT)
        .fails(FailureClass::Permanent)
        .fails(FailureClass::Unknown);
    for b in bindings() {
        sink = sink.input(b.clone());
        delete = delete.input(b);
    }
    d = d.action(sink).action(delete);
    AsyncFnEndpoint::new("learn-release", |inv| {
        Box::pin(async move {
            let n = segment(inv, "n")?;
            let key = key_binding(inv)?;
            let name = release_name(&n, &key);
            match inv.request.verb {
                Verb::Sink => issue(inv, &n, &key).await,
                Verb::Delete => {
                    let who = caller(inv, "revoking a release")?;
                    require_learner(inv, &n, who, "revoke").await?;
                    inv.issue(Request::new(Verb::Delete, file(&release_path(&n, &key))?))
                        .await
                        .map_err(|e| match e {
                            Error::NotFound(_) => Error::NotFound(name.clone()),
                            e => e,
                        })?;
                    Ok(text(format!("revoked {name}\n")))
                }
                _ => {
                    let who = caller(inv, "reading a release")?;
                    let release = release_of(inv, &n, &key).await?;
                    match release {
                        // Cacheable: the answer hangs from the release atom's thread, which
                        // its narrowing or revocation cuts.
                        Some(r) if r.party == who => Ok(turtle(r.turtle()).cacheable()),
                        Some(r) => {
                            require_learner(inv, &n, who, "read").await?;
                            Ok(turtle(r.turtle()).cacheable())
                        }
                        None => {
                            require_learner(inv, &n, who, "read").await?;
                            Err(Error::NotFound(name))
                        }
                    }
                }
            }
        })
    })
    .with_description(d)
}

async fn require_learner(inv: &Invocation<'_>, n: &str, who: &str, what: &str) -> Result<()> {
    if learner_of(inv, n).await?.as_deref() == Some(who) {
        return Ok(());
    }
    Err(Error::Denied(Denial::from(format!(
        "only the learner of enrollment {n} may {what} its releases, and {who} is not that learner"
    ))))
}

async fn issue(inv: &Invocation<'_>, n: &str, key: &str) -> Result<Representation> {
    let who = caller(inv, "issuing a release")?.to_string();
    require_learner(inv, n, &who, "issue").await?;
    let party = inv.inline_str("party")?.trim().to_string();
    Iri::parse(party.as_str())
        .map_err(|e| invalid("party", format!("`{party}` is not an IRI: {e}")))?;
    if party_key(&party) != key {
        return Err(invalid(
            "party",
            format!(
                "the key of {party} is {}, not the {key} this release names",
                party_key(&party)
            ),
        ));
    }
    if party == who {
        return Err(invalid(
            "party",
            "a learner already reads their own record: a release names another party",
        ));
    }
    let scopes = inv
        .inline_str("scope")?
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| Scope::parse(s).map_err(|e| invalid("scope", e)))
        .collect::<Result<BTreeSet<Scope>>>()?;
    if scopes.is_empty() {
        return Err(invalid("scope", "a release shares at least one scope"));
    }
    let expires_text = inv.inline_str("expires")?.trim();
    let expires = millis_of(expires_text).ok_or_else(|| {
        invalid(
            "expires",
            format!("`{expires_text}` is not a UTC date-time such as 2027-06-30T00:00:00Z"),
        )
    })?;
    if expires <= now(inv)? {
        return Err(invalid(
            "expires",
            format!("{expires_text} has already passed"),
        ));
    }
    let release = Release {
        enrollment: n.into(),
        party,
        scopes,
        expires,
    };
    if let Some(existing) = release_of(inv, n, key).await? {
        if !existing.covers(&release) {
            return Err(Error::Conflict(format!(
                "a release only narrows: the one to {} shares {} until {}; delete it and issue a \
                 new one to widen it",
                existing.party,
                existing.scope_list(),
                date_time(existing.expires)
            )));
        }
    }
    let body = release.turtle();
    inv.issue(
        Request::new(Verb::Sink, file(&release_path(n, key))?)
            .with_arg("content", ArgRef::Inline(body.clone().into_bytes())),
    )
    .await?;
    Ok(turtle(body))
}

fn view_door(scope: Scope) -> AsyncFnEndpoint {
    let id = format!("learn-{scope}");
    let mut inputs = vec![binding("n", "the enrollment's id")];
    let what = match scope {
        Scope::Credential => "the credential issued for the enrollment (NotFound until issued)",
        Scope::Completion => {
            "complete or not, and when, as Turtle (`complete false` until the enrollment completes)"
        }
        Scope::Progress => {
            "mastery per unit and the verdict per exercise, as Turtle (NotFound until there is any)"
        }
        Scope::Submission => {
            inputs.push(binding("e", "the exercise's name"));
            "the work submitted for one exercise (NotFound until submitted)"
        }
    };
    let mut d = Description::new(id.clone())
        .title(format!(
            "The {scope} of an enrollment, for its learner or a released party"
        ))
        .summary(format!(
            "{what}. Answered to the enrollment's learner, and to a party only while a release \
             naming it shares `{scope}` and has not expired: checked on every call, and a \
             cached answer hangs from the release, so revoking it denies the next read."
        ))
        .verb(Verb::Source)
        .verb(Verb::Meta)
        .requires(scope.read_cap())
        .fails(FailureClass::Permanent)
        .fails(FailureClass::Unknown);
    for i in inputs {
        d = d.input(i);
    }
    d = match scope {
        Scope::Credential => d.output("application/json"),
        _ => d.output(TURTLE),
    };
    AsyncFnEndpoint::new(id, move |inv| {
        Box::pin(async move {
            let n = segment(inv, "n")?;
            let exercise = match scope {
                Scope::Submission => Some(segment(inv, "e")?),
                _ => None,
            };
            let admitted = admit(inv, &n, scope).await?;
            let path = record_path(scope, &n, exercise.as_deref());
            let rep = match read_atom(inv, &path).await? {
                Some(rep) => rep,
                None if scope == Scope::Completion => turtle(format!(
                    "<urn:learn:completion:enrollment:{n}> <{TERM}complete> false .\n"
                )),
                None => {
                    let mut name = format!("urn:learn:{scope}:enrollment:{n}");
                    if let Some(e) = &exercise {
                        name.push_str(&format!(":exercise:{e}"));
                    }
                    return Err(Error::NotFound(name));
                }
            };
            let rep = Representation::new(rep.repr_type, rep.bytes);
            Ok(match admitted {
                Admitted::Learner => rep.cacheable(),
                Admitted::Party(expires) => {
                    rep.cacheable_until(ikigai_core::Time::from_millis(expires))
                }
            })
        })
    })
    .with_description(d)
}

fn turtle(body: String) -> Representation {
    Representation::new(
        ReprType::new(TURTLE).with_param("charset", "utf-8"),
        body.into_bytes(),
    )
}

fn text(body: String) -> Representation {
    Representation::new(
        ReprType::new(TEXT).with_param("charset", "utf-8"),
        body.into_bytes(),
    )
}

/// The release doors and the four record views they gate. Configuration-free, so it carries
/// its crate's space name.
pub fn space() -> EndpointSpace {
    let t = |s: &str| UriTemplate::parse(s).expect("a release template is valid");
    EndpointSpace::new()
        .bind(
            t("urn:learn:release:enrollment:{n}:party:{p}"),
            release_door(),
        )
        .bind(
            t("urn:learn:credential:enrollment:{n}"),
            view_door(Scope::Credential),
        )
        .bind(
            t("urn:learn:completion:enrollment:{n}"),
            view_door(Scope::Completion),
        )
        .bind(
            t("urn:learn:progress:enrollment:{n}"),
            view_door(Scope::Progress),
        )
        .bind(
            t("urn:learn:submission:enrollment:{n}:exercise:{e}"),
            view_door(Scope::Submission),
        )
        .named(ikigai_core::space_iri("learn"))
}

/// The published grant of an enrollment's learner: their releases, all four views, and on the
/// state root (relative, so the host's mount spelling does not matter) read on the whole
/// enrollment and write and delete on its releases only.
pub fn learner_grant(enrollment: &str, learner: &str) -> Vec<String> {
    let mut grant = vec![CAP_RELEASE_READ.to_string(), CAP_RELEASE_WRITE.to_string()];
    grant.extend(Scope::ALL.iter().map(|s| s.read_cap()));
    grant.push(format!("urn:cap:fs:read:enrollment/{enrollment}"));
    for action in ["write", "delete"] {
        grant.push(format!(
            "urn:cap:fs:{action}:enrollment/{enrollment}/release"
        ));
    }
    grant.extend(principal_scope(learner).ok());
    grant
}

/// The published grant of the party a release names, derived from the release alone: reading
/// the release itself, and for each scope it shares that view and the one record file (or, for
/// a submission, directory) it reads. Nothing else on the state root. A host mints it from the
/// live release on every call, so once the release is deleted there is nothing to mint; the
/// view refuses a grant that outlived its release all the same.
pub fn party_grant(release: &Release) -> Vec<String> {
    let n = &release.enrollment;
    let key = party_key(&release.party);
    let mut grant = vec![
        CAP_RELEASE_READ.to_string(),
        format!("urn:cap:fs:read:{}", release_path(n, &key)),
    ];
    for scope in &release.scopes {
        grant.push(scope.read_cap());
        grant.push(match scope {
            Scope::Submission => format!("urn:cap:fs:read:enrollment/{n}/submission"),
            s => format!("urn:cap:fs:read:{}", record_path(*s, n, None)),
        });
    }
    grant.extend(principal_scope(&release.party).ok());
    grant
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_times_round_trip_and_refuse_what_does_not_exist() {
        let t = "2027-06-30T12:34:56Z";
        assert_eq!(date_time(millis_of(t).unwrap()), t);
        assert_eq!(millis_of("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(millis_of("2027-02-30T00:00:00Z"), None);
        assert_eq!(millis_of("2027-06-30T12:34:56+01:00"), None);
        assert_eq!(millis_of("2027-06-30"), None);
    }

    #[test]
    fn a_release_reads_back_as_written() {
        let r = Release {
            enrollment: "e1".into(),
            party: "urn:example:employer".into(),
            scopes: [Scope::Completion, Scope::Credential].into(),
            expires: millis_of("2027-06-30T00:00:00Z").unwrap(),
        };
        let back = parse_release(&r.turtle(), "e1", &party_key(&r.party)).unwrap();
        assert_eq!(back, r);
        assert_eq!(back.scope_list(), "credential,completion");
    }

    #[test]
    fn a_release_for_another_party_or_enrollment_fails_closed() {
        let r = Release {
            enrollment: "e1".into(),
            party: "urn:example:employer".into(),
            scopes: [Scope::Completion].into(),
            expires: 1_000,
        };
        assert!(parse_release(&r.turtle(), "e1", &party_key("urn:example:other")).is_err());
        assert!(parse_release(&r.turtle(), "e2", &party_key(&r.party)).is_err());
    }

    #[test]
    fn what_is_never_released_is_refused_by_name() {
        for never in NEVER_RELEASED {
            let e = Scope::parse(never).unwrap_err();
            assert!(e.contains("is never released"), "{e}");
        }
        assert_eq!(
            Scope::parse("grades").unwrap_err(),
            "`grades` is not a scope: use credential, completion, progress or submission"
        );
    }

    #[test]
    fn narrowing_is_covered_and_widening_is_not() {
        let wide = Release {
            enrollment: "e1".into(),
            party: "urn:example:p".into(),
            scopes: [Scope::Completion, Scope::Progress].into(),
            expires: 2_000,
        };
        let mut narrow = wide.clone();
        narrow.scopes = [Scope::Completion].into();
        assert!(wide.covers(&narrow));
        assert!(!narrow.covers(&wide));
        let mut later = narrow.clone();
        later.expires = 3_000;
        assert!(!wide.covers(&later));
    }
}
