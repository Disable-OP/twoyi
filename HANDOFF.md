# HANDOFF — twoyi project

> Morning. Overnight a crew of sub-agents pushed 43 commits to
> `improvements/initial-cleanup`, wrote ~89 docs, and got CI green.
> Nothing has booted a guest container end-to-end yet. That's the next
> thing. Coffee's on you.

> **Update (round 68, 2026-08-08):** the original handoff above was
> written when `improvements/initial-cleanup` was still the dev branch.
> That branch has since been merged into `main` and deleted from origin.
> `main` is now the ONLY branch (see `MEMORY.md` §Round 68 for the full
> history). CI was actually broken from rounds 60–67 due to 4 stacked
> bugs (NDK action, CMake cache, missing strings, missing translations);
> all 4 are now fixed and both workflows are genuinely green on `main`
> HEAD `99c940e`.

## 1. What was done

In one session we reverse-engineered Virtual Master's APK (recovered the
AES-128 key `%z89aviCM0KkbEs9`, decrypted its 4 bundled plugins,
identified the GSI), ported AOSP's `libOpenglRender` to arm64 + x86_64,
built `kr64` — an ~11.6K-LOC Rust crate emulating the kernel / binder / HAL
surface a 64-bit Android guest expects — wired up two CI workflows
(both green where finished), and produced a release APK. The honest-status
doc explicitly retracts an earlier "it boots" overclaim. Current state:
**unit tests + builds pass; no end-to-end guest boot verified.**

## 2. Where everything is

| Thing | Path |
|---|---|
| Dev branch | `origin/main` — the only branch (`improvements/initial-cleanup` was merged in and deleted on 2026-08-08). HEAD on round 68 refresh: `99c940e` |
| Local branch | `main` (synced with `origin/main`; no branch-hygiene debt) |
| Java app | `app/src/main/java/io/twoyi/` |
| Rust crates | `app/rs/{kr64,loader,openglrenderer}/` |
| Build config | `app/build.gradle` (`versionName 3.5.5-MMddHHmm`, `versionCode 30505`) |
| CI workflows | `.github/workflows/build.yml`, `kr64-tests.yml` — both green on `main` |
| Release APK | `download/twoyi_3.5.5-08041908-release-unsigned.apk` |
| Docs (curated) | `download/*.md` (40 files) — index at `download/DOCUMENTATION_INDEX.md` |
| Honest status | `download/TWOYI_HONEST_STATUS.md` |
| Project health | `download/PROJECT_HEALTH.md` |
| Roadmap | `download/DEVELOPMENT_ROADMAP.md` (56 tasks, phased) |
| Release eng | `download/RELENG.md` |
| Last verification | `download/FINAL_VERIFICATION.md` |
| Full session log | `worklog.md` (3,814 lines, every task) |

## 3. What to do next

**Boot a guest container end-to-end on a real arm64 device.** This is the
project's #1 risk (called out in `PROJECT_HEALTH.md`) and the only thing
standing between "compiles + unit-tests green" and "actually works". Pick
Phase 1 task 1.1 from `DEVELOPMENT_ROADMAP.md`. 5-min warm-up first:
confirm the latest CI Build-APK job finished green (was in-progress at
last snapshot).

## 4. How to continue

```bash
cd /home/z/my-project
git checkout main && git pull                # round 68: main is the only branch
git status                                   # working tree clean (post round-68 CI fix)

# gh CLI is NOT installed in this codespace
sudo apt-get install -y gh                   # or use the web UI
gh run list --repo Disable-OP/twoyi --limit 5
gh run view <build-apk-run-id> --repo Disable-OP/twoyi

./gradlew assembleRelease -Pabis=arm64-v8a   # build the release APK
# → app/build/outputs/apk/release/twoyi_3.5.5-MMDDHHmm-release.apk

cd app/rs && cargo test -p kr64 --no-fail-fast  # 165 host-runnable tests

cat download/TWOYI_HONEST_STATUS.md                     # read the honest state
cat download/DEVELOPMENT_ROADMAP.md                     # pick a Phase 1 task

adb install -r app/build/outputs/apk/release/twoyi_*.apk
# Launch twoyi → tap Boot → watch logcat. Does the guest reach homescreen?
```

## 5. Loose ends

- ✅ **RESOLVED (round 68):** `main` is now the only branch on origin.
  The historical "`main` is 47 ahead of `origin/main` and isn't the dev
  branch" debt is gone — `improvements/initial-cleanup` was merged in
  and deleted on 2026-08-08.
- ✅ **RESOLVED (round 68):** CI was broken on every push from rounds
  60–67 due to 4 stacked bugs (`nttld/setup-ndk@v2`, stale CMake
  artifacts, missing string resources, missing translations). All 4
  are now fixed in commits `f166b20`/`cd6d0d8`/`7fbf3ad`/`9e3a1fb`/`99c940e`;
  both workflows are verified green on `main`.
- **57 untracked `.md`** in repo root — commit, `.gitignore`, or move under `download/`. In limbo.
- **Release keystore** committed in repo is a self-signed **test key**. Rotate before any public release (`RELENG.md` §3).
- **`versionCode` is static (30505)** — public releases must bump it or PackageManager won't see an upgrade.
- **Bus factor = 1.** 84+ commits since fork point, 0 human-reviewed PRs. First PR to review: #1.

— End of handoff. `worklog.md` has the rest.
