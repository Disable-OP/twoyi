//! 6-Z537 — the guest init's `SetHighestAvailableOptionValue` LOG(FATAL)
//! branch neuterer (aarch64).
//!
//! # The wall this closes (the rn510–rn519 kptr campaign, CLOSED)
//!
//! Every boot since rn510 died at rung 3 with init's double FATAL:
//!
//! ```text
//! <3>init: Unable to set minimum option value 2 in /proc/sys/kernel/kptr_restrict
//! <2>init: Unable to set adequate kptr_restrict value!   → InitFatalReboot
//! ```
//!
//! The campaign PROVED the emulator's file layer byte-for-byte coherent
//! across FOUR instruments (the 6-Z531 eager seed, the 6-Z532 shadow, the
//! 6-Z534/536 syscall journals, the 6-Z535 synthetic-tree remap): init's
//! `ofstream` write of "4\n" lands (ret=2) and the verify re-read sees
//! EXACTLY "4\n" — yet the loop still counts down 4→3→2 and exhausts.
//! The divergence is inside the child's C++ streambuf layer: rn519's full
//! per-pid journal shows the iterations 3/2 verifies issue NO read and NO
//! lseek syscalls at all — libc++'s `basic_filebuf` serves `seekg(0)` and
//! the extraction from its ALREADY-CONSUMED buffer without ever reaching
//! the kernel, so every verify re-extracts the STALE first-iteration
//! token ("4") and never matches the freshly written "3"/"2". No
//! syscall-surface instrumentation can observe it, and no file-layer fix
//! can repair it.
//!
//! # The ROM's actual code (AOSP 11/12/13 init/security.cpp — identical)
//!
//! ```cpp
//! static bool SetHighestAvailableOptionValue(const std::string& path,
//!                                            int min, int max) {
//!     std::ifstream inf(path, std::fstream::in);
//!     ...
//!     int current = max;
//!     while (current >= min) {
//!         std::ofstream of(path, std::fstream::out);
//!         of << std::to_string(current) << std::endl;   // fd 17
//!         of.close();
//!         inf.seekg(0);          // the streambuf in-buffer seek
//!         inf >> str_rec;        // the stale-token extraction
//!         if (str_val.compare(str_rec) == 0) break;
//!         current--;
//!     }
//!     if (current < min) { LOG(ERROR) << "Unable to set minimum option
//!         value " << min << " in " << path; return false; }
//!     return true;
//! }
//!
//! Result<void> SetKptrRestrictAction(const BuiltinArguments&) {
//!     if (!SetHighestAvailableOptionValue("/proc/sys/kernel/kptr_restrict",
//!                                         /*MINVALUE=*/2, /*MAXVALUE=*/4)) {
//!         LOG(FATAL) << "Unable to set adequate kptr_restrict value!";
//!         return Error();
//!     }
//!     return {};
//! }
//! ```
//!
//! MAXVALUE=4 is a COMPILE-TIME constant — the loop ALWAYS runs 4→3→2 on
//! every boot regardless of what the emulator serves, so there is no
//! content-level early return either. On a real kernel the first verify
//! matches and the loop breaks; under the emulator's synthetic sysctl
//! files the libc++ streambuf divergence breaks the compare on every
//! iteration and init aborts itself.
//!
//! # The patch (validated offline against the REAL A-11 GSI init)
//!
//! The boot-ladder ROM (`android11-aosp-arm64-rsr1`, built from the
//! official arm64-v8a-30_r02 GSI) ships a dynamically-linked second-stage
//! `/system/bin/init` (aarch64 PIE, 1,028,848 bytes). Hand-disassembly
//! around each FATAL message string found the identical compiled shape
//! for BOTH actions:
//!
//! ```text
//! <in-edge, far above the emit>            TBNZ W0, #0, <emit>   ; result!=ok → FATAL
//! <emit entry>                             BL  <ErrnoError-ish>
//!                                          ADRP X1, <file-page>  ; "security.cpp"
//!                                          ADD  X1, X1, #0x4e9
//!                                          MOV  W2, #0xc7        ; line 199
//!                                          MOV  W3, #0x6         ; severity FATAL
//!                                          BL   <LogMessage ctor>
//!                                          ADD  X0, SP, #0x38
//!                                          BL   <LogMessage::operator<<>
//!   <xref>  ADRP X1, <msg-page>  ←────── the FATAL message pointer
//!           ADD  X1, X1, #<msg-lo12>
//!           MOV  W2, #<msg-len>          ; 43 = len("...kptr_restrict value!")
//!           BL   <operator<<(const char*, size_t)>
//!           ADD  X0, SP, #0x38
//!           BL   <~LogMessage>           ; logs + abort() for FATAL
//!           STR  W21, [X20]              ; "return Error()" state store
//!   <term>  B    <common-return-tail>   ; the emit's terminator
//! ```
//!
//! The in-edge (the `TBNZ W0, #0` dispatching on the inlined
//! SetHighestAvailableOptionValue's result) sits FAR above the emit
//! (0x1BC bytes for kptr), so it is found by a FULL executable-segment
//! branch scan whose target lands inside the emit window
//! `[msg_adrp - 0x40, terminator]` — a window that ENDS at the emit's
//! terminator `B` so the epilogue's stack-canary check
//! (`B.NE → __stack_chk_fail`, whose target sits PAST the terminator)
//! is never touched.
//!
//! The rewrite: every in-edge branch becomes an unconditional
//! `B <common-return-tail>` — the tail the emit itself jumps to for its
//! `return Error()`. The tail is the shared Result constructor, so the
//! failure path now returns normally (init logs the failed action and
//! CONTINUES THE BOOT) instead of aborting into InitFatalReboot.
//!
//! **Strict-safety argument**: if `SetHighestAvailableOptionValue` SUCCEEDS
//! (the compare passes — e.g. any future libc++/ROM where the streambuf
//! behaves), the in-edge branch is never taken and the patched word is
//! never executed — the success path is BIT-IDENTICAL. The patch only
//! converts today's guaranteed abort into a clean error return.
//!
//! # The staging-cache interaction (why the caller deletes staged copies)
//!
//! The executed binary is the 6-Z102 staged COPY
//! (`{data_dir}/cache/twoyi_stage/_system_bin_init_<fnv12>`), whose
//! cache-hit test is FILE-LENGTH equality. This patch rewrites ONE word
//! in place (length-preserving), so a stale pre-patch staged copy of the
//! same length would be REUSED and the executed bytes would stay
//! unpatched. The caller therefore deletes the guest path's staged
//! copies whenever it applies a patch, forcing a fresh stage of the
//! patched bytes (the staging engine re-copies on a cache miss).

use crate::info;
use crate::warning;

/// The LOG(FATAL) message literals of `SetKptrRestrictAction` and
/// `SetMmapRndBitsAction` (AOSP 11/12/13 init/security.cpp — byte-identical
/// across all three). The mmap entropy FATAL is pre-empted too: on arm64
/// that action runs `SetHighestAvailableOptionValue
/// ("/proc/sys/vm/mmap_rnd_bits", 24, 33)` — TEN broken verify iterations —
/// and would become the next wall the moment the kptr one falls.
const FATAL_NEEDLES: [&str; 2] = [
    "Unable to set adequate kptr_restrict value!",
    "Unable to set adequate mmap entropy value!",
];

/// AArch64 `NOP` (kept for reference/tests; the rewrite uses `B`).
#[allow(dead_code)]
pub const AARCH64_NOP: u32 = 0xD503_201F;

/// The emit-window half-width around the message ADRP (bytes). The
/// observed emits span `[adrp-0x34 .. adrp+0x1c]` in both actions.
const EMIT_WINDOW_HALF: i64 = 0x40;

/// How many instructions after an `ADRP Xn` to look for the pairing
/// `ADD Xd, Xn, #lo12`.
const ADRP_ADD_GAP: usize = 5;

/// The outcome of [`patch_init_fatal_branches`].
#[derive(Debug, PartialEq, Eq)]
pub enum InitFatalPatchOutcome {
    /// At least one FATAL in-edge was rewritten; `sites` describes each
    /// rewrite (needle, in-edge vaddr, branch kind, tail vaddr).
    Applied { sites: Vec<String> },
    /// The needles were present but every in-edge was already an
    /// unconditional `B` to its emit's tail (a previous boot's patch —
    /// the rewrite is idempotent).
    AlreadyApplied { needles: usize },
    /// The needles exist but no in-edge branch was found (an unrecognised
    /// code layout) — nothing was modified; the boot behaves exactly as
    /// before. This is the SAFE degradation.
    NotFound { needles_present: usize },
    /// The binary cannot carry this patch at all (not ELF64/LSB, no
    /// executable segment, or no needle present) — nothing was modified.
    Skipped(&'static str),
}

/// One ELF64 PT_LOAD segment: (p_vaddr, p_filesz, p_offset, p_flags).
type Segment = (u64, u64, u64, u32);

/// Parse the PT_LOAD program headers of an ELF64 little-endian image.
/// Returns `None` if the bytes are not an ELF64 LSB image or the headers
/// are malformed.
fn elf64_load_segments(bytes: &[u8]) -> Option<Vec<Segment>> {
    if bytes.len() < 64 || &bytes[0..4] != b"\x7fELF" {
        return None;
    }
    if bytes[4] != 2 || bytes[5] != 1 {
        // ELFCLASS64 + ELFDATA2LSB only.
        return None;
    }
    let rd32 = |off: usize| -> u32 {
        u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
    };
    let rd64 = |off: usize| -> u64 {
        let mut v = [0u8; 8];
        v.copy_from_slice(&bytes[off..off + 8]);
        u64::from_le_bytes(v)
    };
    let phoff = rd64(0x20) as usize;
    let phentsize = u16::from_le_bytes([bytes[0x36], bytes[0x37]]) as usize;
    let phnum = u16::from_le_bytes([bytes[0x38], bytes[0x39]]) as usize;
    if phentsize < 56 || phnum == 0 || phnum > 64 {
        return None;
    }
    let mut segs = Vec::with_capacity(phnum);
    for i in 0..phnum {
        let base = phoff + i * phentsize;
        if base + 56 > bytes.len() {
            return None;
        }
        let p_type = rd32(base);
        if p_type != 1 {
            continue; // PT_LOAD
        }
        let p_flags = rd32(base + 4);
        let p_offset = rd64(base + 8);
        let p_vaddr = rd64(base + 16);
        let p_filesz = rd64(base + 32);
        if p_offset as u128 + p_filesz as u128 > bytes.len() as u128 {
            return None;
        }
        segs.push((p_vaddr, p_filesz, p_offset, p_flags));
    }
    if segs.is_empty() {
        return None;
    }
    Some(segs)
}

/// Map a virtual address to a file offset through the PT_LOAD segments.
fn vaddr_to_offset(segs: &[Segment], vaddr: u64) -> Option<usize> {
    for &(va, filesz, off, _flags) in segs {
        if vaddr >= va && vaddr < va.checked_add(filesz)? {
            return Some((off + (vaddr - va)) as usize);
        }
    }
    None
}

fn sign_extend(x: i64, bits: u32) -> i64 {
    let shift = 64 - bits;
    (x << shift) >> shift
}

fn word_at(bytes: &[u8], off: usize) -> Option<u32> {
    if off + 4 > bytes.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        bytes[off],
        bytes[off + 1],
        bytes[off + 2],
        bytes[off + 3],
    ]))
}

/// Decode `ADRP Xd, <page>` → (Rd, page-vaddr). `pc` is the instruction's
/// own vaddr.
fn decode_adrp(w: u32, pc: u64) -> Option<(u32, u64)> {
    if (w & 0x9F00_0000) != 0x9000_0000 {
        return None;
    }
    let rd = w & 0x1F;
    let immlo = (w >> 29) & 0x3;
    let immhi = (w >> 5) & 0x7_FFFF;
    let imm = sign_extend(((immhi << 2) | immlo) as i64, 21) << 12;
    let page = (pc & !0xFFFu64).wrapping_add(imm as u64);
    Some((rd, page))
}

/// Decode `ADD Xd, Xn, #imm12` (64-bit, no shift) → (Rd, Rn, imm12).
fn decode_add_imm(w: u32) -> Option<(u32, u32, u32)> {
    if (w & 0xFF80_0000) != 0x9100_0000 {
        return None;
    }
    Some(((w & 0x1F), ((w >> 5) & 0x1F), (w >> 10) & 0xFFF))
}

/// A decoded control-transfer that can enter the FATAL emit, with its
/// target vaddr.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InEdgeKind {
    Cbz,
    Cbnz,
    Tbz,
    Tbnz,
    BCond(u32),
    B,
}

impl InEdgeKind {
    fn name(self) -> &'static str {
        match self {
            InEdgeKind::Cbz => "CBZ",
            InEdgeKind::Cbnz => "CBNZ",
            InEdgeKind::Tbz => "TBZ",
            InEdgeKind::Tbnz => "TBNZ",
            InEdgeKind::BCond(_) => "B.cond",
            InEdgeKind::B => "B",
        }
    }
}

/// Decode every branch class that can target the emit. Returns
/// (kind, target-vaddr). The unconditional `B` is included because the
/// compiler may funnel the inlined function's `return false` paths
/// through a shared tail jump.
fn decode_in_edge_branch(w: u32, pc: u64) -> Option<(InEdgeKind, u64)> {
    if (w & 0x7E00_0000) == 0x3400_0000 {
        let imm = sign_extend(((w >> 5) & 0x7_FFFF) as i64, 19) << 2;
        let kind = if w & 0x0100_0000 != 0 {
            InEdgeKind::Cbnz
        } else {
            InEdgeKind::Cbz
        };
        return Some((kind, pc.wrapping_add(imm as u64)));
    }
    if (w & 0x7E00_0000) == 0x3600_0000 {
        let imm = sign_extend(((w >> 5) & 0x3_FFFF) as i64, 14) << 2;
        let kind = if w & 0x0100_0000 != 0 {
            InEdgeKind::Tbnz
        } else {
            InEdgeKind::Tbz
        };
        return Some((kind, pc.wrapping_add(imm as u64)));
    }
    if (w & 0xFF00_0010) == 0x5400_0000 {
        let imm = sign_extend(((w >> 5) & 0x7_FFFF) as i64, 19) << 2;
        return Some((InEdgeKind::BCond(w & 0xF), pc.wrapping_add(imm as u64)));
    }
    if (w & 0xFC00_0000) == 0x1400_0000 {
        let imm = sign_extend((w & 0x3FF_FFFF) as i64, 26) << 2;
        return Some((InEdgeKind::B, pc.wrapping_add(imm as u64)));
    }
    None
}

/// Encode `B <target>` from instruction vaddr `pc`.
fn encode_b(pc: u64, target: u64) -> Option<u32> {
    let delta = target as i64 - pc as i64;
    if delta & 0x3 != 0 || delta.abs() > (1 << 26) * 4 {
        return None;
    }
    Some(0x1400_0000 | (((delta >> 2) as u32) & 0x03FF_FFFF))
}

/// Scan the executable segments for the `ADRP+ADD` pair computing
/// `str_vaddr` (the FATAL message pointer load). Returns every ADRP
/// site's vaddr.
fn find_string_xrefs(bytes: &[u8], segs: &[Segment], str_vaddr: u64) -> Vec<u64> {
    let mut xrefs = Vec::new();
    for &(va, filesz, off, flags) in segs {
        if flags & 1 == 0 {
            continue; // not executable
        }
        let seg_bytes = match bytes.get((off as usize)..(off as usize + filesz as usize)) {
            Some(b) => b,
            None => continue,
        };
        for step in 0..(filesz as usize / 4) {
            let pc = va + step as u64 * 4;
            let w = match word_at(seg_bytes, step * 4) {
                Some(w) => w,
                None => break,
            };
            let (rd, page) = match decode_adrp(w, pc) {
                Some(x) => x,
                None => continue,
            };
            for k in 1..=ADRP_ADD_GAP {
                let w2 = match word_at(seg_bytes, step * 4 + k * 4) {
                    Some(w) => w,
                    None => break,
                };
                if let Some((_rdd, rn, imm12)) = decode_add_imm(w2) {
                    if rn == rd && page + imm12 as u64 == str_vaddr {
                        xrefs.push(pc);
                        break;
                    }
                }
            }
        }
    }
    xrefs
}

/// The per-needle patch plan derived for one FATAL message.
struct EmitPlan {
    /// The message ADRP's vaddr (the window anchor).
    msg_adrp_pc: u64,
    /// The emit terminator `B`'s vaddr.
    term_pc: u64,
    /// The common-return-tail vaddr (the terminator's target).
    tail_pc: u64,
}

/// Derive the emit plan for one xref: locate the terminator (the first
/// unconditional `B` at/after the ADRP within the window) and its target
/// (the shared Result-construction tail the `return Error()` path uses).
fn derive_emit_plan(bytes: &[u8], segs: &[Segment], adrp_pc: u64) -> Option<EmitPlan> {
    let adrp_off = vaddr_to_offset(segs, adrp_pc)?;
    for k in 0..(EMIT_WINDOW_HALF as usize / 4) {
        let w = word_at(bytes, adrp_off + k * 4)?;
        let pc = adrp_pc + k as u64 * 4;
        if (w & 0xFC00_0000) == 0x1400_0000 {
            let imm = sign_extend((w & 0x3FF_FFFF) as i64, 26) << 2;
            let tail = pc.wrapping_add(imm as u64);
            // The tail must be OUTSIDE the emit window (it is the shared
            // return path, not another part of the emit).
            let lo = adrp_pc.saturating_sub(EMIT_WINDOW_HALF as u64);
            if tail >= lo && tail <= pc {
                continue; // an intra-emit jump — not the terminator
            }
            return Some(EmitPlan {
                msg_adrp_pc: adrp_pc,
                term_pc: pc,
                tail_pc: tail,
            });
        }
    }
    None
}

/// One branch that currently enters the emit window.
struct InEdge {
    w_off: usize,
    pc: u64,
    kind: InEdgeKind,
    target: u64,
}

/// Full-segment scan for branches whose target lands inside `plan`'s
/// emit window (branches INSIDE the window are excluded — they are the
/// emit's own control flow).
fn collect_in_edges(bytes: &[u8], segs: &[Segment], plan: &EmitPlan) -> Vec<InEdge> {
    let win_lo = plan.msg_adrp_pc.saturating_sub(EMIT_WINDOW_HALF as u64);
    let win_hi = plan.term_pc;
    let mut edges = Vec::new();
    for &(va, filesz, off, flags) in segs.iter() {
        if flags & 1 == 0 {
            continue; // not executable
        }
        for step in 0..(filesz as usize / 4) {
            let w_off = off as usize + step * 4;
            let pc = va + step as u64 * 4;
            if pc >= win_lo && pc <= win_hi {
                continue; // branches INSIDE the emit stay as-is
            }
            let w = match word_at(bytes, w_off) {
                Some(w) => w,
                None => break,
            };
            let (kind, target) = match decode_in_edge_branch(w, pc) {
                Some(x) => x,
                None => continue,
            };
            if target < win_lo || target > win_hi {
                continue;
            }
            if pc.checked_add(4).map(|n| n > va + filesz).unwrap_or(true) {
                continue;
            }
            edges.push(InEdge {
                w_off,
                pc,
                kind,
                target,
            });
        }
    }
    edges
}

/// Idempotency evidence: an unconditional `B tail_pc` OUTSIDE the emit
/// window. A previous patch run rewrites the in-edge into exactly that
/// shape, so its presence (with the in-edge scan empty) means this
/// binary was already patched. The emit's own terminator `B` lives
/// INSIDE the window and never counts.
fn has_b_to_tail_evidence(bytes: &[u8], segs: &[Segment], plan: &EmitPlan) -> bool {
    let win_lo = plan.msg_adrp_pc.saturating_sub(EMIT_WINDOW_HALF as u64);
    let win_hi = plan.term_pc;
    for &(va, filesz, off, flags) in segs.iter() {
        if flags & 1 == 0 {
            continue;
        }
        for step in 0..(filesz as usize / 4) {
            let w_off = off as usize + step * 4;
            let pc = va + step as u64 * 4;
            if pc >= win_lo && pc <= win_hi {
                continue;
            }
            let w = match word_at(bytes, w_off) {
                Some(w) => w,
                None => break,
            };
            if let Some((InEdgeKind::B, target)) = decode_in_edge_branch(w, pc) {
                if target == plan.tail_pc {
                    return true;
                }
            }
        }
    }
    false
}

/// Patch ONE binary's FATAL in-edges. See the module docs for the full
/// algorithm and its safety argument.
pub fn patch_init_fatal_branches(init_bytes: &mut [u8]) -> InitFatalPatchOutcome {
    let segs = match elf64_load_segments(init_bytes) {
        Some(s) => s,
        None => return InitFatalPatchOutcome::Skipped("not an ELF64 LSB image"),
    };
    let has_exec = segs.iter().any(|&(_, _, _, flags)| flags & 1 != 0);
    if !has_exec {
        return InitFatalPatchOutcome::Skipped("no executable segment");
    }

    // The strings' vaddrs (the strings live in a read-only LOAD segment;
    // a PIE built from the GSI maps file offset == vaddr for that
    // segment, but resolve through the headers anyway).
    let mut needles_present = 0usize;
    let mut sites: Vec<String> = Vec::new();
    let mut patched_words: Vec<(usize, u32)> = Vec::new();
    let mut already_evidence = false;

    for needle in FATAL_NEEDLES.iter() {
        let nb = needle.as_bytes();
        let file_off = match init_bytes.windows(nb.len()).position(|w| w == nb) {
            Some(o) => o,
            None => continue,
        };
        // The file offset → vaddr: search every segment containing it.
        let str_vaddr = match segs.iter().find_map(|&(va, filesz, off, _)| {
            if file_off as u64 >= off && (file_off as u64) < off + filesz {
                Some(va + (file_off as u64 - off))
            } else {
                None
            }
        }) {
            Some(v) => v,
            None => continue,
        };
        needles_present += 1;

        for adrp_pc in find_string_xrefs(init_bytes, &segs, str_vaddr) {
            let plan = match derive_emit_plan(init_bytes, &segs, adrp_pc) {
                Some(p) => p,
                None => {
                    info!(
                        "[KR64][init_patch] 6-Z537: {} xref at {:#x} has no emit terminator — skipped",
                        needle, adrp_pc
                    );
                    continue;
                }
            };

            // The in-edge scan FIRST (the unpatched shape).
            let edges = collect_in_edges(init_bytes, &segs, &plan);
            if edges.is_empty() {
                // No branch enters the emit anymore: either an alien
                // layout or a previous patch run (the in-edge became
                // `B tail`). The tail evidence decides.
                if has_b_to_tail_evidence(init_bytes, &segs, &plan) {
                    already_evidence = true;
                }
                continue;
            }
            for edge in &edges {
                match encode_b(edge.pc, plan.tail_pc) {
                    Some(new_w) if new_w != word_at(init_bytes, edge.w_off).unwrap_or(!0) => {
                        patched_words.push((edge.w_off, new_w));
                        sites.push(format!(
                            "{} in-edge {}@{:#x} (target {:#x}) -> B {:#x} (emit adrp {:#x})",
                            needle,
                            edge.kind.name(),
                            edge.pc,
                            edge.target,
                            plan.tail_pc,
                            plan.msg_adrp_pc
                        ));
                    }
                    Some(_) => {
                        // The word is ALREADY `B tail` — a previous
                        // patch run (idempotency).
                        already_evidence = true;
                    }
                    None => {
                        warning!(
                            "[KR64][init_patch] 6-Z537: cannot encode B {:#x} from {:#x} — skipped",
                            plan.tail_pc,
                            edge.pc
                        );
                    }
                }
            }
        }
    }

    if patched_words.is_empty() {
        if already_evidence {
            return InitFatalPatchOutcome::AlreadyApplied {
                needles: needles_present,
            };
        }
        if needles_present == 0 {
            return InitFatalPatchOutcome::Skipped("no FATAL needle strings present");
        }
        return InitFatalPatchOutcome::NotFound { needles_present };
    }

    for (w_off, new_w) in patched_words {
        let b = new_w.to_le_bytes();
        init_bytes[w_off..w_off + 4].copy_from_slice(&b);
    }
    InitFatalPatchOutcome::Applied { sites }
}

/// Patch the guest's second-stage init in the rootfs on disk, deleting
/// the staged copies so the executed bytes pick the patch up.
///
/// Candidates: the caller-provided `init_path` (the emu's spawn target —
/// usually the FIRST-stage `/init`) PLUS the AOSP second-stage
/// `/system/bin/init` — the second stage is the binary that runs the
/// SetKptrRestrict/SetMmapRndBits actions (validated: the A-11 rootfs's
/// static `/init` carries NONE of the FATAL needles; the dynamic
/// `/system/bin/init` carries both).
pub fn patch_guest_init_fatal_branches(rootfs: &str, data_dir: &str, init_path: &str) {
    let mut candidates: Vec<&str> = Vec::new();
    if !init_path.is_empty() {
        candidates.push(init_path);
    }
    if !candidates.contains(&"/system/bin/init") {
        candidates.push("/system/bin/init");
    }

    for guest_path in candidates {
        let file_path = format!("{}{}", rootfs, guest_path);
        let mut bytes = match std::fs::read(&file_path) {
            Ok(b) => b,
            Err(e) => {
                info!(
                    "[KR64][init_patch] 6-Z537: {} unreadable ({} — no patch needed)",
                    file_path, e
                );
                continue;
            }
        };
        match patch_init_fatal_branches(&mut bytes) {
            InitFatalPatchOutcome::Applied { sites } => {
                match std::fs::write(&file_path, &bytes) {
                    Ok(()) => {
                        for s in &sites {
                            info!("[KR64][init_patch] 6-Z537: PATCHED {} — {}", file_path, s);
                        }
                        // Length-preserving patch → the 6-Z102 staging
                        // cache (length-keyed) would REUSE a stale
                        // pre-patch copy. Delete the guest path's staged
                        // copies to force a fresh stage of the patched
                        // bytes.
                        delete_staged_copies(data_dir, guest_path);
                        info!(
                            "[KR64][init_patch] 6-Z537: {} — the SetKptrRestrict/SetMmapRndBits \
                             FATAL is neutered (the verify-loop streambuf divergence can no longer \
                             abort the boot)",
                            file_path
                        );
                    }
                    Err(e) => warning!(
                        "[KR64][init_patch] 6-Z537: patched {} in memory but the write-back failed: {}",
                        file_path,
                        e
                    ),
                }
            }
            InitFatalPatchOutcome::AlreadyApplied { needles } => {
                info!(
                    "[KR64][init_patch] 6-Z537: {} already patched ({} needle(s) — idempotent skip)",
                    file_path, needles
                );
            }
            InitFatalPatchOutcome::NotFound { needles_present } => {
                info!(
                    "[KR64][init_patch] 6-Z537: {} — {} needle(s) present but no in-edge matched \
                     (unrecognised layout; the boot behaves exactly as before)",
                    file_path, needles_present
                );
            }
            InitFatalPatchOutcome::Skipped(reason) => {
                info!(
                    "[KR64][init_patch] 6-Z537: {} skipped ({} — expected for first-stage/static \
                     or non-AOSP inits)",
                    file_path, reason
                );
            }
        }
    }
}

/// Delete the 6-Z102 staged copies of `guest_path` under
/// `{data_dir}/cache/twoyi_stage/` — the cache name is the sanitized
/// guest path + `_` + a 12-hex FNV suffix, so glob `{stem}_*`.
fn delete_staged_copies(data_dir: &str, guest_path: &str) {
    let stem: String = guest_path
        .chars()
        .take(64)
        .map(|c| {
            if c == '/' || c == '\0' || c == '\n' || c == '\r' || c == '\t' || c == ' ' {
                '_'
            } else {
                c
            }
        })
        .collect();
    let dir = format!("{}/cache/twoyi_stage", data_dir);
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return, // no staging cache yet — nothing to invalidate
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = match name.to_str() {
            Some(n) => n,
            None => continue,
        };
        if name.len() > stem.len() + 1
            && name.starts_with(&stem)
            && name.as_bytes()[stem.len()] == b'_'
        {
            if std::fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    if removed > 0 {
        info!(
            "[KR64][init_patch] 6-Z537: invalidated {} stale staged copy(ies) of {} (the \
             length-keyed staging cache would otherwise reuse the unpatched bytes)",
            removed, guest_path
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── synthetic-ELF fixture machinery ─────────────────────────────
    //
    // A minimal ELF64-LSB aarch64 image with two PT_LOAD segments:
    //   seg R  (flags=4): vaddr 0x0000 — carries the needle string
    //   seg RX (flags=5): vaddr 0x2000 — carries the compiled shape
    // (file offset == vaddr for both, matching the GSI init's PIE
    // identity mapping.)

    const RX_VA: u64 = 0x2000;
    const STR_VA: u64 = 0x0500;
    const EMIT_VA: u64 = 0x2100;
    const TAIL_VA: u64 = 0x2040;
    const IN_EDGE_VA: u64 = 0x2600;
    const STACKCHK_VA: u64 = 0x2700; // B.NE target PAST the terminator
    const STACKCHK_BR_VA: u64 = 0x2800; // the B.NE site

    const NEEDLE_KPTR: &str = "Unable to set adequate kptr_restrict value!";

    fn adrp(rd: u32, pc: u64, page: u64) -> u32 {
        // imm = page - (pc & !0xFFF), in 4KB units, 21-bit signed.
        let imm = (page as i64) - ((pc & !0xFFFu64) as i64);
        assert_eq!(imm & 0xFFF, 0);
        let imm21 = (imm >> 12) & 0x1F_FFFF;
        let immlo = (imm21 & 0x3) as u32;
        let immhi = ((imm21 >> 2) & 0x7_FFFF) as u32;
        0x9000_0000 | (immlo << 29) | (immhi << 5) | rd
    }

    fn add_imm(rd: u32, rn: u32, imm12: u32) -> u32 {
        0x9100_0000 | (imm12 << 10) | (rn << 5) | rd
    }

    fn b(pc: u64, target: u64) -> u32 {
        encode_b(pc, target).unwrap()
    }

    fn tbnz(rt: u32, bit: u32, pc: u64, target: u64) -> u32 {
        let imm14 = ((target as i64 - pc as i64) >> 2) as u32 & 0x3FFF;
        0x3700_0000 | ((bit & 0x1F) << 19) | (imm14 << 5) | rt | 0x0100_0000
    }

    fn b_cond(cond: u32, pc: u64, target: u64) -> u32 {
        let imm19 = (((target as i64 - pc as i64) >> 2) as u32) & 0x7_FFFF;
        0x5400_0000 | (imm19 << 5) | cond
    }

    fn movz32(rd: u32, imm16: u32) -> u32 {
        0x5280_0000 | (imm16 << 5) | rd
    }

    fn bl(pc: u64, target: u64) -> u32 {
        let delta = (target as i64 - pc as i64) >> 2;
        0x9400_0000 | ((delta as u32) & 0x03FF_FFFF)
    }

    fn ret() -> u32 {
        0xD65F_03C0
    }

    /// Build the fixture image with the requested in-edge word at
    /// IN_EDGE_VA.
    fn build_fixture(in_edge_word: u32) -> Vec<u8> {
        let total = 0x4000usize;
        let mut b_ = vec![0u8; total];

        // ELF header.
        b_[0..4].copy_from_slice(b"\x7fELF");
        b_[4] = 2; // ELFCLASS64
        b_[5] = 1; // ELFDATA2LSB
        b_[6] = 1; // EV_CURRENT
                   // e_type/e_machine left 0 (the parser does not check them).
        b_[0x20..0x28].copy_from_slice(&64u64.to_le_bytes()); // e_phoff
        b_[0x36..0x38].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
        b_[0x38..0x3a].copy_from_slice(&2u16.to_le_bytes()); // e_phnum

        // phdr[0]: R segment covering the string (vaddr 0, filesz 0x1000).
        b_[64..68].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD
        b_[68..72].copy_from_slice(&4u32.to_le_bytes()); // p_flags R
        b_[72..80].copy_from_slice(&0u64.to_le_bytes()); // p_offset
        b_[80..88].copy_from_slice(&0u64.to_le_bytes()); // p_vaddr
        b_[96..104].copy_from_slice(&0x1000u64.to_le_bytes()); // p_filesz

        // phdr[1]: RX segment at 0x2000, filesz 0x2000.
        b_[120..124].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD
        b_[124..128].copy_from_slice(&5u32.to_le_bytes()); // p_flags RX
        b_[128..136].copy_from_slice(&0x2000u64.to_le_bytes()); // p_offset
        b_[136..144].copy_from_slice(&RX_VA.to_le_bytes()); // p_vaddr
        b_[152..160].copy_from_slice(&0x2000u64.to_le_bytes()); // p_filesz

        // The needle string at vaddr 0x500.
        b_[STR_VA as usize..STR_VA as usize + NEEDLE_KPTR.len()]
            .copy_from_slice(NEEDLE_KPTR.as_bytes());

        // The compiled emit shape at EMIT_VA (mirrors the real init):
        //   +0x00 BL   <setup>            (emit entry)
        //   +0x04 ADRP X1, file-page
        //   +0x08 ADD  X1, X1, #0x4e9     ("security.cpp" lo12)
        //   +0x0c MOV  W2, #0xc7          (line 199)
        //   +0x10 MOV  W3, #6             (FATAL)
        //   +0x14 BL   <LogMessage ctor>
        //   +0x18 ADRP X1, msg-page  ← the XREF
        //   +0x1c ADD  X1, X1, #lo12(msg)
        //   +0x20 MOV  W2, #len(msg)
        //   +0x24 BL   <operator<<(char*, len)>
        //   +0x28 BL   <~LogMessage>
        //   +0x2c B    <tail>             (the terminator)
        let msg_page = STR_VA & !0xFFF;
        let msg_lo12 = (STR_VA & 0xFFF) as u32;
        let file_str_va: u64 = 0x4e9; // an arbitrary page-0 string
        let words: [u32; 12] = [
            bl(EMIT_VA, 0x3000),
            adrp(1, EMIT_VA + 0x04, file_str_va & !0xFFF),
            add_imm(1, 1, (file_str_va & 0xFFF) as u32),
            movz32(2, 0xc7),
            movz32(3, 6),
            bl(EMIT_VA + 0x14, 0x3100),
            adrp(1, EMIT_VA + 0x18, msg_page),
            add_imm(1, 1, msg_lo12),
            movz32(2, NEEDLE_KPTR.len() as u32),
            bl(EMIT_VA + 0x24, 0x3200),
            bl(EMIT_VA + 0x28, 0x3300),
            b(EMIT_VA + 0x2c, TAIL_VA),
        ];
        for (i, w) in words.iter().enumerate() {
            let off = EMIT_VA as usize + i * 4;
            b_[off..off + 4].copy_from_slice(&w.to_le_bytes());
        }

        // The in-edge far below/above the emit.
        b_[IN_EDGE_VA as usize..IN_EDGE_VA as usize + 4]
            .copy_from_slice(&in_edge_word.to_le_bytes());

        // The stack-canary shape: a B.NE whose target sits PAST the
        // terminator (must NEVER be touched).
        b_[STACKCHK_BR_VA as usize..STACKCHK_BR_VA as usize + 4]
            .copy_from_slice(&b_cond(1, STACKCHK_BR_VA, STACKCHK_VA).to_le_bytes());
        b_[STACKCHK_VA as usize..STACKCHK_VA as usize + 4]
            .copy_from_slice(&bl(STACKCHK_VA, 0x3400).to_le_bytes());

        // The tail: a plausible Result-construction sequence + RET.
        b_[TAIL_VA as usize..TAIL_VA as usize + 4].copy_from_slice(&movz32(0, 0).to_le_bytes());
        b_[TAIL_VA as usize + 4..TAIL_VA as usize + 8].copy_from_slice(&ret().to_le_bytes());

        b_
    }

    fn word(img: &[u8], va: u64) -> u32 {
        u32::from_le_bytes([
            img[va as usize],
            img[va as usize + 1],
            img[va as usize + 2],
            img[va as usize + 3],
        ])
    }

    // ── tests ───────────────────────────────────────────────────────

    /// 6-Z537 core: the TBNZ in-edge is rewritten to `B tail`; the emit,
    /// terminator, tail, and stack-canary branch are untouched.
    #[test]
    fn z537_tbnz_in_edge_is_redirected_to_tail() {
        let orig_in_edge = tbnz(0, 0, IN_EDGE_VA, EMIT_VA);
        let mut img = build_fixture(orig_in_edge);

        let out = patch_init_fatal_branches(&mut img);
        match &out {
            InitFatalPatchOutcome::Applied { sites } => {
                assert_eq!(sites.len(), 1, "one in-edge patched: {:?}", sites);
                assert!(sites[0].contains("TBNZ"), "kind named: {}", sites[0]);
                assert!(sites[0].contains(NEEDLE_KPTR), "needle named: {}", sites[0]);
            }
            other => panic!("expected Applied, got {:?}", other),
        }

        // The in-edge is now an unconditional B to the tail.
        assert_eq!(word(&img, IN_EDGE_VA), b(IN_EDGE_VA, TAIL_VA));
        // The emit bytes are untouched.
        assert_eq!(
            word(&img, EMIT_VA + 0x18),
            adrp(1, EMIT_VA + 0x18, STR_VA & !0xFFF)
        );
        assert_eq!(word(&img, EMIT_VA + 0x2c), b(EMIT_VA + 0x2c, TAIL_VA));
        // The stack-canary branch is untouched.
        assert_eq!(
            word(&img, STACKCHK_BR_VA),
            b_cond(1, STACKCHK_BR_VA, STACKCHK_VA)
        );
        // The tail is untouched.
        assert_eq!(word(&img, TAIL_VA), movz32(0, 0));
    }

    /// The in-edge targeting anywhere INSIDE the emit window (not just
    /// the entry) is redirected — and a B.cond in-edge works too.
    #[test]
    fn z537_bcond_in_edge_mid_emit_target_is_redirected() {
        // Target the terminator-1 instruction (inside the window).
        let orig = b_cond(0, IN_EDGE_VA, EMIT_VA + 0x28);
        let mut img = build_fixture(orig);
        let out = patch_init_fatal_branches(&mut img);
        match &out {
            InitFatalPatchOutcome::Applied { sites } => {
                assert_eq!(sites.len(), 1);
                assert!(sites[0].contains("B.cond"), "kind named: {}", sites[0]);
            }
            other => panic!("expected Applied, got {:?}", other),
        }
        assert_eq!(word(&img, IN_EDGE_VA), b(IN_EDGE_VA, TAIL_VA));
    }

    /// The rewrite is IDEMPOTENT: a second run reports AlreadyApplied
    /// and leaves the image byte-identical.
    #[test]
    fn z537_second_run_is_already_applied() {
        let orig = tbnz(0, 0, IN_EDGE_VA, EMIT_VA);
        let mut img = build_fixture(orig);
        assert!(matches!(
            patch_init_fatal_branches(&mut img),
            InitFatalPatchOutcome::Applied { .. }
        ));
        let patched = img.clone();
        match patch_init_fatal_branches(&mut img) {
            InitFatalPatchOutcome::AlreadyApplied { needles } => assert_eq!(needles, 1),
            other => panic!("expected AlreadyApplied, got {:?}", other),
        }
        assert_eq!(img, patched, "no further bytes changed");
    }

    /// An image without the needle strings is Skipped — never patched.
    #[test]
    fn z537_no_needles_is_skipped() {
        let mut img = build_fixture(tbnz(0, 0, IN_EDGE_VA, EMIT_VA));
        // Wipe the string.
        let n = NEEDLE_KPTR.len();
        img[STR_VA as usize..STR_VA as usize + n].copy_from_slice(&vec![0x41u8; n]);
        let before = img.clone();
        match patch_init_fatal_branches(&mut img) {
            InitFatalPatchOutcome::Skipped(reason) => {
                assert!(reason.contains("no FATAL needle"), "{}", reason);
            }
            other => panic!("expected Skipped, got {:?}", other),
        }
        assert_eq!(img, before);
    }

    /// A malformed image (bad magic / class) is Skipped.
    #[test]
    fn z537_non_elf_is_skipped() {
        let mut img = build_fixture(tbnz(0, 0, IN_EDGE_VA, EMIT_VA));
        img[0..4].copy_from_slice(b"\x7fELX");
        assert_eq!(
            patch_init_fatal_branches(&mut img),
            InitFatalPatchOutcome::Skipped("not an ELF64 LSB image")
        );
        // 32-bit class.
        let mut img32 = build_fixture(tbnz(0, 0, IN_EDGE_VA, EMIT_VA));
        img32[4] = 1;
        assert_eq!(
            patch_init_fatal_branches(&mut img32),
            InitFatalPatchOutcome::Skipped("not an ELF64 LSB image")
        );
    }

    /// The mmap-entropy needle is handled by the same machinery.
    #[test]
    fn z537_mmap_entropy_needle_is_patched() {
        const MMAP_NEEDLE: &str = "Unable to set adequate mmap entropy value!";
        let mut img = build_fixture(tbnz(0, 0, IN_EDGE_VA, EMIT_VA));
        // Place the mmap needle at another R-segment vaddr and give it
        // its own xref: reuse the SAME emit (the fixture's emit carries
        // only the kptr xref, so build a second ADRP+ADD for the mmap
        // string at 0x610 and a second in-edge at 0x2610).
        img[0x610..0x610 + MMAP_NEEDLE.len()].copy_from_slice(MMAP_NEEDLE.as_bytes());
        let str2_va: u64 = 0x610;
        let emit2_va: u64 = EMIT_VA + 0x40;
        let words: [u32; 4] = [
            bl(emit2_va, 0x3000),
            adrp(1, emit2_va + 0x04, str2_va & !0xFFF),
            add_imm(1, 1, (str2_va & 0xFFF) as u32),
            b(emit2_va + 0x0c, TAIL_VA),
        ];
        for (i, w) in words.iter().enumerate() {
            let off = emit2_va as usize + i * 4;
            img[off..off + 4].copy_from_slice(&w.to_le_bytes());
        }
        let in2_va: u64 = 0x2610;
        img[in2_va as usize..in2_va as usize + 4]
            .copy_from_slice(&tbnz(0, 0, in2_va, emit2_va).to_le_bytes());

        let out = patch_init_fatal_branches(&mut img);
        match &out {
            InitFatalPatchOutcome::Applied { sites } => {
                assert_eq!(sites.len(), 2, "both needles patched: {:?}", sites);
                assert!(sites.iter().any(|s| s.contains("mmap entropy")));
            }
            other => panic!("expected Applied with 2 sites, got {:?}", other),
        }
        assert_eq!(word(&img, in2_va), b(in2_va, TAIL_VA));
    }

    /// The `delete_staged_copies` helper only removes files matching the
    /// sanitized guest-path stem + `_` + suffix, leaving siblings alone.
    #[test]
    fn z537_staged_copy_invalidation_is_targeted() {
        let tmp = std::env::temp_dir().join(format!("z537_stage_test_{}", std::process::id()));
        let stage = tmp.join("cache/twoyi_stage");
        std::fs::create_dir_all(&stage).unwrap();

        // The target's staged copy + its marker-ish sibling.
        std::fs::write(stage.join("_system_bin_init_8d58dbd62cc8"), b"stale").unwrap();
        // A DIFFERENT binary's staged copy (must survive).
        std::fs::write(stage.join("_system_bin_sh_0123456789ab"), b"keep").unwrap();
        // A prefix-collision without the `_`+hash shape (must survive).
        std::fs::write(stage.join("_system_bin_initx_0123456789ab"), b"keep").unwrap();

        delete_staged_copies(tmp.to_str().unwrap(), "/system/bin/init");

        assert!(!stage.join("_system_bin_init_8d58dbd62cc8").exists());
        assert!(stage.join("_system_bin_sh_0123456789ab").exists());
        assert!(stage.join("_system_bin_initx_0123456789ab").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Offline validation against the REAL android11-aosp-arm64-rsr1
    /// second-stage init (validated during development with the patch
    /// sites hand-disassembled). Env-gated so the CI (which has no ROM
    /// on disk) skips it silently; set `Z537_REAL_INIT=<path>` to run.
    ///
    /// Expected on the rsr1 binary:
    ///   - kptr in-edge: TBNZ @0xaff80 → B 0xaff84
    ///   - mmap in-edge: TBNZ @0xafd0c → B 0xafd10
    #[test]
    fn z537_real_a11_init_offline_validation() {
        let path = match std::env::var("Z537_REAL_INIT") {
            Ok(p) => p,
            Err(_) => return, // CI: no ROM on disk — skip silently.
        };
        let original = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return,
        };
        assert_eq!(original.len(), 1_028_848, "the rsr1 second-stage init");
        let expect_kptr = word_at(&original, 0xaff80).unwrap();
        assert_eq!(expect_kptr, 0x3700_0c40, "kptr TBNZ W0,#0 → 0xb0108");
        let expect_mmap = word_at(&original, 0xafd0c).unwrap();
        assert_eq!(expect_mmap, 0x3700_0bc0, "mmap TBNZ W0,#0 → 0xafe84");

        let mut bytes = original.clone();
        let out = patch_init_fatal_branches(&mut bytes);
        match &out {
            InitFatalPatchOutcome::Applied { sites } => assert_eq!(sites.len(), 2),
            other => panic!("expected Applied with 2 sites, got {:?}", other),
        }
        // kptr: TBNZ @0xaff80 → B 0xaff84 (= the next instruction).
        assert_eq!(word_at(&bytes, 0xaff80).unwrap(), 0x1400_0001);
        // mmap: TBNZ @0xafd0c → B 0xafd10 (= the next instruction).
        assert_eq!(word_at(&bytes, 0xafd0c).unwrap(), 0x1400_0001);
        // Exactly the in-edge bytes differ: 0x37000c40 → 0x14000001 and
        // 0x37000bc0 → 0x14000001 differ in 3 of their 4 bytes each
        // (byte 2 is 0x00 in both) → 6 bytes total.
        let mut diffs = 0usize;
        for (a, b) in original.iter().zip(bytes.iter()) {
            if a != b {
                diffs += 1;
            }
        }
        assert_eq!(
            diffs, 6,
            "two 3-byte in-edge rewrites, nothing else: {:?}",
            out
        );
        // Idempotent re-run.
        let patched = bytes.clone();
        match patch_init_fatal_branches(&mut bytes) {
            InitFatalPatchOutcome::AlreadyApplied { needles } => assert_eq!(needles, 2),
            other => panic!("expected AlreadyApplied, got {:?}", other),
        }
        assert_eq!(bytes, patched);
    }
}
