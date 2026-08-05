# RELENG — Release Engineering

How twoyi releases are versioned, built, signed, published, and rolled back.
Active dev branch: `improvements/initial-cleanup`; mirror: `Disable-OP/twoyi`.

## 1. Version numbering

`app/build.gradle` generates a **date-stamped semantic version** at build time:

```
versionName  = "3.5.5-${MMddHHmm}"     # e.g. 3.5.5-08041908
versionCode  = 30505                    # static; timestamp is the discriminator
archivesBaseName = "twoyi_${versionName}"
```

- `3.5.5` — human-readable **major.minor.patch**, bumped manually for breaking changes;
  tracks the upstream `cyanmint/twoyi` lineage.
- `-MMddHHmm` — build timestamp (month/day/hour/minute) from `SimpleDateFormat("MMddHHmm")`;
  every build gets a distinct `versionName` even when `versionCode` is unchanged.
- `versionCode` is intentionally **static**; test builds are ordered by timestamp. For a
  *public* release, bump it (e.g. `30505` → `30506`) so the Play-style upgrade path works.
- APK filename: `twoyi_3.5.5-08041908-release.apk` (cf. the existing
  `download/twoyi_3.5.5-08041908-release-unsigned.apk`).

## 2. Release checklist

Sign off each item in the release PR description:

1. **Green CI** — both `Build APK` and `kr64 unit tests` pass on the release commit.
2. **`CHANGELOG.md`** has a `## [3.5.5-MMddHHmm]` section for user-facing changes.
3. **`versionCode`** bumped if this is a public/upgrade release (see §1).
4. **Rootfs** — confirm the real `rootfs.tar.gz` (~275 MB, from `cyanmint/twoyi`'s
   'original' release) is bundled. Public releases MUST.
5. **Smoke test** on a real arm64 device (boot guest, launch app, verify renderer +
   input) and in an x86_64 redroid container.
6. **No `debuggable=true`**, no secrets in the release `buildType`.
7. **Keystore** — confirm `app/twoyi-release.keystore` is the intended key.
8. **Licenses** — `OPEN_SOURCE_LIBRARIES.md` and in-app `LicensesDialog` current.

## 3. Building the release APK

Prereqs: JDK 17, Android SDK (compileSdk 31, build-tools 30.0.3), NDK r27c, Rust stable with
`aarch64-linux-android` + `x86_64-linux-android` targets, and `cargo-xdk`.

```sh
./gradlew assembleRelease -Pabis=arm64-v8a,x86_64   # both ABIs
./gradlew assembleRelease -Pabis=all                 # build_rs.sh shorthand
./gradlew assembleRelease -Pabis=arm64-v8a           # device-only, smaller
```

Output: `app/build/outputs/apk/release/twoyi_3.5.5-MMDDHHmm-release.apk`.

**Signing.** The `release` buildType uses `signingConfigs.release`, reading
`app/twoyi-release.keystore` (store/key password `twoyi-release`, alias
`twoyi-release`). This is a **self-signed RSA-2048 test key committed to the
repo** for CI/codespace convenience — fine for installable test APKs, NOT for
production. Before a public release, replace the keystore and override via
`~/.gradle/gradle.properties` (`TWOYI_STORE_FILE`, `TWOYI_STORE_PASSWORD`,
`TWOYI_KEY_ALIAS`, `TWOYI_KEY_PASSWORD`). Never commit a production key.

**ABI selection.** `abiFilters` = `arm64-v8a` + `x86_64`. The legacy closed-source
`libOpenglRender.so` / `libloader.so` blobs ship only for `arm64-v8a`; an
`x86_64` install auto-falls-back to the open-source `lib*_new.so` Rust variants
(see `RomManager.LOADER_FILE` / `core::RendererType`).

## 4. Creating a GitHub release

```sh
# 1. Tag the release commit (annotated, signed if possible):
git tag -a v3.5.5-08041908 -m "Release 3.5.5-08041908"
git push origin v3.5.5-08041908

# 2. Build locally (or download the CI artifact from the tagged commit's "Build APK"
#    run) and rename + checksum:
cp app/build/outputs/apk/release/twoyi_3.5.5-08041908-release.apk \
   /tmp/twoyi-3.5.5-08041908.apk
sha256sum /tmp/twoyi-3.5.5-08041908.apk > /tmp/checksums.txt
```

3. GitHub → **Releases → Draft a new release** → pick the tag.
4. **Title** `Twoyi 3.5.5-08041908` (matches `versionName`); **Notes** = the
   matching `CHANGELOG.md` section + a "Known issues" block.
5. **Attach** the signed APK and `checksums.txt`; mark **latest**; publish.

## 5. CI/CD pipeline

Two workflows in `.github/workflows/`:

| Workflow | File | Triggers | Purpose |
|---|---|---|---|
| Build APK | `build.yml` | push to `main`/`develop`/`improvements/**`, PRs, `workflow_dispatch` | `assembleRelease`, upload APK artifact (30-day retention) |
| kr64 unit tests | `kr64-tests.yml` | push to `improvements/**`, PRs, `workflow_dispatch` | `cargo test --no-fail-fast` on the `kr64` crate (host x86_64) |

**`build.yml` `workflow_dispatch` inputs:**
- `abis` — comma-separated (`arm64-v8a`, `x86_64`, or `all`). Default `all`.
- `include_rootfs` — boolean. When `true`, the runner downloads the real
  `rootfs.tar.gz` from the `cyanmint/twoyi` 'original' release before building;
  otherwise the APK ships with placeholder assets and is non-functional.

Build job: JDK 17 temurin, Rust stable with both Android targets, `cargo-xdk`, NDK r27c.
On failure, build logs (`app/build/reports/`, cargo logs) upload as a 7-day artifact.

**Release-candidate build:** Actions → "Build APK" → Run workflow → `abis=all`,
`include_rootfs=true` → Run → download the `twoyi-apk-arm64-v8a` artifact → §4.

## 6. Rollback procedure

If a published release is broken (bootloop, crash on launch, etc.):

1. **Unset "latest"** on the bad release's GitHub page (stops the API and any in-app
   updater from advertising it), then **re-point "latest"** to the previous known-good
   release. Do not delete the bad release — keep it for forensics, mark its title `[RETRACTED]`.
2. **Push a revert** commit on `improvements/initial-cleanup` for the offending
   change(s); let CI rebuild.
3. **Cut a hotfix** following §2–§4 with a fresh `MMddHHmm` timestamp and a
   bumped `versionCode` so users on the bad build can upgrade over it.
4. **Post-mortem:** add a "Retractions" note to `CHANGELOG.md` under the next
   release, referencing the reverted commits.
5. **Never re-use a `versionName`.** A byte-identical rebuild still gets a new
   timestamp suffix so the bad APK and the fixed APK stay unambiguous.
