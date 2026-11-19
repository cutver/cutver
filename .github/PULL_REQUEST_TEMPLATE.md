<!-- One pull request, one unit. Tick a box only when you can point at the evidence: an unticked box
     invites a question, and a ticked box without evidence spends the reviewer's attention twice.
     What each of the four is really asking for lives in the contributing guide; this file is only the
     prompt, kept short because a reviewer reads it in fifteen seconds or not at all. -->

## What changed

<One or two sentences: the behaviour before, the behaviour after, and the command that shows it.>

## The four checks

- [ ] **The gate was run.** `sh scripts/check.sh` exited `0`, and its summary reads zero failed and zero *not run here*. A step this host could not run is reported there as not-run, never as a pass — if one is in that bucket, name it and say why the change is still safe, or run it somewhere it can.

- [ ] **The docs moved with the behaviour.** A new or changed surface sent me back to the pages that describe the flow *around* it, not only to its own reference row: a new flag changes what the first-run page recommends. That is the freshness half of the documentation rules, and shipping the reference row alone buys presence, not freshness.

- [ ] **The slice boundary is stated.** Either this diff sits inside 300–400 lines of effective change, or the section below says why it is one unit and where its reviewable slices are. That range is a target with an evidence rule attached, not a gate: the evidence is the paragraph, not a line count.

- [ ] **Every claim is verified.** Every command, number and behaviour named above was run in this tree, in this state. A claim nobody ran is a rumour, and a reviewer who has to check one is not reviewing the diff.

## The slice boundary, when the diff is past the target

<Delete this section inside 300–400 lines. Otherwise: why is this one unit, and where are the seams a
reviewer reads it along? A chain of pull requests is the sanctioned answer.>

## Required checks

Two contexts must be green before merge, by these exact names: `CI gate` and `Exactly one type label`.

**The label trap.** `gh pr create --label type:fix` creates the pull request first and applies the label
second, so a fresh pull request always shows a red `opened` run and a green `labeled` run — expected.
The latest run is the one that counts, so a `labeled` run that is still red means zero or two `type:*`
labels, and its message says which count it objects to.

The reasoning behind all of it — what the gate collapses, the budgets, the test policy — is in
[the contributing guide](CONTRIBUTING.md).
