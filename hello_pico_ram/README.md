these are just some bulletpoints, this will be documented in details later ;)

### Running code from SRAM on RP2040
1- use a bootloader that memcopies Flash into ram and starts running from there
2- modify linker script to map the .vector_table, .text and .rodata to RAM (VMA) and to be stored in Flash at load time (LMA)
3- modify the entry to point to the bootloader
4- work-arround cortex-m-rt crate's linker script 


### Memory layout after mapping
``` bash 
cargo objdump --release -- -h 

hello_pico_ram: file format elf32-littlearm

Sections:
Idx Name            Size     VMA      LMA      Type
  0                 00000000 00000000 00000000 
  1 .vector_table   000000c0 20000000 10000100 DATA
  2 .boot2          00000100 10000000 10000000 DATA
  3 .text           00000b58 200000c0 100001c0 TEXT
  4 .rodata         000001c0 20000c18 10000d18 DATA
  5 .data           00000000 20000dd8 20000dd8 DATA
  6 .gnu.sgstubs    00000000 10000ee0 10000ee0 TEXT
  7 .bss            00000004 20000dd8 20000dd8 BSS
  8 .uninit         00000000 20000ddc 20000ddc BSS
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
 21 .symtab         00000c90 00000000 00000000 
 22 .shstrtab       000000fd 00000000 00000000 
 23 .strtab         000012be 00000000 00000000
```