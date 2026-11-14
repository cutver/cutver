# Feature: release-surface integrity (#258, #260)

Track 1 of the robustness remediation queue. Branch `fix/258-260-container-backfill`.
The two issues share one verification vehicle — a real dispatch on `main` and a release
cut — so they are one track with two work-unit commits.

## Specs

S1 (#258, remedy 1): `Containerfile`, after the dependency cache layer and before the real source build:
`COPY .github/templates/cutver/RELEASE.md ./.github/templates/cutver/RELEASE.md`

S2 (#258, remedy 2): `.dockerignore`: remove `.github/` and add
`!.github/templates/cutver/RELEASE.md` after `*.md`. Negation under an excluded parent
directory is not reliable — do not depend on it alone.

S3 (#258, remedy 3): Delete the root `Dockerfile` (Pillar V.6: one artifact, one definition).

S4 (#258, remedy 4): Add a `pull_request` job that builds the image with `push: false` and
`platforms: linux/amd64`, `cache-from: type=gha`. "This is the only gate that would have
caught all three failures."

S5 (#258, remedy 5): Do **not** relocate the template under `src/`. `cutver.toml`'s
`template_file` and `cutver init` consume the same file — one artifact, two consumers.

S6 (#258 acceptance): `docker build -f Containerfile .` succeeds **including** a clean
context (`docker build --no-cache`).

S7 (#258 acceptance): The PR-time image job fails if `src/init/scaffold/templates.rs` gains
an `include_str!` resolving outside the copied set (demonstrate with a throwaway commit,
then revert).

S8 (#258 acceptance): Root `Dockerfile` is gone and `git grep -n "docker build -f Dockerfile"`
finds nothing stale.

S9 (#258 acceptance): After the next release,
`docker run ghcr.io/cutver/cutver:<tag> cutver --version` prints that version.

S10 (#258 failure mode, unchanged invariant): `ghcr.io/cutver/cutver:<tag>` must exist for
every published release; the release body links it.

S11 (#260, remedy, preferred branch): "delete the file, or re-derive it as a `workflow_call`
reusable form of `release.yml` with a tag input, so build → package → smoke → publish exists
exactly once." Chosen: delete.

S12 (#260 acceptance): "`git grep -c "Package (unix)"` across the workflow files returns 1."
The honest count after the deletion is one definition per artifact matrix inside `release.yml`
(CLI and plugins); the invariant that matters is that no second workflow carries a copy.

S13 (#260 acceptance): "A backfill run leaves the release title and body byte-identical
(`gh release view --json title,body` compared before and after)." Deleting the path satisfies
this by construction; there is no backfill run left to compare.

S14 (#260 acceptance): "If the path still ships assets, it runs the same smoke jobs that
`release.yml` runs. No workflow other than `release.yml` sets a release `title` or `body`."

S15 (#260 failure mode): "running the backfill after a healthy release rewrites the title and
body the plugin authored, and, when it ships assets, ships binaries that no smoke job ever
executed."

## Tasks

| ID | Spec | Route | Commit |
|---|---|---|---|
| T1 | S1, S2, S3 | inline | `fix(container): give the builder stage the release template it compiles in` |
| T2 | S4, S7 | inline | same commit (the test pins both) |
| T3 | S8, S10 | inline | same commit |
| T4 | S6, S9 | CI / human dispatch | no commit: proof only |
| T5 | S11, S12, S15 | inline | `chore(release): retire the release-assets backfill` |
| T6 | S13, S14 | inline, tests | same commit (`tests/release_metadata_ownership.rs`) |

## Log

L1 (user, verbatim): "Contigo tengo 3 sessiones abiertas, puedes verlas para asignar un par
a cada una"

L2 (user, choice `plan-a-worktrees`): "Yo Par1 (#258+#260) en el árbol principal; 01a1d7fe
Par2 (#261+#262) en ../cutver-261-262; 01a1d803 Par3 (#271+#270) en ../cutver-271-270.
Paralelismo real, tres tracks."

L3 (evidence, 2026-11-13): `docker --version` → `docker: command not found` on this host, so
S6 and the local half of S4 cannot be observed here. The PR-time job and the throwaway
`include_str!` demonstration are the only available proof of S7, and S9 requires the next
release.

L4 (evidence, 2026-11-13): RED/GREEN for T1–T3. The five tests of
`tests/container_build_inputs.rs` pass on the fix; restoring the base `Containerfile`,
`.dockerignore`, `container.yml` and `Dockerfile` from `HEAD` makes three of them fail with the
production message — "no `COPY` in the Containerfile puts it in the builder stage" — plus the
missing `pull_request` trigger and the surviving `Dockerfile`. The `.dockerignore`-only test
passes on the base state because `.github/` was not a `COPY` source then, which is exactly the
coupling the fix creates and the test now guards.

L5 (correction to the audit): the audit claimed `.dockerignore` excluded the template "twice
over: `.github/` and `*.md`". `*.md` is a root-level pattern in Docker's matcher and never
reached a nested file, so `.github/` was the whole defect. `*.md` stays as it was, and
`tests/container_build_inputs.rs` pins the distinction so the next reader is not misled.

L6 (decision, evidence-led): the new `pull_request` trigger reaches every job in
`container.yml`, so the five publishing jobs now carry
`if: github.event_name != 'pull_request'`. Without it the first pull request would push a
rehearsal tag to ghcr. The test asserts the gate on all five.

L7 (evidence, 2026-11-13): before the deletion `Package (unix)` appeared 4 times across the
workflows (twice in `release.yml`, twice in `release-assets.yml`); after it, twice, both inside
`release.yml`. S12's literal "returns 1" is an under-count of the two artifact matrices, so the
test pins the invariant — no other workflow declares packaging steps — instead of the number.
