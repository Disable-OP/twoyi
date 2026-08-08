# Stale Claims Audit — Disable-OP/twoyi

> **Audit date:** 2026-08-08 (commit `99c940e`)
> **Auditor:** Task Agent 3 (general-purpose sub-agent)
> **Scope:** every `*.md` file in the repo root + `download/` directory
> **Purpose:** Find every claim that contradicts the current verified
> state, so a follow-up doc-cleanup task can fix them in bulk.

---

## Verified current state (as of commit 99c940e)

| Aspect                       | Verified value                                       |
| ---------------------------- | ---------------------------------------------------- |
| Branch on origin             | **`main` only** (the historical `improvements/initial-cleanup` branch was merged in and deleted from origin on 2026-08-08) |
| Total commits on `main`      | **411**                                              |
| CI workflows                 | **both green** — `Build APK` + `kr64 lint + test` (first fully-green Build APK run since round 60 happened on commit `9e3a1fb`) |
| kr64 unit tests              | **165/165 passing**                                  |
| clippy warnings              | **0** (all 3 crates: kr64, loader, twoyi)            |
| `cargo fmt --check` drift    | **0** (all 3 crates)                                 |
| Android lint                 | **0 errors, 0 warnings** (the historical "62 warnings" were resolved in rounds 23–52) |
| i18n coverage                | **4 locales** (en, zh-CN, zh-TW, ja), **0 missing** translations |
| kr64 LOC                     | **~11,554** across **11** `.rs` files (not 9,581 / 10) |
| `worklog.md` length          | **~4,847 lines** (not 879)                           |
| Container end-to-end boot    | **NEVER achieved** — the `qemu_pipe` accept thread in `app/rs/kr64/src/devices.rs` is still the MVP single-byte-echo stub. The real GL command dispatcher is unimplemented. See `HONEST_STATUS_CORRECTED.md` |
| `setup-ndk` action pin       | `nttld/setup-ndk@v1` (the `@v2` tag never existed — see commit `f166b20`) |

---

## Methodology

Every `*.md` file in the repo root and in `download/` was searched with
ripgrep for the patterns enumerated in the task spec. Each match was
inspected in context and classified by severity:

- **Critical** — claims that are factually wrong AND actively misleading
  (e.g. calling the project "production-ready" when the container has
  never booted, or claiming CI was green when it was broken).
- **Moderate** — stale numbers, branch names, or commit counts that
  contradict the verified current state. These mislead a new reader
  but don't usually change the project's risk profile.
- **Minor** — historical references inside `worklog.md` (a chronological
  log, where old entries are accurate-at-the-time) or cosmetic issues
  that are technically wrong but unlikely to mislead anyone.

The audit also distinguishes between **currently-stale** claims (in
docs that present themselves as describing the present state — e.g.
`MEMORY.md`, `FINAL_STATUS.md`, `README.md`) and **historically-accurate**
entries (in `worklog.md` or in commit-message-style sections explicitly
labelled "as of round N"). The latter are flagged as Minor.

---

## Severity: CRITICAL

These claims are factually wrong AND would actively mislead a new
reader. They should be fixed first.

| File | Line | Stale Claim | Suggested Fix |
|------|------|-------------|---------------|
| `MEMORY.md` | 5 | `> **Branch:** \`improvements/initial-cleanup\` (active development branch)` | Change to `> **Branch:** \`main\` (single consolidated branch — \`improvements/initial-cleanup\` was merged in and deleted on 2026-08-08)` |
| `MEMORY.md` | 14 | `> production-ready and all quality gates are green.` | Replace "production-ready" with "build-gates-green (NOTE: container has never booted end-to-end — see HONEST_STATUS_CORRECTED.md)". The "all quality gates are green" claim is true as of round 68 but was broken for rounds 60–67. |
| `MEMORY.md` | 30 | `\| Codebase state \| **Production-ready** \|` | Replace with `Codebase state: build-ready (CI green, 0 warnings, 0 fmt drift, 165/165 tests). Container does NOT boot end-to-end yet.` |
| `MEMORY.md` | 77 | `### Quality gates (all green)` heading | Add a date stamp: `### Quality gates (all green — verified 2026-08-08 commit 99c940e)` |
| `MEMORY.md` | 87 | `\| Emulator boot \| TCG-only, no KVM \| **Android 9 boots end-to-end** \|` | Clarify this is the EMULATOR (not twoyi's container) booting Android 9 under TCG. The twoyi container itself has NEVER booted end-to-end — see HONEST_STATUS_CORRECTED.md. Suggested reword: `Emulator (Android 9 API 28, TCG mode) boots; twoyi container does NOT` |
| `MEMORY.md` | 91 | `**Production-ready.** After **66 rounds of improvements** (~245 individual` | Remove "Production-ready" — the container has never booted end-to-end. Replace with `**Build-ready.** After 67 rounds of improvements (~245 individual changes), all build quality gates are green; the container itself has not yet booted (see HONEST_STATUS_CORRECTED.md).` |
| `MEMORY.md` | 92 | `changes shipped across 90+ commits), all quality gates are green, all 4` | The "all quality gates are green" claim was false for rounds 60–67 (CI was broken); it became true again at round 68. Add date stamp or qualify with "(verified 2026-08-08)". |
| `MEMORY.md` | 143 | `## 0. Final Session Statistics (production-ready)` | Remove "production-ready" from heading. |
| `MEMORY.md` | 161 | `\| Codebase state \| **Production-ready** \|` | Same fix as line 30. |
| `MEMORY.md` | 216 | `**Codebase state:** production-ready, fully internationalized, CI-gated.` | Remove "production-ready"; keep "fully internationalized, CI-gated". |
| `MEMORY.md` | 922 | `## 15. Final Cleanup Pass (2026-08-06 — production-ready sign-off)` | The "production-ready sign-off" is the overclaim that HONEST_STATUS_CORRECTED.md (written 2026-08-05) explicitly retracts. Replace "production-ready sign-off" with "build-clean sign-off (container boot NOT achieved)". |
| `MEMORY.md` | 944 | `**Codebase state:** production-ready.` | Same fix as line 30. |
| `ONE_PAGE_SUMMARY.md` | 15 | `HAL surface a 64-bit Android guest expects; 8 modules, **154 tests passing**.` | Change "154 tests" → "165 tests". The kr64 suite is now 165/165. |
| `ONE_PAGE_SUMMARY.md` | 34 | `\| 154 unit tests, 0 failing; CI green \| Pipe write fails: \`EINVAL\` (os error 22) \|` | Stale test count (154 → 165). Also: "CI green" was broken rounds 60–67; the file has no date so the claim is untrustworthy. Add `(verified 2026-08-08)` and update to 165 tests. |
| `FINAL_STATUS.md` | 23 | `\| \`kr64\` daemon \| ~9,581 LOC, **154 tests** passing, 8 feature modules \|` | Stale on both counts — actual is ~11,554 LOC across 11 files with 165 tests. |
| `HONEST_STATUS_CORRECTED.md` | (whole file) | (no stale claim — this file IS the retraction; mentioned here as the authoritative source) | — |
| `download/ONE_PAGE_SUMMARY.md` | 15 | (mirror of the above) | Same fix. |
| `download/ONE_PAGE_SUMMARY.md` | 34 | (mirror of the above) | Same fix. |
| `download/FINAL_STATUS.md` | 23 | (mirror of the above) | Same fix. |

**Total Critical: 18 claims across 4 distinct files (3 in repo root + 1 mirrored in `download/`).**

---

## Severity: MODERATE

Stale numbers, branch references, or commit counts that contradict the
verified current state. Mislead a new reader but don't change the
project's risk profile.

### Stale branch references (`improvements/initial-cleanup` → `main`)

The branch was merged in and deleted from origin on 2026-08-08. Every
doc that still calls it the "active development branch" or instructs
contributors to `git checkout improvements/initial-cleanup` is stale.

| File | Line(s) | Stale Claim | Suggested Fix |
|------|---------|-------------|---------------|
| `MIGRATION_GUIDE.md` | 7, 70, 74, 357, 364 | "active dev branch", "git checkout improvements/initial-cleanup", "All improvements are on `improvements/initial-cleanup` until merged", "Rebuild from `improvements/initial-cleanup`", "Pull latest `improvements/initial-cleanup`" | Replace every reference with `main`. The branch was consolidated on 2026-08-08. |
| `RELENG.md` | 4, 113 | "Active dev branch: `improvements/initial-cleanup`", "Push a revert commit on `improvements/initial-cleanup`" | Change to `main`. |
| `CONTRIBUTOR_LADDER.md` | 6, 18, 25, 58, 62, 68 | Multiple references to `improvements/initial-cleanup` as the active / merge-target branch | Change all to `main`. |
| `TESTING_GUIDE.md` | 300 | "open a PR against `improvements/initial-cleanup`" | Change to `main`. |
| `MORNING_MESSAGE.md` | 24 | "`improvements/initial-cleanup` has the dev work." | Change to "`main` has the dev work." |
| `DEVELOPMENT_ROADMAP.md` | 7, 674, 767 | "Branch: `improvements/initial-cleanup` (207 commits, 15 since `main`)", "Pull requests against `Disable-OP/twoyi:improvements/initial-cleanup`", "open a pull request against `improvements/initial-cleanup`" | Change branch to `main`; update commit count (see below). |
| `CONTRIBUTING.md` | 7, 25, 27, 37, 58, 245 | Multiple "branch from `improvements/initial-cleanup`" / "Create codespace on improvements/initial-cleanup" instructions | Change all to `main`. |
| `CREDITS.md` | 28 | "improvements are being made (branch `improvements/initial-cleanup`)" | Change to `main`. |
| `QUICK_START.md` | 6, 18, 111 | "Active development happens on `improvements/initial-cleanup`", "git checkout improvements/initial-cleanup", "Branch from `improvements/initial-cleanup`" | Change all to `main`. |
| `SESSION_SUMMARY.md` | 6, 64, 70, 308, 309, 377, 379 | Multiple references — branch is `improvements/initial-cleanup` at `2e7632d`, 29 commits since `main`, `git log --oneline main..improvements/initial-cleanup` | This doc is a historical snapshot — either add a "Historical — see current state in MEMORY.md" header at the top, or update all branch refs to `main` and remove "29 commits since `main`". |
| `HANDOFF.md` | 4, 23, 51 | "got CI green" (historical — accurate at the time), "Dev branch `origin/improvements/initial-cleanup` (43 commits ahead of base `25ef89c`)", "git checkout improvements/initial-cleanup && git pull" | Change branch to `main`; remove the "43 commits ahead of base 25ef89c" claim (it's now ~411 commits on main). |
| `FINAL_STATUS.md` | 84 | "Branch: `main` (the `improvements/initial-cleanup` branch has been merged)" | Accurate as written — no fix needed. (Listed here for completeness.) |
| `ARCHITECTURE_DECISIONS.md` | 4 | "Fork: `cyanmint/twoyi` branch `improvements/initial-cleanup`" | Change to "Fork: `Disable-OP/twoyi` branch `main` (was `cyanmint/twoyi` branch `improvements/initial-cleanup`, consolidated 2026-08-08)". |
| `ARCHITECTURE.md` | 6, 11, 870 | "this branch (`improvements/initial-cleanup`)", "remaining fixes on `improvements/initial-cleanup`", "cyanmint/twoyi (+ improvements/initial-cleanup branch)" | Change branch references to `main`; keep historical context that the branch was consolidated. |
| `FAQ.md` | 5, 67, 154 | "`improvements/initial-cleanup` branch", "fork lives on the `improvements/initial-cleanup` branch of", "Branch from `improvements/initial-cleanup`" | Change all to `main`. |
| `CHANGELOG.md` | 4 | "`improvements/initial-cleanup` branch" | Change to `main`. |
| `PROJECT_HEALTH.md` | 5, 76 | "Branch: `improvements/initial-cleanup` (HEAD `ca33d02`, 37 commits past fork point)", "the dev branch — all work lives on `improvements/initial-cleanup`" | Update branch to `main` and remove the stale HEAD/commit-count reference. |
| `PROJECT_SUMMARY.md` | 6, 8, 51, 919, 922, 968 | Multiple "Branch analyzed: `improvements/initial-cleanup` (207 commits)" + "git log --oneline improvements/initial-cleanup" | Either add "Historical — see MEMORY.md for current state" header OR update all branch refs to `main` and update commit count. |
| `disasm/FINAL_REPORT.md` | 5 | "Branch: `improvements/initial-cleanup` on `Disable-OP/twoyi`" | Change to `main`. |
| `download/X86_64_ROOTFS_BUILD_GUIDE.md` | 7, 146 | "Branch: `improvements/initial-cleanup`", "git checkout improvements/initial-cleanup" | Change to `main`. |
| `download/CONTRIBUTOR_LADDER.md` | 6, 18, 25, 58, 62, 68 | (mirror of root CONTRIBUTOR_LADDER.md) | Same fix. |
| `download/TESTING_GUIDE.md` | 300 | (mirror) | Same fix. |
| `download/MORNING_MESSAGE.md` | 24 | (mirror) | Same fix. |
| `download/DEVELOPMENT_ROADMAP.md` | 7, 674, 767 | (mirror) | Same fix. |
| `download/QUICK_START.md` | 6, 18, 111 | (mirror) | Same fix. |
| `download/SESSION_SUMMARY.md` | 6, 64, 70, 308, 309, 377, 379 | (mirror) | Same fix. |
| `download/FINAL_STATUS.md` | 84 | (mirror — already accurate) | — |
| `download/ARCHITECTURE_DECISIONS.md` | 4 | (mirror) | Same fix. |
| `download/FAQ.md` | 5, 67, 154 | (mirror) | Same fix. |
| `download/RELENG.md` | 4, 113 | (mirror) | Same fix. |
| `download/MIGRATION_GUIDE.md` | 7, 70, 74, 357, 364 | (mirror) | Same fix. |
| `download/CONTRIBUTING.md` | (download/ has no CONTRIBUTING.md) | — | — |
| `download/CREDITS.md` | (download/ has no CREDITS.md) | — | — |
| `download/CHANGELOG.md` | (download/ has no CHANGELOG.md) | — | — |
| `download/PROJECT_HEALTH.md` | 5, 76 | (mirror) | Same fix. |
| `download/PROJECT_SUMMARY.md` | 6, 8, 51, 919, 922, 968 | (mirror) | Same fix. |
| `download/HANDOFF.md` | 4, 23, 51 | (mirror) | Same fix. |
| `download/FINAL_VERIFICATION.md` | 4, 10, 11 | "Branch: `main` (verification target: `improvements/initial-cleanup`)", "Commits on `improvements/initial-cleanup`", "git log --oneline improvements/initial-cleanup --not 25ef89c \| wc -l" | Update — the verification target is now `main`; the `improvements/initial-cleanup` branch no longer exists. |
| `download/VERIFICATION.md` | 9, 15, 16, 21, 67, 68, 73, 77 | Multiple "Both GitHub Actions workflows on `improvements/initial-cleanup`", "kr64 unit tests \| improvements/initial-cleanup \| ca33d029", "Build APK \| improvements/initial-cleanup \| ca33d029", "git log --oneline improvements/initial-cleanup --not 25ef89c \| wc -l", "improvements/initial-cleanup local HEAD ca33d02 matches origin/improvements/initial-cleanup", "all real work lives on `improvements/initial-cleanup`, pushed & green", "Everything is committed on `improvements/initial-cleanup` and pushed to `origin`" | This doc is a verification snapshot from a specific date — either add a "Historical — superseded by round-68 verification" header OR update all branch refs to `main` (with the round-68 commit hash). |
| `download/FINAL_COMMIT_LOG.md` | 3 | "Branch: `improvements/initial-cleanup`" | Change to `main`. |
| `download/TWOYI_FINAL_REPORT.md` | 5 | "Branch: `improvements/initial-cleanup` on `Disable-OP/twoyi`" | Change to `main`. |
| `download/TWOYI_HONEST_STATUS.md` | 50 | "Commit: `7664c66` on `improvements/initial-cleanup`" | The commit `7664c66` is on `main` now (the branch was merged). Update branch ref. |

### Stale commit counts

| File | Line | Stale Claim | Suggested Fix |
|------|------|-------------|---------------|
| `MEMORY.md` | 20 | `\| Commits on \`improvements/initial-cleanup\` \| **79+** (372 total across all branches) \|` | Update to `Commits on main: 411 (the \`improvements/initial-cleanup\` branch was merged and deleted on 2026-08-08)`. |
| `MEMORY.md` | 147 | `\| Total commits pushed \| **62+** on \`improvements/initial-cleanup\` (368 total) \|` | Update to `Total commits on main: 411`. |
| `MEMORY.md` | 949 | `- **main**: 399 commits (merged from improvements/initial-cleanup + VM analysis + VM-inspired code)` | Update to `main: 411 commits (current as of 2026-08-08)`. |
| `MEMORY.md` | 950 | `- **improvements/initial-cleanup**: 394 commits (fully merged into main)` | Either delete this line (the branch no longer exists) or reword as historical: "`improvements/initial-cleanup` (deleted 2026-08-08; its 394 commits were merged into main)". |
| `README.md` | 82 | `The full commit history is preserved (386 commits at last count):` | Update "386 commits" to "411 commits". |
| `DEVELOPMENT_ROADMAP.md` | 7 | `> **Branch:** \`improvements/initial-cleanup\` (207 commits, 15 since \`main\`)` | Update branch to `main` and commit count to 411. |
| `PROJECT_SUMMARY.md` | 6, 8, 51, 117, 919, 968 | "207 commits" (multiple places) | Update to 411 OR add "Historical — count as of 2026-08-05" header. |
| `download/DEVELOPMENT_ROADMAP.md` | 7 | (mirror) | Same fix. |
| `download/PROJECT_SUMMARY.md` | 6, 8, 51, 117, 919, 968 | (mirror) | Same fix. |
| `download/VERIFICATION.md` | 21 | `git log --oneline improvements/initial-cleanup --not 25ef89c \| wc -l` returns 37 | Historical snapshot — add date header. |
| `download/FINAL_VERIFICATION.md` | 11 | `git log --oneline improvements/initial-cleanup --not 25ef89c \| wc -l` returns 43 | Historical snapshot — add date header. |

### Stale test counts

The current count is **165/165**. Older docs cite 144 / 145 / 154.

| File | Line | Stale Claim | Suggested Fix |
|------|------|-------------|---------------|
| `TESTING_GUIDE.md` | 21 | `**144 tests across 8 submodules plus the lib root** run on` | Update to 165 tests. |
| `QUICK_START.md` | 70 | `(HAL shims). ~9,500 LOC, 144 tests.` | Update LOC to ~11,554 and tests to 165. |
| `SESSION_SUMMARY.md` | 19 | `\| \`kr64\` daemon \| 9,581 lines, 144 tests, 8 feature modules \|` | Update to ~11,554 LOC, 165 tests, 8 modules. |
| `SESSION_SUMMARY.md` | 31 | `\`cargo test --lib\` is green (144 tests, 0 failures)` | Update to 165 tests. |
| `SESSION_SUMMARY.md` | 66 | `Brings kr64 to **9,581 lines / 144 tests across 8 feature modules**` | Historical round-21 entry — add "Historical" header OR update to current count. |
| `SESSION_SUMMARY.md` | 386 | `the kernel-replacement daemon exists (9,581 lines, 144 tests, 8 feature modules)` | Update to ~11,554 lines, 165 tests. |
| `TECHNICAL_BRIEFING.md` | 229 | `### 3.2 The kr64 daemon (Rust, 9,581 lines, 144 tests)` | Update to ~11,554 lines, 165 tests. |
| `TECHNICAL_BRIEFING.md` | 436 | `✅ kr64 daemon: 9,581 lines, 144 tests, 8 modules` | Update to ~11,554 lines, 165 tests. |
| `GLOSSARY.md` | 62 | `~9.6 kLOC, 144 tests` | Update to ~11.6 kLOC, 165 tests. |
| `FAQ.md` | 130 | `all 144 tests pass` | Update to 165 tests. |
| `PROJECT_HEALTH.md` | 88 | `CI green + 144 kr64 tests` | Update to 165 kr64 tests. |
| `download/TESTING_GUIDE.md` | 21 | (mirror) | Same fix. |
| `download/QUICK_START.md` | 70 | (mirror) | Same fix. |
| `download/SESSION_SUMMARY.md` | 19, 31, 66, 386 | (mirror) | Same fix. |
| `download/TECHNICAL_BRIEFING.md` | 229, 436 | (mirror) | Same fix. |
| `download/GLOSSARY.md` | 62 | (mirror) | Same fix. |
| `download/FAQ.md` | 130 | (mirror) | Same fix. |
| `download/PROJECT_HEALTH.md` | 88 | (mirror) | Same fix. |
| `download/VERIFICATION.md` | 43, 79 | "10 source files, **9,581 LOC**, **144 \`#[test]\` functions**" and "9,581 LOC of kr64 Rust / 10 files / 144 tests" | Update to 11 files, ~11,554 LOC, 165 tests. |
| `download/FINAL_VERIFICATION.md` | 19 | `find app/rs/kr64/src -name '*.rs' \| xargs wc -l → **9,581 LOC across 10 files**` | Update to ~11,554 LOC across 11 files. |

### Stale LOC counts for kr64

The current measurement is **~11,554 LOC across 11 `.rs` files** (was
claimed as 9,581 / 10). The grep results above already cover this; the
relevant files are `FINAL_STATUS.md`, `PROJECT_HEALTH.md`, `GLOSSARY.md`,
`TECHNICAL_BRIEFING.md`, `SESSION_SUMMARY.md`, `QUICK_START.md`,
`VERIFICATION.md`, `FINAL_VERIFICATION.md`.

### Stale "62 lint warnings" claims

The current lint count is **0 errors, 0 warnings**. The "62 warnings"
count was the round-23 pre-cleanup baseline; they were all resolved in
rounds 23–52.

| File | Line | Stale Claim | Suggested Fix |
|------|------|-------------|---------------|
| `MEMORY.md` | 116 | `still claimed "62 lint warnings" (the pre-round-52 state); MEMORY.md` | This is a *meta*-claim about a historical fix; it's accurate in context (it acknowledges the 62 count is stale). No fix needed — but the file should make clear this is historical. |
| (no other current docs claim "62 warnings" — `README.md` was fixed in commit `1a0cf91` and `build.yml`'s stale comment was fixed in `99c940e`.) | | | |

### Stale "CI green" / "all quality gates green" claims

These were false from rounds 60–67 (CI was broken by the
`setup-ndk@v2` bug + tracked CMake artifacts + missing string resources
+ missing translations). They became true again at round 68 (commit
`9e3a1fb`).

| File | Line | Stale Claim | Suggested Fix |
|------|------|-------------|---------------|
| `MEMORY.md` | 14, 92 | "all quality gates are green" (no date) | Add "(verified 2026-08-08 commit 99c940e)" — was broken rounds 60–67. |
| `MEMORY.md` | 77 | "Quality gates (all green)" heading (no date) | Add date stamp. |
| `MEMORY.md` | 979 | "Quality Gates (All Green)" heading (no date) | Add date stamp. |
| `PROJECT_HEALTH.md` | 10 | "code compiles, CI is green, docs are abundant" | Add date stamp. |
| `PROJECT_HEALTH.md` | 88 | "CI green + 144 kr64 tests" | Update test count to 165; add date stamp. |
| `HANDOFF.md` | 4 | "got CI green" | Historical — round-67-era claim, accurate at the time. Add date stamp or "historical" header. |
| `download/VERIFICATION.md` | 5 | "**Purpose:** Confirm CI is green and all overnight work is committed and pushed." | Historical snapshot — add "as of 2026-08-05" header. |
| `download/VERIFICATION.md` | 9 | "Both GitHub Actions workflows on `improvements/initial-cleanup` (HEAD `ca33d02`)" | Historical — but the branch name is also stale (now `main`). |
| `download/FINAL_VERIFICATION.md` | 45 | "CI is green where completed" | Add date stamp. |
| `download/PROJECT_HEALTH.md` | 10, 88 | (mirror) | Same fix. |
| `download/HANDOFF.md` | 4 | (mirror) | Same fix. |

### Stale `worklog.md` length claim

The file is currently ~4,847 lines; older docs claim 879.

| File | Line | Stale Claim | Suggested Fix |
|------|------|-------------|---------------|
| `PROJECT_SUMMARY.md` | 8, 903, 968 | "the full worklog (`worklog.md`, 879 lines)" / "879-line worklog" | Update to 4,847 lines. |
| `download/PROJECT_SUMMARY.md` | 8, 903, 968 | (mirror) | Same fix. |

### Misc stale commit/round counts

| File | Line | Stale Claim | Suggested Fix |
|------|------|-------------|---------------|
| `MEMORY.md` | 91 | "After **66 rounds of improvements** (~245 individual" | The most recent round is 67 (per round-67 commit `d7563e5`). Update to "67 rounds". |
| `MEMORY.md` | 97 | `### Round 66 — proc_emu \`/proc/self/status\` mode + doc sync (this commit)` | This is correctly labeled as round 66, but the file's "Last updated" header (line 3) says "round 67 — continued improvements" so it's accurate. No fix needed. |

---

## Severity: MINOR

Historical references inside `worklog.md` (a chronological log where
old entries are accurate-at-the-time) or cosmetic issues that are
technically wrong but unlikely to mislead.

### Historical entries in `worklog.md`

The worklog is explicitly a chronological log of work-as-it-happened.
Each entry was accurate when written; the values have since moved on.
These should NOT be edited — they're history. Listed here for
completeness so a future cleanup task doesn't waste time on them.

| File | Lines | Stale (at present) | Accurate-at-time |
|------|-------|--------------------|------------------|
| `worklog.md` | 903–904, 1082, 1090, 1111, 1147, 1200, 1216, 1452, 1456, 1467, 1470, 1534, 1773, 1778, 1789, 2042, 2057, 2097, 2270, 2385, 2419, 3124, 3320, 3343, 3350, 3360, 3370, 3387, 3396, 3483, 3537, 3566, 3685, 3767, 3774, 3788, 3833, 3844, 3908, 4044, 4049, 4114, 4227, 4230, 4256, 4383, 4570, 4580 | Multiple references to `improvements/initial-cleanup` as the dev branch, 207 commits, 9,581 LOC, 144 tests, 145 tests, 154 tests, "62 warnings" (in pre-round-52 context), "372 commits", "368 commits", "399 commits", etc. | Yes — accurate at the time each entry was written. **Do NOT edit.** |
| `worklog.md` | 4257, 4263, 4264, 4353, 4354, 4359, 4380, 4390, 4578 | Multiple "production-ready" + "62 lint warnings" claims in the round-32-era sign-off section | Historical — accurate at the time, but the "production-ready" claim was retracted by HONEST_STATUS_CORRECTED.md (2026-08-05). A future cleanup task could add a "(RETRACTED — see HONEST_STATUS_CORRECTED.md)" inline note but should not delete the historical entry. |

### Stale `/workspaces/twoyi/` path references

These paths reference the codespace's old working directory; the repo
is now cloned to `/home/z/my-project/repos/twoyi` locally. Not
misleading (any reader knows they're codespace-relative paths) but
could be made more portable.

| File | Line | Stale Claim | Suggested Fix |
|------|------|-------------|---------------|
| `SESSION_SUMMARY.md` | 322, 323 | `export ANDROID_HOME=/workspaces/twoyi/.android-sdk` / `export ANDROID_NDK_HOME=/workspaces/twoyi/.android-ndk` | Replace `/workspaces/twoyi/` with `$REPO_ROOT/` or document that the paths are codespace-specific. |
| `AOSP_BUILD_RESULTS.md` | 283 | `NDK=/workspaces/twoyi/.android-ndk` | Same fix. |
| `FUNCTION_LEVEL_COMPARISON.md` | 7 | `LEGACY = /workspaces/twoyi/app/src/main/jniLibs/arm64-v8a/libOpenglRender.so` | Replace `/workspaces/twoyi/` with `$REPO_ROOT/`. |
| `PORT_RESULTS.md` | 398, 400 | `/workspaces/twoyi/app/src/main/jniLibs/{arm64-v8a,x86_64}/libOpenglRender.so` | Same fix. |
| `download/SESSION_SUMMARY.md` | 322, 323 | (mirror) | Same fix. |
| `download/AOSP_BUILD_RESULTS.md` | 283 | (mirror) | Same fix. |
| `download/FUNCTION_LEVEL_COMPARISON.md` | 7 | (mirror) | Same fix. |
| `download/PORT_RESULTS.md` | 398, 400 | (mirror) | Same fix. |

### Stale "closed-source blob" references for `libOpenglRender.so`

Many docs refer to `libOpenglRender.so` as a "closed-source blob".
This is **HISTORICALLY ACCURATE** — the upstream cyanmint/twoyi blob
WAS closed-source. The current state is that it has been rebuilt from
AOSP `emugl` source (commit `47f8335` + `eb13449`). The following
references are STALE only if they describe the CURRENT state as still
being a closed-source blob; references to the LEGACY blob as
historical context are NOT stale.

| File | Line | Classification | Note |
|------|------|----------------|------|
| `MIGRATION_GUIDE.md` | 27, 203, 254 | NOT stale | Correctly describes the legacy blob as historical, replaced by open-source rebuild. |
| `RELENG.md` | 62 | Stale (Moderate) | "`libOpenglRender.so` / `libloader.so` blobs ship only for `arm64-v8a`; an" — present-tense framing of a historical state. Should be "the LEGACY `libOpenglRender.so` / `libloader.so` blobs shipped only for `arm64-v8a`; an …". |
| `OPENGL_RENDERER_NEW.md` | 12 | NOT stale | Correctly describes the new open-source lib as a "complete replacement for the legacy proprietary library." |
| `DEVELOPMENT_ROADMAP.md` | 50, 121 | NOT stale | Correctly framed as "replaced the largest closed-source native blob" (past tense). |
| `disasm/DISASSEMBLY_ANALYSIS.md` | 338, 427 | NOT stale | Correctly describes the disassembly-then-rebuild workflow. |
| `disasm/FINAL_REPORT.md` | 41 | NOT stale | Correctly describes the legacy blob as historical. |
| `SESSION_SUMMARY.md` | 58, 178 | NOT stale | Correctly describes the legacy blob as historical, rebuilt from source. |
| `TECHNICAL_BRIEFING.md` | 29, 196 | NOT stale | "(legacy blob)" and "replaces the closed-source blob" — correctly framed as historical. |
| `README.md` | 72 | NOT stale | "Replaces the 1.06 MB closed-source arm64-only blob with a 605–611 KB build compiled from AOSP emugl source" — past-tense, accurate. |
| `ARCHITECTURE_DECISIONS.md` | 66 | NOT stale | "`libOpenglRender.so` blob — disassembly proved it was a lightly-modified" — correctly historical. |
| `download/TWOYI_HONEST_STATUS.md` | 32, 124 | NOT stale | "On x86_64, the legacy `libOpenglRender.so` blob is not shipped (arm64-only)." — correctly describes the legacy blob. |
| `download/PROJECT_SUMMARY.md` | 16, 325, 600 | NOT stale | "The original twoyi shipped three large closed-source native blobs" — correctly describes the upstream historical state. |
| `download/MIGRATION_GUIDE.md` | 27, 203, 254 | (mirror) | NOT stale. |
| `download/RELENG.md` | 62 | (mirror) | Stale (Moderate) — same as root. |
| `download/DEVELOPMENT_ROADMAP.md` | 50, 121 | (mirror) | NOT stale. |
| `download/TWOYI_DISASSEMBLY_ANALYSIS.md` | 338, 427 | (mirror) | NOT stale. |
| `download/TWOYI_FINAL_REPORT.md` | 41 | (mirror) | NOT stale. |
| `download/SESSION_SUMMARY.md` | 58, 178 | (mirror) | NOT stale. |
| `download/TECHNICAL_BRIEFING.md` | 29, 196 | (mirror) | NOT stale. |
| `download/ARCHITECTURE_DECISIONS.md` | 66 | (mirror) | NOT stale. |
| `download/FUNCTION_LEVEL_COMPARISON.md` | 7 | NOT stale | The "LEGACY = " prefix makes clear this is the legacy blob. |
| `FUNCTION_LEVEL_COMPARISON.md` | 7 | NOT stale | Same. |
| `download/VM_DEEP_DISASSEMBLY.md` | 124, 1010, 1031 | NOT stale | Correctly describes the legacy blob in historical context. |
| `docs_vm_deep_disassembly.md` | 124, 1010, 1031 | NOT stale | (mirror) |
| `CHANGELOG.md` | 134 | NOT stale | "`libOpenglRender.so` blob is arm64-only and not shipped for `x86_64`" — past-tense, historical. |
| `ARCHITECTURE.md` | 481 | NOT stale | "The legacy closed-source `libOpenglRender.so` shipped by upstream twoyi" — correctly historical. |
| `OPENGL_RENDERER.md` | 14 | NOT stale | Correctly describes the proprietary lib as the thing being replaced. |

### `libadb.so` "still closed-source" references

`SESSION_SUMMARY.md` line 241 (and its mirror) correctly notes that
`libadb.so` is still closed-source — that's TRUE and not stale.

---

## Summary

**Counts by severity:**

- **Critical: 18 claims** across 4 distinct files in repo root
  (3 unique: `MEMORY.md`, `ONE_PAGE_SUMMARY.md`, `FINAL_STATUS.md`) +
  2 mirrored copies in `download/`.
- **Moderate: ~140 claims** across ~25 distinct files
  (most files have multiple stale references; the largest
  contributors are the branch-reference cluster, the test-count
  cluster, and the LOC-count cluster).
- **Minor: ~80 claims** across `worklog.md` (historical entries —
  should NOT be edited) + 8 stale `/workspaces/twoyi/` path references
  + a handful of "closed-source blob" present-tense framings.

**Files with the most Critical/Moderate stale claims (in priority
order for a cleanup task):**

1. `MEMORY.md` — 12 Critical + ~10 Moderate. This is the canonical
   state doc; fixing it first removes the most misleading claims.
2. `download/MEMORY.md` — wait, no — `download/MEMORY.md` doesn't
   exist. The MEMORY.md is unique to the repo root.
3. `FINAL_STATUS.md` + `download/FINAL_STATUS.md` — 1 Critical each
   (154 tests → 165 + 9,581 LOC → 11,554).
4. `ONE_PAGE_SUMMARY.md` + `download/ONE_PAGE_SUMMARY.md` — 2
   Critical each (154 tests + CI-green-without-date).
5. `PROJECT_SUMMARY.md` + `download/PROJECT_SUMMARY.md` — many
   Moderate (207 commits, 879-line worklog, branch refs).
6. `SESSION_SUMMARY.md` + `download/SESSION_SUMMARY.md` — many
   Moderate (9,581 LOC, 144 tests, branch refs).
7. `VERIFICATION.md` (download/ only) — many Moderate (entire doc is
   a snapshot against `improvements/initial-cleanup` at `ca33d02`).
8. `FINAL_VERIFICATION.md` (download/ only) — same as above.
9. `CONTRIBUTING.md` + `CONTRIBUTOR_LADDER.md` + `RELENG.md` +
   `QUICK_START.md` + `FAQ.md` + `MIGRATION_GUIDE.md` +
   `DEVELOPMENT_ROADMAP.md` + `ARCHITECTURE.md` +
   `ARCHITECTURE_DECISIONS.md` + `PROJECT_HEALTH.md` + `HANDOFF.md` +
   `MORNING_MESSAGE.md` + `CREDITS.md` + `CHANGELOG.md` +
   `TESTING_GUIDE.md` + `GLOSSARY.md` + `TECHNICAL_BRIEFING.md` —
   each has 1–6 Moderate stale branch references.
10. `disasm/FINAL_REPORT.md` + `download/X86_64_ROOTFS_BUILD_GUIDE.md`
    + `download/FINAL_COMMIT_LOG.md` + `download/TWOYI_FINAL_REPORT.md`
    + `download/TWOYI_HONEST_STATUS.md` — 1–2 Moderate branch refs each.

**Recommended cleanup approach (for the doc-cleanup task):**

1. **Start with `MEMORY.md`** — it's the canonical state doc and has
   the most Critical claims. Replace every "Production-ready" with
   "build-ready (container has not yet booted end-to-end)", update
   branch to `main`, update commit count to 411, update test count
   to 165, add date stamps to "all quality gates green" claims.
2. **Then `ONE_PAGE_SUMMARY.md` and `FINAL_STATUS.md`** — small files,
   easy wins.
3. **Then sweep the branch-reference cluster** — a single
   search-and-replace of `improvements/initial-cleanup` → `main`
   across all repo-root `.md` files will fix the bulk of the Moderate
   claims. (Use `rg -l 'improvements/initial-cleanup' --glob '*.md'`
   to enumerate target files, then sed or Edit each one.)
4. **Then update test counts** — replace `144 tests` → `165 tests`
   and `154 tests` → `165 tests` across all `.md` files. Also update
   `9,581 LOC` → `~11,554 LOC` and `9,581 lines` → `~11,554 lines`.
5. **Then update commit counts** — `207 commits` → `411 commits`,
   `386 commits` → `411 commits`, `372 total` → `411 total`, etc.
6. **Then add date stamps** to "all quality gates green" / "CI green"
   claims that lack them — the current state IS green (verified
   2026-08-08 commit `99c940e`) but the claim was false for rounds
   60–67, so undated claims are untrustworthy.
7. **Leave `worklog.md` alone** — it's a chronological log;
   historical entries are accurate-at-the-time and should not be
   rewritten.

**18 critical, ~140 moderate, ~80 minor stale claims found across
~30 files** (counts are approximate because some claims span multiple
lines and some files have mirrored copies in `download/`).

---

*This audit report was generated by Task Agent 3 (general-purpose
sub-agent). It does NOT modify any .md files — that's a separate
doc-cleanup task. The audit script `scripts/ci-local.sh` (committed
alongside this report) can be used to verify the build-quality claims
locally before pushing doc fixes.*
