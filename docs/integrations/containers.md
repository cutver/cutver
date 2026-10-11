# Container Integration Guide

Run `cutver` via Podman, Docker, `docker://` GitHub Actions, and GitLab CI.

Official minimal multi-architecture container images (`linux/amd64`, `linux/arm64`) are published to GitHub Container Registry (`ghcr.io/cutver/cutver`) on Alpine Linux with Git, OpenSSH, CA certificates, and safe repository directories preconfigured.

---

## Container Registries & Tags

- `ghcr.io/cutver/cutver:latest` — Built from `main` and from release tags (`.github/workflows/container.yml` enables `latest` on both), so it can be ahead of the latest stable release; pin a release with an explicit version tag instead.
- `ghcr.io/cutver/cutver:0.13.0` — Pinned SemVer release tag (`:0.13` and the floating `:0` also exist).
- `ghcr.io/cutver/cutver:0` — Floating major version tag.
- `ghcr.io/cutver/cutver:0.13.0-plugins`, `:0.13-plugins`, and `:plugins` — the plugin-enabled image. It publishes `<version>-plugins`, `<major>.<minor>-plugins`, and the floating `:plugins`; there is no `:0-plugins` and no `:latest-plugins` ([plugin reference](../reference/plugins.md)).

---

## Local Execution (Podman, Docker & WSLC)

Mount your repository into `/workspace` inside the container:

### Podman
```bash
# Verify workspace health and manifest drift
podman run --rm -v "$PWD:/workspace" ghcr.io/cutver/cutver:latest doctor

# Simulate automated release
podman run --rm -v "$PWD:/workspace" ghcr.io/cutver/cutver:latest bump auto --dry-run
```

### Docker
```bash
# Verify workspace health and manifest drift
docker run --rm -v "$PWD:/workspace" ghcr.io/cutver/cutver:latest doctor

# Simulate automated release
docker run --rm -v "$PWD:/workspace" ghcr.io/cutver/cutver:latest bump auto --dry-run
```

### WSL Containers (WSLC / Windows Native)

Run directly from Windows PowerShell or Command Prompt without installing Docker Desktop or Podman:

```powershell
# Verify workspace health and manifest drift
wslc run --rm -v "${PWD}:/workspace" ghcr.io/cutver/cutver:latest doctor

# Simulate automated release
wslc run --rm -v "${PWD}:/workspace" ghcr.io/cutver/cutver:latest bump auto --dry-run
```

---

## GitHub Actions: Running via `docker://`

Run `cutver` directly inside GitHub Actions without downloading or compiling binaries by using GitHub Actions' native `docker://` protocol:

```yaml
name: Check Workspace Health

on:
  pull_request:

jobs:
  doctor:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0

      - name: Verify Manifest Sync
        uses: docker://ghcr.io/cutver/cutver:latest
        with:
          args: doctor --check-changelog
```

---

## GitLab CI/CD

Run `cutver` inside GitLab CI pipelines using the container image directly:

```yaml
stages:
  - verify
  - release

cutver:doctor:
  stage: verify
  image:
    name: ghcr.io/cutver/cutver:latest
    entrypoint: [""]
  script:
    - /usr/local/bin/cutver doctor --check-changelog

cutver:release:
  stage: release
  image:
    name: ghcr.io/cutver/cutver:latest
    entrypoint: [""]
  only:
    - main
  before_script:
    - git config --global user.name "${GITLAB_USER_NAME}"
    - git config --global user.email "${GITLAB_USER_EMAIL}"
    - git remote set-url origin "https://gitlab-ci-token:${CI_JOB_TOKEN}@${CI_SERVER_HOST}/${CI_PROJECT_PATH}.git"
  script:
    - /usr/local/bin/cutver bump auto
```

---

## Container Environment & Gotchas

1. **Working Directory**: The default container working directory is `/workspace`.
2. **Git Safe Directory**: The container image runs `git config --system safe.directory '*'` so that directory ownership differences between host and container do not trigger git security warnings.
3. **SSH Keys & Git Credentials**: For write operations (e.g. `cutver bump` pushing upstream), forward Git credentials or SSH agents into the container (`-v $SSH_AUTH_SOCK:/ssh-agent -e SSH_AUTH_SOCK=/ssh-agent`).

---

## Related Documentation

- [GitHub Actions Integration Guide](github-actions.md): GitHub Actions setup and release workflows.
- [CLI Commands and Exit Codes](../reference/cli.md): All command arguments and options.
- [Complete cutver.toml Configuration Reference](../reference/configuration.md): Preflight timeouts and git settings.
