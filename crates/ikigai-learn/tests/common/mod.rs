//! A host for the tests: the learn space, ikigai-fs `cacheable_space` over a scratch state
//! root, and a clock the test moves. Every call but the seeding runs under a published grant
//! ([`ikigai_learn::release::learner_grant`], [`ikigai_learn::release::party_grant`]).
#![allow(dead_code)] // each test binary uses a different subset

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use futures::executor::block_on;
use ikigai_core::{
    ArgRef, Capability, Clock, EndpointSpace, Fallback, Iri, Kernel, Representation, Request,
    Result, Space, Time, Verb,
};
use ikigai_learn::release;

pub const N: &str = "e1";
pub const LEARNER: &str = "urn:example:learner";
pub const EMPLOYER: &str = "urn:example:employer";
pub const STRANGER: &str = "urn:example:stranger";
/// 2026-11-26T12:00:00Z
pub const NOW: u64 = 1_795_694_400_000;
pub const EXPIRES: &str = "2027-06-30T00:00:00Z";

pub struct TestClock(AtomicU64);

impl TestClock {
    pub fn set(&self, millis: u64) {
        self.0.store(millis, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> Time {
        Time::from_millis(self.0.load(Ordering::SeqCst))
    }
}

pub struct Host {
    pub kernel: Kernel,
    pub clock: Arc<TestClock>,
    pub root: Scratch,
    pub learn: Arc<EndpointSpace>,
    pub root_space: Arc<Fallback>,
}

impl Host {
    /// An enrollment `e1` of [`LEARNER`] with a record in every scope, and the responses,
    /// attempts and drafts no release reaches.
    pub fn new() -> Host {
        let root = Scratch::new();
        let learn = Arc::new(release::space());
        let spaces: Vec<Arc<dyn Space>> = vec![
            learn.clone(),
            Arc::new(ikigai_fs::cacheable_space(root.path())),
        ];
        let root_space = Arc::new(Fallback::new(spaces));
        let clock = Arc::new(TestClock(AtomicU64::new(NOW)));
        let kernel =
            Kernel::with_meta_renderer(root_space.clone(), Arc::new(ikigai_vocab::TurtleRenderer))
                .with_clock(clock.clone());
        let h = Host {
            kernel,
            clock,
            root,
            learn,
            root_space,
        };
        h.seed(
            &release::enrollment_path(N),
            &release::enrollment_turtle(N, LEARNER),
        );
        h.seed(
            &release::record_path(release::Scope::Completion, N, None),
            "<urn:learn:completion:enrollment:e1> <urn:learn:term:complete> true .\n",
        );
        h.seed(
            &release::record_path(release::Scope::Credential, N, None),
            "{\"type\": [\"VerifiableCredential\"]}\n",
        );
        h.seed(
            &release::record_path(release::Scope::Progress, N, None),
            "<urn:learn:progress:enrollment:e1> <urn:learn:term:mastered> <urn:learn:unit:name:intro> .\n",
        );
        h.seed(
            &release::record_path(release::Scope::Submission, N, Some("first")),
            "<urn:learn:submission:enrollment:e1:exercise:first> <urn:learn:term:text> \"my work\" .\n",
        );
        h.seed(
            "enrollment/e1/response/q1.ttl",
            "<urn:x> <urn:y> \"a response\" .\n",
        );
        h.seed(
            "enrollment/e1/draft/first.ttl",
            "<urn:x> <urn:y> \"a draft\" .\n",
        );
        h
    }

    /// Write a file under the state root as the host (root authority), not through a door.
    pub fn seed(&self, path: &str, body: &str) {
        self.issue_as(
            &Capability::root(),
            Verb::Sink,
            &format!("urn:file:{path}"),
            &[("content", body)],
        )
        .unwrap_or_else(|e| panic!("seed {path}: {e}"));
    }

    pub fn learner(&self) -> Capability {
        Capability::scoped(release::learner_grant(N, LEARNER))
    }

    /// The published grant of the party a release names.
    pub fn party(&self, r: &release::Release) -> Capability {
        Capability::scoped(release::party_grant(r))
    }

    pub fn issue_as(
        &self,
        cap: &Capability,
        verb: Verb,
        iri: &str,
        args: &[(&str, &str)],
    ) -> Result<Representation> {
        block_on(self.kernel.issue(request(verb, iri, args), cap))
    }

    pub fn text_as(
        &self,
        cap: &Capability,
        verb: Verb,
        iri: &str,
        args: &[(&str, &str)],
    ) -> String {
        let rep = self
            .issue_as(cap, verb, iri, args)
            .unwrap_or_else(|e| panic!("{verb:?} {iri}: {e}"));
        String::from_utf8(rep.bytes).unwrap()
    }

    /// The learner releases `scope` to `party` until [`EXPIRES`].
    pub fn release(&self, party: &str, scope: &str) -> Result<Representation> {
        self.release_until(party, scope, EXPIRES)
    }

    pub fn release_until(&self, party: &str, scope: &str, expires: &str) -> Result<Representation> {
        self.issue_as(
            &self.learner(),
            Verb::Sink,
            &release_iri(party),
            &[("party", party), ("scope", scope), ("expires", expires)],
        )
    }
}

pub fn release_iri(party: &str) -> String {
    release::release_name(N, &release::party_key(party))
}

pub fn view(scope: &str) -> String {
    match scope {
        "submission" => format!("urn:learn:submission:enrollment:{N}:exercise:first"),
        s => format!("urn:learn:{s}:enrollment:{N}"),
    }
}

pub fn request(verb: Verb, iri: &str, args: &[(&str, &str)]) -> Request {
    let mut request = Request::new(verb, Iri::parse(iri).unwrap());
    for (name, value) in args {
        request = request.with_arg(*name, ArgRef::Inline(value.as_bytes().to_vec()));
    }
    request
}

/// A fresh scratch directory under the system temp dir, removed on drop.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new() -> Scratch {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "ikigai-learn-{}-{}-{nanos}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(std::fs::canonicalize(dir).unwrap())
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
