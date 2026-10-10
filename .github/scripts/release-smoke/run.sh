#!/bin/sh
# Hermetic release-artifact smoke test for Cutver (issue #174).
#
# Usage: run.sh <binary-path> <expected-version>
#
# No artifact is published without first being executed. This script copies a
# self-contained fixture (its own minimal project under
# tests/fixtures/release-smoke/) into a temporary directory, turns it into a
# fresh git repository with a commit and tags, then runs the shipped binary
# against it. The result never depends on the state of the cutver repository.
#
# Checks:
#   1. `<binary> --version` prints the expected released version.
#   2. `doctor --check-changelog` validates config, manifests, git tags and the
#      changelog (real release logic, not only --version).
#   3. `changelog latest` parses the fixture changelog and assembles a release
#      context (real release logic).
#
# Fails closed: any error exits non-zero with a clear message. No silent skips.
# Written in POSIX sh so it runs under Git Bash on Windows, under bash on
# Ubuntu/macOS, and under BusyBox sh inside the published Alpine container image.

set -eu

fail() {
  printf 'release-smoke: FAIL: %s\n' "$*" >&2
  exit 1
}

if [ "$#" -ne 2 ]; then
  fail "usage: run.sh <binary-path> <expected-version>"
fi

BINARY_PATH="$1"
EXPECTED_VERSION="$2"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FIXTURE_SRC="$SCRIPT_DIR/../../../tests/fixtures/release-smoke"

[ -d "$FIXTURE_SRC" ] || fail "fixture source not found at '$FIXTURE_SRC'"

# Windows artifacts ship cutver.exe; accept a bare path too.
if [ ! -f "$BINARY_PATH" ] && [ -f "$BINARY_PATH.exe" ]; then
  BINARY_PATH="$BINARY_PATH.exe"
fi
[ -f "$BINARY_PATH" ] || fail "binary not found at '$BINARY_PATH'"

# Resolve to an absolute path so it survives the later cd into the fixture.
BINARY_DIR="$(cd "$(dirname "$BINARY_PATH")" && pwd)"
BINARY_PATH="$BINARY_DIR/$(basename "$BINARY_PATH")"

normalize_version() {
  v="$1"
  # Trim leading/trailing whitespace, then an optional leading v/V.
  v="${v#"${v%%[![:space:]]*}"}"
  v="${v%"${v##*[![:space:]]}"}"
  v="${v#v}"
  v="${v#V}"
  printf '%s' "$v"
}

EXPECTED_VERSION="$(normalize_version "$EXPECTED_VERSION")"
[ -n "$EXPECTED_VERSION" ] || fail "expected version is empty"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cutver-release-smoke.XXXXXX")" || fail "could not create temp dir"
cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup 0 1 2 3 15

FIXTURE="$WORK_DIR/fixture"
mkdir -p "$FIXTURE"
cp "$FIXTURE_SRC"/* "$FIXTURE/"

# Fresh git repo with a local identity and no signing; independent of any
# global git config on the runner.
git -C "$FIXTURE" init -q
git -C "$FIXTURE" config user.email "release-smoke@example.com"
git -C "$FIXTURE" config user.name "Release Smoke"
git -C "$FIXTURE" config commit.gpgsign false
git -C "$FIXTURE" config tag.gpgsign false
git -C "$FIXTURE" add -A
git -C "$FIXTURE" commit -q -m "chore: seed release-smoke fixture"
git -C "$FIXTURE" tag v1.0.0
git -C "$FIXTURE" tag v1.1.0

# 1) --version must report the released version.
VERSION_OUTPUT="$("$BINARY_PATH" --version 2>&1)" || fail "--version exited non-zero: $VERSION_OUTPUT"
printf 'release-smoke: --version => %s\n' "$VERSION_OUTPUT"
ACTUAL_VERSION="$(normalize_version "${VERSION_OUTPUT##* }")"
[ "$ACTUAL_VERSION" = "$EXPECTED_VERSION" ] || \
  fail "version mismatch: expected '$EXPECTED_VERSION', got '$ACTUAL_VERSION' from '$VERSION_OUTPUT'"

# 2) doctor --check-changelog: config, manifests, git tags and changelog.
DOCTOR_OUTPUT="$(cd "$FIXTURE" && "$BINARY_PATH" doctor --check-changelog 2>&1)" || \
  fail "doctor --check-changelog failed: $DOCTOR_OUTPUT"
printf '%s\n' "$DOCTOR_OUTPUT" | grep -q "is valid" || \
  fail "doctor did not report a valid config: $DOCTOR_OUTPUT"

# 3) changelog latest --json: changelog parsing and release-context assembly.
CHANGELOG_OUTPUT="$(cd "$FIXTURE" && "$BINARY_PATH" changelog latest --json 2>&1)" || \
  fail "changelog latest --json failed: $CHANGELOG_OUTPUT"
printf '%s\n' "$CHANGELOG_OUTPUT" | grep -q '"version": "1.1.0"' || \
  fail "changelog latest --json did not report version 1.1.0: $CHANGELOG_OUTPUT"

printf 'release-smoke: OK %s\n' "$ACTUAL_VERSION"
