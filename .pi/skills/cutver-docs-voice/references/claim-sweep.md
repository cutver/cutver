# The claim sweep

Why this exists: on 2026-10-11 four verification rounds found **14 false claims across 8
documents** in this repository. Three of them came from the author's own writing or from a fix that
corrected one instance and left the others. The sweep is what closed the class, not the fix.

## The recipe

1. **Grep the surface for absolutes.** The class lives in these words:

   ```bash
   grep -rniE "never|always|every|only|nothing else|no path|no flag|exactly one|guarantee" \
     README.md docs/ --include='*.md'
   ```

2. **Check each hit against the code, not against intuition.** Quote the `path:line` that decides it.
   A claim is false when the code has a branch, an option or a default the sentence denies.
3. **Triage by who it misleads.** This is what keeps the loop finite:
   - *user-facing* — behaviour someone relies on when installing, configuring or running the tool.
     Fix it now.
   - *contributor-facing / design-intent* — architecture, in a document that describes design
     history or a roadmap. Record it as debt; do not let it block the user-facing fixes.
4. **Sweep the repeats before fixing.** A claim wrong in one place is usually wrong in every place it
   is copied to. Fixing only the reported instance leaves the class alive.
5. **Re-sweep to confirm closure.** Expect zero; do not stretch to agree with that expectation.

## Worked examples

| Claim | Why it was false | The scoping that fixed it |
| --- | --- | --- |
| "The loader re-verifies the digest on **every** load" (3 instances, one document) | `verify_sha256` returns `Ok(())` when `expected_hash` is `None`, and `hash` is `Option<String>` | "re-verified on every load **when present**; `add` always writes it" |
| "**No flag and no code path** skips verification" | the `None` branch above **is** a code path | "No flag skips the install-time gate" |
| "A failed step rolls the workspace back" (4 documents) | `transaction.commit()` runs before the tag and the publish; rollback stops there | "a failure **before the release commit** rolls back; the tag and the publish are reported, not undone" |
| "**Exactly one line** changed per version update" (2 documents) | Gradle also increments `versionCode`; the regex editor replaces every match | "for the AST and byte-span editors the diff is the version; Gradle and regex can touch more" |
| "The binary **never** hardcodes any framework" | `init`'s discovery knows the common manifests; `KNOWN_LOCKFILES` is hardcoded | name the two deliberate exceptions and where they live |
| "Identical Git states yield identical changelogs" | the changelog entry is stamped with the date it was cut | scope to bumps and manifest diffs; the date is part of the changelog |
| "Tag resolution **always** uses full three-part SemVer tags" | only the floating major tag is skipped | "other tags stay eligible" |

## Residuals and debt

State them in the document, not in the conversation: an unverified platform, an uninspected
external artifact, a file over the documentation budget. A residual that lives only in a chat is a
residual the next reader will not see.

## When a claim contradicts another document

Two documents describing the same rule and disagreeing is worse than one wrong document: the reader
cannot tell which to believe. Pick the canonical home for the rule, fix both, and link the second.
