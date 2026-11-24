# Security

`cutver` runs commands you configure, uses the credentials your shell has, writes into your working
tree and pushes tags. This page says which defects belong here and what happens after you report one.

## What a report is for here

A report belongs here when the defect is in `cutver` itself and its effect is on the machine or the
repository a run touches. Those surfaces, with the code that shows each:

- **Configured commands run as you.** `[hooks] pre_bump` and `post_bump` go through the platform shell
  in the workspace root (`src/bump/exec/hooks.rs:89-91`, `src/proc.rs:92-102`), and a
  `runtime = "process"` plugin is spawned as a program with no shell and no privilege change
  (`src/plugin/driver/process.rs:114-122`). Running your own code is the feature; a way to run a
  command the configuration did not name is the vulnerability.
- **The plugin environment boundary.** A native plugin starts from `env_clear()` plus a named system
  baseline plus the keys its `permissions.env` declares (`src/plugin/driver/process.rs:118-120`), and
  the baseline carries no `*_TOKEN` or `*_KEY` (`src/plugin/driver/baseline.rs:19-22`). A path that
  hands a child an undeclared key breaks that boundary, and it is in scope.
- **Credentials reach a push.** The `git` process a push spawns inherits the caller's environment, with
  no `env_clear` on it (`src/git/ops.rs:201-204`), so the credential helper or token variable a
  `git push` in that shell would use is the one `cutver`'s push uses. Reading, logging or forwarding
  one of those is a vulnerability.
- **The working tree is written.** A bump rewrites each manifest version in place
  (`src/bump/exec/manifest.rs:87`, `src/bump/exec/manifest.rs:169`) and `post_bump` stages the
  lockfiles it touched (`src/bump/exec/hooks.rs:76`). A write outside the workspace root, or a leftover
  file after a failure that claims the workspace is untouched, is in scope.
- **A release publishes.** The commit and tag exist before `[publish].commands` run as shell commands
  (`src/bump/exec/publish.rs:416-433`), and the tag pushes are issued from the same module
  (`src/bump/exec/publish.rs:377`). A failure that pushes, force-moves a tag or publishes without the
  summary saying so belongs here.

## The channel

Report through the private advisory form: https://github.com/cutver/cutver/security/advisories/new

Private vulnerability reporting is enabled for this repository; it was verified on 2026-11-24. That
form is the only channel this project runs — no security address, no chat, no discussions board — so a
message sent anywhere else is a message nobody reads.

**Do not open a public issue with reproduction details or an exploit.** The form exists so the fix can
land before the technique is public; a public issue spends that advantage in the first comment.

## What to expect

- A fix lands on `main` and ships in the next release. There is no private fix branch and no embargo
  machinery beyond that.
- No backports are promised before 1.0. If you run an older release the fix may not reach it; say so in
  the report if that is your situation.
- There is no response-time guarantee: nothing here promises an acknowledgement within a number of
  days, a triage date, or a credit line, and the advisory is where the discussion stays.

## Out of scope

- **A plugin's own bugs.** A plugin is a program its author wrote; its parsing, its logic and its
  output belong to that plugin's repository. `cutver` runs it and bounds it with a deadline.
- **A misconfiguration you wrote.** A hook that deleted your files ran because `cutver.toml` named it,
  and an undeclared-but-expected environment key is fail-closed rather than a leak
  (`src/plugin/driver/process.rs:99-100`). Neither is a defect in `cutver`.
