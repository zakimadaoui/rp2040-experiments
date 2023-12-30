### Rtic application running entirely from ram
this one is similar to `../hello_pico_ram` in terms of memory layout. The only addition to this 
was to change the VTOR value to point to the vector table offset in ram as follows:

```rust
// set (ADDR(.vector_table) + 4) in VTOR to point to the RAM vector table offset. 
// the +4 is because _stack_start location is stored at the beginning of .vector_table section.
unsafe {
    cx.core.SCB.vtor.write(0x20000000+4);
}
``` 

### Application Memory layout

``` bash
cargo objdump --release -- -h
pico_rtic_timer_ram:    file format elf32-littlearm

Sections:
Idx Name            Size     VMA      LMA      Type
  0                 00000000 00000000 00000000 
  1 .vector_table   000000c0 20000000 10000100 DATA
  2 .boot2          00000100 10000000 10000000 DATA
  3 .text           0000142c 200000c0 100001c0 TEXT
  4 .rodata         000001e4 200014ec 100015ec DATA
  5 .data           00000004 200016d0 100017d0 DATA
  6 .gnu.sgstubs    00000000 100017e0 100017e0 TEXT
  7 .bss            00000004 200016d4 200016d4 BSS
  8 .uninit         00000000 200016d8 200016d8 BSS
  9 .comment        00000040 00000000 00000000 
 10 .ARM.attributes 00000032 00000000 00000000 
 11 .debug_frame    00006c1c 00000000 00000000 DEBUG
 12 .debug_loc      0000006d 00000000 00000000 DEBUG
 13 .debug_abbrev   00001638 00000000 00000000 DEBUG
 14 .debug_info     0002da48 00000000 00000000 DEBUG
 15 .debug_aranges  00001f98 00000000 00000000 DEBUG
 16 .debug_ranges   0001c948 00000000 00000000 DEBUG
 17 .debug_str      00046932 00000000 00000000 DEBUG
 18 .debug_pubnames 00019088 00000000 00000000 DEBUG
 19 .debug_pubtypes 0000047f 00000000 00000000 DEBUG
 20 .debug_line     0002e7cd 00000000 00000000 DEBUG
 21 .symtab         00000f30 00000000 00000000 
 22 .shstrtab       000000fd 00000000 00000000 
 23 .strtab         000017d9 00000000 00000000
 ```