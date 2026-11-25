#!/bin/sh
# Local gate — one command that mirrors `.github/workflows/ci.yml` closely enough that a red CI run
# is never the first time a defect becomes visible (odd/tasks/cutver-local-gate.md, S1-S3).
#
# Three CI runs went red across three phases of this roadmap and none of them was a logic defect: an
# assertion that only holds on one host, a suite that only passed without the Actions environment,
# and an `unused import` inside a `#![cfg(unix)]` test file that a Windows host never type-checks.
# Each cost a full CI cycle to learn what this gate shows in seconds. So the steps below run in the
# workflow's order, the two test runs execute in the environment Actions gives them, and the one step
# this host cannot run says so instead of passing quietly.
#
# Usage: scripts/check.sh   — exit 0 means every runnable step passed. It runs from the repository
# root wherever it was invoked from, writes nothing outside a private temporary directory it removes,
# and changes nothing in the caller's shell.
#
# POSIX `sh` on purpose: Git Bash on Windows and the shell on a Linux or macOS host both run it.

set -u

# The one host this gate cannot fully mirror: CI's ubuntu legs lint the plugins feature natively,
# and this host reaches that code only by cross-checking the linux target.
LINUX_TARGET='x86_64-unknown-linux-gnu'
LINUX_CC='x86_64-linux-gnu-gcc'

cd "$(dirname "$0")/.." || {
	printf 'check: cannot enter the repository root\n' >&2
	exit 1
}

# --- private temporary directory ---------------------------------------------------------------
# The GitHub sinks below live here, so the gate has to create it, and it has to disappear even when
# a step fails or the run is interrupted. Nothing else is written outside it.
GATE_TMP=$(mktemp -d "${TMPDIR:-/tmp}/cutver-gate.XXXXXX") || {
	printf 'check: cannot create a temporary directory; nothing ran\n' >&2
	exit 1
}
cleanup() {
	case "$GATE_TMP" in
	*/cutver-gate.*) [ -d "$GATE_TMP" ] && rm -rf -- "$GATE_TMP" ;;
	esac
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

GATE_RESULTS="$GATE_TMP/results"
: >"$GATE_RESULTS" || exit 1
GATE_STEP_INDEX=0

record() {
	printf '%s\t%s\t%s\n' "$1" "$2" "$3" >>"$GATE_RESULTS"
}

# --- S2: the environment every Actions `run:` step actually has ---------------------------------
# `GITHUB_ACTIONS`, `GITHUB_OUTPUT` and `GITHUB_STEP_SUMMARY` are set on every step in a workflow,
# and a suite that only passes without them is a lie: that exact lie turned PR #332 red. The
# assignments apply to the child only, so the caller's shell is untouched and nothing survives the
# run once the directory above is gone.
#
# `CARGO_PROFILE_DEV_DEBUG=0` is here because this gate has no use for debug symbols and the
# alternative costs disk by the hundred gigabytes. It is not a cure, and it does not claim to be one.
#
# Measured on this repository: 1108 `*.pdb` files under `target/debug`, 91 GB in total, the largest
# ~483 MB each — every feature combination, every clippy variant and every integration-test binary
# links a fresh `cutver-<hash>.pdb`, and nothing prunes them. Clearing them took the tree from 111 GB
# to 14 GB.
#
# With this set, four gate runs produced 163 `*.pdb` totalling 1.6 GB, the largest 99 MB: about a
# five-fold reduction per artefact, and no longer the dominant cost of a build. PDBs do **not**
# disappear, and why they are still emitted is not established — so do not read this as "no more
# PDBs", and re-measure before claiming a bigger win.
#
# It changes no compilation semantics — no optimisation, no output, nothing a test observes — and
# `panicked at src/…:line` survives because it is compile-time metadata. It is set here rather than in
# `Cargo.toml` so a contributor's own build stays fully debuggable.
cargo_with_ci_env() {
	GATE_STEP_INDEX=$((GATE_STEP_INDEX + 1))
	for sink in "$GATE_TMP/github-output-$GATE_STEP_INDEX" "$GATE_TMP/github-summary-$GATE_STEP_INDEX"; do
		: >"$sink" || return 1
	done
	CARGO_PROFILE_DEV_DEBUG=0 \
		GITHUB_ACTIONS=true \
		GITHUB_OUTPUT="$GATE_TMP/github-output-$GATE_STEP_INDEX" \
		GITHUB_STEP_SUMMARY="$GATE_TMP/github-summary-$GATE_STEP_INDEX" \
		"$@"
}

# --- S3: what this host cannot run ---------------------------------------------------------------
# A step that cannot run prints its reason and lands in the third summary bucket. It is never a pass.
linux_c_compiler() {
	if [ "$(uname -s 2>/dev/null)" = 'Linux' ]; then
		command -v cc >/dev/null 2>&1 && {
			printf 'cc'
			return 0
		}
		command -v gcc >/dev/null 2>&1 && {
			printf 'gcc'
			return 0
		}
		return 1
	fi
	command -v "$LINUX_CC" >/dev/null 2>&1 && {
		printf '%s' "$LINUX_CC"
		return 0
	}
	return 1
}

linux_target_installed() {
	# Without rustup there is no way to ask, so let the step itself report the truth.
	command -v rustup >/dev/null 2>&1 || return 0
	rustup target list --installed 2>/dev/null | tr -d '\r' | grep -qx "$LINUX_TARGET"
}

SKIP_NO_TARGET="the $LINUX_TARGET target is not installed; \`rustup target add $LINUX_TARGET\` enables this step"
SKIP_NO_CC="zstd-sys (via extism) compiles C for $LINUX_TARGET and needs $LINUX_CC, which is not on PATH here"

# --- the steps, in the workflow's order --------------------------------------------------------
STEPS='toolchain dependency-shape build test test-plugins clippy clippy-plugins linux-clippy linux-plugins-clippy fmt'

label() {
	case "$1" in
	toolchain) printf 'toolchain (rustc -V, cargo -V)' ;;
	dependency-shape) printf 'lean dependency shape (no WebAssembly runtime)' ;;
	build) printf 'build (default features)' ;;
	test) printf 'test (default features)' ;;
	test-plugins) printf 'test (plugins feature)' ;;
	clippy) printf 'clippy (default features)' ;;
	clippy-plugins) printf 'clippy (plugins feature)' ;;
	linux-clippy) printf 'clippy (%s, default features)' "$LINUX_TARGET" ;;
	linux-plugins-clippy) printf 'clippy (%s, plugins feature)' "$LINUX_TARGET" ;;
	fmt) printf 'format (cargo fmt --all --check)' ;;
	esac
}

# One step: return 0 for passed, 2 for "not run here" with $SKIP_REASON set, anything else failed.
run_step() {
	case "$1" in
	toolchain)
		rustc -V && cargo -V
		;;
	dependency-shape)
		# Pillar VI.4 became a shape invariant when the ~4 MB budget was dropped: without
		# `--features plugins` the resolved graph must reach no WebAssembly runtime. It runs
		# before `build` because a shape defect is cheaper to see than a compile failure.
		graph=$(cargo tree --locked --no-default-features -e normal --prefix none) || return 1
		names=$(printf '%s\n' "$graph" | awk '{print $1}' | sort -u)
		runtimes=$(printf '%s\n' "$names" | grep -E '^(extism|wasmtime)' || true)
		if [ -n "$runtimes" ]; then
			printf 'the lean build reaches a WebAssembly runtime:\n%s\n' "$runtimes" >&2
			printf 'extism and wasmtime belong behind --features plugins\n' >&2
			return 1
		fi
		printf 'lean dependency graph: %s crates, no WebAssembly runtime\n' "$(printf '%s\n' "$names" | wc -l | tr -d ' ')"
		;;
	build)
		cargo_with_ci_env cargo build --locked --workspace --all-targets
		;;
	test)
		cargo_with_ci_env cargo test --locked --workspace
		;;
	test-plugins)
		cargo_with_ci_env cargo test --locked --workspace --features plugins
		;;
	clippy)
		cargo_with_ci_env cargo clippy --locked --workspace --all-targets -- -D warnings
		;;
	clippy-plugins)
		cargo_with_ci_env cargo clippy --locked --workspace --all-targets --features plugins -- -D warnings
		;;
	linux-clippy)
		linux_target_installed || {
			SKIP_REASON=$SKIP_NO_TARGET
			return 2
		}
		cargo_with_ci_env cargo clippy --locked --workspace --all-targets --target "$LINUX_TARGET" -- -D warnings
		;;
	linux-plugins-clippy)
		# The linux leg CI runs natively is the only place a `#[cfg(unix)]` file is type-checked
		# and the plugins feature is compiled at all, so this step is the one that catches what a
		# Windows-only run cannot see — PR #332's `unused import` lived exactly here.
		linux_target_installed || {
			SKIP_REASON=$SKIP_NO_TARGET
			return 2
		}
		linux_c_compiler >/dev/null || {
			SKIP_REASON=$SKIP_NO_CC
			return 2
		}
		cargo_with_ci_env cargo clippy --locked --workspace --all-targets --features plugins \
			--target "$LINUX_TARGET" -- -D warnings
		;;
	fmt)
		cargo_with_ci_env cargo fmt --all --check
		;;
	esac
}

# --- run, stopping at the first failure --------------------------------------------------------
FAILED_AT=''
for step in $STEPS; do
	name=$(label "$step")
	if [ -n "$FAILED_AT" ]; then
		record notrun "$name" "not reached: the gate stopped at '$FAILED_AT'"
		continue
	fi
	SKIP_REASON=''
	printf '\n====> %s\n' "$name"
	run_step "$step"
	status=$?
	case $status in
	0) record pass "$name" '' ;;
	2) printf '      not run here: %s\n' "$SKIP_REASON"
		record notrun "$name" "$SKIP_REASON" ;;
	*) FAILED_AT=$name
		printf '\n====> FAILED: %s\n' "$name"
		record fail "$name" '' ;;
	esac
done

# --- the summary: three buckets, so a skipped step can never read as a pass ---------------------
count_status() {
	awk -F '	' -v want="$1" '$1 == want { found++ } END { print found + 0 }' "$GATE_RESULTS"
}

print_bucket() {
	printf '%s (%s):\n' "$2" "$(count_status "$1")"
	seen=0
	while IFS='	' read -r status name detail; do
		[ "$status" = "$1" ] || continue
		seen=$((seen + 1))
		if [ -n "$detail" ]; then
			printf '  - %s: %s\n' "$name" "$detail"
		else
			printf '  - %s\n' "$name"
		fi
	done <"$GATE_RESULTS"
	[ "$seen" -eq 0 ] && printf '  - none\n'
	return 0
}

PASSED=$(count_status pass)
FAILED=$(count_status fail)
NOT_RUN=$(count_status notrun)

printf '\n=============================== check summary ===============================\n'
print_bucket pass 'passed'
print_bucket fail 'failed'
print_bucket notrun 'not run here'
printf '%s\n' '-----------------------------------------------------------------------------'
if [ "$FAILED" -ne 0 ]; then
	printf 'gate: FAIL (%s passed, %s failed, %s not run here) at: %s\n' \
		"$PASSED" "$FAILED" "$NOT_RUN" "$FAILED_AT"
else
	printf 'gate: PASS (%s passed, %s failed, %s not run here)\n' "$PASSED" "$FAILED" "$NOT_RUN"
fi
if [ "$NOT_RUN" -ne 0 ]; then
	printf 'note: %s step(s) did not run here. A skipped step is not a pass; the reason is above.\n' "$NOT_RUN"
fi
printf '=============================================================================\n'

[ "$FAILED" -eq 0 ]
