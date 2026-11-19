# Visual Tour

What cutver looks like at each step of the release lifecycle.

---

## Preflight Diagnostics — `cutver doctor`

Validating `cutver.toml`, tracking the primary manifest, and reporting preflight steps without anomalies.

<p align="center">
  <img src="../../assets/cutver_doctor_showcase.png" width="640" alt="Terminal output of cutver doctor: Validating cutver.toml configuration, tracking Cargo.toml at version 0.13.0, and reporting preflight test steps without anomalies." />
</p>

---

## Release Plan — `cutver plan`

The whole release as data, before anything changes: the deduced version, every manifest diff, the
changelog body, the commit and the tag. No file is written, no preflight step runs and no plugin is
dispatched — not even `on_pre_bump`, which the simulation below still dispatches. The same tree is what
the [first release walkthrough](getting-started.md) teaches you to read first.

```text
ℹ Release Plan: 0.1.0 -> 0.2.0 (minor (deduced from 1 feature))
├── Manifests
│   └── package.json (0.1.0 -> 0.2.0)
├── Preflight
│   └── test: npm test
├── Changelog
│   └── CHANGELOG.md (prepends 0.2.0)
├── Git
│   ├── commit: chore(release): v0.2.0 [skip ci]
│   └── tag: v0.2.0
└── Publish
    └── push: true

Nothing was written, staged, committed, tagged or pushed.
```

---

## Atomic Bump Simulation — `cutver bump auto --dry-run`

The boxed SIMULATION MODE banner, SemVer deduction from conventional commits, and the hierarchical
tree of manifests, changelog, git tag, and `cargo publish`.

<p align="center">
  <img src="../../assets/cutver_bump_showcase.png" width="640" alt="Terminal output of cutver bump auto in simulation mode: Demonstrating the boxed SIMULATION MODE banner, SemVer minor deduction from conventional commits, hierarchical tree of manifests, changelog, git tag, and cargo publish." />
</p>

---

## WebAssembly Plugin Engine — `cutver plugin add`

Registry index resolution, wasm binary download URL, SHA-256 digest validation, and `cutver.toml`
injection — before and after the plugin is declared.

<p align="center">
  <img src="../../assets/cutver_plugin_showcase.png" width="640" alt="Terminal output of cutver plugin add github-publish --event on_pre_bump --event on_post_release --dry-run: Demonstrating registry index resolution, wasm binary download URL, SHA-256 digest validation, and cutver.toml injection." />
</p>

---

## Formatted Release Notes — `cutver changelog latest`

Generated release notes with conventional commit links, emojis for fixes and features, and the full
changelog diff.

<p align="center">
  <img src="../../assets/cutver_changelog_latest_showcase.png" width="640" alt="Terminal output of cutver changelog latest: Displaying generated release notes with conventional commit links, emojis for bug fixes, features, and full changelog diff." />
</p>

---

## Related Documentation

- [Getting Started Walkthrough](getting-started.md): the same flow with copyable commands.
- [CLI Commands and Exit Codes](../reference/cli.md): every command shown here with its flags.
- [Installing and Managing Plugins](../reference/plugins.md): what `cutver plugin add` writes and verifies.
