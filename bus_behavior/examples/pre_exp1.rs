// Validate that systick timer can be used to measure clock cycles accurately
// make 100 read, write and nop operations and see how many clock cycles will those operations take
// according to the rp2040 datasheet:
// - read operation using LDR instruction takes 2 clock cycles in the case where 1 master is the only one accessing a specific RAM bank
// - write operation using STR instruction takes 2 clock cycles in the case where 1 master is the only one accessing a specific RAM bank
// - nop instruction takes 1 clock cycle
//
// # Assumption/Expected result
// - 100 reads take 200 clock cycles on both cores
// - 100 writes take 200 clock cycles on both cores
// - 100 NOPs take 100 clock cycles on both cores
//
// # Obtained Results
// - 100 reads  on core 0 => 200 clock cycles
// - 100 writes on core 0 => 200 clock cycles
// - 100 NOPs   on core 0 => 100 clock cycles
// - 100 reads  on core 1 => 200 clock cycles
// - 100 writes on core 1 => 200 clock cycles
// - 100 NOPs   on core 1 => 100 clock cycles
//
// # Conclusions
// Systick timer can be used to accurately measure clock cycles taken for certain operations
// Lockstep different interrupts ? or reading from different srams in lockstep

#![no_std]
#![no_main]

use bus_behavior::{hundred_nops, hundred_reads, hundred_writes, systic_init};
use cortex_m::asm;
use defmt::*;
use defmt_rtt as _;

use embedded_hal::digital::v2::ToggleableOutputPin;
use hal::{
    multicore::{Multicore, Stack},
    pac::{self},
    Clock,
};
use panic_probe as _;
use rp2040_hal as hal;

const XTAL_FREQ_HZ: u32 = 12_000_000u32;
static mut CORE1_STACK: Stack<4096> = Stack::new();

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

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

    println!("Running at {} MHz", clocks.system_clock.freq().to_MHz());
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

    println!("100 NOPs   on core 1 => {} clock cycles", hundred_nops());
    loop {
        asm::nop();
    }
}

#[link_section = ".sram4_code"]
#[no_mangle]
static mut SHARED_RW_DATA: u32 = 77;
