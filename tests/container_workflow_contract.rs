//! Contract tests binding `.github/workflows/container.yml` to the release it publishes.
//!
//! The invariant: a release tag must never exist before that image's smoke has passed. The build
//! therefore pushes only a rehearsal ref, and promotion retags the digest the smoke pulled. #259
//! found the ordering inverted — the build pushed the semver tags and `latest` in the same step
//! that produced the digest, so the smoke ran against an image users could already pull under its
//! release tag and a red smoke retracted nothing. The workflow cannot be run from a test, but every
//! coupling that would silently publish unverified bytes is text-visible: exact lines, per-job
//! slices, and one named regression per assertion.

use std::path::PathBuf;

fn repo_text(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    // Git checks these files out with CRLF on Windows. Line endings are a checkout setting, not
    // content, so every assertion in this file compares normalised text.
    raw.replace("\r\n", "\n")
}

const CONTAINER: &str = ".github/workflows/container.yml";

/// One image's leg through the container workflow. Both images share one repository and are told
/// apart by tag, so each has its own build job, smoke job and rehearsal ref.
struct Image {
    label: &'static str,
    build: &'static str,
    smoke: &'static str,
    /// The only `tags:` rule the build job may declare.
    rehearsal: &'static str,
    /// A release tag rule the promotion for this image declares and no build job may.
    release_rule: &'static str,
}

const IMAGES: [Image; 2] = [
    Image {
        label: "CLI image",
        build: "build-and-push",
        smoke: "smoke",
        rehearsal: "type=raw,value=rehearsal-${{ github.run_id }}-${{ github.run_attempt }}",
        release_rule: "type=semver,pattern={{version}}",
    },
    Image {
        label: "plugins image",
        build: "build-and-push-plugins",
        smoke: "smoke-plugins",
        rehearsal: "type=raw,value=rehearsal-plugins-${{ github.run_id }}-${{ github.run_attempt }}",
        release_rule: "type=semver,pattern={{version}}-plugins",
    },
];

/// The offset just past `jobs:` in the container workflow.
fn container_jobs_at(workflow: &str) -> usize {
    const MARKER: &str = "\njobs:\n";
    workflow
        .find(MARKER)
        .unwrap_or_else(|| panic!("{CONTAINER} must declare a `jobs:` section"))
        + MARKER.len()
}

/// A two-space `key:` line that opens a job, as opposed to the four-space keys inside one.
fn is_container_job_key(line: &str) -> bool {
    line.strip_prefix("  ")
        .is_some_and(|rest| !rest.starts_with(' ') && !rest.trim().is_empty() && !rest.starts_with('#'))
}

/// Every job id declared under `jobs:`, in order.
fn container_job_ids(workflow: &str) -> Vec<String> {
    workflow[container_jobs_at(workflow)..]
        .lines()
        .filter(|line| is_container_job_key(line))
        .filter_map(|line| line.strip_suffix(':'))
        .map(|id| id.trim().to_string())
        .collect()
}

/// One job body as lines: from its two-space key to the next job key. Anchored on the exact key, so
/// a `needs:` entry or a step name mentioning the job cannot stand in for the job itself.
fn container_job<'a>(workflow: &'a str, id: &str) -> Vec<&'a str> {
    let body: Vec<&str> = workflow[container_jobs_at(workflow)..].lines().collect();
    let key = format!("  {id}:");
    let start = body
        .iter()
        .position(|line| *line == key)
        .unwrap_or_else(|| panic!("{CONTAINER} must declare the `{id}` job"));
    body[start + 1..]
        .iter()
        .copied()
        .take_while(|line| !is_container_job_key(line))
        .collect()
}

/// The rules a `docker/metadata-action` invocation in a job body declares: its twelve-space
/// `type=` lines. `cache-from:`/`cache-to:` sit at ten spaces, so a cache backend never reads as a
/// tag rule.
fn container_tag_rules(job: &[&str]) -> Vec<String> {
    job.iter()
        .filter_map(|line| line.strip_prefix("            "))
        .filter(|rule| rule.starts_with("type="))
        .map(str::to_string)
        .collect()
}

/// The job ids on one job's `needs:`, scalar or inline list.
fn container_needs(job: &[&str]) -> Vec<String> {
    let line = job
        .iter()
        .find(|line| line.starts_with("    needs:"))
        .unwrap_or_else(|| panic!("the job must declare `needs:`, or it runs before what it reads"));
    let value = line
        .trim()
        .strip_prefix("needs:")
        .map(str::trim)
        .unwrap_or_else(|| panic!("unexpected `needs:` line `{line}`"));
    list_elements(value).into_iter().map(str::to_string).collect()
}

/// The elements of a `needs:` value written either as `[a, b]` or as one bare job id.
fn list_elements(value: &str) -> Vec<&str> {
    let inner = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(value);
    inner.split(',').map(str::trim).filter(|id| !id.is_empty()).collect()
}

/// #259 (S3): "The container smoke runs against an unpushed digest or a non-`latest` tag."
///
/// Pushing the release tags in the build step meant a broken image was served as `:0.18.0` and as
/// `:latest` for the whole duration of the smoke. The build may now push exactly one rehearsal ref,
/// and no tag a user or an installer resolves.
#[test]
fn the_container_build_jobs_push_only_a_rehearsal_tag() {
    let workflow = repo_text(CONTAINER);
    for image in IMAGES {
        let build = container_job(&workflow, image.build);
        assert_eq!(
            container_tag_rules(&build),
            vec![image.rehearsal.to_string()],
            "the {} build job must push exactly the rehearsal tag `{}`: any release tag there is \
             served to users before its smoke has run",
            image.label,
            image.rehearsal
        );
        assert!(
            build.iter().any(|line| line.trim() == "push: true"),
            "the {} build job must still push: the smoke runs on another runner and pulls the \
             digest from the registry",
            image.label
        );
        assert!(
            build.iter().any(|line| line.trim() == "load: false"),
            "the {} build job must not load the image locally: only the pushed digest is smoked",
            image.label
        );
        assert!(
            build
                .iter()
                .any(|line| line.trim() == "digest: ${{ steps.build.outputs.digest }}"),
            "the {} build job must still expose the digest its smoke and the promotion pull",
            image.label
        );
    }
}

/// #259 (S3): a release tag may only exist after the smoke that verified those exact bytes.
///
/// For each image, the job declaring that image's release tags must need both its build and its
/// smoke, and must retag the smoked digest. A promotion that needs only the build publishes an
/// image nothing verified; one that rebuilds publishes bytes the smoke never executed.
#[test]
fn the_container_release_tags_are_promoted_only_after_that_images_smoke_passes() {
    let workflow = repo_text(CONTAINER);
    let jobs = container_job_ids(&workflow);
    for image in IMAGES {
        let declaring: Vec<String> = jobs
            .iter()
            .filter(|id| {
                container_tag_rules(&container_job(&workflow, id.as_str()))
                    .iter()
                    .any(|rule| *rule == image.release_rule)
            })
            .cloned()
            .collect();
        assert_eq!(
            declaring.len(),
            1,
            "exactly one job may declare the {} release tag rule `{}`, found {declaring:?}: a copy of \
             the rule in a second job pushes release tags from a job that never smoked",
            image.label,
            image.release_rule
        );

        let promote = container_job(&workflow, &declaring[0]);
        let needs = container_needs(&promote);
        for required in [image.build, image.smoke] {
            assert!(
                needs.iter().any(|need| *need == required),
                "the job promoting the {} release tags must need `{}`: without it the promotion runs \
                 on an image that failed verification. needs = {needs:?}",
                image.label,
                required
            );
        }
        let smoked = format!("@${{{{ needs.{}.outputs.digest }}}}", image.build);
        assert!(
            promote.iter().any(|line| line.contains(&smoked)),
            "the {} promotion must retag the digest `{}` its smoke pulled, not a fresh build",
            image.label,
            smoked
        );
        assert!(
            promote
                .iter()
                .any(|line| line.contains("docker buildx imagetools create")),
            "the {} promotion must retag with `docker buildx imagetools create`: a rebuild or a \
             re-push there publishes an artifact the smoke never executed",
            image.label
        );
        assert!(
            !promote.iter().any(|line| line.contains("docker/build-push-action")),
            "the {} promotion must not build: a rebuild publishes bytes the smoke never ran",
            image.label
        );
        assert!(
            !promote.iter().any(|line| line.trim() == "push: true"),
            "the {} promotion must not push a build: the smoked digest is already in the registry",
            image.label
        );
    }
}

/// The behaviour the ordering fix must leave alone. Losing a trigger silently stops publishing the
/// images; losing a platform silently ships an amd64-only image under a multi-arch tag.
#[test]
fn the_container_workflow_keeps_its_triggers_platforms_and_smoke_digests() {
    let workflow = repo_text(CONTAINER);
    let jobs_at = container_jobs_at(&workflow);
    let triggers: Vec<&str> = workflow[..jobs_at].lines().collect();
    assert!(
        triggers.contains(&"  release:") && triggers.contains(&"    types: [published]"),
        "the images must still publish on `release: published`"
    );
    assert!(
        triggers.contains(&"  workflow_dispatch:"),
        "the images must still be publishable by hand"
    );

    for image in IMAGES {
        let build = container_job(&workflow, image.build);
        assert!(
            build
                .iter()
                .any(|line| line.trim() == "platforms: linux/amd64,linux/arm64"),
            "the {} build job must keep both platforms: a single-arch image under a multi-arch tag \
             breaks every arm64 pull",
            image.label
        );
        assert!(
            build.iter().any(|line| line.trim() == "packages: write"),
            "the {} build job must still be allowed to push to the registry",
            image.label
        );

        let smoke = container_job(&workflow, image.smoke);
        let pulled = format!("@${{{{ needs.{}.outputs.digest }}}}", image.build);
        assert!(
            smoke.iter().any(|line| line.contains(&pulled)),
            "the {} smoke must pull `{}`: a tag there smokes whatever the tag resolves to, not the \
             artifact this run built",
            image.label,
            pulled
        );
        assert!(
            container_needs(&smoke).iter().any(|need| *need == image.build),
            "the {} smoke must need its build, or it pulls a digest from another run",
            image.label
        );
    }
}
