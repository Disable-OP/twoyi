# Contributor Ladder

> The four contributor roles in the twoyi project — what each can do, what's
> expected, and how to advance. Pair with [`CONTRIBUTING.md`](../CONTRIBUTING.md)
> (process) and the [Development Roadmap](DEVELOPMENT_ROADMAP.md) (what to work on).
> Active branch: `improvements/initial-cleanup` on `Disable-OP/twoyi`.

## 1. New Contributor

Anyone who has forked the repo and is reading the docs — no PRs merged yet.

**Requirements** — Read in order: `README.md` → `ARCHITECTURE.md` → `CONTRIBUTING.md` →
`DEVELOPMENT_ROADMAP.md` §10 → `GLOSSARY.md`. Set up a working build (Codespace `standardLinux32gb`;
`CONTRIBUTING.md` §2). Acknowledge the honesty policy (`TWOYI_HONEST_STATUS.md`) — overclaims are
the one thing we push back on in review.

**Permissions** — Open issues and Discussions; open PRs from your fork against
`improvements/initial-cleanup`; comment on issues and PRs; self-assign issues labelled `good first
issue`.

**Responsibilities** — Ask in Discussions before sinking time into code; follow the PR template
(Conventional Commits, `cargo fmt` + `clippy -D warnings`, honest "What I tested / Not tested");
pick a task from §5 or `DEVELOPMENT_ROADMAP.md` §10.2.

**How to advance to Contributor** — Get **one** PR merged into `improvements/initial-cleanup`. Any
size counts — typo, docs, or a `good first issue`.

## 2. Contributor

A developer with at least one merged PR — the steady-state role most people stay in long-term.

**Requirements** — Landed a change end-to-end (branch → test → PR → review → merge). Working
knowledge of either the Rust crates (`app/rs/`, `kr64/`, `loader/`, `openglrenderer/`) or the Java
side (`app/src/main/java/io/twoyi/`), per `CODE_STYLE_GUIDE.md`. Writes honest "Testing" / "Not
tested" PR sections without prompting.

**Permissions** — Everything a New Contributor can do, plus: review others' PRs (non-blocking
comments welcome); propose new `good first issue` entries; trigger CI reruns on your own PRs; co-own
a sub-area after 3+ merged PRs in it (e.g. "kr64 devices", "renderer port").

**Responsibilities** — Keep CI green on your PRs (fix breakage before opening the next); respond to
review feedback within ≤1 week (leave a comment if you need longer); help triage `needs-triage`
issues; update `download/` docs when your PR changes behaviour a doc describes.

**How to advance to Maintainer** — 5+ merged PRs (at least one **medium-effort M** from roadmap
§10.3) and one non-trivial PR review you drove to merge. Nominated by an existing Maintainer in a
Discussion; consensus of Maintainers (silent approval for 1 week) confirms.

## 3. Maintainer

A trusted contributor who can review and merge PRs.

**Requirements** — Deep familiarity with at least one layer (Java app, `libtwoyi.so` Rust crates, or
the AOSP-derived `libOpenglRender.so` rebuild); reviewed 3+ PRs to completion before elevation;
understands `ARCHITECTURE_DECISIONS.md` (Rust+JNI over C++; container over KVM; PIE-as-cdylib; open-
source everything; defer binder virtualisation).

**Permissions** — Approve and merge PRs into `improvements/initial-cleanup` (squash small, merge-
commit large per `CONTRIBUTING.md` §5). One approval suffices for non-architectural changes; **two**
approvals required for: new Rust crate, new AIDL/JNI surface, anything in
`app/rs/kr64/src/seccomp.rs` or `app/rs/src/interp.c`. Label/close issues, edit the roadmap, `force-
with-lease` feature branches (never `improvements/initial-cleanup` or `main`), cut release-candidate
builds.

**Responsibilities** — Review PRs within 1 week of assignment; enforce the honesty policy in PR
descriptions; don't merge your own PRs unless another Maintainer is unavailable >2 weeks and the
change is urgent; mentor at least one Contributor or New Contributor (§6); keep
`improvements/initial-cleanup` green — bisect and revert if a merge breaks CI and the author is
unreachable; document architectural changes in `ARCHITECTURE_DECISIONS.md` as a new ADR.

**How to advance to Lead Maintainer** — Sustained contribution over 6+ months (review load, releases
cut, ADRs authored, multi-layer ownership). Existing Lead nominates; consensus of all Maintainers
required.

## 4. Lead Maintainer

Project direction, release management, final say on disputes.

**Requirements** — Multi-year familiarity with Android containerisation, the AOSP build system, and
the twoyi codebase history (including the original `twoyi/twoyi` archive and the Virtual Master
reverse-engineering in `download/`). Track record of cutting releases, writing roadmap entries, and
arbitrating architectural disagreements.

**Permissions** — Everything a Maintainer can do, plus: cut tagged releases and publish APK
artifacts; merge to `main` (release branch); approve ADRs with Status = Accepted; decide
architectural disputes when Maintainers disagree; add or remove Maintainers (after community
discussion).

**Responsibilities** — Keep the roadmap honest and current (prune completed items each release); cut
a release at least once per roadmap phase and write the `CHANGELOG.md` entry; escalate security
issues per `SECURITY.md`; ensure every Maintainer has a documented area of ownership and a backup;
step in when a Maintainer is MIA to keep PRs unblocked.

**How to advance** — Top of the ladder. There is currently one Lead Maintainer. Succession is by
Maintainer nomination + Lead sign-off.

## 5. Good first issues

Five tasks from `DEVELOPMENT_ROADMAP.md` §10.2, each labelled `good first issue` and completable in
≤2 days by a New Contributor with a working build:

1. **Drop-in test the AOSP renderer on a real arm64 device.** Install the APK and confirm it boots
   with `libOpenglRender_aosp_arm64.so`. Phase 1 task 1.1. ~1 day. Needs physical arm64 hardware.
2. **Add `set_emugl_*` no-op stubs to the AOSP renderer.** Extend the patch series in
   `download/port_files/` and rebuild via `app/rs/openglrenderer/build.sh`. Phase 1 task 1.5. ~1
   day.
3. **Port `set_emugl_logger` to the Rust `log` crate.** Wire the emugl logger callback into twoyi's
   `log` macros. Phase 1 task 1.7. ~1 day. Good introduction to the JNI boundary.
4. **Wire `kr64` into the boot flow.** Add `kr64` as a workspace member of `app/rs/Cargo.toml`,
   extend `build_rs.sh`, add the spawn call in `core.rs`. Phase 1 task 1.4. ~2 days. Good for
   learning the kr64 codebase.
5. **Implement the battery HAL.** File-based, no real-time complexity. Phase 4 task 4.10. ~1 day.
   Spec in `BATTERY_IMPL.md`. Good first HAL.

See `CONTRIBUTING.md` §6 for medium-effort projects and hard problems that need a design discussion first.

## 6. Mentorship

- **Pair on the first PR.** A Maintainer or experienced Contributor pairs on your first PR end-to-end. Request via a Discussion labelled `mentor-request`.
- **Area owners.** Each sub-area (kr64, renderer, ROM manager, HAL, docs) has a named owner in `DEVELOPMENT_ROADMAP.md` §12 — ping them in PRs.
- **Office hours.** Maintainers rotate a weekly async thread in Discussions for newcomer questions — no question is too basic.
- **Review-as-teaching.** Maintainers leave explanatory comments, not just "change this"; Contributors with 3+ merged PRs start reviewing too.
- **Anti-gatekeeping.** If a PR is close but rough, the reviewer commits the fix themselves (with attribution) rather than bouncing it back. Landing the change matters more than purity of authorship.

## 7. Recognition

- **`CONTRIBUTORS.md`** — every merged-PR author is listed via `git shortlog`; Lead adds a one-line bio on request.
- **Release notes.** Every `CHANGELOG.md` entry credits the contributor by GitHub handle for the PRs in that release.
- **Role callouts.** Maintainers and the Lead are listed in `README.md` with their area of ownership.
- **First-PR shout-out.** New Contributors who land their first PR get a mention in the next release's `CHANGELOG.md`.
- **Report authorship.** Authors of major `download/*.md` analysis reports (e.g. the `VM_*_ANALYSIS.md` series) are credited in the header — current practice.
- **No bot awards.** No stars, badges, or levels — the ladder above is the only formal status system, and it maps to real repo permissions.

## Summary table

| Role | Merged PRs | Can merge | Reviews | Mentors | Cuts releases |
|---|---|---|---|---|---|
| New Contributor | 0 | — | — | — | — |
| Contributor | ≥1 | — | optional | optional | — |
| Maintainer | ≥5 + 1 (M) | yes | required | required | — |
| Lead Maintainer | sustained | yes | arbiter | required | yes |

Questions? Open a [GitHub Discussion](https://github.com/Disable-OP/twoyi/discussions) with the `ladder` label.

