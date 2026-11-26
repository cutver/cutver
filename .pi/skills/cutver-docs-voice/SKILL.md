---
name: cutver-docs-voice
description: "Trigger: write docs, update the README, document a feature, review documentation, docs tone, check a claim. Write cutver documentation evidence-first and in the plainly comprehensible register, with every absolute claim verified against the code."
license: Apache-2.0
metadata:
  author: cutver
  version: "1.0"
---

## Activation Contract

Activate when writing or editing `README.md`, anything under `docs/`, or documentation prose in `odd/tasks/**` and PR bodies.

Do not activate for code comments, commit messages, or CLI help text.

## Hard Rules

- **Evidence first.** Every factual claim must be checkable — a `path:line`, a command's output, a hash, a measurement. No evidence, no claim.
- **No hype.** Pillar IV.5 forbids marketing wording ("blazing", "revolutionary", "seamless", "powerful"). Convince with the mechanism, the numbers and the limits.
- **Scope every absolute.** `never`, `always`, `every`, `only`, `no flag`, `nothing else` must hold unconditionally or the sentence names its exception — the most frequent defect in this repository.
- **State the residual.** Say what was not verified, which platform was not exercised, what was left as debt. Never let a reader assume more coverage than exists.
- **Gotchas where they bite.** A visible callout beside the instruction, never a footnote or a sibling document.
- **Semantic link text.** Conceptual destinations only; never a raw path, never "click here".
- **Budgets.** `README.md` and `docs/**`: target 100–150 lines, hard ceiling 200. Decompose and link instead of growing.
- **Outcome first.** Frame by what the user gets; name implementation libraries second.
- **Plain register.** Write for a reader who did not write the code. Name the actor and the action before the
  mechanism ("the plugin drivers need the project folder" beats "gain a workspace-root parameter"); explain
  a term once, where it first appears, in that reader's words; keep one idea per sentence; give the number
  instead of the adjective (`1.3 MB against 5.0 MB`, never "much smaller"). Translate the concept, never the
  identifier: `latest`, `--channel` and command names stay verbatim in any language, because a reader who
  translates them cannot find them in the guide.
- **Clarity is not softening.** The limit, the residual and the case that cannot be recovered are stated as
  plainly as the gain. A rewrite that drops a caveat is a regression, not an edit.
- **Shared copy has its own shape.** Text that leaves the repository — a release body pasted into a chat, a
  summary for someone outside the project — leads with the summary and carries no markdown; in a chat the
  formatting is `*bold*`, `•` bullets and a plain URL.

## Decision Gates

| Situation | Action |
| --- | --- |
| About to write an absolute claim | Check it against the code, then scope it or drop it |
| A claim was found false once | Sweep every document that repeats it before fixing anything |
| A behaviour has an exception | Name the exception in the same sentence |
| Behaviour and design intent disagree | Document what the code does; record the intent separately |
| Something is unverified | Add it to a residual or debt section |
| The file would pass 200 lines | Decompose into `docs/` and link |
| Two documents state the same rule | Keep one canonical copy, link the other |
| A sentence carries two claims | Split it; a semicolon between them is a decision to hide one |
| An adjective stands where a number belongs | Replace it with the number or the mechanism |
| The text will be read outside the repository | Write the summary first, drop the markdown |

## Execution Steps

1. Read the code the claim is about and quote its `path:line`.
2. Draft, then grep your own draft for absolutes: `never|always|every|only|nothing else|no flag`.
3. Check each hit against the code. Scope or drop it.
4. Sweep the rest of the documentation for the same claim and fix every instance, not the reported one.
5. Verify: line count against the budget, links resolve, images exist, no hype wording.
6. Read the draft back as the person it is for: if a sentence names a mechanism before its effect, or an
   adjective stands where a number belongs, rewrite it.
7. Record what remains unverified.

## Output Contract

Return: files changed with line counts; each claim checked with its evidence; every absolute claim found false and where else it appeared; the sentences rewritten from mechanism-first or adjective-first; what was left unverified; and whether the class is closed.

## References

- `references/claim-sweep.md` — the sweep recipe, the triage rule, and worked examples from the 2026-10-11 sweep.
- `CONTRACT.md` at the repository root, Pillar IV.5 — the normative documentation rules.
