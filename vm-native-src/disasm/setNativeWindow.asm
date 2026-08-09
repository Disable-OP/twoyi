=== 0x457158 first 40 instrs ===

libvm.so:     file format elf64-littleaarch64


Disassembly of section .text:

0000000000457158 <.datadiv_decode7109460572589435888@@Base+0xe130>:
  457158:	str	d8, [sp, #-112]!
  45715c:	stp	x29, x30, [sp, #16]
  457160:	stp	x28, x27, [sp, #32]
  457164:	stp	x26, x25, [sp, #48]
  457168:	stp	x24, x23, [sp, #64]
  45716c:	stp	x22, x21, [sp, #80]
  457170:	stp	x20, x19, [sp, #96]
  457174:	add	x29, sp, #0x10
  457178:	sub	sp, sp, #0x60
  45717c:	stp	w3, w4, [x29, #-80]
  457180:	stur	x2, [x29, #-88]
  457184:	stur	w1, [x29, #-92]
  457188:	mrs	x8, tpidr_el0
  45718c:	stur	x8, [x29, #-104]
  457190:	ldr	x8, [x8, #40]
  457194:	mov	v8.16b, v0.16b
  457198:	mov	x23, x0
  45719c:	stur	x8, [x29, #-24]
  4571a0:	adrp	x8, 72b000 <y.496@@Base-0x44508>
  4571a4:	ldr	x8, [x8, #3984]
  4571a8:	ldr	w8, [x8]
  4571ac:	cmp	w8, #0xa
  4571b0:	mov	w8, #0xf24f                	// #62031
  4571b4:	movk	w8, #0x6baf, lsl #16
  4571b8:	b.lt	4571d4 <.datadiv_decode7109460572589435888@@Base+0xe1ac>  // b.tstop
  4571bc:	adrp	x9, 72b000 <y.496@@Base-0x44508>
  4571c0:	ldr	x9, [x9, #3992]
  4571c4:	ldr	w9, [x9]
  4571c8:	add	w10, w9, #0x1
  4571cc:	mul	w9, w10, w9
  4571d0:	tbnz	w9, #0, 457260 <.datadiv_decode7109460572589435888@@Base+0xe238>
  4571d4:	sub	x9, sp, #0x10
  4571d8:	mov	sp, x9
  4571dc:	stur	x9, [x29, #-72]
  4571e0:	sub	x9, sp, #0x10
  4571e4:	mov	sp, x9
  4571e8:	stur	x9, [x29, #-64]
  4571ec:	ldur	x9, [x29, #-64]

=== function containing ANativeWindow_fromSurface call (start 0x459d68) - first 60 instrs ===

libvm.so:     file format elf64-littleaarch64


Disassembly of section .text:

0000000000459d68 <.datadiv_decode7109460572589435888@@Base+0x10d40>:
  459d68:	stp	x29, x30, [sp, #16]
  459d6c:	stp	x28, x27, [sp, #32]
  459d70:	stp	x26, x25, [sp, #48]
  459d74:	stp	x24, x23, [sp, #64]
  459d78:	stp	x22, x21, [sp, #80]
  459d7c:	stp	x20, x19, [sp, #96]
  459d80:	add	x29, sp, #0x10
  459d84:	sub	sp, sp, #0x40
  459d88:	mrs	x8, tpidr_el0
  459d8c:	stur	x8, [x29, #-72]
  459d90:	ldr	x8, [x8, #40]
  459d94:	mov	v8.16b, v0.16b
  459d98:	mov	w19, w6
  459d9c:	mov	w20, w5
  459da0:	stur	x8, [x29, #-24]
  459da4:	adrp	x8, 72c000 <y.1016@@Base-0x42e78>
  459da8:	ldr	x8, [x8, #1840]
  459dac:	mov	x21, x4
  459db0:	mov	w22, w3
  459db4:	mov	x23, x0
  459db8:	ldr	w8, [x8]
  459dbc:	cmp	w8, #0xa
  459dc0:	mov	w8, #0x7741                	// #30529
  459dc4:	movk	w8, #0xa80d, lsl #16
  459dc8:	b.lt	459de4 <.datadiv_decode7109460572589435888@@Base+0x10dbc>  // b.tstop
  459dcc:	adrp	x9, 72c000 <y.1016@@Base-0x42e78>
  459dd0:	ldr	x9, [x9, #1848]
  459dd4:	ldr	w9, [x9]
  459dd8:	add	w10, w9, #0x1
  459ddc:	mul	w9, w10, w9
  459de0:	tbnz	w9, #0, 459e68 <.datadiv_decode7109460572589435888@@Base+0x10e40>
  459de4:	sub	x9, sp, #0x10
  459de8:	mov	sp, x9
  459dec:	stur	x9, [x29, #-64]
  459df0:	sub	x9, sp, #0x10
  459df4:	mov	sp, x9
  459df8:	stur	x9, [x29, #-56]
  459dfc:	ldur	x9, [x29, #-56]
  459e00:	str	x2, [x9]
  459e04:	ldur	x9, [x29, #-56]
  459e08:	ldr	x9, [x9]
  459e0c:	ldur	x10, [x29, #-64]
  459e10:	cmp	x9, #0x0
  459e14:	cset	w9, ne  // ne = any
  459e18:	strb	w9, [x10]
  459e1c:	sub	x9, sp, #0x10
  459e20:	mov	sp, x9
  459e24:	stur	x9, [x29, #-48]
  459e28:	ldur	x9, [x29, #-48]
  459e2c:	str	w8, [x9]
  459e30:	sub	x9, sp, #0x10
  459e34:	mov	sp, x9
  459e38:	adrp	x10, 72c000 <y.1016@@Base-0x42e78>
  459e3c:	ldr	x10, [x10, #1856]
  459e40:	stur	x9, [x29, #-40]
  459e44:	ldr	w10, [x10]
  459e48:	cmp	w10, #0xa
  459e4c:	b.lt	459ec4 <.datadiv_decode7109460572589435888@@Base+0x10e9c>  // b.tstop
