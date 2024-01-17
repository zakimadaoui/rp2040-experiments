// DEMO1: Ping Pong using FIFO/Mailbox with Blocking approach (to be compared with DEMO4, the non-blocking exmaple)

#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
// use panic_halt as _;
use panic_probe as _;

// Alias for our HAL crate
use rp2040_hal as hal;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use hal::pac;

// Some traits we need
use embedded_hal::digital::v2::OutputPin;

use hal::clocks::Clock;
use hal::multicore::{Multicore, Stack};

static mut CORE1_STACK: Stack<4096> = Stack::new();

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

fn core1_task(sys_freq: u32) -> ! {
    let pac = unsafe { pac::Peripherals::steal() };
    let core = unsafe { pac::CorePeripherals::steal() };

    let mut sio = hal::Sio::new(pac.SIO);

    let mut delay = cortex_m::delay::Delay::new(core.SYST, sys_freq);
    let cpuid = unsafe { pac::Peripherals::steal().SIO.cpuid.read().bits() };

    loop {
        let ping = sio.fifo.read_blocking();
        let pong = ping + 1;
        info!("CORE-{}: Got Ping={}, Sending Pong={}", cpuid, ping, pong);
        delay.delay_ms(100);
        sio.fifo.write_blocking(pong);
    }
}

#[rp2040_hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();

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

    let sys_freq = clocks.system_clock.freq().to_Hz();

    // The single-cycle I/O block controls our GPIO pins
    let mut sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _ = core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || {
        core1_task(sys_freq)
    });

    // Configure GPIO25 as an output
    let mut led_pin = pins.gpio25.into_push_pull_output();
    let cpuid = unsafe { pac::Peripherals::steal().SIO.cpuid.read().bits() };
    info!("CORE-{}: Sending first Ping=0", cpuid);
    sio.fifo.write_blocking(0);
    loop {
        let pong = sio.fifo.read_blocking();
        let ping = pong + 2;
        info!("CORE-{}: Got Pong={}, Sending Ping={}", cpuid, pong, ping);
        led_pin.set_high().unwrap();
        sio.fifo.write_blocking(ping);
        led_pin.set_low().unwrap();
    }
}
