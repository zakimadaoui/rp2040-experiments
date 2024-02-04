// Lockstep different interrupts ? or reading from different srams in lockstep

#![no_std]
#![no_main]

use bus_behavior::{hundred_nops, hundred_reads, hundred_writes, systic_init};
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
    Clock, Sio,
};

// Some traits we need

static mut CORE1_STACK: Stack<4096> = Stack::new();
const CORE1_READY: u32 = 7;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[rp2040_hal::entry]
fn main1() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let mut sio = hal::Sio::new(pac.SIO);

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
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

    let sys_freq = clocks.system_clock.freq().to_MHz();

    // configure systic to prepare for measurements
    systic_init();

    // measure clock cycles for 100 reads & 100 writes on core 0 before waking up core 1
    println!(
        "100 reads  on core 0 => {} clock cycles",
        hundred_reads(unsafe { &SHARED_RW_DATA as *const u32 })
    );
    println!(
        "100 writes on core 0 => {} clock cycles",
        hundred_writes(unsafe { &mut SHARED_RW_DATA as *mut u32 })
    );
    println!("100 NOPs   on core 0 => {} clock cycles", hundred_nops());

    // drain the fifo from core 1 side & start the second core
    sio.fifo.drain();
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _ = core1.spawn(unsafe { &mut CORE1_STACK.mem }, || main2());

    // configure bus priorities to max for core 0 and core 1
    pac.RESETS.reset.modify(|_, w| w.busctrl().clear_bit()); // Take BUSCTRL out of reset mode
    pac.BUSCTRL.bus_priority.write(|wr| {
        wr.proc0().set_bit();
        wr.proc1().set_bit()
    });

    // unpend and unmask timer interrupt
    unsafe { pac::NVIC::unpend(pac::Interrupt::TIMER_IRQ_0) };
    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };

    // wait for Core 1 to make measurement for 100 reads & 100 writes before continueing
    while sio.fifo.read_blocking() != CORE1_READY {}

    // force trigger timer0 interupt
    pac.TIMER.inte.write(|wr| wr.alarm_0().set_bit());
    pac.TIMER.intf.write(|wr| wr.alarm_0().set_bit());

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    // Configure GPIO25 as an output
    let mut led_pin = pins.gpio25.into_push_pull_output();

    // sign of life
    loop {
        led_pin.toggle().unwrap();
        asm::delay(12_500_000u32);
    }
}

fn main2() -> ! {
    let pac = unsafe { pac::Peripherals::steal() };
    let mut sio = Sio::new(pac.SIO);

    // configure systic to prepare for measurements
    systic_init();

    // measure clock cycles for 100 reads & 100 writes on Core 1
    println!(
        "100 reads  on core 1 => {} clock cycles",
        hundred_reads(unsafe { &SHARED_RW_DATA as *const u32 })
    );
    println!(
        "100 writes on core 1 => {} clock cycles",
        hundred_writes(unsafe { &mut SHARED_RW_DATA as *mut u32 })
    );

    // unpend and unmask timer interrupt
    unsafe { pac::NVIC::unpend(pac::Interrupt::TIMER_IRQ_0) };
    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };

    // inform Core 0 that Core 1 has made the 100 reads & writes and that the timer interrupt is unmasked
    sio.fifo.write_blocking(CORE1_READY);

    loop {
        asm::nop();
    }
}

#[interrupt]
fn TIMER_IRQ_0() {
    let core = unsafe { pac::Peripherals::steal() };
    let coreid = core.SIO.cpuid.read().bits();

    // the branch will take 2 clock cycles no matter what the value of coreid is because of this generated assembly
    // bne	.LBB87_2 (2cycles if succeeds, 1 if not)
    // b	.LBB87_1 (1 clock cycles)
    // so if it succeeds then its 2 clock cycles. if not then bne takes 1 cycle and b instruction also takes another cycles => 2 clock cyles
    // .LBB87_1:

    let (a, b, c) = if coreid == 0 {
        let a = hundred_reads(unsafe { &SHARED_RW_DATA as *const u32 });
        let c = hundred_nops();
        let b = hundred_writes(unsafe { &mut SHARED_RW_DATA as *mut u32 });
        (a, b, c)
    } else {
        let a = hundred_reads(unsafe { &SHARED_RW_DATA as *const u32 });
        let c = hundred_nops();
        let b = hundred_writes(unsafe { &mut SHARED_RW_DATA as *mut u32 });
        (a, b, c)
    };
    let pac = unsafe { pac::Peripherals::steal() };
    pac.TIMER.intf.write(|wr| wr.alarm_0().clear_bit());
    println!(
        "TIMER0@CPU{} => reads: {}, writes {}, nops {}",
        coreid, a, b, c,
    );
}

#[link_section = ".sram4_code"]
#[no_mangle]
static mut SHARED_RW_DATA: u32 = 77;

#[link_section = ".sram5_code"]
#[no_mangle]
static mut SRAM5_RW_DATA: u32 = 77;
