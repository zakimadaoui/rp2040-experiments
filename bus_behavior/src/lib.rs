#![no_std]

use cortex_m::peripheral::syst::SystClkSource;
use rp2040_hal::pac;

pub fn systic_init() {
    let mut core = unsafe { pac::CorePeripherals::steal() };
    core.SYST.disable_interrupt();
    core.SYST.set_clock_source(SystClkSource::Core);
    core.SYST.enable_counter();
    core.SYST.set_reload(0x00FFFFFF);
}

/// performs 100 reads to the memory location pointerd by `ptr_from_ram` and returns the clock cycles taken for the process
/// `systic_init()` must be used on each core to initialize systick timer correctly for making the measurements

// #[no_mangle]
#[link_section = ".sram5_code"]
static SYST_CVR2: u32 = 0xE000_E018;

#[no_mangle]
#[link_section = ".sram4_code"]
pub fn hundred_reads(ptr_from_ram: *const u32) -> u32 {
    unsafe {
        let start: u32;
        let end: u32;
        core::arch::asm!(
            "ldr {0}, [{3}]", // read systick current value register CVR
            // 100 unrolled loop
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 10

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 20

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 30

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 40

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 50

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 60

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 70

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 80

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 90

            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]",
            "ldr {1}, [{4}]", // 100

            "ldr {2}, [{3}]", // read systick current value register CVR
            out(reg) start,
            out(reg) _,
            out(reg) end,
            in(reg) SYST_CVR2,
            in(reg) ptr_from_ram,
        );

        // The cycle count for an operation can then be obtained by reading the STCVR immediately before and immediately after the operation in question.
        // Because STCVR is a down counter, the number of core clock cycles taken by the operation is given by:
        // (STCVR1 - STCVR2 - 2)
        // The overhead of two cycles is because the read of the STCVR is Strongly-Ordered with regard to other memory accesses or data processing instructions.

        start - end - 2 // todo: this could panic if start is read then STCVR wraps to zero
    }
}

/// performs 100 reads to the memory location pointerd by `ptr_from_ram` and returns the clock cycles taken for the process
/// `systic_init()` must be used on each core to initialize systick timer correctly for making the measurements
#[no_mangle]
#[link_section = ".sram4_code"]
pub fn hundred_reads2(ptr_from_ram: *const u32) -> u32 {
    unsafe {
        core::arch::asm!(
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 10

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 20

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 30

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 40

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 50

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 60

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 70

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 80

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 90

            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]",
            "ldr {0}, [{1}]", // 100
            out(reg) _,
            in(reg) ptr_from_ram,
        );

        // The cycle count for an operation can then be obtained by reading the STCVR immediately before and immediately after the operation in question.
        // Because STCVR is a down counter, the number of core clock cycles taken by the operation is given by:
        // (STCVR1 - STCVR2 - 2)
        // The overhead of two cycles is because the read of the STCVR is Strongly-Ordered with regard to other memory accesses or data processing instructions.

        // start - end - 2 // todo: this could panic if start is read then STCVR wraps to zero
        0
    }
}

/// performs 100 writes to the memory location pointerd by `ptr_from_ram` and returns the clock cycles taken for the process
/// `systic_init()` must be used on each core to initialize systick timer correctly for making the measurements
#[no_mangle]
#[link_section = ".sram4_code"]
pub fn hundred_writes(ptr_to_ram: *mut u32) -> u32 {
    unsafe {
        const SYST_CVR: *const u32 = 0xE000_E018 as *const u32;
        let start: u32;
        let end: u32;

        core::arch::asm!(
            "ldr {0}, [{2}]",
            // 100 unrolled loop
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",
            "str {3}, [{4}]",

            "ldr {1}, [{2}]",
            out(reg) start, // 0
            out(reg) end, // 1
            in(reg) SYST_CVR, // 2

            in(reg) 2507, // 3

            in(reg) ptr_to_ram, // 4
        );

        // The cycle count for an operation can then be obtained by reading the STCVR immediately before and immediately after the operation in question.
        // Because STCVR is a down counter, the number of core clock cycles taken by the operation is given by:
        // (STCVR1 - STCVR2 - 2)
        // The overhead of two cycles is because the read of the STCVR is Strongly-Ordered with regard to other memory accesses or data processing instructions.
        start - end - 2
    }
}

/// performs 100 NOPs and returns the clock cycles taken for the process
/// `systic_init()` must be used on each core to initialize systick timer correctly for making the measurements
#[no_mangle]
#[link_section = ".sram4_code"]
pub fn hundred_nops() -> u32 {
    unsafe {
        const SYST_CVR: *const u32 = 0xE000_E018 as *const u32;
        let start: u32;
        let end: u32;

        core::arch::asm!(
            "ldr {0}, [{2}]",
            // 100 unrolled loop
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",

            "ldr {1}, [{2}]",
            out(reg) start, // 0
            out(reg) end, // 1
            in(reg) SYST_CVR, // 2
        );

        // The cycle count for an operation can then be obtained by reading the STCVR immediately before and immediately after the operation in question.
        // Because STCVR is a down counter, the number of core clock cycles taken by the operation is given by:
        // (STCVR1 - STCVR2 - 2)
        // The overhead of two cycles is because the read of the STCVR is Strongly-Ordered with regard to other memory accesses or data processing instructions.
        start - end - 2
    }
}
