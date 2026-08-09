/* Custom linker script to create PIE executable compatible with dlopen */
/* Based on default Android linker script but modified for EXEC type */

OUTPUT_FORMAT("elf64-littleaarch64", "elf64-bigaarch64", "elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(main)

SECTIONS
{
  /* Mark this as an executable for direct execution */
  . = 0;
  
  /* Program headers */
  .interp : { *(.interp) }
  .note.android.ident : { *(.note.android.ident) }
  .dynsym : { *(.dynsym) }
  .dynstr : { *(.dynstr) }
  .gnu.hash : { *(.gnu.hash) }
  .hash : { *(.hash) }
  .gnu.version : { *(.gnu.version) }
  .gnu.version_d : { *(.gnu.version_d) }
  .gnu.version_r : { *(.gnu.version_r) }
  .rela.dyn : { *(.rela.*) }
  .rela.plt : { *(.rela.plt) }
  
  /* Code sections */
  .plt : { *(.plt) *(.iplt) }
  .text : { *(.text .text.*) }
  .rodata : { *(.rodata .rodata.*) }
  .eh_frame_hdr : { *(.eh_frame_hdr) }
  .eh_frame : { *(.eh_frame) }
  
  /* Data sections */
  .init_array : { *(.init_array) }
  .fini_array : { *(.fini_array) }
  .data.rel.ro : { *(.data.rel.ro*) }
  .dynamic : { *(.dynamic) }
  .got : { *(.got) }
  .got.plt : { *(.got.plt) }
  .data : { *(.data .data.*) }
  .bss : { *(.bss .bss.*) *(COMMON) }
  
  /* Debug sections */
  .debug_info : { *(.debug_info) }
  .debug_abbrev : { *(.debug_abbrev) }
  .debug_line : { *(.debug_line) }
  .debug_str : { *(.debug_str) }
  
  /DISCARD/ : { *(.note.GNU-stack) *(.gnu_debuglink) *(.gnu.lto_*) }
}
