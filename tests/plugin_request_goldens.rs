//! Golden request bytes for every request shape that crosses the process-plugin
//! boundary.
//!
//! A `runtime = "process"` plugin reads its request off stdin and parses it with
//! whatever JSON library its language ships. That makes the *bytes* the contract:
//! an appended newline that a lenient parser tolerates in one language, a length
//! prefix, or a field that stops being omitted because someone dropped a
//! `skip_serializing_if` are all invisible until a plugin written in Python
//! breaks (#369 S5). Nothing in this suite asserted the bytes before this file:
//! every fixture that reads the request uses `cat` or `read -r line`, and both
//! are framing-agnostic.
//!
//! ## What this pins, and what it does not
//!
//! - **Pinned here, byte for byte**: the envelope's field order, the capability
//!   and operation wire strings, the omission of absent `Option` fields against
//!   their presence as `false`/`[]`, the sorted key order inside `payload`, and
//!   the framing (no line ending, no length prefix, nothing before `{`). The bytes
//!   are produced by `encode_invocation`, the same function the driver calls, so
//!   the framing cannot be added at the driver without failing here.
//! - **Pinned here, by digest**: the sha256 of each request, so the `cutver/plugins`
//!   repository can reconcile its decoder with one `sha256sum` instead of a byte
//!   diff. It runs only under `--features plugins`, because `sha2` enters the
//!   build with that feature; the byte assertions above run everywhere.
//! - **Not pinned here**: the byte count a child actually receives — the fixture
//!   suite that spawns a plugin and parses a real request covers that half — and
//!   the *values* production computes (a real commit list, a real `CHANGELOG.md`),
//!   which belong to the tests of each capability. What this file owns is the shape
//!   and the framing of one request per shape.
//!
//! ## Why `changelog.v1` has two files
//!
//! `tests/fixtures/plugin_wire_contract/changelog_request.json` is committed
//! byte-identical to `plugins/github-releases/tests/fixtures/changelog_request.json`,
//! and that twin pins one *full* instance (every optional present). The golden
//! here is the same shape with every optional absent, which is the half of the
//! contract the twin cannot show. Neither is redundant and neither moves.
//!
//! ## Reconciling a failure
//!
//! A failing test writes the bytes it produced to the OS temp directory and names
//! that path in the panic, together with the digest. If the change is intended,
//! the fix is to copy that file over the fixture *and* update the digest, in both
//! repositories when the shape is shared.

use std::path::{Path, PathBuf};

use cutver::plugin::driver::process::encode_invocation;
use cutver::plugin::dto::{
    ChangelogRenderRequest, ManifestReadRequest, ManifestWriteRequest, PluginCommitEntry, PluginInvocation,
    PostBumpPayload, PostReleasePayload, PreBumpPayload, VersioningComputeRequest,
};
use cutver::plugin::types::{PluginCall, PluginName};

/// One committed request, and the call that produced it.
struct Golden {
    /// Repository-relative path of the committed bytes.
    fixture: &'static str,
    /// The dispatch call, which fixes the capability and the operation on the wire.
    call: PluginCall,
    /// Builds the typed request exactly as production does, then wraps it.
    request: fn(PluginCall) -> PluginInvocation,
    /// `sha256` of the bytes on the wire, lowercase hex.
    sha256: &'static str,
}

/// Every request shape the host can put on a plugin's stdin.
///
/// `manifest.v1` and `lifecycle.v1` each carry more than one request, so this is
/// one entry per *shape*, not one per capability name: a shape nobody pins is a
/// shape a non-Rust author discovers by hand.
fn goldens() -> Vec<Golden> {
    vec![
        Golden {
            fixture: "tests/fixtures/plugin_requests/changelog-render.json",
            call: PluginCall::ChangelogRender,
            request: changelog_render,
            sha256: "c0ceb2bd361ff8ccfa8b1119e9ff1483f3fb9d1cf8b164b9d3dc8c280c955916",
        },
        Golden {
            fixture: "tests/fixtures/plugin_requests/manifest-read.json",
            call: PluginCall::ManifestRead,
            request: manifest_read,
            sha256: "3e20e0889ea8ac8dfbf5f22dfdf1e7cff47908a305e2af979402c10f17638077",
        },
        Golden {
            fixture: "tests/fixtures/plugin_requests/manifest-write.json",
            call: PluginCall::ManifestWrite,
            request: manifest_write,
            sha256: "62481dd6e86b7e1348cfce7c201a8efc30e9cbcc419ac3c39f8e8f623a5c1cff",
        },
        Golden {
            fixture: "tests/fixtures/plugin_requests/versioning-compute.json",
            call: PluginCall::VersioningCompute,
            request: versioning_compute,
            sha256: "10d7425cfc5ff6fb5196765268f13b3cfa3f964d0ca61910e4547509c34458d2",
        },
        Golden {
            fixture: "tests/fixtures/plugin_requests/lifecycle-pre-bump.json",
            call: PluginCall::PreBump,
            request: pre_bump,
            sha256: "a5b3750f1d4ee98b15c49d6408893d10a7b8558c65c94f299ac145ebf4fa657a",
        },
        Golden {
            fixture: "tests/fixtures/plugin_requests/lifecycle-post-bump.json",
            call: PluginCall::PostBump,
            request: post_bump,
            sha256: "edfcc151530a14f6b2e8092bdebab0124cf73763a86abbc82274211c86134713",
        },
        Golden {
            fixture: "tests/fixtures/plugin_requests/lifecycle-post-release.json",
            call: PluginCall::PostRelease,
            request: post_release,
            sha256: "13ce98a1d035bf33799ab0462fd7df8cbb687a56a39bd816a1fda66014f83b2f",
        },
    ]
}

/// `changelog.v1` with **every optional absent**: `previous_tag`, `repository`
/// and `compare_url` are omitted rather than sent as `null`, and the two
/// `#[serde(default)]` collections are present as `[]` because a plugin may read
/// them unconditionally (`docs/guides/plugin-overview.md`).
fn changelog_render(call: PluginCall) -> PluginInvocation {
    let request = ChangelogRenderRequest {
        root_dir: "/workspace".to_string(),
        version: "1.2.0".to_string(),
        tag_name: "v1.2.0".to_string(),
        previous_tag: None,
        release_date: "2026-03-14".to_string(),
        commits: vec![PluginCommitEntry {
            sha: "a1b2c3d4e5f6a7b8c9d0".to_string(),
            message: "feat: add tree view".to_string(),
            r#type: Some("feat".to_string()),
            scope: None,
            author_name: None,
            pr_number: None,
            is_breaking: false,
        }],
        repository: None,
        compare_url: None,
        is_prerelease: true,
        contributors: vec![],
    };
    PluginInvocation::new(call, &request).expect("the changelog request serializes")
}

fn manifest_read(call: PluginCall) -> PluginInvocation {
    let request = ManifestReadRequest {
        path: "Chart.yaml".to_string(),
        content: "apiVersion: v2\nname: chart\nversion: 1.2.3\n".to_string(),
    };
    PluginInvocation::new(call, &request).expect("the manifest read request serializes")
}

fn manifest_write(call: PluginCall) -> PluginInvocation {
    let request = ManifestWriteRequest {
        path: "Chart.yaml".to_string(),
        content: "apiVersion: v2\nname: chart\nversion: 1.2.4\n".to_string(),
        current_version: "1.2.3".to_string(),
        next_version: "1.2.4".to_string(),
    };
    PluginInvocation::new(call, &request).expect("the manifest write request serializes")
}

/// `versioning.v1` on a first release, so `previous_tag` is absent.
fn versioning_compute(call: PluginCall) -> PluginInvocation {
    let request = VersioningComputeRequest {
        root_dir: "/workspace".to_string(),
        current_version: "0.0.0".to_string(),
        bump_level: "minor".to_string(),
        tag_prefix: "v".to_string(),
        previous_tag: None,
        commits: vec![
            "feat: add tree view".to_string(),
            "fix!: correct tag prefix".to_string(),
        ],
    };
    PluginInvocation::new(call, &request).expect("the versioning request serializes")
}

fn pre_bump(call: PluginCall) -> PluginInvocation {
    let request = PreBumpPayload {
        root_dir: "/workspace".to_string(),
        current_version: "1.2.3".to_string(),
        next_version: "1.3.0".to_string(),
        bump_level: "minor".to_string(),
        tag_name: "v1.3.0".to_string(),
        dry_run: false,
    };
    PluginInvocation::new(call, &request).expect("the pre-bump payload serializes")
}

fn post_bump(call: PluginCall) -> PluginInvocation {
    let request = PostBumpPayload {
        root_dir: "/workspace".to_string(),
        current_version: "1.2.3".to_string(),
        next_version: "1.3.0".to_string(),
        tag_name: "v1.3.0".to_string(),
        modified_files: vec!["Cargo.toml".to_string(), "CHANGELOG.md".to_string()],
        dry_run: false,
    };
    PluginInvocation::new(call, &request).expect("the post-bump payload serializes")
}

/// `post_release` with no notes, so `notes` is absent rather than `null`.
fn post_release(call: PluginCall) -> PluginInvocation {
    let request = PostReleasePayload {
        root_dir: "/workspace".to_string(),
        version: "1.3.0".to_string(),
        tag_name: "v1.3.0".to_string(),
        commit_sha: "4f2c1ab".to_string(),
        dry_run: false,
        notes: None,
    };
    PluginInvocation::new(call, &request).expect("the post-release payload serializes")
}

/// The bytes the host hands to the child, produced through the driver's own encoder.
///
/// Going through [`encode_invocation`] rather than calling `serde_json` here is the point: the
/// framing has one author, so appending a newline in the driver fails these tests. What this still
/// does not cover is the call site that passes the result to the supervisor, and the byte count a
/// child actually receives; the fixture suite that parses real requests covers that half.
fn wire_bytes(golden: &Golden) -> Vec<u8> {
    let name = PluginName::new("golden-probe").expect("'golden-probe' is a valid plugin name");
    encode_invocation(&name, &(golden.request)(golden.call)).expect("a golden request serializes to bytes")
}

fn fixture_path(golden: &Golden) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(golden.fixture)
}

/// Describes one mismatch, preserving the produced bytes somewhere the author can copy from.
///
/// Every mismatch of a run is collected and reported together: fixing one fixture at a time and
/// re-running to discover the next is the kind of friction that teaches an author to stop running
/// the test.
fn describe_mismatch(golden: &Golden, produced: &[u8], reason: &str) -> String {
    let name = golden.fixture.rsplit('/').next().unwrap_or(golden.fixture);
    let saved = std::env::temp_dir().join(format!("cutver-golden-{name}"));
    let saved_note = match std::fs::write(&saved, produced) {
        Ok(()) => format!("the bytes produced now were written to {}", saved.display()),
        Err(error) => format!("the bytes could not be written to {}: {error}", saved.display()),
    };

    #[cfg(feature = "plugins")]
    let digest = format!("sha256 {}", sha256_hex(produced));
    #[cfg(not(feature = "plugins"))]
    let digest = "sha256 unavailable (run with --features plugins)".to_string();

    format!(
        "{reason}\n  fixture: {}\n  wire:    {}\n  {digest}\n  {saved_note}\n  \
         If the change is intended, copy that file over the fixture and update the digest — in both \
         repositories when the shape is shared.",
        golden.fixture,
        String::from_utf8_lossy(produced),
    )
}

#[test]
fn every_request_shape_the_host_can_write_has_a_golden() {
    // A capability added without a golden is a shape a non-Rust author discovers by hand, which is
    // the failure this file exists to prevent (#369 S5). The list is written out rather than derived
    // so that adding a `PluginCall` variant fails here until its bytes are pinned.
    let every_call = [
        PluginCall::ManifestRead,
        PluginCall::ManifestWrite,
        PluginCall::PreBump,
        PluginCall::PostBump,
        PluginCall::PostRelease,
        PluginCall::ChangelogRender,
        PluginCall::VersioningCompute,
    ];

    let pinned: Vec<PluginCall> = goldens().iter().map(|golden| golden.call).collect();
    for call in every_call {
        assert!(
            pinned.contains(&call),
            "{call:?} puts a request on a plugin's stdin and no golden pins its bytes"
        );
    }

    // The digest is read on every feature set, not only where `sha2` is in the build: a truncated or
    // mistyped digest is a broken reconciliation for the plugins repository, not a detail of this run.
    for golden in &goldens() {
        assert_eq!(
            golden.sha256.len(),
            64,
            "{} must record a full sha256, not a placeholder",
            golden.fixture
        );
        assert!(
            golden
                .sha256
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "{} records a digest that is not lowercase hex: {}",
            golden.fixture,
            golden.sha256
        );
    }
}

#[test]
fn every_golden_matches_the_bytes_the_host_writes() {
    let mut failures = Vec::new();
    for golden in goldens() {
        let produced = wire_bytes(&golden);
        let path = fixture_path(&golden);
        match std::fs::read(&path) {
            Ok(committed) if committed == produced => {}
            Ok(_) => failures.push(describe_mismatch(
                &golden,
                &produced,
                "the host writes bytes that differ from the committed golden",
            )),
            Err(error) => failures.push(describe_mismatch(
                &golden,
                &produced,
                &format!("the golden {} cannot be read: {error}", path.display()),
            )),
        }
    }

    assert!(failures.is_empty(), "\n\n{}", failures.join("\n\n"));
}

#[test]
fn the_wire_carries_no_framing_of_its_own() {
    // A plugin reads to the end of stdin and parses what it got. Anything the host adds around the
    // document — a trailing newline, a byte count in front of it — is a rule the author has to learn
    // the hard way, so it is pinned here instead.
    for golden in goldens() {
        let produced = wire_bytes(&golden);

        assert_eq!(
            produced.first(),
            Some(&b'{'),
            "{} must start with the document itself, never a length prefix or whitespace",
            golden.fixture
        );
        assert_eq!(
            produced.last(),
            Some(&b'}'),
            "{} must end with the document itself, never a trailing newline",
            golden.fixture
        );
        assert!(
            !produced.contains(&b'\n') && !produced.contains(&b'\r'),
            "{} contains a line ending; the request is one document with no line endings at all",
            golden.fixture
        );

        let decoded: PluginInvocation = serde_json::from_slice(&produced)
            .unwrap_or_else(|error| panic!("{} does not decode back: {error}", golden.fixture));
        assert_eq!(
            decoded,
            (golden.request)(golden.call),
            "{} decodes into a different envelope than the one that was sent",
            golden.fixture
        );
    }
}

#[cfg(feature = "plugins")]
#[test]
fn every_golden_carries_the_digest_both_repositories_reconcile_by() {
    let mut failures = Vec::new();
    for golden in goldens() {
        let produced = wire_bytes(&golden);
        if sha256_hex(&produced) != golden.sha256 {
            failures.push(describe_mismatch(
                &golden,
                &produced,
                "the host writes bytes whose sha256 is not the one this file records",
            ));
        }
    }

    assert!(failures.is_empty(), "\n\n{}", failures.join("\n\n"));
}

/// Lowercase hex `sha256`, without pulling a hex crate for one call site.
#[cfg(feature = "plugins")]
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}
