//

#![no_std]
#![no_main]

use bus_behavior::systic_init;
use cortex_m::asm;
use defmt::*;
use defmt_rtt as _;

use embedded_hal::digital::v2::ToggleableOutputPin;
// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
// use panic_halt as _;
use panic_probe as _;

// Alias for our HAL crate
use rp2040_hal as hal;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use hal::{
    multicore::{Multicore, Stack},
    pac::{self, interrupt},
    Sio,
};

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;
/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;
static mut CORE1_STACK: Stack<4096> = Stack::new();
const CORE1_READY: u32 = 7;

#[rp2040_hal::entry]
fn main1() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let mut sio = hal::Sio::new(pac.SIO);

    // configure systic to prepare for measurements
    systic_init();

    // configure bus priorities for both cores to be MAX
    pac.RESETS.reset.modify(|_, w| w.busctrl().clear_bit()); // take BUSCTRL out of reset mode
    pac.BUSCTRL.bus_priority.write(|w| {
        w.proc0().clear_bit();
        w.proc1().set_bit()
    });
    pac.BUSCTRL.perfctr0.reset(); // reset counter

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let _clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // drain the fifo from core 1 side & start the second core
    sio.fifo.drain();
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _ = core1.spawn(unsafe { &mut CORE1_STACK.mem }, || main2());

    // unpend and unmask timer interrupts
    pac::NVIC::unpend(pac::Interrupt::TIMER_IRQ_0);
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    }

    // wait for Core 1
    while sio.fifo.read_blocking() != CORE1_READY {}

    // configure performance counters to measure contested reads on sram5
    pac.BUSCTRL.perfsel0.reset();

    pac.BUSCTRL
        .perfsel0
        .write(|w| w.perfsel0().sram4_contested()); // select counter 0 measurement
    pac.BUSCTRL.perfctr0.reset(); // reset counter

    // write something to the shared data
    let ptr_sram4 = 0x20040000 as *mut u32;
    unsafe { ptr_sram4.write_volatile(77) };

    // force trigger separate interupt lines both cores (TIMER0 on CORE0 and TIMER1 on CORE1)
    pac.TIMER
        .inte
        .write(|wr| wr.alarm_0().set_bit().alarm_1().set_bit());
    pac.TIMER
        .intf
        .write(|wr| wr.alarm_0().set_bit().alarm_1().set_bit());
    // wait a bit to make sure the timer interrupts execute
    asm::delay(12_500_000u32);
    // read performace counter for contested reads on srams
    println!(
        "contested RAM accesses [sram4 = {}]",
        pac.BUSCTRL.perfctr0.read().bits(),
    );
    pac.BUSCTRL.perfctr0.reset(); // reset counter

    // config led pin
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut led_pin = pins.gpio25.into_push_pull_output();
    loop {
        // sign of life
        led_pin.toggle().unwrap();
        asm::delay(12_500_000u32);
    }
}

fn main2() -> ! {
    let pac = unsafe { pac::Peripherals::steal() };
    let mut sio = Sio::new(pac.SIO);

    // configure systic to prepare for measurements
    systic_init();

    // unpend and unmask timer interrupts
    pac::NVIC::unpend(pac::Interrupt::TIMER_IRQ_1);
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_1);
    }
    // inform Core 0 that timers interrupts are unmasked
    sio.fifo.write_blocking(CORE1_READY);

    loop {
        asm::nop();
    }
}

#[interrupt]
#[link_section = ".sram2_code"]
unsafe fn TIMER_IRQ_0() {
    let mut read_value: u32;
    let a = {
        const SYST_CVR: *const u32 = 0xE000_E018 as *const u32;
        let ptr_sram4 = 0x20040000 as *mut u32;
        let start: u32;
        let end: u32;
        core::arch::asm!(
            "ldr {0}, [{3}]", // read systick current value register CVR
            "ldr {1}, [{4}]", // read from sram4
            "ldr {2}, [{3}]", // read systick current value register CVR
            out(reg) start,
            out(reg) read_value,
            out(reg) end,
            in(reg) SYST_CVR,
            in(reg) ptr_sram4,
        );

        start - end - 2
    };

    println!(
        "concurrent read on core 0 took {} clock cycles. read val is {}",
        a, read_value
    );
    // stop this triggering interrupt
    unsafe { (0x4005403c as *mut u32).write(0) }; /* TIMER->INTF = 0*/
}

#[interrupt]
#[link_section = ".sram3_code"]
unsafe fn TIMER_IRQ_1() {
    // 140 cycles delay needed to match with TIMER0 interupt
    core::arch::asm!(
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
    );

    let mut read_value: u32;
    let a = {
        const SYST_CVR: *const u32 = 0xE000_E018 as *const u32;
        let ptr_sram4 = 0x20040000 as *mut u32;
        let start: u32;
        let end: u32;
        core::arch::asm!(
            "ldr {0}, [{3}]", // read systick current value register CVR
            "ldr {1}, [{4}]", // read from sram4
            "ldr {2}, [{3}]", // read systick current value register CVR
            out(reg) start,
            out(reg) read_value,
            out(reg) end,
            in(reg) SYST_CVR,
            in(reg) ptr_sram4,
        );

        start - end - 2
    };

    println!(
        "concurrent read on core 1 took {} clock cycles. read val is {}",
        a, read_value
    );
    // stop this triggering interrupt
    unsafe { (0x4005403c as *mut u32).write(0) }; /* TIMER->INTF = 0*/
}
