
libvm.so:     file format elf64-littleaarch64


Disassembly of section .text:

0000000000392220 <initOpenGLRenderer@@Base>:
  392220:	stp	x29, x30, [sp, #-96]!
  392224:	stp	x28, x27, [sp, #16]
  392228:	stp	x26, x25, [sp, #32]
  39222c:	stp	x24, x23, [sp, #48]
  392230:	stp	x22, x21, [sp, #64]
  392234:	stp	x20, x19, [sp, #80]
  392238:	mov	x29, sp
  39223c:	sub	sp, sp, #0x80
  392240:	stur	x4, [x29, #-104]
  392244:	stp	w1, w2, [x29, #-112]
  392248:	stur	w0, [x29, #-116]
  39224c:	mrs	x8, tpidr_el0
  392250:	stur	x8, [x29, #-128]
  392254:	ldr	x8, [x8, #40]
  392258:	mov	x20, x3
  39225c:	stur	x8, [x29, #-8]
  392260:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392264:	ldr	x8, [x8, #1064]
  392268:	ldr	w9, [x8]
  39226c:	mov	w8, #0xd5f7                	// #54775
  392270:	movk	w8, #0xeab2, lsl #16
  392274:	cmp	w9, #0xa
  392278:	b.lt	392294 <initOpenGLRenderer@@Base+0x74>  // b.tstop
  39227c:	adrp	x9, 72a000 <x.30@@Base-0x432ac>
  392280:	ldr	x9, [x9, #1072]
  392284:	ldr	w9, [x9]
  392288:	add	w10, w9, #0x1
  39228c:	mul	w9, w10, w9
  392290:	tbnz	w9, #0, 39231c <initOpenGLRenderer@@Base+0xfc>
  392294:	sub	x9, sp, #0x10
  392298:	mov	sp, x9
  39229c:	stur	x9, [x29, #-96]
  3922a0:	sub	x9, sp, #0x10
  3922a4:	mov	sp, x9
  3922a8:	adrp	x10, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  3922ac:	ldr	x10, [x10, #3456]
  3922b0:	stur	x9, [x29, #-88]
  3922b4:	ldur	x9, [x29, #-88]
  3922b8:	cmp	x10, #0x0
  3922bc:	cset	w10, ne  // ne = any
  3922c0:	strb	w10, [x9]
  3922c4:	sub	x9, sp, #0x10
  3922c8:	mov	sp, x9
  3922cc:	stur	x9, [x29, #-80]
  3922d0:	ldur	x9, [x29, #-80]
  3922d4:	str	w8, [x9]
  3922d8:	sub	x9, sp, #0x10
  3922dc:	mov	sp, x9
  3922e0:	stur	x9, [x29, #-72]
  3922e4:	sub	x9, sp, #0x10
  3922e8:	mov	sp, x9
  3922ec:	adrp	x10, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3922f0:	ldr	x10, [x10, #2544]
  3922f4:	stur	x9, [x29, #-64]
  3922f8:	ldr	w10, [x10]
  3922fc:	cmp	w10, #0xa
  392300:	b.lt	39237c <initOpenGLRenderer@@Base+0x15c>  // b.tstop
  392304:	adrp	x9, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392308:	ldr	x9, [x9, #2552]
  39230c:	ldr	w9, [x9]
  392310:	add	w10, w9, #0x1
  392314:	mul	w9, w10, w9
  392318:	tbz	w9, #0, 39237c <initOpenGLRenderer@@Base+0x15c>
  39231c:	sub	x9, sp, #0x10
  392320:	mov	sp, x9
  392324:	stur	x9, [x29, #-96]
  392328:	sub	x9, sp, #0x10
  39232c:	mov	sp, x9
  392330:	adrp	x10, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  392334:	ldr	x10, [x10, #3456]
  392338:	stur	x9, [x29, #-88]
  39233c:	ldur	x9, [x29, #-88]
  392340:	cmp	x10, #0x0
  392344:	cset	w10, ne  // ne = any
  392348:	strb	w10, [x9]
  39234c:	sub	x9, sp, #0x10
  392350:	mov	sp, x9
  392354:	stur	x9, [x29, #-80]
  392358:	ldur	x9, [x29, #-80]
  39235c:	str	w8, [x9]
  392360:	sub	x9, sp, #0x10
  392364:	mov	sp, x9
  392368:	stur	x9, [x29, #-72]
  39236c:	sub	x9, sp, #0x10
  392370:	mov	sp, x9
  392374:	stur	x9, [x29, #-64]
  392378:	b	392294 <initOpenGLRenderer@@Base+0x74>
  39237c:	adrp	x27, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392380:	adrp	x10, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392384:	adrp	x19, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392388:	adrp	x22, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39238c:	adrp	x25, 72a000 <x.30@@Base-0x432ac>
  392390:	adrp	x28, 72a000 <x.30@@Base-0x432ac>
  392394:	ldr	x27, [x27, #2368]
  392398:	ldr	x10, [x10, #2376]
  39239c:	ldr	x19, [x19, #3080]
  3923a0:	ldr	x22, [x22, #3088]
  3923a4:	ldr	x25, [x25, #656]
  3923a8:	ldr	x28, [x28, #648]
  3923ac:	mov	w21, #0x1c3d                	// #7229
  3923b0:	mov	w23, #0x5d07                	// #23815
  3923b4:	mov	w24, #0x61d9                	// #25049
  3923b8:	adrp	x26, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  3923bc:	movk	w21, #0x5662, lsl #16
  3923c0:	movk	w23, #0x3c26, lsl #16
  3923c4:	movk	w24, #0x1e55, lsl #16
  3923c8:	add	x26, x26, #0xd90
  3923cc:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3923d0:	ldr	x8, [x8, #2360]
  3923d4:	ldr	w8, [x8]
  3923d8:	cmp	w8, #0xa
  3923dc:	b.lt	3923f8 <initOpenGLRenderer@@Base+0x1d8>  // b.tstop
  3923e0:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3923e4:	ldr	x8, [x8, #2352]
  3923e8:	ldr	w8, [x8]
  3923ec:	add	w9, w8, #0x1
  3923f0:	mul	w8, w9, w8
  3923f4:	tbnz	w8, #0, 392420 <initOpenGLRenderer@@Base+0x200>
  3923f8:	ldur	x8, [x29, #-80]
  3923fc:	ldr	w9, [x10]
  392400:	ldr	w8, [x8]
  392404:	cmp	w9, #0xa
  392408:	stur	w8, [x29, #-56]
  39240c:	b.lt	392430 <initOpenGLRenderer@@Base+0x210>  // b.tstop
  392410:	ldr	w8, [x27]
  392414:	add	w9, w8, #0x1
  392418:	mul	w8, w9, w8
  39241c:	tbz	w8, #0, 392430 <initOpenGLRenderer@@Base+0x210>
  392420:	ldur	x8, [x29, #-80]
  392424:	ldr	w8, [x8]
  392428:	stur	w8, [x29, #-56]
  39242c:	b	3923f8 <initOpenGLRenderer@@Base+0x1d8>
  392430:	ldur	w8, [x29, #-56]
  392434:	mov	w9, #0x94b7                	// #38071
  392438:	movk	w9, #0x592, lsl #16
  39243c:	cmp	w8, w9
  392440:	b.gt	3924b4 <initOpenGLRenderer@@Base+0x294>
  392444:	mov	w9, #0xf900                	// #63744
  392448:	movk	w9, #0xcde4, lsl #16
  39244c:	cmp	w8, w9
  392450:	b.gt	392524 <initOpenGLRenderer@@Base+0x304>
  392454:	mov	w9, #0x921                 	// #2337
  392458:	movk	w9, #0x834c, lsl #16
  39245c:	cmp	w8, w9
  392460:	b.eq	392684 <initOpenGLRenderer@@Base+0x464>  // b.none
  392464:	mov	w9, #0xf83a                	// #63546
  392468:	movk	w9, #0xac7f, lsl #16
  39246c:	cmp	w8, w9
  392470:	b.eq	3925f4 <initOpenGLRenderer@@Base+0x3d4>  // b.none
  392474:	mov	w9, #0x42b8                	// #17080
  392478:	movk	w9, #0xbd04, lsl #16
  39247c:	cmp	w8, w9
  392480:	b.ne	3926f4 <initOpenGLRenderer@@Base+0x4d4>  // b.any
  392484:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392488:	ldr	x8, [x8, #2712]
  39248c:	ldr	w8, [x8]
  392490:	cmp	w8, #0xa
  392494:	b.lt	392730 <initOpenGLRenderer@@Base+0x510>  // b.tstop
  392498:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39249c:	ldr	x8, [x8, #2704]
  3924a0:	ldr	w8, [x8]
  3924a4:	add	w9, w8, #0x1
  3924a8:	mul	w8, w9, w8
  3924ac:	tbnz	w8, #0, 39279c <initOpenGLRenderer@@Base+0x57c>
  3924b0:	b	392730 <initOpenGLRenderer@@Base+0x510>
  3924b4:	mov	w9, #0x867b                	// #34427
  3924b8:	movk	w9, #0x4278, lsl #16
  3924bc:	cmp	w8, w9
  3924c0:	b.le	392574 <initOpenGLRenderer@@Base+0x354>
  3924c4:	mov	w9, #0x867c                	// #34428
  3924c8:	movk	w9, #0x4278, lsl #16
  3924cc:	cmp	w8, w9
  3924d0:	b.eq	3926b4 <initOpenGLRenderer@@Base+0x494>  // b.none
  3924d4:	mov	w9, #0xea2b                	// #59947
  3924d8:	movk	w9, #0x55d4, lsl #16
  3924dc:	cmp	w8, w9
  3924e0:	b.eq	392624 <initOpenGLRenderer@@Base+0x404>  // b.none
  3924e4:	mov	w9, #0x557c                	// #21884
  3924e8:	movk	w9, #0x631e, lsl #16
  3924ec:	cmp	w8, w9
  3924f0:	b.ne	3926f4 <initOpenGLRenderer@@Base+0x4d4>  // b.any
  3924f4:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  3924f8:	ldr	x8, [x8, #1088]
  3924fc:	ldr	w8, [x8]
  392500:	cmp	w8, #0xa
  392504:	b.lt	392ae4 <initOpenGLRenderer@@Base+0x8c4>  // b.tstop
  392508:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  39250c:	ldr	x8, [x8, #1096]
  392510:	ldr	w8, [x8]
  392514:	add	w9, w8, #0x1
  392518:	mul	w8, w9, w8
  39251c:	tbnz	w8, #0, 392b30 <initOpenGLRenderer@@Base+0x910>
  392520:	b	392ae4 <initOpenGLRenderer@@Base+0x8c4>
  392524:	mov	w9, #0xf901                	// #63745
  392528:	movk	w9, #0xcde4, lsl #16
  39252c:	cmp	w8, w9
  392530:	b.eq	392654 <initOpenGLRenderer@@Base+0x434>  // b.none
  392534:	mov	w9, #0xd5f7                	// #54775
  392538:	movk	w9, #0xeab2, lsl #16
  39253c:	cmp	w8, w9
  392540:	b.ne	3926e4 <initOpenGLRenderer@@Base+0x4c4>  // b.any
  392544:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392548:	ldr	x8, [x8, #2664]
  39254c:	ldr	w8, [x8]
  392550:	cmp	w8, #0xa
  392554:	b.lt	3928b4 <initOpenGLRenderer@@Base+0x694>  // b.tstop
  392558:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39255c:	ldr	x8, [x8, #2736]
  392560:	ldr	w8, [x8]
  392564:	add	w9, w8, #0x1
  392568:	mul	w8, w9, w8
  39256c:	tbnz	w8, #0, 392908 <initOpenGLRenderer@@Base+0x6e8>
  392570:	b	3928b4 <initOpenGLRenderer@@Base+0x694>
  392574:	mov	w9, #0x94b8                	// #38072
  392578:	movk	w9, #0x592, lsl #16
  39257c:	cmp	w8, w9
  392580:	b.eq	3925c4 <initOpenGLRenderer@@Base+0x3a4>  // b.none
  392584:	mov	w9, #0xd5ba                	// #54714
  392588:	movk	w9, #0xd87, lsl #16
  39258c:	cmp	w8, w9
  392590:	b.ne	3926f4 <initOpenGLRenderer@@Base+0x4d4>  // b.any
  392594:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392598:	ldr	x8, [x8, #2840]
  39259c:	ldr	w8, [x8]
  3925a0:	cmp	w8, #0xa
  3925a4:	b.lt	3927d8 <initOpenGLRenderer@@Base+0x5b8>  // b.tstop
  3925a8:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3925ac:	ldr	x8, [x8, #2848]
  3925b0:	ldr	w8, [x8]
  3925b4:	add	w9, w8, #0x1
  3925b8:	mul	w8, w9, w8
  3925bc:	tbnz	w8, #0, 392838 <initOpenGLRenderer@@Base+0x618>
  3925c0:	b	3927d8 <initOpenGLRenderer@@Base+0x5b8>
  3925c4:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3925c8:	ldr	x8, [x8, #2680]
  3925cc:	ldr	w8, [x8]
  3925d0:	cmp	w8, #0xa
  3925d4:	b.lt	392934 <initOpenGLRenderer@@Base+0x714>  // b.tstop
  3925d8:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3925dc:	ldr	x8, [x8, #2672]
  3925e0:	ldr	w8, [x8]
  3925e4:	add	w9, w8, #0x1
  3925e8:	mul	w8, w9, w8
  3925ec:	tbnz	w8, #0, 392978 <initOpenGLRenderer@@Base+0x758>
  3925f0:	b	392934 <initOpenGLRenderer@@Base+0x714>
  3925f4:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  3925f8:	ldr	x8, [x8, #1120]
  3925fc:	ldr	w8, [x8]
  392600:	cmp	w8, #0xa
  392604:	b.lt	39344c <initOpenGLRenderer@@Base+0x122c>  // b.tstop
  392608:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  39260c:	ldr	x8, [x8, #1128]
  392610:	ldr	w8, [x8]
  392614:	add	w9, w8, #0x1
  392618:	mul	w8, w9, w8
  39261c:	tbnz	w8, #0, 393498 <initOpenGLRenderer@@Base+0x1278>
  392620:	b	39344c <initOpenGLRenderer@@Base+0x122c>
  392624:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392628:	ldr	x8, [x8, #2920]
  39262c:	ldr	w8, [x8]
  392630:	cmp	w8, #0xa
  392634:	b.lt	392bb4 <initOpenGLRenderer@@Base+0x994>  // b.tstop
  392638:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39263c:	ldr	x8, [x8, #2928]
  392640:	ldr	w8, [x8]
  392644:	add	w9, w8, #0x1
  392648:	mul	w8, w9, w8
  39264c:	tbnz	w8, #0, 393054 <initOpenGLRenderer@@Base+0xe34>
  392650:	b	392bb4 <initOpenGLRenderer@@Base+0x994>
  392654:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392658:	ldr	x8, [x8, #2856]
  39265c:	ldr	w8, [x8]
  392660:	cmp	w8, #0xa
  392664:	b.lt	392b54 <initOpenGLRenderer@@Base+0x934>  // b.tstop
  392668:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39266c:	ldr	x8, [x8, #2864]
  392670:	ldr	w8, [x8]
  392674:	add	w9, w8, #0x1
  392678:	mul	w8, w9, w8
  39267c:	tbnz	w8, #0, 392b98 <initOpenGLRenderer@@Base+0x978>
  392680:	b	392b54 <initOpenGLRenderer@@Base+0x934>
  392684:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392688:	ldr	x8, [x8, #2760]
  39268c:	ldr	w8, [x8]
  392690:	cmp	w8, #0xa
  392694:	b.lt	392994 <initOpenGLRenderer@@Base+0x774>  // b.tstop
  392698:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39269c:	ldr	x8, [x8, #2768]
  3926a0:	ldr	w8, [x8]
  3926a4:	add	w9, w8, #0x1
  3926a8:	mul	w8, w9, w8
  3926ac:	tbnz	w8, #0, 392a10 <initOpenGLRenderer@@Base+0x7f0>
  3926b0:	b	392994 <initOpenGLRenderer@@Base+0x774>
  3926b4:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3926b8:	ldr	x8, [x8, #2808]
  3926bc:	ldr	w8, [x8]
  3926c0:	cmp	w8, #0xa
  3926c4:	b.lt	392a64 <initOpenGLRenderer@@Base+0x844>  // b.tstop
  3926c8:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3926cc:	ldr	x8, [x8, #2816]
  3926d0:	ldr	w8, [x8]
  3926d4:	add	w9, w8, #0x1
  3926d8:	mul	w8, w9, w8
  3926dc:	tbnz	w8, #0, 392ab8 <initOpenGLRenderer@@Base+0x898>
  3926e0:	b	392a64 <initOpenGLRenderer@@Base+0x844>
  3926e4:	mov	w9, #0x200c                	// #8204
  3926e8:	movk	w9, #0xd108, lsl #16
  3926ec:	cmp	w8, w9
  3926f0:	b.eq	393538 <initOpenGLRenderer@@Base+0x1318>  // b.none
  3926f4:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3926f8:	ldr	x8, [x8, #2440]
  3926fc:	adrp	x9, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392700:	ldr	w8, [x8]
  392704:	ldr	x9, [x9, #2432]
  392708:	cmp	w8, #0xa
  39270c:	ldr	w9, [x9]
  392710:	cset	w10, lt  // lt = tstop
  392714:	add	w8, w9, #0x1
  392718:	mul	w8, w8, w9
  39271c:	tst	w8, #0x1
  392720:	cset	w8, eq  // eq = none
  392724:	orr	w8, w10, w8
  392728:	cbz	w8, 392728 <initOpenGLRenderer@@Base+0x508>
  39272c:	b	392870 <initOpenGLRenderer@@Base+0x650>
  392730:	mov	w0, #0x18                  	// #24
  392734:	bl	718540 <_Znwm@plt>
  392738:	ldp	w2, w8, [x29, #-112]
  39273c:	ldur	w1, [x29, #-116]
  392740:	mov	w3, #0x1                   	// #1
  392744:	mov	x27, x0
  392748:	and	w4, w8, #0x1
  39274c:	bl	3977fc <.datadiv_decode16147822148815391081@@Base+0x378>
  392750:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  392754:	str	x27, [x8, #3464]
  392758:	adrp	x27, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39275c:	ldr	x27, [x27, #2368]
  392760:	ldur	x8, [x29, #-80]
  392764:	mov	w9, #0xd5ba                	// #54714
  392768:	movk	w9, #0xd87, lsl #16
  39276c:	str	w9, [x8]
  392770:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392774:	ldr	x8, [x8, #2728]
  392778:	ldr	w8, [x8]
  39277c:	cmp	w8, #0xa
  392780:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  392784:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392788:	ldr	x8, [x8, #2720]
  39278c:	ldr	w8, [x8]
  392790:	add	w9, w8, #0x1
  392794:	mul	w8, w9, w8
  392798:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  39279c:	mov	w0, #0x18                  	// #24
  3927a0:	bl	718540 <_Znwm@plt>
  3927a4:	ldp	w2, w8, [x29, #-112]
  3927a8:	ldur	w1, [x29, #-116]
  3927ac:	mov	w3, #0x1                   	// #1
  3927b0:	mov	x27, x0
  3927b4:	and	w4, w8, #0x1
  3927b8:	bl	3977fc <.datadiv_decode16147822148815391081@@Base+0x378>
  3927bc:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  3927c0:	str	x27, [x8, #3464]
  3927c4:	ldur	x8, [x29, #-80]
  3927c8:	mov	w9, #0xd5ba                	// #54714
  3927cc:	movk	w9, #0xd87, lsl #16
  3927d0:	str	w9, [x8]
  3927d4:	b	392730 <initOpenGLRenderer@@Base+0x510>
  3927d8:	ldur	x1, [x29, #-104]
  3927dc:	mov	x0, x20
  3927e0:	bl	37faec <.datadiv_decode16168515696006275425@@Base+0xad0>
  3927e4:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  3927e8:	str	x0, [x8, #3456]
  3927ec:	ldur	x8, [x29, #-80]
  3927f0:	mov	w9, #0xf901                	// #63745
  3927f4:	mov	w10, #0xea2b                	// #59947
  3927f8:	cmp	x0, #0x0
  3927fc:	movk	w9, #0xcde4, lsl #16
  392800:	movk	w10, #0x55d4, lsl #16
  392804:	csel	w9, w9, w10, eq  // eq = none
  392808:	str	w9, [x8]
  39280c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392810:	ldr	x8, [x8, #2888]
  392814:	ldr	w8, [x8]
  392818:	cmp	w8, #0xa
  39281c:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  392820:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392824:	ldr	x8, [x8, #2896]
  392828:	ldr	w8, [x8]
  39282c:	add	w9, w8, #0x1
  392830:	mul	w8, w9, w8
  392834:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  392838:	ldur	x1, [x29, #-104]
  39283c:	mov	x0, x20
  392840:	bl	37faec <.datadiv_decode16168515696006275425@@Base+0xad0>
  392844:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  392848:	str	x0, [x8, #3456]
  39284c:	ldur	x8, [x29, #-80]
  392850:	mov	w9, #0xf901                	// #63745
  392854:	mov	w10, #0xea2b                	// #59947
  392858:	cmp	x0, #0x0
  39285c:	movk	w9, #0xcde4, lsl #16
  392860:	movk	w10, #0x55d4, lsl #16
  392864:	csel	w9, w9, w10, eq  // eq = none
  392868:	str	w9, [x8]
  39286c:	b	3927d8 <initOpenGLRenderer@@Base+0x5b8>
  392870:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392874:	ldr	x8, [x8, #3008]
  392878:	adrp	x9, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39287c:	ldr	w8, [x8]
  392880:	ldr	x9, [x9, #3000]
  392884:	cmp	w8, #0xa
  392888:	ldr	w9, [x9]
  39288c:	cset	w10, lt  // lt = tstop
  392890:	add	w8, w9, #0x1
  392894:	mul	w8, w8, w9
  392898:	tst	w8, #0x1
  39289c:	cset	w8, eq  // eq = none
  3928a0:	orr	w8, w10, w8
  3928a4:	adrp	x10, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3928a8:	ldr	x10, [x10, #2376]
  3928ac:	tbz	w8, #0, 3928ac <initOpenGLRenderer@@Base+0x68c>
  3928b0:	b	3923cc <initOpenGLRenderer@@Base+0x1ac>
  3928b4:	ldur	x8, [x29, #-88]
  3928b8:	mov	w10, #0x94b8                	// #38072
  3928bc:	movk	w10, #0x592, lsl #16
  3928c0:	ldrb	w8, [x8]
  3928c4:	ldur	x9, [x29, #-80]
  3928c8:	cmp	w8, #0x0
  3928cc:	mov	w8, #0x42b8                	// #17080
  3928d0:	movk	w8, #0xbd04, lsl #16
  3928d4:	csel	w8, w10, w8, ne  // ne = any
  3928d8:	str	w8, [x9]
  3928dc:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3928e0:	ldr	x8, [x8, #2744]
  3928e4:	ldr	w8, [x8]
  3928e8:	cmp	w8, #0xa
  3928ec:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  3928f0:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3928f4:	ldr	x8, [x8, #2752]
  3928f8:	ldr	w8, [x8]
  3928fc:	add	w9, w8, #0x1
  392900:	mul	w8, w9, w8
  392904:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  392908:	ldur	x8, [x29, #-88]
  39290c:	mov	w10, #0x94b8                	// #38072
  392910:	movk	w10, #0x592, lsl #16
  392914:	ldrb	w8, [x8]
  392918:	ldur	x9, [x29, #-80]
  39291c:	cmp	w8, #0x0
  392920:	mov	w8, #0x42b8                	// #17080
  392924:	movk	w8, #0xbd04, lsl #16
  392928:	csel	w8, w10, w8, ne  // ne = any
  39292c:	str	w8, [x9]
  392930:	b	3928b4 <initOpenGLRenderer@@Base+0x694>
  392934:	ldur	x8, [x29, #-80]
  392938:	mov	w9, #0x200c                	// #8204
  39293c:	movk	w9, #0xd108, lsl #16
  392940:	str	w9, [x8]
  392944:	ldur	x8, [x29, #-64]
  392948:	str	wzr, [x8]
  39294c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392950:	ldr	x8, [x8, #2696]
  392954:	ldr	w8, [x8]
  392958:	cmp	w8, #0xa
  39295c:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  392960:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392964:	ldr	x8, [x8, #2688]
  392968:	ldr	w8, [x8]
  39296c:	add	w9, w8, #0x1
  392970:	mul	w8, w9, w8
  392974:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  392978:	ldur	x8, [x29, #-80]
  39297c:	mov	w9, #0x200c                	// #8204
  392980:	movk	w9, #0xd108, lsl #16
  392984:	str	w9, [x8]
  392988:	ldur	x8, [x29, #-64]
  39298c:	str	wzr, [x8]
  392990:	b	392934 <initOpenGLRenderer@@Base+0x714>
  392994:	adrp	x1, 762000 <_ZTISt10bad_typeid@@Base+0x38fa0>
  392998:	adrp	x2, 762000 <_ZTISt10bad_typeid@@Base+0x38fa0>
  39299c:	mov	w0, #0x6                   	// #6
  3929a0:	add	x1, x1, #0x7e0
  3929a4:	add	x2, x2, #0x830
  3929a8:	bl	718760 <__android_log_print@plt>
  3929ac:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  3929b0:	ldr	x8, [x8, #3464]
  3929b4:	ldur	x9, [x29, #-96]
  3929b8:	mov	w10, #0x867c                	// #34428
  3929bc:	movk	w10, #0x4278, lsl #16
  3929c0:	str	x8, [x9]
  3929c4:	ldur	x8, [x29, #-96]
  3929c8:	ldr	x8, [x8]
  3929cc:	ldur	x9, [x29, #-80]
  3929d0:	cmp	x8, #0x0
  3929d4:	mov	w8, #0x557c                	// #21884
  3929d8:	movk	w8, #0x631e, lsl #16
  3929dc:	csel	w8, w8, w10, eq  // eq = none
  3929e0:	str	w8, [x9]
  3929e4:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3929e8:	ldr	x8, [x8, #2776]
  3929ec:	ldr	w8, [x8]
  3929f0:	cmp	w8, #0xa
  3929f4:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  3929f8:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3929fc:	ldr	x8, [x8, #2784]
  392a00:	ldr	w8, [x8]
  392a04:	add	w9, w8, #0x1
  392a08:	mul	w8, w9, w8
  392a0c:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  392a10:	adrp	x1, 762000 <_ZTISt10bad_typeid@@Base+0x38fa0>
  392a14:	adrp	x2, 762000 <_ZTISt10bad_typeid@@Base+0x38fa0>
  392a18:	mov	w0, #0x6                   	// #6
  392a1c:	add	x1, x1, #0x7e0
  392a20:	add	x2, x2, #0x830
  392a24:	bl	718760 <__android_log_print@plt>
  392a28:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  392a2c:	ldr	x8, [x8, #3464]
  392a30:	ldur	x9, [x29, #-96]
  392a34:	mov	w10, #0x867c                	// #34428
  392a38:	movk	w10, #0x4278, lsl #16
  392a3c:	str	x8, [x9]
  392a40:	ldur	x8, [x29, #-96]
  392a44:	ldr	x8, [x8]
  392a48:	ldur	x9, [x29, #-80]
  392a4c:	cmp	x8, #0x0
  392a50:	mov	w8, #0x557c                	// #21884
  392a54:	movk	w8, #0x631e, lsl #16
  392a58:	csel	w8, w8, w10, eq  // eq = none
  392a5c:	str	w8, [x9]
  392a60:	b	392994 <initOpenGLRenderer@@Base+0x774>
  392a64:	ldur	x8, [x29, #-96]
  392a68:	ldr	x0, [x8]
  392a6c:	bl	3985b4 <.datadiv_decode16147822148815391081@@Base+0x1130>
  392a70:	ldur	x8, [x29, #-96]
  392a74:	ldr	x0, [x8]
  392a78:	bl	7185f0 <_ZdlPv@plt>
  392a7c:	ldur	x8, [x29, #-80]
  392a80:	mov	w9, #0x557c                	// #21884
  392a84:	movk	w9, #0x631e, lsl #16
  392a88:	str	w9, [x8]
  392a8c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392a90:	ldr	x8, [x8, #2800]
  392a94:	ldr	w8, [x8]
  392a98:	cmp	w8, #0xa
  392a9c:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  392aa0:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392aa4:	ldr	x8, [x8, #2792]
  392aa8:	ldr	w8, [x8]
  392aac:	add	w9, w8, #0x1
  392ab0:	mul	w8, w9, w8
  392ab4:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  392ab8:	ldur	x8, [x29, #-96]
  392abc:	ldr	x0, [x8]
  392ac0:	bl	3985b4 <.datadiv_decode16147822148815391081@@Base+0x1130>
  392ac4:	ldur	x8, [x29, #-96]
  392ac8:	ldr	x0, [x8]
  392acc:	bl	7185f0 <_ZdlPv@plt>
  392ad0:	ldur	x8, [x29, #-80]
  392ad4:	mov	w9, #0x557c                	// #21884
  392ad8:	movk	w9, #0x631e, lsl #16
  392adc:	str	w9, [x8]
  392ae0:	b	392a64 <initOpenGLRenderer@@Base+0x844>
  392ae4:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  392ae8:	str	xzr, [x8, #3464]
  392aec:	ldur	x8, [x29, #-80]
  392af0:	mov	w9, #0xf83a                	// #63546
  392af4:	movk	w9, #0xac7f, lsl #16
  392af8:	str	w9, [x8]
  392afc:	ldur	x8, [x29, #-72]
  392b00:	str	wzr, [x8]
  392b04:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392b08:	ldr	x8, [x8, #2824]
  392b0c:	ldr	w8, [x8]
  392b10:	cmp	w8, #0xa
  392b14:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  392b18:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392b1c:	ldr	x8, [x8, #2832]
  392b20:	ldr	w8, [x8]
  392b24:	add	w9, w8, #0x1
  392b28:	mul	w8, w9, w8
  392b2c:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  392b30:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  392b34:	str	xzr, [x8, #3464]
  392b38:	ldur	x8, [x29, #-80]
  392b3c:	mov	w9, #0xf83a                	// #63546
  392b40:	movk	w9, #0xac7f, lsl #16
  392b44:	str	w9, [x8]
  392b48:	ldur	x8, [x29, #-72]
  392b4c:	str	wzr, [x8]
  392b50:	b	392ae4 <initOpenGLRenderer@@Base+0x8c4>
  392b54:	ldur	x8, [x29, #-80]
  392b58:	mov	w9, #0xf83a                	// #63546
  392b5c:	movk	w9, #0xac7f, lsl #16
  392b60:	str	w9, [x8]
  392b64:	ldur	x8, [x29, #-72]
  392b68:	str	wzr, [x8]
  392b6c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392b70:	ldr	x8, [x8, #2904]
  392b74:	ldr	w8, [x8]
  392b78:	cmp	w8, #0xa
  392b7c:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  392b80:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392b84:	ldr	x8, [x8, #2912]
  392b88:	ldr	w8, [x8]
  392b8c:	add	w9, w8, #0x1
  392b90:	mul	w8, w9, w8
  392b94:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  392b98:	ldur	x8, [x29, #-80]
  392b9c:	mov	w9, #0xf83a                	// #63546
  392ba0:	movk	w9, #0xac7f, lsl #16
  392ba4:	str	w9, [x8]
  392ba8:	ldur	x8, [x29, #-72]
  392bac:	str	wzr, [x8]
  392bb0:	b	392b54 <initOpenGLRenderer@@Base+0x934>
  392bb4:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392bb8:	ldr	x8, [x8, #1104]
  392bbc:	mov	x27, sp
  392bc0:	ldr	w8, [x8]
  392bc4:	cmp	w8, #0xa
  392bc8:	b.lt	392be4 <initOpenGLRenderer@@Base+0x9c4>  // b.tstop
  392bcc:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392bd0:	ldr	x8, [x8, #1112]
  392bd4:	ldr	w8, [x8]
  392bd8:	add	w9, w8, #0x1
  392bdc:	mul	w8, w9, w8
  392be0:	tbnz	w8, #0, 392c50 <initOpenGLRenderer@@Base+0xa30>
  392be4:	sub	x8, sp, #0x10
  392be8:	mov	sp, x8
  392bec:	stur	x8, [x29, #-48]
  392bf0:	ldur	x8, [x29, #-48]
  392bf4:	mov	w9, #0x1                   	// #1
  392bf8:	strb	w9, [x8]
  392bfc:	sub	x8, sp, #0x10
  392c00:	mov	sp, x8
  392c04:	stur	x8, [x29, #-40]
  392c08:	ldur	x8, [x29, #-40]
  392c0c:	mov	w9, #0xa895                	// #43157
  392c10:	movk	w9, #0x4b4e, lsl #16
  392c14:	str	w9, [x8]
  392c18:	sub	x8, sp, #0x10
  392c1c:	mov	sp, x8
  392c20:	adrp	x9, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392c24:	ldr	x9, [x9, #3104]
  392c28:	stur	x8, [x29, #-32]
  392c2c:	ldr	w9, [x9]
  392c30:	cmp	w9, #0xa
  392c34:	b.lt	392e4c <initOpenGLRenderer@@Base+0xc2c>  // b.tstop
  392c38:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392c3c:	ldr	x8, [x8, #3096]
  392c40:	ldr	w8, [x8]
  392c44:	add	w9, w8, #0x1
  392c48:	mul	w8, w9, w8
  392c4c:	tbz	w8, #0, 392e4c <initOpenGLRenderer@@Base+0xc2c>
  392c50:	sub	x8, sp, #0x10
  392c54:	mov	sp, x8
  392c58:	stur	x8, [x29, #-48]
  392c5c:	ldur	x8, [x29, #-48]
  392c60:	mov	w9, #0x1                   	// #1
  392c64:	strb	w9, [x8]
  392c68:	sub	x8, sp, #0x10
  392c6c:	mov	sp, x8
  392c70:	stur	x8, [x29, #-40]
  392c74:	ldur	x8, [x29, #-40]
  392c78:	mov	w9, #0xa895                	// #43157
  392c7c:	movk	w9, #0x4b4e, lsl #16
  392c80:	str	w9, [x8]
  392c84:	sub	x8, sp, #0x10
  392c88:	mov	sp, x8
  392c8c:	stur	x8, [x29, #-32]
  392c90:	b	392be4 <initOpenGLRenderer@@Base+0x9c4>
  392c94:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392c98:	ldr	x8, [x8, #3040]
  392c9c:	adrp	x9, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392ca0:	ldr	w8, [x8]
  392ca4:	ldr	x9, [x9, #3032]
  392ca8:	cmp	w8, #0xa
  392cac:	ldr	w9, [x9]
  392cb0:	cset	w10, lt  // lt = tstop
  392cb4:	add	w8, w9, #0x1
  392cb8:	mul	w8, w8, w9
  392cbc:	tst	w8, #0x1
  392cc0:	cset	w8, eq  // eq = none
  392cc4:	orr	w8, w10, w8
  392cc8:	cbz	w8, 392cc8 <initOpenGLRenderer@@Base+0xaa8>
  392ccc:	b	392d50 <initOpenGLRenderer@@Base+0xb30>
  392cd0:	mov	w2, #0x100                 	// #256
  392cd4:	mov	w3, #0x100                 	// #256
  392cd8:	mov	x4, #0xffffffffffffffff    	// #-1
  392cdc:	mov	x0, x26
  392ce0:	mov	x1, x20
  392ce4:	bl	7188f0 <__strncpy_chk2@plt>
  392ce8:	ldur	x8, [x29, #-40]
  392cec:	str	w21, [x8]
  392cf0:	ldur	x8, [x29, #-32]
  392cf4:	str	x0, [x8]
  392cf8:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392cfc:	ldr	x8, [x8, #576]
  392d00:	ldr	w8, [x8]
  392d04:	cmp	w8, #0xa
  392d08:	b.lt	392d50 <initOpenGLRenderer@@Base+0xb30>  // b.tstop
  392d0c:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392d10:	ldr	x8, [x8, #568]
  392d14:	ldr	w8, [x8]
  392d18:	add	w9, w8, #0x1
  392d1c:	mul	w8, w9, w8
  392d20:	tbz	w8, #0, 392d50 <initOpenGLRenderer@@Base+0xb30>
  392d24:	mov	w2, #0x100                 	// #256
  392d28:	mov	w3, #0x100                 	// #256
  392d2c:	mov	x4, #0xffffffffffffffff    	// #-1
  392d30:	mov	x0, x26
  392d34:	mov	x1, x20
  392d38:	bl	7188f0 <__strncpy_chk2@plt>
  392d3c:	ldur	x8, [x29, #-40]
  392d40:	str	w21, [x8]
  392d44:	ldur	x8, [x29, #-32]
  392d48:	str	x0, [x8]
  392d4c:	b	392cd0 <initOpenGLRenderer@@Base+0xab0>
  392d50:	ldr	w8, [x28]
  392d54:	ldr	w9, [x25]
  392d58:	cmp	w8, #0xa
  392d5c:	add	w8, w9, #0x1
  392d60:	mul	w8, w8, w9
  392d64:	cset	w10, lt  // lt = tstop
  392d68:	tst	w8, #0x1
  392d6c:	cset	w8, eq  // eq = none
  392d70:	orr	w8, w10, w8
  392d74:	tbz	w8, #0, 392d74 <initOpenGLRenderer@@Base+0xb54>
  392d78:	b	392e4c <initOpenGLRenderer@@Base+0xc2c>
  392d7c:	ldur	x8, [x29, #-48]
  392d80:	ldrb	w8, [x8]
  392d84:	ldur	x9, [x29, #-40]
  392d88:	cmp	w8, #0x0
  392d8c:	csel	w8, w24, w23, ne  // ne = any
  392d90:	str	w8, [x9]
  392d94:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392d98:	ldr	x8, [x8, #3128]
  392d9c:	ldr	w8, [x8]
  392da0:	cmp	w8, #0xa
  392da4:	b.lt	392d50 <initOpenGLRenderer@@Base+0xb30>  // b.tstop
  392da8:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392dac:	ldr	x8, [x8, #3136]
  392db0:	ldr	w8, [x8]
  392db4:	add	w9, w8, #0x1
  392db8:	mul	w8, w9, w8
  392dbc:	tbz	w8, #0, 392d50 <initOpenGLRenderer@@Base+0xb30>
  392dc0:	ldur	x8, [x29, #-48]
  392dc4:	ldrb	w8, [x8]
  392dc8:	ldur	x9, [x29, #-40]
  392dcc:	cmp	w8, #0x0
  392dd0:	csel	w8, w24, w23, ne  // ne = any
  392dd4:	str	w8, [x9]
  392dd8:	b	392d7c <initOpenGLRenderer@@Base+0xb5c>
  392ddc:	mov	w2, #0x100                 	// #256
  392de0:	mov	x0, x26
  392de4:	mov	x1, x20
  392de8:	bl	718900 <strncpy@plt>
  392dec:	ldur	x8, [x29, #-40]
  392df0:	str	w21, [x8]
  392df4:	ldur	x8, [x29, #-32]
  392df8:	str	x26, [x8]
  392dfc:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392e00:	ldr	x8, [x8, #3056]
  392e04:	ldr	w8, [x8]
  392e08:	cmp	w8, #0xa
  392e0c:	b.lt	392d50 <initOpenGLRenderer@@Base+0xb30>  // b.tstop
  392e10:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392e14:	ldr	x8, [x8, #3048]
  392e18:	ldr	w8, [x8]
  392e1c:	add	w9, w8, #0x1
  392e20:	mul	w8, w9, w8
  392e24:	tbz	w8, #0, 392d50 <initOpenGLRenderer@@Base+0xb30>
  392e28:	mov	w2, #0x100                 	// #256
  392e2c:	mov	x0, x26
  392e30:	mov	x1, x20
  392e34:	bl	718900 <strncpy@plt>
  392e38:	ldur	x8, [x29, #-40]
  392e3c:	str	w21, [x8]
  392e40:	ldur	x8, [x29, #-32]
  392e44:	str	x26, [x8]
  392e48:	b	392ddc <initOpenGLRenderer@@Base+0xbbc>
  392e4c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392e50:	ldr	x8, [x8, #3072]
  392e54:	ldr	w8, [x8]
  392e58:	cmp	w8, #0xa
  392e5c:	b.lt	392e78 <initOpenGLRenderer@@Base+0xc58>  // b.tstop
  392e60:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392e64:	ldr	x8, [x8, #3064]
  392e68:	ldr	w8, [x8]
  392e6c:	add	w9, w8, #0x1
  392e70:	mul	w8, w9, w8
  392e74:	tbnz	w8, #0, 392ea0 <initOpenGLRenderer@@Base+0xc80>
  392e78:	ldur	x8, [x29, #-40]
  392e7c:	ldr	w9, [x22]
  392e80:	ldr	w8, [x8]
  392e84:	cmp	w9, #0xa
  392e88:	stur	w8, [x29, #-20]
  392e8c:	b.lt	392eb0 <initOpenGLRenderer@@Base+0xc90>  // b.tstop
  392e90:	ldr	w8, [x19]
  392e94:	add	w9, w8, #0x1
  392e98:	mul	w8, w9, w8
  392e9c:	tbz	w8, #0, 392eb0 <initOpenGLRenderer@@Base+0xc90>
  392ea0:	ldur	x8, [x29, #-40]
  392ea4:	ldr	w8, [x8]
  392ea8:	stur	w8, [x29, #-20]
  392eac:	b	392e78 <initOpenGLRenderer@@Base+0xc58>
  392eb0:	ldur	w8, [x29, #-20]
  392eb4:	mov	w9, #0xa894                	// #43156
  392eb8:	movk	w9, #0x4b4e, lsl #16
  392ebc:	cmp	w8, w9
  392ec0:	b.gt	392f04 <initOpenGLRenderer@@Base+0xce4>
  392ec4:	cmp	w8, w24
  392ec8:	b.eq	392f44 <initOpenGLRenderer@@Base+0xd24>  // b.none
  392ecc:	cmp	w8, w23
  392ed0:	b.ne	392c94 <initOpenGLRenderer@@Base+0xa74>  // b.any
  392ed4:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392ed8:	ldr	x8, [x8, #560]
  392edc:	ldr	w8, [x8]
  392ee0:	cmp	w8, #0xa
  392ee4:	b.lt	392cd0 <initOpenGLRenderer@@Base+0xab0>  // b.tstop
  392ee8:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392eec:	ldr	x8, [x8, #552]
  392ef0:	ldr	w8, [x8]
  392ef4:	add	w9, w8, #0x1
  392ef8:	mul	w8, w9, w8
  392efc:	tbnz	w8, #0, 392d24 <initOpenGLRenderer@@Base+0xb04>
  392f00:	b	392cd0 <initOpenGLRenderer@@Base+0xab0>
  392f04:	mov	w9, #0xa895                	// #43157
  392f08:	movk	w9, #0x4b4e, lsl #16
  392f0c:	cmp	w8, w9
  392f10:	b.ne	392f74 <initOpenGLRenderer@@Base+0xd54>  // b.any
  392f14:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392f18:	ldr	x8, [x8, #3112]
  392f1c:	ldr	w8, [x8]
  392f20:	cmp	w8, #0xa
  392f24:	b.lt	392d7c <initOpenGLRenderer@@Base+0xb5c>  // b.tstop
  392f28:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  392f2c:	ldr	x8, [x8, #3120]
  392f30:	ldr	w8, [x8]
  392f34:	add	w9, w8, #0x1
  392f38:	mul	w8, w9, w8
  392f3c:	tbnz	w8, #0, 392dc0 <initOpenGLRenderer@@Base+0xba0>
  392f40:	b	392d7c <initOpenGLRenderer@@Base+0xb5c>
  392f44:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392f48:	ldr	x8, [x8, #1192]
  392f4c:	ldr	w8, [x8]
  392f50:	cmp	w8, #0xa
  392f54:	b.lt	392ddc <initOpenGLRenderer@@Base+0xbbc>  // b.tstop
  392f58:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392f5c:	ldr	x8, [x8, #1232]
  392f60:	ldr	w8, [x8]
  392f64:	add	w9, w8, #0x1
  392f68:	mul	w8, w9, w8
  392f6c:	tbnz	w8, #0, 392e28 <initOpenGLRenderer@@Base+0xc08>
  392f70:	b	392ddc <initOpenGLRenderer@@Base+0xbbc>
  392f74:	cmp	w8, w21
  392f78:	b.ne	392c94 <initOpenGLRenderer@@Base+0xa74>  // b.any
  392f7c:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392f80:	ldr	x8, [x8, #608]
  392f84:	ldr	w8, [x8]
  392f88:	cmp	w8, #0xa
  392f8c:	b.lt	392fa8 <initOpenGLRenderer@@Base+0xd88>  // b.tstop
  392f90:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392f94:	ldr	x8, [x8, #600]
  392f98:	ldr	w8, [x8]
  392f9c:	add	w9, w8, #0x1
  392fa0:	mul	w8, w9, w8
  392fa4:	tbnz	w8, #0, 392fe0 <initOpenGLRenderer@@Base+0xdc0>
  392fa8:	adrp	x9, 72a000 <x.30@@Base-0x432ac>
  392fac:	ldur	x8, [x29, #-32]
  392fb0:	ldr	x9, [x9, #624]
  392fb4:	ldr	x8, [x8]
  392fb8:	ldr	w9, [x9]
  392fbc:	stur	x8, [x29, #-16]
  392fc0:	cmp	w9, #0xa
  392fc4:	b.lt	392ff0 <initOpenGLRenderer@@Base+0xdd0>  // b.tstop
  392fc8:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  392fcc:	ldr	x8, [x8, #616]
  392fd0:	ldr	w8, [x8]
  392fd4:	add	w9, w8, #0x1
  392fd8:	mul	w8, w9, w8
  392fdc:	tbz	w8, #0, 392ff0 <initOpenGLRenderer@@Base+0xdd0>
  392fe0:	ldur	x8, [x29, #-32]
  392fe4:	ldr	x8, [x8]
  392fe8:	stur	x8, [x29, #-16]
  392fec:	b	392fa8 <initOpenGLRenderer@@Base+0xd88>
  392ff0:	ldur	x8, [x29, #-16]
  392ff4:	mov	sp, x27
  392ff8:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  392ffc:	ldr	x0, [x8, #3456]
  393000:	bl	26ed78 <.datadiv_decode3211576236516147487@@Base+0x1ec>
  393004:	ldur	x8, [x29, #-80]
  393008:	mov	w9, #0xf83a                	// #63546
  39300c:	movk	w9, #0xac7f, lsl #16
  393010:	adrp	x27, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393014:	str	w9, [x8]
  393018:	ldur	x8, [x29, #-72]
  39301c:	mov	w9, #0x1                   	// #1
  393020:	str	w9, [x8]
  393024:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393028:	ldr	x8, [x8, #2936]
  39302c:	ldr	w8, [x8]
  393030:	ldr	x27, [x27, #2368]
  393034:	cmp	w8, #0xa
  393038:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  39303c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393040:	ldr	x8, [x8, #2944]
  393044:	ldr	w8, [x8]
  393048:	add	w9, w8, #0x1
  39304c:	mul	w8, w9, w8
  393050:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  393054:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  393058:	ldr	x8, [x8, #1104]
  39305c:	mov	x27, sp
  393060:	ldr	w8, [x8]
  393064:	cmp	w8, #0xa
  393068:	b.lt	393084 <initOpenGLRenderer@@Base+0xe64>  // b.tstop
  39306c:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  393070:	ldr	x8, [x8, #1112]
  393074:	ldr	w8, [x8]
  393078:	add	w9, w8, #0x1
  39307c:	mul	w8, w9, w8
  393080:	tbnz	w8, #0, 3930f0 <initOpenGLRenderer@@Base+0xed0>
  393084:	sub	x8, sp, #0x10
  393088:	mov	sp, x8
  39308c:	stur	x8, [x29, #-48]
  393090:	ldur	x8, [x29, #-48]
  393094:	mov	w9, #0x1                   	// #1
  393098:	strb	w9, [x8]
  39309c:	sub	x8, sp, #0x10
  3930a0:	mov	sp, x8
  3930a4:	stur	x8, [x29, #-40]
  3930a8:	ldur	x8, [x29, #-40]
  3930ac:	mov	w9, #0xa895                	// #43157
  3930b0:	movk	w9, #0x4b4e, lsl #16
  3930b4:	str	w9, [x8]
  3930b8:	sub	x8, sp, #0x10
  3930bc:	mov	sp, x8
  3930c0:	adrp	x9, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3930c4:	ldr	x9, [x9, #3104]
  3930c8:	stur	x8, [x29, #-32]
  3930cc:	ldr	w9, [x9]
  3930d0:	cmp	w9, #0xa
  3930d4:	b.lt	3932ec <initOpenGLRenderer@@Base+0x10cc>  // b.tstop
  3930d8:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3930dc:	ldr	x8, [x8, #3096]
  3930e0:	ldr	w8, [x8]
  3930e4:	add	w9, w8, #0x1
  3930e8:	mul	w8, w9, w8
  3930ec:	tbz	w8, #0, 3932ec <initOpenGLRenderer@@Base+0x10cc>
  3930f0:	sub	x8, sp, #0x10
  3930f4:	mov	sp, x8
  3930f8:	stur	x8, [x29, #-48]
  3930fc:	ldur	x8, [x29, #-48]
  393100:	mov	w9, #0x1                   	// #1
  393104:	strb	w9, [x8]
  393108:	sub	x8, sp, #0x10
  39310c:	mov	sp, x8
  393110:	stur	x8, [x29, #-40]
  393114:	ldur	x8, [x29, #-40]
  393118:	mov	w9, #0xa895                	// #43157
  39311c:	movk	w9, #0x4b4e, lsl #16
  393120:	str	w9, [x8]
  393124:	sub	x8, sp, #0x10
  393128:	mov	sp, x8
  39312c:	stur	x8, [x29, #-32]
  393130:	b	393084 <initOpenGLRenderer@@Base+0xe64>
  393134:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393138:	ldr	x8, [x8, #3040]
  39313c:	adrp	x9, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393140:	ldr	w8, [x8]
  393144:	ldr	x9, [x9, #3032]
  393148:	cmp	w8, #0xa
  39314c:	ldr	w9, [x9]
  393150:	cset	w10, lt  // lt = tstop
  393154:	add	w8, w9, #0x1
  393158:	mul	w8, w8, w9
  39315c:	tst	w8, #0x1
  393160:	cset	w8, eq  // eq = none
  393164:	orr	w8, w10, w8
  393168:	cbz	w8, 393168 <initOpenGLRenderer@@Base+0xf48>
  39316c:	b	3931f0 <initOpenGLRenderer@@Base+0xfd0>
  393170:	mov	w2, #0x100                 	// #256
  393174:	mov	w3, #0x100                 	// #256
  393178:	mov	x4, #0xffffffffffffffff    	// #-1
  39317c:	mov	x0, x26
  393180:	mov	x1, x20
  393184:	bl	7188f0 <__strncpy_chk2@plt>
  393188:	ldur	x8, [x29, #-40]
  39318c:	str	w21, [x8]
  393190:	ldur	x8, [x29, #-32]
  393194:	str	x0, [x8]
  393198:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  39319c:	ldr	x8, [x8, #576]
  3931a0:	ldr	w8, [x8]
  3931a4:	cmp	w8, #0xa
  3931a8:	b.lt	3931f0 <initOpenGLRenderer@@Base+0xfd0>  // b.tstop
  3931ac:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  3931b0:	ldr	x8, [x8, #568]
  3931b4:	ldr	w8, [x8]
  3931b8:	add	w9, w8, #0x1
  3931bc:	mul	w8, w9, w8
  3931c0:	tbz	w8, #0, 3931f0 <initOpenGLRenderer@@Base+0xfd0>
  3931c4:	mov	w2, #0x100                 	// #256
  3931c8:	mov	w3, #0x100                 	// #256
  3931cc:	mov	x4, #0xffffffffffffffff    	// #-1
  3931d0:	mov	x0, x26
  3931d4:	mov	x1, x20
  3931d8:	bl	7188f0 <__strncpy_chk2@plt>
  3931dc:	ldur	x8, [x29, #-40]
  3931e0:	str	w21, [x8]
  3931e4:	ldur	x8, [x29, #-32]
  3931e8:	str	x0, [x8]
  3931ec:	b	393170 <initOpenGLRenderer@@Base+0xf50>
  3931f0:	ldr	w8, [x28]
  3931f4:	ldr	w9, [x25]
  3931f8:	cmp	w8, #0xa
  3931fc:	add	w8, w9, #0x1
  393200:	mul	w8, w8, w9
  393204:	cset	w10, lt  // lt = tstop
  393208:	tst	w8, #0x1
  39320c:	cset	w8, eq  // eq = none
  393210:	orr	w8, w10, w8
  393214:	tbz	w8, #0, 393214 <initOpenGLRenderer@@Base+0xff4>
  393218:	b	3932ec <initOpenGLRenderer@@Base+0x10cc>
  39321c:	ldur	x8, [x29, #-48]
  393220:	ldrb	w8, [x8]
  393224:	ldur	x9, [x29, #-40]
  393228:	cmp	w8, #0x0
  39322c:	csel	w8, w24, w23, ne  // ne = any
  393230:	str	w8, [x9]
  393234:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393238:	ldr	x8, [x8, #3128]
  39323c:	ldr	w8, [x8]
  393240:	cmp	w8, #0xa
  393244:	b.lt	3931f0 <initOpenGLRenderer@@Base+0xfd0>  // b.tstop
  393248:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39324c:	ldr	x8, [x8, #3136]
  393250:	ldr	w8, [x8]
  393254:	add	w9, w8, #0x1
  393258:	mul	w8, w9, w8
  39325c:	tbz	w8, #0, 3931f0 <initOpenGLRenderer@@Base+0xfd0>
  393260:	ldur	x8, [x29, #-48]
  393264:	ldrb	w8, [x8]
  393268:	ldur	x9, [x29, #-40]
  39326c:	cmp	w8, #0x0
  393270:	csel	w8, w24, w23, ne  // ne = any
  393274:	str	w8, [x9]
  393278:	b	39321c <initOpenGLRenderer@@Base+0xffc>
  39327c:	mov	w2, #0x100                 	// #256
  393280:	mov	x0, x26
  393284:	mov	x1, x20
  393288:	bl	718900 <strncpy@plt>
  39328c:	ldur	x8, [x29, #-40]
  393290:	str	w21, [x8]
  393294:	ldur	x8, [x29, #-32]
  393298:	str	x26, [x8]
  39329c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3932a0:	ldr	x8, [x8, #3056]
  3932a4:	ldr	w8, [x8]
  3932a8:	cmp	w8, #0xa
  3932ac:	b.lt	3931f0 <initOpenGLRenderer@@Base+0xfd0>  // b.tstop
  3932b0:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3932b4:	ldr	x8, [x8, #3048]
  3932b8:	ldr	w8, [x8]
  3932bc:	add	w9, w8, #0x1
  3932c0:	mul	w8, w9, w8
  3932c4:	tbz	w8, #0, 3931f0 <initOpenGLRenderer@@Base+0xfd0>
  3932c8:	mov	w2, #0x100                 	// #256
  3932cc:	mov	x0, x26
  3932d0:	mov	x1, x20
  3932d4:	bl	718900 <strncpy@plt>
  3932d8:	ldur	x8, [x29, #-40]
  3932dc:	str	w21, [x8]
  3932e0:	ldur	x8, [x29, #-32]
  3932e4:	str	x26, [x8]
  3932e8:	b	39327c <initOpenGLRenderer@@Base+0x105c>
  3932ec:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3932f0:	ldr	x8, [x8, #3072]
  3932f4:	ldr	w8, [x8]
  3932f8:	cmp	w8, #0xa
  3932fc:	b.lt	393318 <initOpenGLRenderer@@Base+0x10f8>  // b.tstop
  393300:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393304:	ldr	x8, [x8, #3064]
  393308:	ldr	w8, [x8]
  39330c:	add	w9, w8, #0x1
  393310:	mul	w8, w9, w8
  393314:	tbnz	w8, #0, 393340 <initOpenGLRenderer@@Base+0x1120>
  393318:	ldur	x8, [x29, #-40]
  39331c:	ldr	w9, [x22]
  393320:	ldr	w8, [x8]
  393324:	cmp	w9, #0xa
  393328:	stur	w8, [x29, #-20]
  39332c:	b.lt	393350 <initOpenGLRenderer@@Base+0x1130>  // b.tstop
  393330:	ldr	w8, [x19]
  393334:	add	w9, w8, #0x1
  393338:	mul	w8, w9, w8
  39333c:	tbz	w8, #0, 393350 <initOpenGLRenderer@@Base+0x1130>
  393340:	ldur	x8, [x29, #-40]
  393344:	ldr	w8, [x8]
  393348:	stur	w8, [x29, #-20]
  39334c:	b	393318 <initOpenGLRenderer@@Base+0x10f8>
  393350:	ldur	w8, [x29, #-20]
  393354:	mov	w9, #0xa894                	// #43156
  393358:	movk	w9, #0x4b4e, lsl #16
  39335c:	cmp	w8, w9
  393360:	b.gt	3933a4 <initOpenGLRenderer@@Base+0x1184>
  393364:	cmp	w8, w24
  393368:	b.eq	3933e4 <initOpenGLRenderer@@Base+0x11c4>  // b.none
  39336c:	cmp	w8, w23
  393370:	b.ne	393134 <initOpenGLRenderer@@Base+0xf14>  // b.any
  393374:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  393378:	ldr	x8, [x8, #560]
  39337c:	ldr	w8, [x8]
  393380:	cmp	w8, #0xa
  393384:	b.lt	393170 <initOpenGLRenderer@@Base+0xf50>  // b.tstop
  393388:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  39338c:	ldr	x8, [x8, #552]
  393390:	ldr	w8, [x8]
  393394:	add	w9, w8, #0x1
  393398:	mul	w8, w9, w8
  39339c:	tbnz	w8, #0, 3931c4 <initOpenGLRenderer@@Base+0xfa4>
  3933a0:	b	393170 <initOpenGLRenderer@@Base+0xf50>
  3933a4:	mov	w9, #0xa895                	// #43157
  3933a8:	movk	w9, #0x4b4e, lsl #16
  3933ac:	cmp	w8, w9
  3933b0:	b.ne	393414 <initOpenGLRenderer@@Base+0x11f4>  // b.any
  3933b4:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3933b8:	ldr	x8, [x8, #3112]
  3933bc:	ldr	w8, [x8]
  3933c0:	cmp	w8, #0xa
  3933c4:	b.lt	39321c <initOpenGLRenderer@@Base+0xffc>  // b.tstop
  3933c8:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  3933cc:	ldr	x8, [x8, #3120]
  3933d0:	ldr	w8, [x8]
  3933d4:	add	w9, w8, #0x1
  3933d8:	mul	w8, w9, w8
  3933dc:	tbnz	w8, #0, 393260 <initOpenGLRenderer@@Base+0x1040>
  3933e0:	b	39321c <initOpenGLRenderer@@Base+0xffc>
  3933e4:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  3933e8:	ldr	x8, [x8, #1192]
  3933ec:	ldr	w8, [x8]
  3933f0:	cmp	w8, #0xa
  3933f4:	b.lt	39327c <initOpenGLRenderer@@Base+0x105c>  // b.tstop
  3933f8:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  3933fc:	ldr	x8, [x8, #1232]
  393400:	ldr	w8, [x8]
  393404:	add	w9, w8, #0x1
  393408:	mul	w8, w9, w8
  39340c:	tbnz	w8, #0, 3932c8 <initOpenGLRenderer@@Base+0x10a8>
  393410:	b	39327c <initOpenGLRenderer@@Base+0x105c>
  393414:	cmp	w8, w21
  393418:	b.ne	393134 <initOpenGLRenderer@@Base+0xf14>  // b.any
  39341c:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  393420:	ldr	x8, [x8, #608]
  393424:	ldr	w8, [x8]
  393428:	cmp	w8, #0xa
  39342c:	b.lt	3934bc <initOpenGLRenderer@@Base+0x129c>  // b.tstop
  393430:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  393434:	ldr	x8, [x8, #600]
  393438:	ldr	w8, [x8]
  39343c:	add	w9, w8, #0x1
  393440:	mul	w8, w9, w8
  393444:	tbnz	w8, #0, 3934f4 <initOpenGLRenderer@@Base+0x12d4>
  393448:	b	3934bc <initOpenGLRenderer@@Base+0x129c>
  39344c:	ldur	x8, [x29, #-72]
  393450:	mov	w10, #0x200c                	// #8204
  393454:	movk	w10, #0xd108, lsl #16
  393458:	ldr	w8, [x8]
  39345c:	ldur	x9, [x29, #-80]
  393460:	str	w10, [x9]
  393464:	ldur	x9, [x29, #-64]
  393468:	str	w8, [x9]
  39346c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393470:	ldr	x8, [x8, #2872]
  393474:	ldr	w8, [x8]
  393478:	cmp	w8, #0xa
  39347c:	b.lt	392870 <initOpenGLRenderer@@Base+0x650>  // b.tstop
  393480:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393484:	ldr	x8, [x8, #2880]
  393488:	ldr	w8, [x8]
  39348c:	add	w9, w8, #0x1
  393490:	mul	w8, w9, w8
  393494:	tbz	w8, #0, 392870 <initOpenGLRenderer@@Base+0x650>
  393498:	ldur	x8, [x29, #-72]
  39349c:	mov	w10, #0x200c                	// #8204
  3934a0:	movk	w10, #0xd108, lsl #16
  3934a4:	ldr	w8, [x8]
  3934a8:	ldur	x9, [x29, #-80]
  3934ac:	str	w10, [x9]
  3934b0:	ldur	x9, [x29, #-64]
  3934b4:	str	w8, [x9]
  3934b8:	b	39344c <initOpenGLRenderer@@Base+0x122c>
  3934bc:	adrp	x9, 72a000 <x.30@@Base-0x432ac>
  3934c0:	ldur	x8, [x29, #-32]
  3934c4:	ldr	x9, [x9, #624]
  3934c8:	ldr	x8, [x8]
  3934cc:	ldr	w9, [x9]
  3934d0:	stur	x8, [x29, #-16]
  3934d4:	cmp	w9, #0xa
  3934d8:	b.lt	393504 <initOpenGLRenderer@@Base+0x12e4>  // b.tstop
  3934dc:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  3934e0:	ldr	x8, [x8, #616]
  3934e4:	ldr	w8, [x8]
  3934e8:	add	w9, w8, #0x1
  3934ec:	mul	w8, w9, w8
  3934f0:	tbz	w8, #0, 393504 <initOpenGLRenderer@@Base+0x12e4>
  3934f4:	ldur	x8, [x29, #-32]
  3934f8:	ldr	x8, [x8]
  3934fc:	stur	x8, [x29, #-16]
  393500:	b	3934bc <initOpenGLRenderer@@Base+0x129c>
  393504:	ldur	x8, [x29, #-16]
  393508:	mov	sp, x27
  39350c:	adrp	x8, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  393510:	ldr	x0, [x8, #3456]
  393514:	bl	26ed78 <.datadiv_decode3211576236516147487@@Base+0x1ec>
  393518:	ldur	x8, [x29, #-80]
  39351c:	mov	w9, #0xf83a                	// #63546
  393520:	movk	w9, #0xac7f, lsl #16
  393524:	str	w9, [x8]
  393528:	ldur	x8, [x29, #-72]
  39352c:	mov	w9, #0x1                   	// #1
  393530:	str	w9, [x8]
  393534:	b	392bb4 <initOpenGLRenderer@@Base+0x994>
  393538:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  39353c:	ldr	x8, [x8, #2952]
  393540:	ldr	w8, [x8]
  393544:	cmp	w8, #0xa
  393548:	b.lt	393564 <initOpenGLRenderer@@Base+0x1344>  // b.tstop
  39354c:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393550:	ldr	x8, [x8, #2960]
  393554:	ldr	w8, [x8]
  393558:	add	w9, w8, #0x1
  39355c:	mul	w8, w9, w8
  393560:	tbnz	w8, #0, 39359c <initOpenGLRenderer@@Base+0x137c>
  393564:	adrp	x9, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393568:	ldur	x8, [x29, #-64]
  39356c:	ldr	x9, [x9, #2968]
  393570:	ldr	w8, [x8]
  393574:	ldr	w9, [x9]
  393578:	stur	w8, [x29, #-52]
  39357c:	cmp	w9, #0xa
  393580:	b.lt	3935ac <initOpenGLRenderer@@Base+0x138c>  // b.tstop
  393584:	adrp	x8, 729000 <_ZNSt10bad_typeidD2Ev@@Base+0x15170>
  393588:	ldr	x8, [x8, #2976]
  39358c:	ldr	w8, [x8]
  393590:	add	w9, w8, #0x1
  393594:	mul	w8, w9, w8
  393598:	tbz	w8, #0, 3935ac <initOpenGLRenderer@@Base+0x138c>
  39359c:	ldur	x8, [x29, #-64]
  3935a0:	ldr	w8, [x8]
  3935a4:	stur	w8, [x29, #-52]
  3935a8:	b	393564 <initOpenGLRenderer@@Base+0x1344>
  3935ac:	ldur	w0, [x29, #-52]
  3935b0:	ldur	x8, [x29, #-128]
  3935b4:	ldr	x8, [x8, #40]
  3935b8:	ldur	x9, [x29, #-8]
  3935bc:	cmp	x8, x9
  3935c0:	b.ne	3935e4 <initOpenGLRenderer@@Base+0x13c4>  // b.any
  3935c4:	mov	sp, x29
  3935c8:	ldp	x20, x19, [sp, #80]
  3935cc:	ldp	x22, x21, [sp, #64]
  3935d0:	ldp	x24, x23, [sp, #48]
  3935d4:	ldp	x26, x25, [sp, #32]
  3935d8:	ldp	x28, x27, [sp, #16]
  3935dc:	ldp	x29, x30, [sp], #96
  3935e0:	ret
  3935e4:	bl	7184f0 <__stack_chk_fail@plt>

Disassembly of section .plt:
