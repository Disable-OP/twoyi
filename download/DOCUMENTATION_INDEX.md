# Documentation Index

Master index of every Markdown documentation file in the twoyi project.
Scope: repo root (`/`), `download/` analysis docs, and `.devcontainer/`
(which currently contains no `.md` files). Files are grouped by category
and sorted alphabetically within each group. Many root files have an
identical curated copy in `download/`; both entries are listed.

Legend: `LOC` = line count. Categories: **Analysis**, **Code**, **Guide**,
**Policy**, **Config**.

## Analysis (reverse engineering, comparisons, status, results)

| File | LOC | Description |
|------|-----|-------------|
| AOSP_BUILD_RESULTS.md (root + download/) | 509 | Outcome of building twoyi against AOSP source trees. |
| AOSP_VS_LEGACY_COMPARISON.md (download/) | 231 | Side-by-side comparison of AOSP and legacy Android trees. |
| BINDER_SKELETON.md (root + download/) | 375 | Skeleton mapping of Binder IPC services in the VM. |
| docs_vm_deep_disassembly.md (root) / VM_DEEP_DISASSEMBLY.md (download/) | 1140 | Deep disassembly walkthrough of the VM native loader. |
| docs_vm_java_analysis.md (root) / VM_JAVA_ANALYSIS.md (download/) | 973 | Analysis of the VM's Java-side bootstrap and installer. |
| docs_vm_kr64_analysis.md (root) / VM_KR64_ANALYSIS.md (download/) | 1042 | Static analysis of the KR64 kernel/runtime component. |
| docs_vm_rom_analysis.md (root) / VM_ROM_ANALYSIS.md (download/) | 390 | Analysis of the bundled VM ROM image and its filesystem. |
| FINAL_STATUS.md (root + download/) | 91 | Short status snapshot of the port at a point in time. |
| FUNCTION_LEVEL_COMPARISON.md (root + download/) | 866 | Function-by-function comparison of legacy vs AOSP symbols. |
| HAL_VIRTUALIZATION_ANALYSIS.md (root + download/) | 393 | How HALs are virtualized across the guest/host boundary. |
| JNI_VERIFICATION.md (root) | 36 | Verification notes for JNI bridge integrity. |
| KR64_SKELETON.md (root + download/) | 228 | Skeleton map of the KR64 module's exported functions. |
| PORT_RESULTS.md (root + download/) | 445 | Results log of the x86_64 porting effort. |
| PROJECT_HEALTH.md (root + download/) | 101 | Health-check summary of build, tests, and coverage. |
| PROJECT_SUMMARY.md (root + download/) | 968 | Long-form summary of the entire twoyi project. |
| SESSION_SUMMARY.md (root + download/) | 428 | Narrative summary of a multi-day work session. |
| TECHNICAL_BRIEFING.md (root + download/) | 462 | Technical briefing for new contributors and reviewers. |
| TWOYI_DISASSEMBLY_ANALYSIS.md (download/) | 504 | Focused disassembly analysis of twoyi's own binaries. |
| TWOYI_FINAL_REPORT.md (download/) | 165 | Final consolidated report for the twoyi port. |
| TWOYI_HONEST_STATUS.md (download/) | 167 | Candid, no-hype assessment of what works and what doesn't. |
| VERIFICATION.md (download/) | 80 | Verification checklist for release readiness. |
| VIRTUAL_MASTER_ANALYSIS.md (download/) | 212 | Initial analysis of the Virtual Master APK. |
| VIRTUAL_MASTER_FULL_ANALYSIS.md (download/) | 193 | Expanded analysis of Virtual Master internals. |
| worklog.md (root) | 3721 | Append-only chronological work log of every sub-agent task. |
| X86_64_BREAKTHROUGH.md (root + download/) | 109 | Notes on the breakthrough that enabled x86_64 boot. |

## Code (architecture, implementation, modules, changelogs)

| File | LOC | Description |
|------|-----|-------------|
| ARCHITECTURE.md (root) | 1324 | Canonical architecture overview of twoyi components. |
| AUDIO_IMPL.md (download/) | 268 | Implementation notes for the audio HAL bridge. |
| AUDIO_SENSOR_HAL.md (download/) | 757 | Combined audio + sensor HAL virtualization design. |
| BATTERY_IMPL.md (download/) | 349 | Implementation notes for the battery HAL bridge. |
| CHANGELOG.md (root) | 271 | Release-oriented changelog of user-visible changes. |
| CHANGES.md (root) | 228 | Running log of code-level changes per session. |
| CHANGES_SUMMARY.md (root) | 147 | Condensed summary of recent CHANGES.md entries. |
| FIX_SUMMARY.md (root) | 141 | Summary of bug fixes applied in the latest cycle. |
| IMPLEMENTATION_SUMMARY.md (root) | 293 | High-level summary of implemented features. |
| LOADER_NEW.md (root) | 268 | Design of the new native loader used at guest boot. |
| OPENGL_RENDERER.md (root) | 209 | Original OpenGL renderer design document. |
| OPENGL_RENDERER_NEW.md (root) | 245 | Revised OpenGL renderer design with streaming pipeline. |
| PIE_IMPLEMENTATION.md (root) | 139 | Notes on Position-Independent Executable support. |
| PROFILE_MANAGER.md (root) | 152 | Design of the guest profile/identity manager. |
| REFACTORING_SUMMARY.md (root) | 176 | Summary of structural refactors applied to the codebase. |
| SENSOR_IMPL.md (download/) | 497 | Implementation notes for the sensor HAL bridge. |
| SHELL_EXECUTION.md (root) | 274 | Design of the guest shell command execution path. |

## Guide (getting started, how-tos, FAQs, glossary)

| File | LOC | Description |
|------|-----|-------------|
| DEBUG_RENDERER_TESTING.md (root) | 208 | How-to for debugging and testing the renderer. |
| DEVELOPMENT_ROADMAP.md (root + download/) | 769 | Forward-looking roadmap of milestones and themes. |
| FAQ.md (root + download/) | 198 | Frequently asked questions about twoyi. |
| GETTING_STARTED.md (root) | 246 | Onboarding guide to build and run twoyi locally. |
| GLOSSARY.md (root + download/) | 130 | Glossary of twoyi and virtualization terminology. |
| GSI_BOOT_PLAN.md (download/) | 997 | Plan for booting a GSI image inside the VM. |
| MIGRATION_GUIDE.md (root + download/) | 469 | Guide for migrating from legacy to AOSP-based builds. |
| QUICK_START.md (root + download/) | 146 | Fastest path to a running twoyi environment. |
| README.md (root) | 378 | Project README: overview, build, run, links. |
| README_CN.md (root) | 101 | Chinese-language README mirror. |
| REDROID_TESTING.md (root) | 72 | How to test twoyi against a redroid reference image. |
| TESTING_DIRECT_INVOCATION.md (root) | 146 | How to test components via direct JVM invocation. |
| TESTING_GUIDE.md (root + download/) | 300 | End-to-end testing strategy and procedures. |
| X86_64_ROOTFS_BUILD_GUIDE.md (download/) | 994 | Step-by-step guide for building an x86_64 rootfs. |

## Policy (contributing, style, security, release engineering)

| File | LOC | Description |
|------|-----|-------------|
| ARCHITECTURE_DECISIONS.md (root + download/) | 431 | Architecture Decision Records (ADRs). |
| CODE_STYLE_GUIDE.md (root + download/) | 361 | Code style and formatting rules for contributors. |
| CONTRIBUTING.md (root) | 404 | How to contribute patches, review, and land changes. |
| CONTRIBUTOR_LADDER.md (root + download/) | 144 | Roles and progression ladder for contributors. |
| OPEN_SOURCE_LIBRARIES.md (root) | 328 | Inventory and license notes for bundled OSS libraries. |
| RELENG.md (root + download/) | 120 | Release engineering process and tagging policy. |
| SECURITY.md (root) | 154 | Security policy, disclosure, and hardening notes. |

## Config

No `.md` files currently exist under `.devcontainer/`. Configuration for
the dev container lives in `.devcontainer/devcontainer.json`,
`.devcontainer/Dockerfile`, and `.devcontainer/scripts/*.sh`.

---

Total: 89 documentation files (50 in repo root, 39 in `download/`,
0 in `.devcontainer/`). Generated for task KEEP-WORKING-13.
