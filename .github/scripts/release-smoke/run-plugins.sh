#!/bin/sh
# Plugin-surface release-artifact smoke test for Cutver (#169, decision A).
#
# Usage: run-plugins.sh <binary-path> <expected-version>
#
# The lean artifact contract is owned by run.sh. This script delegates to it so
# the base contract stays single-sourced, then proves the plugin surface really
# is compiled into the artifact under test: `plugin list --json` must exit 0 and
# report an empty plugin list against the hermetic fixture project. A lean
# binary fails here on purpose: no plugin archive is published without the
# surface actually being inside it.
#
# Checks:
#   1. Everything run.sh checks (--version, doctor --check-changelog,
#      changelog latest --json).
#   2. `plugin list --json` exits 0 and prints an empty plugin list.
#
# Fails closed: any error exits non-zero with a clear message. No silent skips.
# Written in POSIX sh so it runs under Git Bash on Windows, under bash on
# Ubuntu/macOS, and under BusyBox sh inside the published Alpine container image.

set -eu

fail() {
  printf 'release-plugins-smoke: FAIL: %s\n' "$*" >&2
  exit 1
}

if [ "$#" -ne 2 ]; then
  fail "usage: run-plugins.sh <binary-path> <expected-version>"
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

# 1) Base release contract, single-sourced from the proven lean script.
sh "$SCRIPT_DIR/run.sh" "$BINARY_PATH" "$EXPECTED_VERSION" || \
  fail "base release smoke (run.sh) failed for '$BINARY_PATH'"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cutver-plugins-smoke.XXXXXX")" || fail "could not create temp dir"
cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup 0 1 2 3 15

FIXTURE="$WORK_DIR/fixture"
mkdir -p "$FIXTURE"
cp "$FIXTURE_SRC"/* "$FIXTURE/"

# Hermetic cache root: never read a developer's or runner's ambient plugin cache.
CUTVER_PLUGIN_CACHE_DIR="$WORK_DIR/cache"
export CUTVER_PLUGIN_CACHE_DIR
mkdir -p "$CUTVER_PLUGIN_CACHE_DIR"

# 2) The plugin surface must be present: exit 0 and an empty plugin list.
PLUGIN_OUTPUT="$(cd "$FIXTURE" && "$BINARY_PATH" plugin list --json 2>&1)" || \
  fail "plugin list --json failed; this artifact lacks the plugin surface: $PLUGIN_OUTPUT"
printf 'release-plugins-smoke: plugin list --json =>\n%s\n' "$PLUGIN_OUTPUT"

printf '%s\n' "$PLUGIN_OUTPUT" | grep -q '"plugins": \[\]' || \
  fail "plugin list --json did not report an empty plugin list (unexpected configured plugins): $PLUGIN_OUTPUT"

printf 'release-plugins-smoke: OK %s\n' "$EXPECTED_VERSION"
