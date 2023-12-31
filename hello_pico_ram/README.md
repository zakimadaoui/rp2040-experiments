### Running code from SRAM on RP2040
1) define overall memory layout using `memory.x`
```
MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

EXTERN(BOOT2_FIRMWARE)

SECTIONS {
    /* ### Boot loader */
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;
```

2) The `memory.x` is a **child** linker script which is included as part of `link.x` script which actually does all the heavy lifting for defining the memory layout. And `memory.x` MUST only define the memory regions and addresses for FLASH and RAM in addition to any non standard sections and regions, such as the .boot2 section that belongs to the BOOT2 region in this example. Unfortunately, it is not always obvious that this `link.x` is managed by `cortex-m-rt` crate and it is even more unfortunate that it can't be modified directly if we want full control of the memory layout. So, to workaround it and define our own linker script while keeping the rest of what `cortex-m-rt` provides, one has to do the following:
    - copy link.x.in from `cortex-m-rt` github repo into the root of the crate, this will act as a great starting point for out later customizations. Let's call this copy `link.ram.x` for now. 
    - insert this block at the end of `link.ram.x`
    ```
    /* Provides weak aliases (cf. PROVIDED) for device specific interrupt handlers */
    /* This will usually be provided by a device crate generated using svd2rust (see `device.x`) */
    INCLUDE device.x

    ASSERT(SIZEOF(.vector_table) <= 0xc0, "
    There can't be more than 32 interrupt handlers. This may be a bug in
    your device crate, or you may have registered more than 32 interrupt
    handlers.");
    ```
    - write a build script that copies our `link.ram.x` and `memory.x` into appropriate locations where they can be used by rust-lld. (see `build.rs` in this example) 
    - modify `.cargo/config.toml` to use the new script instead of `link.x` (see `.cargo/config.toml` for more details)
  
3) Now we can customize the memory layout to our liking. In this case we want to map .vector_table, .text and .rodata sections to be stored in FLASH, but at boot time get copied to RAM and all symbols should point to RAM address space. For more details see `link.ram.x`, but overall the following changes were made:
    - change the entry to BOOT2_FIRMWARE (Beginning of second stage bootloader)
    - change .vector_table start address to the first address in RAM  
    - use the optional section command FLAGs [>VMA] [AT>LMA] for all 3 sections in flash that we want to map to RAM.


3) Finally use a bootloader that memcopies the content of Flash memory (.vector_table, .text and .rodata) into RAM and starts running from there.
```
/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running. and here we 
/// specify a second stage bootloader that copies flash contents to ram before starting to run 
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_RAM_MEMCPY;
```

4) an extra step if .vector_table is used is to change the VTOR offset to the one in RAM. There's definitely a neat way to do it, but her's a hacky one that also works
```rust
// set (ADDR(.vector_table) + 4) in VTOR to point to the RAM vector table offset. 
// the +4 is because _stack_start location is stored at the beginning of .vector_table section.
cortex_m::Peripherals::take().unwrap().SCB.vtor.write(<addr_of_vector_table>+4);
```


### Memory layout after mapping

We can clearly see that the virtual memory addresses (VMAs) of .vector_table, .text and .rodata all belong to the RAM region. LMAs (Loadable memory addresses) are used for storing those sections in the flash for persistant storage but all the symbols inside those sections use RAM address space, so as soon as the device boots up, those sections are copied to RAM and execution continues from there.

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