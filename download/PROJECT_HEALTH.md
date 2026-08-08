# Project Health — twoyi fork

> **Task ID:** KEEP-WORKING-11 · **Author:** general-purpose sub-agent
> **Date:** 2026-08-05 ~06:50 UTC · **Last refreshed:** 2026-08-08 (round 68)
> **Branch:** `main` (the ONLY branch — `improvements/initial-cleanup` was
> merged in and deleted from origin on 2026-08-08; HEAD on round 68 refresh
> is `99c940e`, ~84 commits past the cyanmint fork point)
> **Sources:** `VERIFICATION.md`, `CHANGELOG.md`, `DEVELOPMENT_ROADMAP.md`, `TWOYI_HONEST_STATUS.md`,
> `CONTRIBUTOR_LADDER.md`, `.github/workflows/*.yml`, `app/rs/kr64/src/`.

> **Round 68 update:** this is the historical copy of `PROJECT_HEALTH.md`
> preserved in `download/`. The live, current version is at the repo root.
> Two stale claims below have been corrected in-place: (1) the branch
> field — `improvements/initial-cleanup` was merged into `main` and deleted
> on 2026-08-08; (2) the "Branch hygiene" debt — `main` is now the only
> branch and IS the dev branch. The original "CI status: GREEN" claim
> from rounds 60–67 was actually broken (4 stacked bugs); all 4 are now
> fixed and CI is genuinely green on `main` HEAD `99c940e`. For full
> details see `MEMORY.md` §Round 68 in the repo root.

An honest assessment across five dimensions. The project is in **early-stage but
well-disciplined** shape: code compiles, **CI is now actually green** (round 68
fixed 4 stacked bugs that had broken every CI run since round 60), docs are
abundant, but the end-to-end product (a booting guest container) is not yet
demonstrable.

## 1. Code health — 🟡 Mixed

- **CI status: GREEN (verified in round 68).** Both `build.yml`
  (assembleRelease for arm64-v8a + x86_64 + Android lint) and
  `kr64-tests.yml` (165 `cargo` tests + `cargo fmt --check` +
  `cargo clippy -- -D warnings`) pass on `main` HEAD `99c940e`.
  ⚠️ **Caveat:** CI was actually **broken on every push from rounds 60
  through 67** (8+ consecutive red runs). The original version of this
  doc claimed "CI status: GREEN" from round 60 onward, but that claim
  was only ever true for the local cargo/gradle invocations — CI never
  got far enough to invoke any of them. The 4 stacked root causes were
  fixed in round 68 (see repo-root `MEMORY.md` §Round 68). Both
  workflows are now genuinely green on `main`.
- **Test coverage: thin but real.** `kr64` has 9,581 LOC across 10 files with
  165 `#[test]` functions; all pass on host and in CI. **No Java unit tests
  beyond the Gradle template `ExampleUnitTest`**, and **no instrumented tests
  beyond `ExampleInstrumentedTest`**. Coverage of the actual JNI surface
  (`libtwoyi.so` ↔ `Renderer.java`) is zero.
- **Build quality: clean.** `kr64` compiles with zero warnings; `twoyi`
  and `loader` are also clippy-clean. `clippy -D warnings` is enforced in
  `CONTRIBUTING.md` and gated in `kr64-tests.yml`.
- **Multi-ABI: works.** Both `arm64-v8a` and `x86_64` produce APKs; x86_64
  defaults to the new Rust renderer (defence-in-depth via
  `effective_renderer_type()`).

## 2. Documentation health — ✅ Strong, with redundancy

- **Volume: 46 tracked `.md` files in repo root + 33 in `download/`.** Coverage
  spans README (310 lines, bilingual EN/中文), ARCHITECTURE (1,324 lines),
  CONTRIBUTING (404), CHANGELOG (Keep-a-Changelog v1.1.0), ROADMAP (769 lines,
  5 phases / 56 tasks), SECURITY, CODE_STYLE_GUIDE, TESTING_GUIDE, FAQ,
  MIGRATION_GUIDE, GLOSSARY (38 terms), CONTRIBUTOR_LADDER.
- **Accuracy: high.** Every doc claim is traceable to a commit or analysis file;
  `TWOYI_HONEST_STATUS.md` explicitly retracts a previous overclaim ("container
  booted" was the AVD launcher, not twoyi). Honesty is a documented cultural
  norm (`CONTRIBUTING.md` §6).
- **Accessibility: good.** Codespace devcontainer (Ubuntu 22.04, glibc) with
  preinstalled NDK r27c / Rust stable / Android emulator.
- **Debt:** overlapping summaries (`PROJECT_SUMMARY.md`, `SESSION_SUMMARY.md`,
  `FINAL_STATUS.md`, `TECHNICAL_BRIEFING.md`, `TWOYI_FINAL_REPORT.md` cover
  similar ground) and legacy `CHANGES.md` / `CHANGES_SUMMARY.md` still tracked
  alongside the new `CHANGELOG.md`.

## 3. Community health — 🔴 Nascent

- **Contributor pipeline: defined but unproven.** `CONTRIBUTOR_LADDER.md`
  specifies 4 roles (New Contributor → Contributor → Maintainer → Lead
  Maintainer) with concrete advancement criteria; `DEVELOPMENT_ROADMAP.md` §10.2
  lists 10 `good first issue`-grade tasks.
- **Contributors: effectively 1.** All 37 commits since the fork point were
  authored by general-purpose sub-agents overnight. **Bus factor = 1.** No
  human-reviewed PRs have merged yet.
- **Issue tracking: not yet in use.** GitHub Issues/Discussions referenced in
  docs but no triage history, no `needs-triage` workflow exercised.
- **Communication channels:** GitHub Issues + Discussions only. No chat
  platform, no meeting cadence — appropriate for current size.

## 4. Technical debt — 🟡 Tracked, mostly deliberate

- **Closed-source blobs:** only `libadb.so` remains (legacy NDK r21d).
  `libOpenglRender.so` rebuilt from AOSP (`47f8335`); `libloader.so` is a Rust
  crate but **was Copilot-driven and not yet audited against the deep
  `TWOYI_DISASSEMBLY_ANALYSIS.md`** (roadmap task 2.3).
- **Skeletons, not products:** `kr64` daemon, new Rust renderer, and binder
  skeleton all compile + unit-test but have stubs (`SIGSYS` handler emulates
  few syscalls; renderer's GL protocol incomplete; binder not wired to boot).
- **x86_64 rootfs not built.** Container cannot boot on x86_64 emulator
  (arch mismatch with arm64 rootfs); only real arm64 hardware can fully
  smoke-test today.
- **Branch hygiene:** ✅ RESOLVED in round 68. `main` IS the dev branch and
  the only branch on origin — `improvements/initial-cleanup` was merged in
  and deleted on 2026-08-08. (Previously: `main` was 47 commits ahead of
  `origin/main` and was *not* the dev branch — all work lived on
  `improvements/initial-cleanup`.) No reconciliation debt remains.
- **Test keystore committed** (`app/twoyi-release.keystore`) — intentional for
  CI usability; production distributors MUST replace (documented inline in
  `app/build.gradle`).

## 5. Risk assessment — 🟡 Mitigations exist but untested

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Container never boots (kr64 stubs, no x86_64 rootfs) | **High today** | Critical | Roadmap Phase 3 (weeks 5–12) is the entire boot path; gated by GsiExtractor → kr64 spawn. |
| Single maintainer disappears | Medium | Critical | `CONTRIBUTOR_LADDER.md` defines succession; needs a second Maintainer. |
| Sub-agent-authored code has subtle bugs | Medium | High | CI green (round 68) + 165 kr64 tests + honesty policy reduce but don't eliminate; needs human review pass. |
| Test keystore leaks into production build | Low | High | Documented inline; add a Gradle `checkReleaseBuilds` gate (not present). |
| Renderer regression on arm64 (only verified symbol exports, not boot) | Medium | High | Roadmap task 1.1: drop-in test on real arm64 device. |
| Documentation rot (46+ `.md` files) | Medium | Medium | Roadmap §0 says roadmap is "living"; no automated doc-freshness check exists. |
| `libloader.so` Rust port has hidden behaviour gap | Medium | Medium | Audit task 2.3 tracked; not yet executed. |

### Bottom line
The fork is in a **credible Phase-1 state**: green CI, comprehensive honest docs,
clean multi-ABI builds, and a 56-task roadmap to a booting MVP. The biggest
gap is end-to-end verification — **nothing has actually booted a guest
container yet**, and the kr64/renderer/binder skeletons need real
implementation work (Phase 3) before the project can claim to "work." Honest
status reporting and a defined contributor ladder are the strongest signals of
long-term health; bus factor of 1 and zero human-reviewed PRs are the weakest.
