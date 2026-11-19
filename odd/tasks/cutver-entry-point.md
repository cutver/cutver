# Entry point — README and CONTRIBUTING

The repository has no contributor entry point and its user entry point sells architecture instead of the moment of
value. Both are the same defect: a stranger cannot find out what this is for, or how to work in it, without reading
1041 lines of workflow YAML and asking someone.
Requirement source: `odd/tasks/cutver-local-gate.md` L4-L6 and this session's README audit.

## Specs

S1. The README states the problem before the architecture. Within the first ~40 lines a stranger reads what it does in
one sentence and **the two-phase release** — stop before the tag, review and rewrite the release notes, then publish —
as the headline capability. The plugin story stays, as the second act rather than the title.

S2. The README carries one runnable two-phase example, taken from a real run in this repository: `plan` →
`bump --defer-publish` → rewrite the notes → `publish --notes-file -`. No example that was not executed.

S3. **Every technical claim is verified against the tree in this PR.** A claim that cannot be verified is removed, not
softened. Build sizes are either measured in this PR for both builds or replaced with a qualitative statement — today
`CONTRACT.md` says ~4 MB for the lean build while the README says ~1.25 MB lean and ~5 MB with plugins, and three numbers
that disagree are worse than no number.

S4. `CONTRIBUTING.md` is the entry point a stranger needs: the local gate (`scripts/check.sh` and what it covers, and that
a skipped step is not a pass), the three budgets (200 lines per document, enforced; 400 per source file, enforced by a
ratchet; 350 is a facade decision left to judgement), the commit convention and why, the pull-request mechanics
**including the label-after-create trap**, the working record (`odd/tasks/<feature>.md` with `## Specs`, `## Tasks` and a
`## Log` that quotes the requester verbatim), and the test discipline that this repository actually practises.

S5. Both files stay at or under 200 lines, the enforced ceiling, and every documentation link they contain resolves.

S6. Voice: the repository's own — direct, evidence first, no marketing adjectives. `CONTRACT.md` and the existing
`docs/` pages are the reference for register.

## Non-goals

- **No invented biography and no private projects.** The motivation, if stated, is the problem the tool solves, never a
  person, a client or an unrelated product. The requester's own account of why the project exists is theirs to write.
- **No new capability claims.** If the README sells it, the tree does it today.
- No restructuring of `docs/`. Pages may be linked; they are not rewritten here.

## Tasks

- [ ] T1 — S1, S3: the README's opening and Overview, with every claim verified.
- [ ] T2 — S2: the two-phase example, executed first, pasted second.
- [ ] T3 — S3: the build-size numbers settled by measurement or removed.
- [ ] T4 — S4: `CONTRIBUTING.md`.
- [ ] T5 — S5, S6: ceilings, links, and the register pass.

## Log

L1 (user, verbatim): `Vamos hacer el README.md + CONTRIBUTING.md ahora`

L2 (user, verbatim, on what cutver is for): `estoy seguro que ahí fuera hay personas que todavía no conocen cutver que
provablemente esté buscando algo como esto, que le permita detener en un punto la publicación para poder validar y
rescribir las release notes si es necesario, que que pueda crearce un plugin o consumir uno que le notifique`

L3 (evidence, README as of `a1a3030`): the headline is `A release engine you can extend: one core, four typed capability
slots, and plugins that run sandboxed by default.` — an architecture sentence for someone already convinced. `--notes-file`
appears **zero** times in the whole README; `--defer-publish` appears **once**, at line 87, inside a paragraph about
atomicity. `## Overview` starts at line 70, after the logo, the badge wall and the promo block. So every capability L2
names ships today and none of it is on the front page.

L4 (decision, parent): the two files are one unit, not two. `CONTRIBUTING.md` is the door for someone who contributes and
the README is the door for someone who uses; splitting them across PRs would produce two voices and two half-briefs.

L5 (constraint, parent): the size discrepancy is a fact to resolve, not to paper over. `CONTRACT.md` (Pillar VI.4) says
~4 MB for the lean build; the README says ~1.25 MB lean and ~5 MB with plugins. Measure both builds in this PR, or drop
the numbers. A budget nobody can point at is the defect class the ratchet exists to end.
