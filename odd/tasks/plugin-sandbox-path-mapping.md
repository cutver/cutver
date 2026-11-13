# Plugin sandbox path mapping (issue #251)

Make a declared `filesystem` grant land where the guest is told the workspace is.

`Risk: item 2 (sandbox capability boundary) + item 3 (guest path convention is a contract plugin authors consume).`

## Specs

- **S1** — A `[plugins.<name>.permissions] filesystem` entry must be resolved against the workspace root during configuration load, exactly like `manifest[].path`, `changelog.path` and `changelog.template_file` already are: `filesystem = ["."]` becomes an absolute path, forward-slashed, with the Windows verbatim prefix (`\\?\`) stripped. Reuse the existing resolver in `src/config/discovery.rs`; do not add a second normalizer.
- **S2** — A sandboxed WASM guest must find the mounted directory under a guest name equal to the absolute path the lifecycle payload advertises in `root_dir` (`src/bump/exec/pipeline.rs`, `repo.to_string_lossy().replace('\\', "/")`), and must be able to open and read a file inside it. Proven end to end by a WAT fixture that discovers the preopen name with `wasi_snapshot_preview1::fd_prestat_dir_name`, then `path_open` + `fd_read` on a file of that directory. The test must be RED before S1 and GREEN after.
- **S3** — Guest-visible path strings use forward slashes only; the host side keeps a native `PathBuf`. Never let a backslash reach the guest name (CONTRACT Pillar II.3).
- **S4** — Zero ambient authority is preserved: only explicitly declared directories are mounted, nothing else becomes reachable (CONTRACT Pillar VI.2).
- **S5** — CONTRACT Pillar VI.4 holds: the feature stays behind the optional `plugins` cargo feature, and a lean build keeps compiling.
- **S6** — `docs/guides/plugin-authoring.md` states the convention in one short section: which path a guest must use to reach a declared directory, and that the guest name equals `root_dir`. Documentation ships in the same commit as the behavior (CONTRACT Pillar IV.4).
- **S7** — Quality gates: `cargo fmt --all -- --check` clean, `cargo clippy --locked --workspace --all-targets --features plugins -- -D warnings` clean, `cargo test --locked --workspace --features plugins` green. No `unwrap()`/`expect()`/`panic!()` in `src/` (Pillar III.2); functions ≤ 40 effective lines and files within budget (Pillar V.4).

## Tasks

| # | Task | Specs | Route | Commit |
|---|------|-------|-------|--------|
| T1 | Resolve `filesystem` grants; e2e WAT proof (preopen name + file read); docs section | S1-S7 | worker | one commit |

## Log

- **L1** (user, verbatim): "Abre el issue y atácalo de raíz con su e2e\n\nAbre el issue y atácalo de raíz con su e2e cumpliendo con @CONTRACT.md"
- **L2**: Issue #251 opened with the diagnosis: `src/plugin/driver/wasm.rs` passes the raw config string as both host dir and guest mount name, so `filesystem = ["."]` mounts a preopen literally named `.` while the payload advertises an absolute root; the guest's absolute read returns WASI `EPERM` (`os error 63`).
