#!/usr/bin/env bash
# scripts/ci-local.sh — run the same quality checks CI runs, locally.
#
# Usage:
#   ./scripts/ci-local.sh           # run all checks
#   ./scripts/ci-local.sh --rust    # run only Rust checks (fmt + clippy + test)
#   ./scripts/ci-local.sh --i18n    # run only i18n audit
#   ./scripts/ci-local.sh --lint    # run only Android lint (requires SDK)
#   ./scripts/ci-local.sh --help    # show help
#
# The script exits non-zero if any check fails. Each section prints
# a clear header and result. Rust checks don't require the Android
# SDK; the --lint check does (and is skipped if SDK is missing).
#
# Mirrors what CI runs in .github/workflows/kr64-tests.yml
# (fmt + clippy + test for the kr64 crate) and .github/workflows/build.yml
# (Android lint + assembleRelease). The Android assembleRelease step is
# NOT included here — it produces a 9 MB APK and takes ~10 min, so it's
# left to CI. The `lint` task transitively compiles native code, so it
# serves as a local smoke test for the chosen ABIs.

set -u
# NOTE: we deliberately do NOT set -e — each check is run in a subshell
# and its pass/fail status captured in the summary table. The script
# exits non-zero at the very end if any check failed.

# ---------------------------------------------------------------------------
# Resolve the repo root from the script's location. The script lives at
# $REPO_ROOT/scripts/ci-local.sh, so the parent of the script's directory
# is the repo root. Resolving via `readlink -f` (Linux) or a manual loop
# (macOS) keeps it working when invoked via a relative or symlinked path.
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ---------------------------------------------------------------------------
# Colours. Use tput if available so this works on dumb terminals; fall
# back to plain text otherwise. All ANSI escapes are gated on
# `[[ -t 1 ]]` so logs piped to a file stay clean.
# ---------------------------------------------------------------------------
if [[ -t 1 ]] && command -v tput >/dev/null 2>&1; then
    C_GREEN="$(tput setaf 2)"
    C_RED="$(tput setaf 1)"
    C_YELLOW="$(tput setaf 3)"
    C_BLUE="$(tput setaf 4)"
    C_BOLD="$(tput bold)"
    C_RESET="$(tput sgr0)"
else
    C_GREEN="" C_RED="" C_YELLOW="" C_BLUE="" C_BOLD="" C_RESET=""
fi

# ---------------------------------------------------------------------------
# Result accumulator. Each entry is "NAME|STATUS" where STATUS is one of
# PASS, FAIL, SKIP, WARN. The summary table at the end iterates this list.
# ---------------------------------------------------------------------------
declare -a RESULTS=()

add_result() {
    # add_result <name> <status>
    RESULTS+=("$1|$2")
}

# ---------------------------------------------------------------------------
# Section header / footer helpers. Each check prints a banner before it
# runs and a single-line [PASS]/[FAIL]/[SKIP] verdict after.
# ---------------------------------------------------------------------------
section_start() {
    # section_start <title>
    echo
    echo "${C_BOLD}${C_BLUE}=== $1 ===${C_RESET}"
}

print_pass() {
    echo "${C_GREEN}[PASS]${C_RESET} $1"
}
print_fail() {
    echo "${C_RED}[FAIL]${C_RESET} $1"
}
print_skip() {
    echo "${C_YELLOW}[SKIP]${C_RESET} $1"
}
print_warn() {
    echo "${C_YELLOW}[WARN]${C_RESET} $1"
}

# ---------------------------------------------------------------------------
# run_check — run a command in a subshell, capture its exit code, and
# print the right [PASS]/[FAIL] line. Doesn't propagate the exit code
# (we exit non-zero once at the very end based on the RESULTS array).
#
# Usage: run_check <name> <command...>
# ---------------------------------------------------------------------------
run_check() {
    local name="$1"; shift
    if "$@"; then
        print_pass "$name"
        add_result "$name" PASS
        return 0
    else
        local rc=$?
        print_fail "$name (exit $rc)"
        add_result "$name" FAIL
        return 0  # don't abort the script
    fi
}

# ---------------------------------------------------------------------------
# Tooling availability probes.
# ---------------------------------------------------------------------------
have_cargo() {
    command -v cargo >/dev/null 2>&1
}
have_gradlew() {
    [[ -x "$REPO_ROOT/gradlew" ]]
}
have_python3() {
    command -v python3 >/dev/null 2>&1
}
have_git() {
    command -v git >/dev/null 2>&1
}
have_android_sdk() {
    # The Android lint task needs both ANDROID_HOME (the SDK root) and
    # ANDROID_NDK_HOME (the NDK root) — same env that build.yml sets via
    # nttld/setup-ndk's `ndk-path` output. If either is missing, the
    # gradle `lint` task fails at the cmakeBuild/loaderBuild native
    # steps. Skip the lint check with a warning rather than fail.
    [[ -n "${ANDROID_HOME:-}" && -n "${ANDROID_NDK_HOME:-}" ]]
}

# ---------------------------------------------------------------------------
# Which check groups to run. Defaults to "all"; --rust / --i18n / --lint
# restrict it. Multiple flags may be passed and are unioned.
# ---------------------------------------------------------------------------
RUN_RUST=0
RUN_I18N=0
RUN_LINT=0
RUN_MISC=0   # cmake-cache check + branch check; always part of "all"
ANY_FILTER=0

show_help() {
    cat <<EOF
ci-local.sh — run the same quality checks CI runs, locally.

Usage: ./scripts/ci-local.sh [options]

Options:
  --rust    Run only Rust checks (fmt + clippy + test on kr64, loader, twoyi)
  --i18n    Run only the i18n audit (find_missing_translations.py)
  --lint    Run only Android lint (requires ANDROID_HOME + ANDROID_NDK_HOME)
  --help    Show this help message

With no options, runs every check (Rust, i18n, lint, plus the
misc sanity checks: stale CMake cache + branch check).

Exits non-zero if any non-skipped check failed. Checks that are
skipped (e.g. lint without an SDK, Rust without cargo) do NOT
count as failures.

Environment:
  ANDROID_HOME      Path to the Android SDK (required for --lint)
  ANDROID_NDK_HOME  Path to the Android NDK (required for --lint)
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --rust)
            RUN_RUST=1; ANY_FILTER=1; shift ;;
        --i18n)
            RUN_I18N=1; ANY_FILTER=1; shift ;;
        --lint)
            RUN_LINT=1; ANY_FILTER=1; shift ;;
        --help|-h)
            show_help; exit 0 ;;
        *)
            echo "Unknown option: $1" >&2
            show_help >&2
            exit 2 ;;
    esac
done

# If no --filter flag was passed, run everything.
if [[ "$ANY_FILTER" -eq 0 ]]; then
    RUN_RUST=1
    RUN_I18N=1
    RUN_LINT=1
    RUN_MISC=1
else
    # The misc sanity checks (CMake cache + branch check) are cheap and
    # always useful, so include them in any filtered run too.
    RUN_MISC=1
fi

# ---------------------------------------------------------------------------
# Pre-flight: print what's available so the user understands any SKIPs.
# ---------------------------------------------------------------------------
section_start "Environment"
echo "repo root:        $REPO_ROOT"
echo "cargo available:  $(have_cargo  && echo yes || echo no)"
echo "python3 available:$(have_python3 && echo yes || echo no)"
echo "gradlew available:$(have_gradlew && echo yes || echo no)"
echo "ANDROID_HOME set: $([[ -n "${ANDROID_HOME:-}" ]] && echo yes || echo no)"
echo "ANDROID_NDK_HOME set: $([[ -n "${ANDROID_NDK_HOME:-}" ]] && echo yes || echo no)"

if ! have_cargo && [[ "$RUN_RUST" -eq 1 ]]; then
    print_warn "cargo not on PATH — Rust checks will be SKIPPED. Install Rust via https://rustup.rs/ to enable them."
fi
if ! have_android_sdk && [[ "$RUN_LINT" -eq 1 ]]; then
    print_warn "ANDROID_HOME / ANDROID_NDK_HOME not set — Android lint will be SKIPPED."
fi
if ! have_python3 && [[ "$RUN_I18N" -eq 1 ]]; then
    print_warn "python3 not on PATH — i18n audit will be SKIPPED."
fi

# Track whether we couldn't run a requested check because the tool is
# missing. These count as SKIP, not FAIL.
SKIPPED_DUE_TO_MISSING_TOOL=0

# ===========================================================================
# Rust checks
# ===========================================================================
if [[ "$RUN_RUST" -eq 1 ]]; then
    if ! have_cargo; then
        section_start "Rust checks (skipped — cargo not available)"
        add_result "rust fmt (kr64)"     SKIP
        add_result "rust fmt (loader)"   SKIP
        add_result "rust fmt (twoyi)"    SKIP
        add_result "rust clippy (kr64)"  SKIP
        add_result "rust clippy (loader)" SKIP
        add_result "rust clippy (twoyi)" SKIP
        add_result "rust test (kr64)"    SKIP
        add_result "rust test (loader)"  SKIP
        SKIPPED_DUE_TO_MISSING_TOOL=1
    else
        section_start "Rust checks (fmt + clippy + test)"

        # kr64 — covered by .github/workflows/kr64-tests.yml on CI.
        section_start "Rust: kr64 crate"
        run_check "rust fmt (kr64)"    bash -c "cd '$REPO_ROOT/app/rs/kr64' && cargo fmt --check"
        run_check "rust clippy (kr64)" bash -c "cd '$REPO_ROOT/app/rs/kr64' && cargo clippy --all-targets -- -D warnings"
        run_check "rust test (kr64)"   bash -c "cd '$REPO_ROOT/app/rs/kr64' && cargo test --no-fail-fast"

        # loader — not currently in any CI workflow but should stay clean
        # (it's the open-source replacement for the legacy libloader.so).
        if [[ -f "$REPO_ROOT/app/rs/loader/Cargo.toml" ]]; then
            section_start "Rust: loader crate"
            run_check "rust fmt (loader)"    bash -c "cd '$REPO_ROOT/app/rs/loader' && cargo fmt --check"
            run_check "rust clippy (loader)" bash -c "cd '$REPO_ROOT/app/rs/loader' && cargo clippy --all-targets -- -D warnings"
            run_check "rust test (loader)"   bash -c "cd '$REPO_ROOT/app/rs/loader' && cargo test --no-fail-fast"
        else
            print_skip "loader crate (Cargo.toml not found at app/rs/loader/)"
            add_result "rust fmt (loader)"    SKIP
            add_result "rust clippy (loader)" SKIP
            add_result "rust test (loader)"   SKIP
        fi

        # twoyi — the parent crate at app/rs/. Its build.rs compiles
        # interp.c via cc, so it needs a C toolchain on the host; the
        # --all-targets flag would also build the cdylib, which needs
        # Android NDK cross-linker flags (we don't have those locally).
        # So we lint only the lib + tests, not --all-targets. fmt --check
        # is still safe to run as-is.
        if [[ -f "$REPO_ROOT/app/rs/Cargo.toml" ]]; then
            section_start "Rust: twoyi crate (app/rs/)"
            run_check "rust fmt (twoyi)"    bash -c "cd '$REPO_ROOT/app/rs' && cargo fmt --check"
            run_check "rust clippy (twoyi)" bash -c "cd '$REPO_ROOT/app/rs' && cargo clippy --all-targets -- -D warnings"
            # NOTE: we intentionally do NOT run `cargo test` for the
            # twoyi crate — its tests need the Android NDK and the
            # cdylib/PIE linker flags configured in app/rs/.cargo/config.toml,
            # which only work when targeting aarch64-linux-android /
            # x86_64-linux-android. CI's kr64-tests.yml only runs tests
            # for the kr64 crate for the same reason.
        else
            print_skip "twoyi crate (Cargo.toml not found at app/rs/)"
            add_result "rust fmt (twoyi)"    SKIP
            add_result "rust clippy (twoyi)" SKIP
        fi
    fi
fi

# ===========================================================================
# i18n audit
# ===========================================================================
if [[ "$RUN_I18N" -eq 1 ]]; then
    if ! have_python3; then
        section_start "i18n audit (skipped — python3 not available)"
        add_result "i18n audit" SKIP
        SKIPPED_DUE_TO_MISSING_TOOL=1
    elif [[ ! -f "$REPO_ROOT/scripts/find_missing_translations.py" ]]; then
        section_start "i18n audit (skipped — script missing)"
        print_skip "scripts/find_missing_translations.py not found"
        add_result "i18n audit" SKIP
        SKIPPED_DUE_TO_MISSING_TOOL=1
    else
        section_start "i18n audit (find_missing_translations.py)"
        # The script exits non-zero if any translatable string in the
        # default values/strings.xml is missing from any of the three
        # locale files (zh-rCN, zh-rTW, ja). It also reports orphan
        # translations but those don't fail the script.
        run_check "i18n audit" python3 "$REPO_ROOT/scripts/find_missing_translations.py"
    fi
fi

# ===========================================================================
# Android lint
# ===========================================================================
if [[ "$RUN_LINT" -eq 1 ]]; then
    if ! have_gradlew; then
        section_start "Android lint (skipped — gradlew not found)"
        add_result "android lint" SKIP
        SKIPPED_DUE_TO_MISSING_TOOL=1
    elif ! have_android_sdk; then
        section_start "Android lint (skipped — SDK not configured)"
        print_skip "ANDROID_HOME / ANDROID_NDK_HOME not set"
        print_warn "Set both env vars to your local SDK + NDK paths to enable this check."
        add_result "android lint" SKIP
        SKIPPED_DUE_TO_MISSING_TOOL=1
    else
        section_start "Android lint (./gradlew lint -Pabis=all)"
        # Same command CI runs in build.yml. The lint task transitively
        # depends on javaPreCompileRelease + cargoBuild + cmakeBuild +
        # loaderBuild (wired in app/build.gradle), so it doubles as a
        # native-compile smoke test. -Pabis=all matches CI.
        run_check "android lint" bash -c "cd '$REPO_ROOT' && ./gradlew lint -Pabis=all"
    fi
fi

# ===========================================================================
# Misc sanity checks (always run; they're cheap and need no toolchain)
# ===========================================================================
if [[ "$RUN_MISC" -eq 1 ]]; then
    section_start "Misc sanity checks"

    # 1) Stale CMake cache — app/cpp/build/ is gitignored (see .gitignore
    # line 27). If any files there are tracked, the build will fail on
    # any runner that clones to a different absolute path (this was the
    # root cause of CI bug #2 — see worklog round 60 / commit cd6d0d8).
    if have_git; then
        section_start "Stale CMake cache check (app/cpp/build/)"
        if git -C "$REPO_ROOT" ls-files app/cpp/build/ | grep -q .; then
            print_fail "app/cpp/build/ contains tracked files (should be gitignored)"
            add_result "stale cmake cache" FAIL
        else
            print_pass "app/cpp/build/ is clean (no tracked files)"
            add_result "stale cmake cache" PASS
        fi
    else
        print_skip "git not available — cmake cache check skipped"
        add_result "stale cmake cache" SKIP
    fi

    # 2) Branch check — only `main` exists on origin now (the historical
    # improvements/initial-cleanup branch was merged in and deleted on
    # 2026-08-08). Sub-agents might land work on the wrong branch if
    # they don't notice.
    if have_git; then
        section_start "Branch check"
        BRANCH="$(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
        if [[ "$BRANCH" == "main" ]]; then
            print_pass "on main branch"
            add_result "branch check" PASS
        else
            print_warn "not on main (on $BRANCH) — sub-agents might land work on the wrong branch"
            # WARN doesn't fail the build, but it shows up in the
            # summary table so it's visible. The repo historically had
            # work accidentally land on improvements/initial-cleanup
            # instead of main, so this is worth flagging loudly.
            add_result "branch check" WARN
        fi
    else
        print_skip "git not available — branch check skipped"
        add_result "branch check" SKIP
    fi
fi

# ===========================================================================
# Summary table
# ===========================================================================
section_start "Summary"

# Compute column widths for a tidy table.
NAME_WIDTH=4   # "Name"
STAT_WIDTH=6   # "Status"
for entry in "${RESULTS[@]}"; do
    name="${entry%%|*}"
    if [[ ${#name} -gt $NAME_WIDTH ]]; then
        NAME_WIDTH=${#name}
    fi
done

# Header
printf "+-%-*s-+-%s-+\n" "$NAME_WIDTH" "$(printf '%*s' "$NAME_WIDTH" '')" "$(printf '%*s' "$STAT_WIDTH" '')"
printf "| ${C_BOLD}%-*s${C_RESET} | ${C_BOLD}%-*s${C_RESET} |\n" "$NAME_WIDTH" "Name" "$STAT_WIDTH" "Status"
printf "+-%-*s-+-%s-+\n" "$NAME_WIDTH" "$(printf '%*s' "$NAME_WIDTH" '')" "$(printf '%*s' "$STAT_WIDTH" '')"

# Rows
ANY_FAIL=0
ANY_PASS=0
ANY_SKIP=0
ANY_WARN=0
for entry in "${RESULTS[@]}"; do
    name="${entry%%|*}"
    status="${entry##*|}"
    case "$status" in
        PASS) color="$C_GREEN" ;;
        FAIL) color="$C_RED";   ANY_FAIL=1 ;;
        SKIP) color="$C_YELLOW"; ANY_SKIP=1 ;;
        WARN) color="$C_YELLOW"; ANY_WARN=1 ;;
        *)    color="" ;;
    esac
    printf "| %-*s | ${color}%-*s${C_RESET} |\n" "$NAME_WIDTH" "$name" "$STAT_WIDTH" "$status"
done

# Footer
printf "+-%-*s-+-%s-+\n" "$NAME_WIDTH" "$(printf '%*s' "$NAME_WIDTH" '')" "$(printf '%*s' "$STAT_WIDTH" '')"

# Counts
TOTAL=${#RESULTS[@]}
PASS_COUNT=$(printf '%s\n' "${RESULTS[@]}" | grep -c '|PASS$' || true)
FAIL_COUNT=$(printf '%s\n' "${RESULTS[@]}" | grep -c '|FAIL$' || true)
SKIP_COUNT=$(printf '%s\n' "${RESULTS[@]}" | grep -c '|SKIP$' || true)
WARN_COUNT=$(printf '%s\n' "${RESULTS[@]}" | grep -c '|WARN$' || true)

echo
echo "Total: $TOTAL — $PASS_COUNT pass, $FAIL_COUNT fail, $SKIP_COUNT skip, $WARN_COUNT warn"

# Exit code: non-zero if any non-skipped check failed. SKIPs (e.g. lint
# without an SDK) do NOT fail the script — that would make this useless
# on dev machines without the Android SDK installed.
if [[ "$ANY_FAIL" -eq 1 ]]; then
    echo
    echo "${C_RED}${C_BOLD}FAILED${C_RESET} — at least one check failed. Fix the issues above before pushing."
    exit 1
fi

if [[ "$SKIPPED_DUE_TO_MISSING_TOOL" -eq 1 ]]; then
    echo
    echo "${C_YELLOW}PASSED (with skips)${C_RESET} — all runnable checks passed, but some were skipped because required tools are missing."
    exit 0
fi

echo
echo "${C_GREEN}${C_BOLD}PASSED${C_RESET} — all checks green. Safe to push."
exit 0
