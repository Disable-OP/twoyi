=== initOpenGLRenderer first 50 instrs ===

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

=== createOpenGLSubwindow first 60 instrs ===

libvm.so:     file format elf64-littleaarch64


Disassembly of section .text:

0000000000395988 <createOpenGLSubwindow@@Base>:
  395988:	str	d8, [sp, #-112]!
  39598c:	stp	x29, x30, [sp, #16]
  395990:	stp	x28, x27, [sp, #32]
  395994:	stp	x26, x25, [sp, #48]
  395998:	stp	x24, x23, [sp, #64]
  39599c:	stp	x22, x21, [sp, #80]
  3959a0:	stp	x20, x19, [sp, #96]
  3959a4:	add	x29, sp, #0x10
  3959a8:	sub	sp, sp, #0x40
  3959ac:	mrs	x8, tpidr_el0
  3959b0:	stur	x8, [x29, #-72]
  3959b4:	ldr	x8, [x8, #40]
  3959b8:	mov	v8.16b, v0.16b
  3959bc:	mov	w19, w4
  3959c0:	mov	w20, w3
  3959c4:	stur	x8, [x29, #-24]
  3959c8:	adrp	x8, 72a000 <x.30@@Base-0x432ac>
  3959cc:	ldr	x8, [x8, #2376]
  3959d0:	mov	w21, w2
  3959d4:	mov	w22, w1
  3959d8:	mov	x23, x0
  3959dc:	ldr	w9, [x8]
  3959e0:	mov	w8, #0xb779                	// #46969
  3959e4:	movk	w8, #0x478, lsl #16
  3959e8:	cmp	w9, #0xa
  3959ec:	adrp	x9, 76a000 <__cxa_unexpected_handler@@Base+0x340>
  3959f0:	b.lt	395a0c <createOpenGLSubwindow@@Base+0x84>  // b.tstop
  3959f4:	adrp	x10, 72a000 <x.30@@Base-0x432ac>
  3959f8:	ldr	x10, [x10, #2384]
  3959fc:	ldr	w10, [x10]
  395a00:	add	w11, w10, #0x1
  395a04:	mul	w10, w11, w10
  395a08:	tbnz	w10, #0, 395a94 <createOpenGLSubwindow@@Base+0x10c>
  395a0c:	sub	x10, sp, #0x10
  395a10:	mov	sp, x10
  395a14:	stur	x10, [x29, #-64]
  395a18:	sub	x10, sp, #0x10
  395a1c:	mov	sp, x10
  395a20:	stur	x10, [x29, #-56]
  395a24:	ldr	x10, [x9, #3464]
  395a28:	ldur	x11, [x29, #-56]
  395a2c:	str	x10, [x11]
  395a30:	ldur	x10, [x29, #-56]
  395a34:	ldr	x10, [x10]
  395a38:	ldur	x11, [x29, #-64]
  395a3c:	cmp	x10, #0x0
  395a40:	cset	w10, ne  // ne = any
  395a44:	strb	w10, [x11]
  395a48:	sub	x10, sp, #0x10
  395a4c:	mov	sp, x10
  395a50:	stur	x10, [x29, #-48]
  395a54:	ldur	x10, [x29, #-48]
  395a58:	str	w8, [x10]
