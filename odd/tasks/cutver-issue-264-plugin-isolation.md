# Feature: plugin isolation on the process runtime (issue #264)

Large by rule (security: ambient credentials), small in code. Branch `fix/issue-264-plugin-isolation`.
Specs quote the issue verbatim.

## Specs

- **S1** "A test plugin prints its environment; the assertion is that only granted keys are present
  and that a non-granted key set in the parent is absent."
- **S2** "A test plugin that never reads stdin fails with a typed timeout error within
  `timeout_seconds`, and the release aborts before any write (Pillar VI.3 pre-bump rule)."
- **S3** "The `release.yml:111` comment is either true or removed."
- **S4** "A declared-but-unset environment grant produces an actionable error naming the plugin and
  the key."
- **S5** (remedy, verbatim) "`.env_clear()` followed by `.envs(granted)`, where `granted` is the
  resolved `permissions.env` map." · "Put the stdin write under the same timeout as the wait, so a
  hung peer fails closed. The whole exchange — write, wait, drain — must be bounded by
  `timeout_seconds`." · "Keep both runtimes honouring the same grant semantics (Pillar VI.1); if WASM
  is the reference, make the process runtime match it."

## Tasks

- [ ] T1 One authority for the resolved grants: `resolve_env_grants(plugin, &permissions.env) ->
  Result<BTreeMap<String, String>, PluginError>` reading `std::env::var`, erroring on an unset
  declared key with the plugin and the key in the message. Use it in the process factory **and** in
  `driver/wasm.rs` (today `unwrap_or_default()` silently yields `""`). S4, S5
- [ ] T2 `spawn_child` gets `.env_clear()` before `.envs(&self.env)`, and the factory passes the
  resolved map instead of `BTreeMap::new()`. S1, S5
- [ ] T3 The stdin write moves into the bounded window: a writer thread owns the pipe, `poll_or_kill`
  stays the timeout authority, the writer joins after the child ends (a kill closes the pipe), and a
  write failure becomes a typed error instead of `let _ =`. S2, S5
- [ ] T4 Tests, cross-platform in the driver's existing `cfg!(windows)` idiom: granted-only env,
  hung stdin failing closed with `Timeout`, declared-but-unset grant naming plugin and key. S1, S2, S4
- [ ] T5 `release.yml` comment becomes true (or is removed). S3
- [ ] T6 Gates + independent verifier (security item), then RDD assess.

## Log

- **L1** (user, verbatim) "Vamos con el 164 y cuando terminemos tenemos que verificar a qué más le
  podemos activar la caché en el ci para mejorar los tiempos de compilación o ejecuciones"
- **L2** (user correction, verbatim) "Perdón 264 es el issue a atacar" — #164 stays closed.
- **L3** (evidence) `src/plugin/manager/factory.rs:74` builds the process driver with
  `BTreeMap::new()`, so `permissions.env` is not even applied on this runtime;
  `src/plugin/driver/process.rs:71-78` spawns with `.envs(&self.env)` and no `.env_clear()`;
  `src/plugin/driver/process.rs:116-121` writes stdin with `write_all` + `let _` before the timeout
  loop starts.
- **L4** (evidence) the process driver's tests are **not** unix-gated: they branch on
  `cfg!(windows)` (`cmd /C findstr` vs `cat`), so all three acceptance tests can run on this host.
- **L5** (scope note) the CI caching work the user asked for after this is tracked separately, as a
  unit of its own with its own measurements.
