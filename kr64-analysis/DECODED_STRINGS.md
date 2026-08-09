# libkr64.so — XOR-decoded string catalog

## .rodata strings (no XOR — plaintext)
shadowhook-enter
shadowhook-exit
shadowhook_tag
exit: alloc %s library, exit %lx, pc %lx, distance %lx, range [-%zx, %zx]
exit: alloc crashed
exit: gap, %lx - %lx (load_bias %lx, %lx - %lx), NFZ %d, READABLE %d
exit: gap fill zero, %lx - %lx (load_bias %lx, %lx - %lx), READABLE %d
exit: gap resize, %lx - %lx (load_bias %lx, %lx - %lx)
exit: in-library alloc, at %lx (load_bias %lx, %lx), len %zu
exit: free crashed
shadowhook-hub-trampo
hub: fill in code crashed
hub: create trampo for target_addr %lx at %lx, size %zu + %zu = %zu
hub: add(re-enable) func %lx
hub: add(new) func %lx
hub: del func %lx
shadowhook-hub-stack
linker64
__dl_g_dl_mutex
__dl__ZL10g_dl_mutex
__dl__Z9do_dlopenPKciPK17android_dlextinfoPKv
__dl__Z9do_dlopenPKciPK17android_dlextinfoPv
__dl__Z9do_dlopenPKciPK17android_dlextinfo
dlopen
linker: hook dlopen %s, return: %d
FAILED
task: get dlinfo by target addr: target_addr %p, sym_name %s, sym_sz %zu, load_bias %lx, pathname %s
(NULL)
unknown
%lx,
9999-99-99T00:00:00.000+00:00,
error,
%04d-%02d-%02dT%02d:%02d:%02d.%03ld%c%02ld:%02ld,
error
hook_sym_addr
hook_sym_name
unhook
libc.so
pthread_getspecific
pthread_setspecific
abort
switch: hook in %s mode OK: target_addr %lx, new_addr %lx
UNIQUE
SHARED
switch: hook(invisible) in %s mode OK: target_addr %lx, new_addr %lx
switch: unhook in UNIQUE mode OK: target_addr %lx
switch: unhook in SHARED mode OK: target_addr %lx, new_addr %lx
task: hook dlopen/do_dlopen internal. target-address %lx
task: start monitor %s, return: %d
shadowhook-task
task: dliterate crashed
shadowhook version 1.0.8
%s: shadowhook init(mode: %s, debuggable: %s), return: %d, real-init: %s
true
false
shadowhook: unhook(%p) ...
shadowhook: unhook(%p) OK
shadowhook: unhook(%p) FAILED. %d - %s
shadowhook: dlopen crashed - %s
shadowhook: dlsym_dynsym crashed - %p, %s
shadowhook: dlsym_symtab crashed - %p, %s
shadowhook: hook_%s_addr(%p, %p) ...
func
shadowhook: hook_%s_addr(%p, %p) OK. return: %p
shadowhook: hook_%s_addr(%p, %p) FAILED. %d - %s
shadowhook: hook_sym_name(%s, %s, %p) ...
shadowhook: hook_sym_name(%s, %s, %p) OK. return: %p. %d - %s
shadowhook: hook_sym_name(%s, %s, %p) FAILED. %d - %s
sigaction64
sigprocmask64
sigaction
sigprocmask
Pending task
Not initialized
Invalid argument
Out of memory
MProtect failed
Write to arbitrary address crashed
Init errno mod failed
Init bytesig SIGSEGV mod failed
Init bytesig SIGBUS mod failed
Init enter mod failed
Init safe mod failed
Init linker mod failed
Init hub mod failed
Create hub failed
Monitor dlopen failed
Create monitor thread failed
Open ELF crashed
Find symbol in ELF failed
Find symbol in ELF crashed
Duplicate hook
Dladdr crashed
Find dlinfo failed
Symbol size too small
Alloc enter failed
Instruction rewrite crashed
Instruction rewrite failed
Switch not found
Verify original instruction crashed
Verify original instruction failed
Exit instruction mismatch
Free exit crashed
Unhook on an error status task
Unhook on an unfinished task
ELF with an unsupported architecture
Linker with an unsupported architecture
Unknown error number
ro.build.version.sdk
/system/build.prop
ro.build.version.sdk=
(null)
/system/bin/linker64
[vdso]
app_process64
/system/bin/app_process64
%s/%s
/system/lib64
.symtab
.gnu_debugdata

## .data strings (XOR-decoded, key per string)
--- Key 0x03 (3) — 5 hits ---
  @0x00002460 [/data/                        ] ........;.........../data/data/com.clone.android.dual.space/vm/vm%
  @0x00002465 [/data/                        ] ...;.........../data/data/com.clone.android.dual.space/vm/vm%d/dev
  @0x00000e28 [/dev/                         ] .{{TVKTAVPMAW{{$..../dev/__kmsg__...O...O??....R??`..IH[.^BNFHY-.
  @0x0000248f [/dev/                         ] d.dual.space/vm/vm%d/dev/touch.......QJxSHRDO'...................
  @0x0000246f [clone                         ] ...../data/data/com.clone.android.dual.space/vm/vm%d/dev/touch...
--- Key 0x05 (5) — 3 hits ---
  @0x00000958 [/proc/                        ] ..................../proc/self/exe...................RS@.B[FPE6...
  @0x00000958 [/proc/self/                   ] ..................../proc/self/exe...................RS@.B[FPE6........
  @0x00000958 [/proc/self/exe                ] ..................../proc/self/exe...................RS@.B[FPE6...........
--- Key 0x06 (6) — 1 hits ---
  @0x0000279c [ION                           ] CATEDaTTRIBUTES.{OPTIONAL} ....................................
--- Key 0x0a (10) — 4 hits ---
  @0x00000d48 [/dev/ashmem                   ] ...R......"X."X.}.../dev/ashmem........................................
  @0x00000d48 [/dev/                         ] ...R......"X."X.}.../dev/ashmem..................................
  @0x00000d4e [shmem                         ] ...."X."X.}.../dev/ashmem........................................
  @0x00000d4d [ashmem                        ] ....."X."X.}.../dev/ashmem........................................
--- Key 0x0c (12) — 1 hits ---
  @0x000005e0 [mount                         ] T...{...............mount_mgr: /dev is special, skip.............
--- Key 0x0d (13) — 1 hits ---
  @0x00000fb4 [/sys/                         ] A....A...1......n.../sys/.......................7|yly7|yly7{wu6{t
--- Key 0x15 (21) — 8 hits ---
  @0x00000e78 [/dev/ashmem                   ] ..................../dev/ashmemsim..................................S..
  @0x00000fd0 [/data/                        ] ..................../data/data/com.clone.android.dual.space.......
  @0x00000fd5 [/data/                        ] .............../data/data/com.clone.android.dual.space.........r9<
  @0x000008e8 [/proc/                        ] ........]....]r...../proc/%d.................................Y[FJ.
  @0x00000e78 [/dev/                         ] ..................../dev/ashmemsim...............................
  @0x00000fdf [clone                         ] ...../data/data/com.clone.android.dual.space.........r9<)<r9<)<r>
  @0x00000e7e [shmem                         ] ............../dev/ashmemsim..................................S..
  @0x00000e7d [ashmem                        ] .............../dev/ashmemsim..................................S..
--- Key 0x17 (23) — 1 hits ---
  @0x00000421 [event                         ] ...................ueventd.........`bxcyR`j.7-(~- 3-(~- 3-(~.....
--- Key 0x19 (25) — 1 hits ---
  @0x00000bf4 [kr64                          ] t"qt"qt"qt"qt"qQ....kr64........_FX[UPQF..4.....................
--- Key 0x1a (26) — 1 hits ---
  @0x00000430 [mount                         ] ....xh{hcyi.........mount_mgr: %s -> %s -> %s.......C...C.....l..
--- Key 0x1c (28) — 2 hits ---
  @0x00000dd0 [/proc/                        ] Z..9................/proc/%d/%s.e./<eJ............................
  @0x00000dd0 [/proc/%d/                     ] Z..9................/proc/%d/%s.e./<eJ...............................
--- Key 0x1d (29) — 3 hits ---
  @0x000006b3 [ion                           ] .mount_mgr: propagation %s not supported.QHUCV%..%'=&<.%/:rh*!&
  @0x000006ab [prop                          ] ..j......mount_mgr: propagation %s not supported.QHUCV%..%'=&<.%
  @0x000006a0 [mount                         ] .J...J.......j......mount_mgr: propagation %s not supported.QHUCV
--- Key 0x1e (30) — 5 hits ---
  @0x00001400 [/data/                        ] ....ZQ@PQB]WQ.][W@X4/data/data/com.clone.android.dual.space/vm/vm%
  @0x00001405 [/data/                        ] Q@PQB]WQ.][W@X4/data/data/com.clone.android.dual.space/vm/vm%d/dev
  @0x0000142f [/dev/                         ] d.dual.space/vm/vm%d/dev/netlink_client/netdevice_%d_%d..........
  @0x00000a0a [init                          ] ..........>%!,UM.hvminit.pid..D .............nv7S......ph>M.....
  @0x0000140f [clone                         ] [W@X4/data/data/com.clone.android.dual.space/vm/vm%d/dev/netlink_
--- Key 0x1f (31) — 1 hits ---
  @0x00002ad0 [sys.                          ] ........rB..........sys.game.touch.opt.enable.......XRX.LJFN.FNF
--- Key 0x21 (33) — 3 hits ---
  @0x00000f80 [/proc/                        ] !!!!..............!!/proc/cmdline.!!.............!!!m20-!m/,6.2-+,
  @0x00002b30 [sys.                          ] =t.v`=v}rq.v.!!!!!!!sys.game.vsr.crypto.enable.!!!!!),'!)k6*E!!!
  @0x00000610 [mount                         ] ^FD]-!!!.....!!!!!!!mount_mgr: /mnt is special, skip.!!!!!!!#.xc~
--- Key 0x25 (37) — 2 hits ---
  @0x00000db8 [/proc/                        ] ......%%.....%%%%%%%/proc/1..........%%%%%%%.IKVZ..]..J9\...\s%%..
  @0x00000f30 [/proc/                        ] ....W...W.....W...x%/proc/net/if_inet6/.%%%%%%%%%%%%A....A...A..1.
--- Key 0x26 (38) — 1 hits ---
  @0x0000279c [ion                           ] catedAttributes-[optional].&&&&&&&&&&&&&.......................
--- Key 0x27 (39) — 6 hits ---
  @0x00000e10 [/dev/__properties__           ] ........''''''''''''/dev/__properties__.''''.@AR.{{OIWC{{$''k !2k../)7#v..D'&ml
  @0x00000930 [/proc/                        ] ......''''''''''''''/proc/self/maps................'....''''.RPMA.
  @0x00000e10 [/dev/                         ] ........''''''''''''/dev/__properties__.''''.@AR.{{OIWC{{$''k !2k
  @0x00000e17 [prop                          ] .''''''''''''/dev/__properties__.''''.@AR.{{OIWC{{$''k !2k../)7#
  @0x00000930 [/proc/self/                   ] ......''''''''''''''/proc/self/maps................'....''''.RPMA.QGND.
  @0x00000930 [/proc/self/maps               ] ......''''''''''''''/proc/self/maps................'....''''.RPMA.QGND.GZG"
--- Key 0x2a (42) — 1 hits ---
  @0x000013fa [ioctl                         ] **********netdevice-ioctl..PU@U.PU@U.W[Y.WX[ZQ.UZPF[]P.PAUX.GDUWQ
--- Key 0x2c (44) — 4 hits ---
  @0x00001320 [/data/                        ] ...................,/data/data/com.clone.android.dual.space/vm/vm%
  @0x00001325 [/data/                        ] ..............,/data/data/com.clone.android.dual.space/vm/vm%d/dev
  @0x0000134f [/dev/                         ] d.dual.space/vm/vm%d/dev/netlink_server.,,,,,,,,,,,,,4.zoz4.zoz4x
  @0x0000132f [clone                         ] ....,/data/data/com.clone.android.dual.space/vm/vm%d/dev/netlink_
--- Key 0x2d (45) — 1 hits ---
  @0x00000c00 [krloader                      ] @.Ee...._F..4-------krloader64.-.......................-.L]HU---...-
--- Key 0x2e (46) — 4 hits ---
  @0x00000e48 [/dev/socket                   ] r-..b)(;b..& >*...M./dev/socket.....F...F......i....................._^
  @0x00000500 [/proc/                        ] 9>$#.u4P............/proc/mnt_points....................mp|<....-Q
  @0x00000e48 [/dev/                         ] r-..b)(;b..& >*...M./dev/socket.....F...F......i.................
  @0x00000e4d [socket                        ] )(;b..& >*...M./dev/socket.....F...F......i....................._^
--- Key 0x32 (50) — 1 hits ---
  @0x00002b10 [sys.                          ] e(ajcu(chgdjc.222222sys.game.vaa.gles.enable.2222222`j`=tr~v=e`a
--- Key 0x33 (51) — 5 hits ---
  @0x000002d0 [/data/                        ] ...3.....l3333333333/data/data/com.clone.android.dual.space/vm/vm%
  @0x000002d5 [/data/                        ] ....l3333333333/data/data/com.clone.android.dual.space/vm/vm%d%s.3
  @0x00000978 [/dev/                         ] NS63..............33/dev/tmpfs.33333...............3.........333r
  @0x000002df [clone                         ] 33333/data/data/com.clone.android.dual.space/vm/vm%d%s.3333333333
  @0x0000097d [tmpfs                         ] .............33/dev/tmpfs.33333...............3.........333rim`$3
--- Key 0x34 (52) — 1 hits ---
  @0x00002af0 [sys.                          ] HC.D[_.NEJIGN+444444sys.game.memc.gles.enable.444444u.u(agkc(pgg
--- Key 0x37 (55) — 4 hits ---
  @0x00001370 [/data/                        ] h~im~i.7777777777777/data/data/com.clone.android.dual.space/vm/vm%
  @0x00001375 [/data/                        ] i.7777777777777/data/data/com.clone.android.dual.space/vm/vm%d/dev
  @0x0000139f [/dev/                         ] d.dual.space/vm/vm%d/dev/netlink_client/nl_dhcp_%d_%d.7777777NXIN
  @0x0000137f [clone                         ] 77777/data/data/com.clone.android.dual.space/vm/vm%d/dev/netlink_
--- Key 0x38 (56) — 3 hits ---
  @0x000006c8 [tmpfs                         ] JK..V.KJQ.VPUUJWQ@A%tmpfs.88.....2...WM....M....M........MH.m888.
  @0x00000340 [mount                         ] WLCAV_\RW.8888888888mount_mgr: mount arg source %s is bad.8888888
  @0x0000034b [mount                         ] 888888888mount_mgr: mount arg source %s is bad.8888888888.....=..
--- Key 0x3c (60) — 1 hits ---
  @0x00000910 [/proc/                        ] ...........<..<<<<<</proc/maps_.......<<<<<<<<<<<<<<4kitx4h~w}4vzk
--- Key 0x3d (61) — 2 hits ---
  @0x000006b3 [ION                           ] =MOUNT.MGR..PROPAGATION..S.NOT.SUPPORTED qhucv.==.....7...RH...
  @0x00000a44 [clone                         ] n===============com.clone.android.dual.space.===.................
--- Key 0x3e (62) — 1 hits ---
  @0x00000a0a [INIT                          ] ......>>>>....um,HVMINIT_PID=%d.>>%>,><&=',= NV.s>; =&*PH.m>>>>>
--- Key 0x44 (68) — 2 hits ---
  @0x00002cee [open                          ] ...DDD-(#%-o2.ADDDdlopen.DDDDDDDDDDDDD..........................
  @0x00002cec [dlopen                        ] .....DDD-(#%-o2.ADDDdlopen.DDDDDDDDDDDDD..........................
--- Key 0x47 (71) — 1 hits ---
  @0x00000e58 [/dev/                         ] {{$GF...F......iGGGG/dev/vmproc.GGGG.........GGGGGGG}67$}3!:?7?!;
--- Key 0x48 (72) — 4 hits ---
  @0x000010d0 [/system/                      ] HHHHwryxnorwh5ht.HHH/system/lib64/libcutils.so.HHHHH................
  @0x00001000 [/data/                        ] s9(<1s.-<>8]HHHHHHHH/data/data/com.clone.android.dual.space/vm/vm.
  @0x00001005 [/data/                        ] s.-<>8]HHHHHHHH/data/data/com.clone.android.dual.space/vm/vm.HH6.j
  @0x0000100f [clone                         ] HHHHH/data/data/com.clone.android.dual.space/vm/vm.HH6.j.....7HHH
--- Key 0x4a (74) — 2 hits ---
  @0x000026f6 [ion                           ] .........JJJJJextensions-[optional].JJJJJJJJJJWMCJEPQVAeHCKVMPL
  @0x000026ff [ion                           ] JJJJJextensions-[optional].JJJJJJJJJJWMCJEPQVAeHCKVMPLI$JJJJJ%?
--- Key 0x4b (75) — 2 hits ---
  @0x00000f50 [/proc/                        ] 1....XAnKKKKKKKKKKKK/proc/net/if_inet6.K......KKKKKK..............
  @0x00000535 [mount                         ] KKK...YmKKKH4KKms.remount KKKKKOQ]@KLF.YGKY[BQ...........KKKKKK..
--- Key 0x4d (77) — 2 hits ---
  @0x000028ac [ion                           ] R[N+MMMMcontent-[optional].MMMMMMMMMMMMM.......................
  @0x00002d92 [error                         ] MMandroid_fdsan_set_error_level.MMMMMMMMMMBMMMMMMMMMMMMMMM.`ZMMMM
--- Key 0x4f (79) — 1 hits ---
  @0x00002a30 [/system/                      ] ............OOOOOOOO/system/lib/libVSR.so.OOOOOOOOOO................
--- Key 0x50 (80) — 1 hits ---
  @0x00000a8a [ION                           ] PP..........PPPPPPMRIONZWDVZHO^I.P.....PPP.....PPP4:3;>92W.GEXT
--- Key 0x55 (85) — 5 hits ---
  @0x00002530 [/dev/socket                   ] .]9U.UTG.G\AC^R1UUUU/dev/socket/process_pid.0lflspx0:{1kgk.U)...UUUU...
  @0x00002530 [/dev/                         ] .]9U.UTG.G\AC^R1UUUU/dev/socket/process_pid.0lflspx0:{1kgk.U)...U
  @0x00002535 [socket                        ] UTG.G\AC^R1UUUU/dev/socket/process_pid.0lflspx0:{1kgk.U)...UUUU...
  @0x000006db [bind                          ] H.....mUUmount_mgr: bind loop detected %s.UUU..UUUUUUUUUU.......
  @0x000006d0 [mount                         ] <h;=88':<-,H.....mUUmount_mgr: bind loop detected %s.UUU..UUUUUUU
--- Key 0x56 (86) — 4 hits ---
  @0x00000c50 [/proc/                        ] NCC@Lr%/VVVVVVVVVVVV/proc/self/status.VVVVVVVVVVVVVV..............
  @0x00000ddc [/dev/                         ] .VVVVVVVe:8%)eo.eo9J/dev/.VV.....VVVVVVV....................VVVVV
  @0x00000c50 [/proc/self/                   ] NCC@Lr%/VVVVVVVVVVVV/proc/self/status.VVVVVVVVVVVVVV.................VV
  @0x00000c50 [/proc/self/status             ] NCC@Lr%/VVVVVVVVVVVV/proc/self/status.VVVVVVVVVVVVVV.................VVVVVVVV
--- Key 0x5a (90) — 2 hits ---
  @0x00000370 [mount                         ] G.B..B...bZZZZZZZZZZmount_mgr: mount arg target %s is bad.ZZZZZZZ
  @0x0000037b [mount                         ] ZZZZZZZZZmount_mgr: mount arg target %s is bad.ZZZZZZZZZZ........
--- Key 0x5d (93) — 1 hits ---
  @0x00000f20 [/sys/                         ] ]]]]................/sys/class/net.]W....W...W..'....NWx]]]]]]]]]
--- Key 0x5e (94) — 2 hits ---
  @0x0000024b [open                          ] ^^^^^^^^^__loader_dlopen........^up{z7jv..BHBET\.]XS...]XSR.B^1^
  @0x00000249 [dlopen                        ] B-^^^^^^^^^__loader_dlopen........^up{z7jv..BHBET\.]XS...]XSR.B^1^
--- Key 0x63 (99) — 2 hits ---
  @0x00000fa0 [/proc/                        ] 'Bcc.............ccc/proc/mnt_points.cccA...Ancc.....ccccccccccccc
  @0x00000e38 [/dev/                         ] ccccO...O??....??`cc/dev/__kmsg2__.cb)(;b>".&(9Mcccc.@AR.RITVKG$c
--- Key 0x64 (100) — 1 hits ---
  @0x00002520 [/dev/                         ] dddd'lm~'~exzgk'-l.d/dev/vmproc.dddd.UTG.B^RZTE.AC^RTBBnAXU1.]W]B
--- Key 0x65 (101) — 2 hits ---
  @0x00002603 [/system/                      ] ...eeeeeeeeeeeeee/fs/system/build.prop.eeeeeeeeee...................
  @0x00002611 [prop                          ] eee/fs/system/build.prop.eeeeeeeeee.....................eee$2#.5
--- Key 0x67 (103) — 3 hits ---
  @0x000008dc [/proc/                        ] ..gggggg............/proc/.ggggg]....]W.rggggggg...............g..
  @0x00000ab0 [/proc/                        ] .ggg.....ggg.......`/proc/%d/cmdline.gggLK^KJL?ggggg..............
  @0x00000ab0 [/proc/%d/                     ] .ggg.....ggg.......`/proc/%d/cmdline.gggLK^KJL?ggggg................z
--- Key 0x68 (104) — 1 hits ---
  @0x00000460 [mount                         ] hhhh1z{h1jsnxm.hhhhhmount_mgr: %s -> %s.n2.hhhhhhhhh.T..T..T..T..
--- Key 0x69 (105) — 2 hits ---
  @0x00000ea8 [/dev/                         ] .iii..............ii/dev/tmp.iiiiiii...............iiiiiiiii.....
  @0x00000543 [bind                          ] iOQ]PGOMWLV.iiiiims.bind {ei{y`s6..........iiiiii...........iiii
--- Key 0x6a (106) — 2 hits ---
  @0x000026f6 [ION                           ] .........jjjjjEXTENSIONS.{OPTIONAL} jjjjjjjjjjwmcjepqvaEhckvmpl
  @0x000026ff [ION                           ] jjjjjEXTENSIONS.{OPTIONAL} jjjjjjjjjjwmcjepqvaEhckvmpli.jjjjj..
--- Key 0x6c (108) — 1 hits ---
  @0x00002510 [/dev/                         ] ....RJ.ollll|I{lllll/dev/vmproc/%d.l'lm~'~exzgk.llll.]\O.JVZR\M.I
--- Key 0x6d (109) — 6 hits ---
  @0x000028ac [ION                           ] r{n.mmmmCONTENT.{OPTIONAL} mmmmmmmmmmmmm.......................
  @0x00000870 [/dev/input                    ] ..................mm/dev/input/touch.mmmo9eJmmmmmmmm.................m
  @0x00000870 [/dev/input/touch              ] ..................mm/dev/input/touch.mmmo9eJmmmmmmmm.................mmmmmmm
  @0x00000870 [/dev/                         ] ..................mm/dev/input/touch.mmmo9eJmmmmmmmm.............
  @0x00002d92 [ERROR                         ] mmANDROID.FDSAN.SET.ERROR.LEVEL mmmmmmmmmmbmmmmmmmmmmmmmmm.@zmmmm
  @0x00000a1b [mount                         ] ...........nv7Smmvm.mount.ns..D mhsnuy..M>mmmmmmmmmmmmmmm3?=~3<?>
--- Key 0x6e (110) — 2 hits ---
  @0x00000590 [mount                         ] nnnn..............nnmount_mgr: no mounts.nnnnnnnnnnn.............
  @0x0000059e [mount                         ] ....nnmount_mgr: no mounts.nnnnnnnnnnn..........................n
--- Key 0x6f (111) — 1 hits ---
  @0x00000260 [/system/                      ] AT_1.......oDAJK.[G(/system/lib64/libc.so.ooq}pps.Cuhyn}hy.o........
--- Key 0x70 (112) — 1 hits ---
  @0x00000a8a [ion                           ] pp..........ppppppmrionzwdvzho~i;p.....ppp.....ppp.......w8gext
--- Key 0x73 (115) — 1 hits ---
  @0x00000220 [/system/                      ] I.VJ%sssssssssssssss/system/lib64/libdl.so.sssssssssrrABLIH_rIAB]HC-
--- Key 0x76 (118) — 2 hits ---
  @0x00000450 [/dev/                         ] .LARLI.LARLI.lvvvvvv/dev/tmpfs.vvvvvsqkpjAsyl$>;m>3 >;m.p,.vvvvvv
  @0x00000455 [tmpfs                         ] I.LARLI.lvvvvvv/dev/tmpfs.vvvvvsqkpjAsyl$>;m>3 >;m.p,.vvvvvvvvv.J
--- Key 0x77 (119) — 4 hits ---
  @0x00000d32 [/proc/                        ] ...wwwwwwwwwwwwwww%s/proc/mounts_%d_%d.wwwR...R......}wwww........
  @0x00000680 [mount                         ] .........wwwwwwwwwwwmount_mgr: %s not mounted.wwwwww.....5...PJ..
  @0x00000692 [mount                         ] wwmount_mgr: %s not mounted.wwwwww.....5...PJ...........JO.J...J.
  @0x00000d38 [mount                         ] wwwwwwwwwwww%s/proc/mounts_%d_%d.wwwR...R......}wwww...........ww
--- Key 0x7a (122) — 3 hits ---
  @0x0000081d [qemu_pipe                     ] .zzBBlz.....zzz/dev/qemu_pipe.zzzzzzzzz..............................
  @0x00000818 [/dev/qemu_pipe                ] ...s..zzBBlz.....zzz/dev/qemu_pipe.zzzzzzzzz..............................
  @0x00000818 [/dev/                         ] ...s..zzBBlz.....zzz/dev/qemu_pipe.zzzzzzzzz.....................
--- Key 0x7e (126) — 1 hits ---
  @0x000004e0 [/proc/                        ] .......~~~~~~~~~~~~~/proc/mnt_points_%d.~~~~~~~~~~~~. "?3.=>$. ?9>
--- Key 0x80 (128) — 2 hits ---
  @0x00000c70 [/proc/                        ] ..................../proc/%d/statu%c.................O.LNS_.OH]HIO
  @0x00000c70 [/proc/%d/                     ] ..................../proc/%d/statu%c.................O.LNS_.OH]HIOc.X
--- Key 0x81 (129) — 1 hits ---
  @0x00000f7a [ION                           ] = ,O.......PROC.VERSION ..................d;9$(d&$>%?8K........
--- Key 0x82 (130) — 3 hits ---
  @0x000008f8 [/proc/                        ] ..................../proc/%d/fd/%d..s\..................?~E?~.....
  @0x0000259a [init                          ] )0TVb.............vminit.pid .....g|n|~d.en.b1............4.zoz4
  @0x000008f8 [/proc/%d/                     ] ..................../proc/%d/fd/%d..s\..................?~E?~........
--- Key 0x83 (131) — 4 hits ---
  @0x00000b90 [/proc/                        ] .....VSFS.SBB.2...../proc/%d/map%c...V.UWJF.HDUVz.Az.A%.g.........
  @0x000029a0 [property                      ] ....;...d...........property_get_bool...............adojl`h`lcljh.Rl
  @0x000029a0 [prop                          ] ....;...d...........property_get_bool...............adojl`h`lclj
  @0x00000b90 [/proc/%d/                     ] .....VSFS.SBB.2...../proc/%d/map%c...V.UWJF.HDUVz.Az.A%.g............
--- Key 0x84 (132) — 3 hits ---
  @0x000003c0 [mount                         ] ..Y.=.......~.......mount_mgr: umount arg target %s is bad.......
  @0x000003cc [mount                         ] ~.......mount_mgr: umount arg target %s is bad...........[..[..[.
  @0x000003cb [umount                        ] .~.......mount_mgr: umount arg target %s is bad...........[..[..[.
--- Key 0x88 (136) — 1 hits ---
  @0x00000e88 [/dev/                         ] ..................../dev/.magisk....{01"{z6!'-6;,T...............
--- Key 0x8a (138) — 1 hits ---
  @0x00002869 [ION                           ] .2o........CRLS.{OPTIONAL} ......................$...m.........
--- Key 0x8b (139) — 4 hits ---
  @0x00000b00 [/system/                      ] ..................../system/........@......o.................XK@JA\.
  @0x00000d10 [/proc/                        ] ='<&!R............../proc/%d/mount%c..............................
  @0x00000d19 [mount                         ] .........../proc/%d/mount%c......................................
  @0x00000d10 [/proc/%d/                     ] ='<&!R............../proc/%d/mount%c.................................
--- Key 0x8c (140) — 2 hits ---
  @0x00000ad0 [/proc/                        ] ..................../proc/%d/status...............................
  @0x00000ad0 [/proc/%d/                     ] ..................../proc/%d/status.................................(
--- Key 0x8d (141) — 1 hits ---
  @0x00002a50 [/system/                      ] ..................../system/lib64/libgamemanager_aidl.so............
--- Key 0x8e (142) — 1 hits ---
  @0x00002ba0 [/vendor/                      ] GUYGAZS.G[4........./vendor/lib/egl/libGLESv2_samsung.so............
--- Key 0x90 (144) — 2 hits ---
  @0x00001101 [ION                           ] ...ASHMEM.CREATE.REGION ...oooo.........TA......A....a.........
  @0x00000942 [/proc/                        ] ..................%s/proc/exe_%d..YDY<....................A....AK.
--- Key 0x93 (147) — 3 hits ---
  @0x000026c3 [ION                           ] .ISSUERuNIQUEid.{OPTIONAL} ......X^IANH_~EBZ^Nbo.pD[_BDEJGv+...
  @0x00000dc0 [/proc/                        ] ....r.............../proc/1/............................%no|......
  @0x000025ab [mount                         ] .g|x.xenaxu1.....vm.mount.ns ............%nk~k%nk~k%ieg$ifedo$kdn
--- Key 0x94 (148) — 2 hits ---
  @0x00000798 [init                          ] ....................init_seccomp............4:95=32v%/%57::v07?:
  @0x0000079d [seccomp                       ] ...............init_seccomp............4:95=32v%/%57::v07?:32vs2V..
--- Key 0x95 (149) — 5 hits ---
  @0x00000d9f [/dev/input                    ] d.dual.space/vm/vm%d/dev/input.......t...............)vtie)7).........
  @0x00000d70 [/data/                        ]  fa.z{............../data/data/com.clone.android.dual.space/vm/vm%
  @0x00000d75 [/data/                        ] {............../data/data/com.clone.android.dual.space/vm/vm%d/dev
  @0x00000d9f [/dev/                         ] d.dual.space/vm/vm%d/dev/input.......t...............)vtie)7)....
  @0x00000d7f [clone                         ] ...../data/data/com.clone.android.dual.space/vm/vm%d/dev/input...
--- Key 0x98 (152) — 1 hits ---
  @0x000026e4 [ION                           ] SUBJECTuNIQUEid.{OPTIONAL} ....................................
--- Key 0x99 (153) — 3 hits ---
  @0x000025c0 [/data/                        ] e.d~udy*............/data/data/com.clone.android.dual.space/vm/vm%
  @0x000025c5 [/data/                        ] dy*............/data/data/com.clone.android.dual.space/vm/vm%d%s..
  @0x000025cf [clone                         ] ...../data/data/com.clone.android.dual.space/vm/vm%d%s...........
--- Key 0x9a (154) — 7 hits ---
  @0x00000d58 [/dev/input                    ] ..................../dev/input.............. kn{n kn{n l`b!lc`aj!nak}`
  @0x000028c0 [/data/                        ] ..................../data/data/com.clone.android.dual.space%s.....
  @0x000028c5 [/data/                        ] .............../data/data/com.clone.android.dual.space%s..........
  @0x00000d58 [/dev/                         ] ..................../dev/input.............. kn{n kn{n l`b!lc`aj!
  @0x00000252 [open                          ] ..................dlopen........................................
  @0x000028cf [clone                         ] ...../data/data/com.clone.android.dual.space%s...................
  @0x00000250 [dlopen                        ] ....................dlopen........................................
--- Key 0x9c (156) — 2 hits ---
  @0x00001130 [/vendor/                      ] .M....m............./vendor/build.prop................A...A.....Z0..
  @0x0000113e [prop                          ] ....../vendor/build.prop................A...A.....Z0.......o....
--- Key 0x9d (157) — 1 hits ---
  @0x00000e68 [/dev/                         ] ..................../dev/log........................:qpc:;xtr|f~.
--- Key 0xa1 (161) — 5 hits ---
  @0x00000f7a [ion                           ] ...o....../proc/version...................D....D......k........
  @0x000008d0 [/proc/                        ] ........SBZZN7....../proc/self/..............................SQL@.
  @0x00000f70 [/proc/                        ] ........@....o....../proc/version...................D....D......k.
  @0x00002ab0 [sys.                          ]  -%g:&I.............sys.game.vsr.enable.........................
  @0x000008d0 [/proc/self/                   ] ........SBZZN7....../proc/self/..............................SQL@..G.EG
--- Key 0xa2 (162) — 1 hits ---
  @0x0000259a [INIT                          ] ..tvB.............VMINIT_PID......G\N\^D_EN_B.............._ZOZ.
--- Key 0xa3 (163) — 1 hits ---
  @0x00002a10 [/system/                      ] }Zl{.`jlL.........../system/lib64/libVSR.so.........................
--- Key 0xa5 (165) — 1 hits ---
  @0x00000b28 [/vendor/                      ] 5$,A................/vendor/.................HYRU.;..XIBE+..........
--- Key 0xa6 (166) — 3 hits ---
  @0x00000ba2 [/proc/                        ] ...UWJF..A.HDU.F%.%s/proc/maps_%d_%d..B5..........................
  @0x00000f10 [/sys/                         ] .....}............../sys/class/net/..............................
  @0x00000eb8 [/dev/                         ] .z................../dev/__krlog__..........S...S......S.....|...
--- Key 0xa7 (167) — 2 hits ---
  @0x00000313 [/system/                      ] ................./fs/system/bin/linker64............................
  @0x0000031f [linker                        ] ...../fs/system/bin/linker64......................................
--- Key 0xa8 (168) — 1 hits ---
  @0x00000585 [bind                          ] ...............ms.unbindable ..................................~
--- Key 0xaa (170) — 1 hits ---
  @0x00002869 [ion                           ] #.O........crls-[optional].................>$*#(?.#+"M.........
--- Key 0xad (173) — 2 hits ---
  @0x000010a0 [/proc/                        ] ....=.......S}....../proc/self/fd/%d....*.........................
  @0x000010a0 [/proc/self/                   ] ....=.......S}....../proc/self/fd/%d....*..............................
--- Key 0xb0 (176) — 6 hits ---
  @0x00001101 [ion                           ] ...ashmem_create_region....OOOO....2."*2ta' (-$%a %%3A.........
  @0x000004a0 [/data/                        ] ..................../data/data/com.clone.android.dual.space/vm/vm%
  @0x000004a5 [/data/                        ] .............../data/data/com.clone.android.dual.space/vm/vm%d/fs.
  @0x000004af [clone                         ] ...../data/data/com.clone.android.dual.space/vm/vm%d/fs..........
  @0x000010f1 [shmem                         ] ...................ashmem_create_region....OOOO....2."*2ta' (-$%a
  @0x000010f0 [ashmem                        ] ....................ashmem_create_region....OOOO....2."*2ta' (-$%a
--- Key 0xb1 (177) — 2 hits ---
  @0x00000b80 [/data/                        ] .....@KBQDFQ#......./data/app/.......B@]Q..V._SB.Q2.2d8gext8zvgdH2
  @0x00000700 [mount                         ] ........0...........mount_mgr: unsupported filesystemtype %s.....
--- Key 0xb3 (179) — 1 hits ---
  @0x000026c3 [ion                           ] .issuerUniqueID-[optional].......x~ianh.^ebz~nBO&Pd{.bdejgV....
--- Key 0xb4 (180) — 1 hits ---
  @0x00000798 [INIT                          ] ....................INIT.SECCOMP ..................V.......V....
--- Key 0xb8 (184) — 1 hits ---
  @0x000026e4 [ion                           ] subjectUniqueID-[optional].....................................
--- Key 0xba (186) — 7 hits ---
  @0x00000864 [qemu_pipe                     ] l.space/vm/vm%d/dev/qemu_pipe...................................(wuhd
  @0x0000085f [/dev/qemu_pipe                ] d.dual.space/vm/vm%d/dev/qemu_pipe...................................(wuhd
  @0x00002b70 [/vendor/                      ] !)6<(E............../vendor/lib64/egl/libGLESv2_samsung.so..........
  @0x00000830 [/data/                        ] ..................../data/data/com.clone.android.dual.space/vm/vm%
  @0x00000835 [/data/                        ] .............../data/data/com.clone.android.dual.space/vm/vm%d/dev
  @0x0000085f [/dev/                         ] d.dual.space/vm/vm%d/dev/qemu_pipe...............................
  @0x0000083f [clone                         ] ...../data/data/com.clone.android.dual.space/vm/vm%d/dev/qemu_pip
--- Key 0xbb (187) — 2 hits ---
  @0x000005bd [read                          ] .......mount_mgr: already latest................................
  @0x000005b0 [mount                         ] ....................mount_mgr: already latest....................
--- Key 0xbc (188) — 1 hits ---
  @0x00000c92 [/proc/                        ] ._<...............%s/proc/status_%d_%d....pLA.%....- ~Ma Ma Ma Ma 
--- Key 0xbd (189) — 2 hits ---
  @0x00000890 [/proc/                        ] ..................../proc/self/fd/%d........8>(?c8$)M.............
  @0x00000890 [/proc/self/                   ] ..................../proc/self/fd/%d........8>(?c8$)M..................
--- Key 0xc2 (194) — 1 hits ---
  @0x000007c0 [failed                        ] ....blocked syscall failed %d.....................................
--- Key 0xc5 (197) — 1 hits ---
  @0x00002851 [ION                           ] ...CERTIFICATES.{OPTIONAL} ............B4........2o............
--- Key 0xc6 (198) — 1 hits ---
  @0x000027fe [ION                           ] CATEDaTTRIBUTES.{OPTIONAL} ............",.8?.',$9"?#&8K.......V
--- Key 0xc7 (199) — 2 hits ---
  @0x00000df0 [/dev/                         ] ........q:;(^......./dev/.coldboot_done..........................
  @0x00002560 [/dev/                         ] ..................../dev/__krlog__..[@^_NHAK....\GAX98..qjlu..'..
--- Key 0xc8 (200) — 2 hits ---
  @0x0000262d [ion                           ] .......ro.build.version.sdk....................jynousr.........
  @0x00002620 [ro.build.                     ] ....................ro.build.version.sdk....................jynousr..
--- Key 0xc9 (201) — 1 hits ---
  @0x00002d72 [error                         ] %Qandroid_fdsan_get_error_level..................................
--- Key 0xca (202) — 2 hits ---
  @0x00000f90 [/proc/                        ] .k................../proc/mounts................................J.
  @0x00000f96 [mount                         ] ............../proc/mounts................................J...e..
--- Key 0xcb (203) — 1 hits ---
  @0x00002c72 [Init                          ] ..........GamesAwareInit........................................
--- Key 0xd1 (209) — 1 hits ---
  @0x00001117 [FAILED                        ] .a...........SOCKS..FAILED.ADDR .............b;(#)"?b/8$!)c=?"=M..
--- Key 0xd2 (210) — 5 hits ---
  @0x00002ca0 [/system/                      ] ..................../system/lib64/libGamesAware.so..)u.urck)jod)jodA
  @0x000012d0 [/data/                        ] ..................../data/data/com.clone.android.dual.space/vm/vm%
  @0x000012d5 [/data/                        ] .............../data/data/com.clone.android.dual.space/vm/vm%d/dev
  @0x000012ff [/dev/                         ] d.dual.space/vm/vm%d/dev/netlink_client/nl_%d_%d_%d..............
  @0x000012df [clone                         ] ...../data/data/com.clone.android.dual.space/vm/vm%d/dev/netlink_
--- Key 0xd4 (212) — 3 hits ---
  @0x0000264c [ion                           ] ................version........................................
  @0x000027bd [ion                           ] .......digestEncryptionAlgorithmId.....yr.nelhyxXu{yoh.G\SGFZW\
  @0x00002cc0 [/system/                      ] 2)jodAgkcuGqgtc(ui../system/lib/libGamesAware.so....................
--- Key 0xd7 (215) — 1 hits ---
  @0x000009b2 [INIT                          ] 7={...............VMINIT_PID......RI[IKQJP[JW...................
--- Key 0xd9 (217) — 3 hits ---
  @0x00000cf0 [/proc/                        ] ..................../proc/self/mounts...............}" =1}w6}?='<&
  @0x00000cfb [mount                         ] ........./proc/self/mounts...............}" =1}w6}?='<&w1R.......
  @0x00000cf0 [/proc/self/                   ] ..................../proc/self/mounts...............}" =1}w6}?='<&w1R..
--- Key 0xda (218) — 3 hits ---
  @0x00000ed0 [/dev/socket                   ] S##.....##|........./dev/socket/logdw................edw.rnbjdu.mnfes..
  @0x00000ed0 [/dev/                         ] S##.....##|........./dev/socket/logdw................edw.rnbjdu.m
  @0x00000ed5 [socket                        ] ...##|........./dev/socket/logdw................edw.rnbjdu.mnfes..
--- Key 0xdb (219) — 4 hits ---
  @0x00000ef0 [/dev/socket                   ] mnfev.............../dev/socket/logdr...............R...R.....R...R}...
  @0x00000ef0 [/dev/                         ] mnfev.............../dev/socket/logdr...............R...R.....R..
  @0x00000ef5 [socket                        ] .............../dev/socket/logdr...............R...R.....R...R}...
  @0x00000650 [mount                         ] ....................mount_mgr: /storage is special, skip.........
--- Key 0xdc (220) — 1 hits ---
  @0x00000e98 [/dev/                         ] ....{01"{z953='?T.../dev/.busybox...................U...U%%.....%
--- Key 0xde (222) — 2 hits ---
  @0x00000988 [/proc/                        ] C ................../proc/self/fd/..$?! 17>4r..............._D@G@]
  @0x00000988 [/proc/self/                   ] C ................../proc/self/fd/..$?! 17>4r..............._D@G@]VY@M.
--- Key 0xe2 (226) — 2 hits ---
  @0x000007c0 [FAILED                        ] ....BLOCKED.SYSCALL.FAILED..D ....................................
  @0x00000a98 [bind                          ] ....................bind....0f0f................................
--- Key 0xe3 (227) — 1 hits ---
  @0x00001060 [/system/                      ] ..................../system/bin.....0lflkzr0g}vq.....; ,>9(?M.......
--- Key 0xe5 (229) — 1 hits ---
  @0x00002851 [ion                           ] ...certificates-[optional].........,=#<b. ?;& !.#.O............
--- Key 0xe6 (230) — 1 hits ---
  @0x000027fe [ion                           ] catedAttributes-[optional]............/.....*.........k.......v
--- Key 0xe7 (231) — 2 hits ---
  @0x00002988 [property                      ] &1C.................property_get....................;...;....d......
  @0x00002988 [prop                          ] &1C.................property_get....................;...;....d..
--- Key 0xe8 (232) — 2 hits ---
  @0x0000262d [ION                           ] .......RO.BUILD.VERSION.SDK ...................JYNOUSR<........
  @0x00002a80 [/system/                      ] :....K..e.........../system/lib/libgamemanager_aidl.so..............
--- Key 0xe9 (233) — 1 hits ---
  @0x00002d72 [ERROR                         ] .qANDROID.FDSAN.GET.ERROR.LEVEL .................................
--- Key 0xee (238) — 2 hits ---
  @0x000024b0 [/vendor/                      ] ..................../vendor/build.prop..............WJ.VC.IFAzRLAQM.
  @0x000024be [prop                          ] ....../vendor/build.prop..............WJ.VC.IFAzRLAQM..A%.......
--- Key 0xf1 (241) — 1 hits ---
  @0x00001117 [failed                        ] /A...........socks5 failed addr..............B......B.....C....m..
--- Key 0xf3 (243) — 1 hits ---
  @0x000009c3 [mount                         ] .rimjmp{tm`$.....vm.mount.ns ...........................e3@......
--- Key 0xf4 (244) — 2 hits ---
  @0x0000264c [ION                           ] ................VERSION .......................................
  @0x000027bd [ION                           ] .......DIGESTeNCRYPTIONaLGORITHMiD ....YR_NELHYXxU[YOH<g|sgfzw|
--- Key 0xf7 (247) — 1 hits ---
  @0x000009b2 [init                          ] ..[...............vminit.pid .....ri{ikqjp{jw$..................
--- Key 0xf8 (248) — 1 hits ---
  @0x00000a7a [connect                       ] >3d9/&#$?2J.......__connect.......................xst~....*|*|.....
--- Key 0xfc (252) — 1 hits ---
  @0x00001070 [/system/                      ] ....0lflkzr0}vq...../system/xbin.....$?3!&7 R.......lZAM_XI^.,......
--- Key 0xfe (254) — 2 hits ---
  @0x00000968 [/proc/                        ] ..................../proc/%d/ex%c....................PROC.SELF.FD.
  @0x00000968 [/proc/%d/                     ] ..................../proc/%d/ex%c....................PROC.SELF.FD. ..
--- Key 0xff (255) — 1 hits ---
  @0x00002b5c [dlsym                         ] ....................dlsym...............j3 +!*7j),'sqj ")j),'....
