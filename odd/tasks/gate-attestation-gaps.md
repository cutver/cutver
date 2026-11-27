# Gate attestations: what is worth improving

Found while answering "what did `cutver attest` emit" about `v0.22.0-beta.1`. Three items, and they are not
the same kind: one is a gap in what the release publishes, one is a decision the format already made, and one
is a word doing two jobs. Evidence is from the release run `45926734505` and `main` at `135a34d`.

## 1. The verdicts do not travel with the release

The release assets are the 16 archives, `SHA256SUMS` and `SHA256SUMS.bundle`. The two attestations
(`smoke`, `smoke-plugins`, both for commit `90da3ef`) exist only as the `attestations` artifact of the CI run
(`runs/45926734505`, artifact downloaded to verify this). `.cutver/` is gitignored (`.gitignore:11`), so they
are in no tree either.

What that costs: someone who downloads the release can verify the **bytes** (checksums, and the cosign
signature in the bundle) but cannot see that the smoke matrix passed for that commit, which is the thing
`publish` refused to proceed without.

What improving it would take, in order of what it buys:
- Attach the directory — or one `attestations.json` — to the release in the publish job. Small: the files are
  already in the workspace when `publish` runs (`release.yml:779-783`).
- Say plainly what an attached verdict means. Unsigned, it is a record that a workflow wrote, and anyone can
  forge one; a reader who trusts it without that sentence trusts the wrong thing.
- Signing them (`cosign attest` on the same keyless flow the checksums already use, `release.yml:812-819`)
  is the step that would make the record checkable by a third party, and that is a larger change than
  attaching them.

## 2. The reader ignores the version's value

`#409`/`#412` made the file open with `schemaVersion`, and the decision recorded there was deliberate: the
reader is total, so a `schemaVersion: 99` file still reads as the current shape rather than disabling a gate
that had passed. That is stated in the reader's doc comment (`src/config/gates/store.rs`) and on the
release-tail page.

So the key's promise is one-directional: a consumer reading the raw file can refuse a shape it does not know,
and `cutver` itself will not. Worth improving only if a decision is made: refuse an unknown version with a
named error, count it in `doctor`, or leave it. The cost of the first is exactly the failure the current
choice avoids.

## 3. "Attestation" names two different things

In this release pipeline the word covers cutver's gate verdicts (data, no signature, no key) and sigstore's
cryptographic attestation/signature (the cosign bundle). The job that writes the first is called
`Attest the release gates` (`release.yml:648`) and it runs a few steps from `Sign SHA256SUMS with Cosign`
(`:812-819`).

The beta.1 notes already had to separate the two tags that get confused (`cutver`'s moving major tag and the
setup action's `@v1` ref), and the notes document the "channel" collision the same way. This one is not
written down anywhere. The cheapest fix is prose — a paragraph where the gates are documented, stating what
each thing guarantees and what it does not — and it needs no code.

## Next step

Each item either becomes an issue in the core repository (with a row in `odd/tasks/deferred-findings.md`, which
is how that index works) or is dropped. Nothing here is filed yet, and none of the three is large enough to
justify opening three issues without a decision on the second one.
