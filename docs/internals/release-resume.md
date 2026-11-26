# Resuming a Closing Phase

> Outcome first: `cutver publish --resume` repeats only what the repository proves was left undone,
> and refuses everything else by naming the step and the evidence. The flag itself and its table row
> live in the [`cutver publish` reference](../reference/cli.md#cutver-publish).

## What `--resume` refuses

Nothing persists a run record, so a resume reads the repository for its plan: the release tag here and
on the remote, the floating major tag, and one `publish-<i>` attestation per `[publish] commands` entry.
Within that evidence it repeats only what is harmless to repeat — creating a tag that is absent, pushing
it, moving the floating major tag, verifying the gates read-only — and it refuses, naming the step and
the evidence it found, whenever the repository says this is not the release it continues:

- **no `--commit`** — a resume cannot re-identify the release it continues, which is why the flag
  requires the commit rather than guessing at `HEAD`;
- **`--commit` is not `HEAD`** — the release moved on, and the same rule a plain `cutver publish`
  applies stands here;
- **the version the manifests declare at `HEAD` does not produce `--tag`** — the same derivation a plain
  `cutver publish` performs;
- **the local or the remote tag `--tag` points at another commit** — a tag naming a different commit is
  a different release, so repeating a step over it would rewrite something this run did not do;
- **a `[publish] commands` entry has no `publish-<i>` attestation with conclusion `success` for
  `--commit`**, where `<i>` is the entry's 1-based position in declaration order. An arbitrary command
  is the one step a resume may not repeat on the tool's own judgement: record the verdict with
  `cutver attest publish-1 --commit <sha> --conclusion success` once it really succeeded, then resume.
- **an `on_post_release` plugin is not dispatched at all** — a plugin is arbitrary code too, and it may
  already have run in the attempt this resume continues, so repeating it is exactly the guess a resume
  refuses to make. The tree names the plugins it left alone. Inspect the release, and run `cutver publish`
  without `--resume` if the plugin still has to run.

A refusal names the entry's position and the command verbatim, and writes, tags, pushes and executes
nothing. A resume whose steps are all already in place exits `0`, reports itself as `Resumed` rather
than `Published`, and leaves the tag and the remote exactly as they were.
