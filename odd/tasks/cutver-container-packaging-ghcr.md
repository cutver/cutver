# Feature: OCI Container Packaging & GHCR Publishing Workflow

Goal: Provide official multi-architecture OCI container packaging for `cutver` via `Containerfile` / `Dockerfile` and automated publishing to GitHub Container Registry (`ghcr.io`) on releases, resolving the setup-action caching bottleneck and enabling hermetic CI/CD pipelines.

## Tasks

- [x] Task 1: Create `Containerfile`, `Dockerfile`, and `.dockerignore` for multi-stage Rust musl build and Alpine 3.24 runtime with Git, ca-certificates, and safe.directory
- [x] Task 2: Implement `.github/workflows/container.yml` for automated multi-arch (amd64/arm64) OCI image build, validation, and GHCR publishing on release
- [x] Task 3: Verify local container build with Podman, testing `cutver --version` and git workspace command execution
- [x] Task 4: Document container image usage in `README.md` (Podman, Docker, GitHub Actions `docker://`, GitLab CI)
- [x] Task 5: Run full project verification suite (`cargo test`, `cargo clippy`, `cargo fmt`) and finalize work-unit commits

## Verification Evidence

| Task | Commit | Checks |
| --- | --- | --- |
| 1 | f6388a6 | `podman build -t cutver:test -f Containerfile .` completed successfully; verified multi-stage static musl binary build + Alpine 3.24 runtime with git, ca-certificates, openssh-client, and safe.directory '*' |
| 2 | f6388a6 | `.github/workflows/container.yml` created with QEMU, Buildx, GHCR login, docker/metadata-action@v6, and docker/build-push-action@v7 targeting `linux/amd64,linux/arm64` with GHA caching |
| 3 | f6388a6 | Podman execution verified: `cutver --version` -> `cutver 0.10.0`, `cutver doctor` -> `✔ cutver.toml is valid`, and `cutver changelog show 0.10.0` inside container with workspace volume mount |
| 4 | f6388a6 | Updated `README.md` with install section OCI references and comprehensive guide covering Podman, Docker, GitHub Actions `docker://`, and GitLab CI |
| 5 | f6388a6 | `cargo test` (385 passed; 0 failed), `cargo clippy --all-targets -- -D warnings` (clean), `cargo fmt -- --check` (clean) |
