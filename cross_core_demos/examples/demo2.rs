// DEMO2: Trigger the same interrupt and handle it on Both Cores
// when both cores handle the same interrupt in the logs we could observe the following output:
// "core0 interrupted ? 1, core1 interrupted ? 1"

#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;

use hal::fugit::MicrosDurationU32;
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
use hal::pac::interrupt;
use hal::timer::{Alarm, Alarm0};

static mut CORE1_STACK: Stack<4096> = Stack::new();
static mut CORE0_INT_FLAG: u32 = 0;
static mut CORE1_INT_FLAG: u32 = 0;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

fn core1_task(sys_freq: u32) -> ! {
    info!("core 1 running...");
    // Unmask TIMER0 IRQ in for Core1 to allow Core1 to handle the interrupt
    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };
    loop {}
}

#[rp2040_hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

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
    let mut delay = cortex_m::delay::Delay::new(core.SYST, sys_freq);

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

    // configure the timer peripheral on Core 0
    let mut timer = rp2040_hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut alarm0 = timer.alarm_0().unwrap();
    let _ = alarm0.schedule(MicrosDurationU32::millis(1000));
    alarm0.enable_interrupt();
    unsafe { ALARM0 = Some(alarm0) };

    // Unmask TIMER0 IRQ in for Core0 to allow core 0 to handle the interrupt
    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };

    // Configure GPIO25 as an output
    let mut led_pin = pins.gpio25.into_push_pull_output();

    loop {
        // this text will print "core0 interrupted ? 1, core1 interrupted ? 1" in the case where both cores
        // handle the TIMER0 interrupt.
        unsafe {
            info!(
                "core0 interrupted ? {}, core1 interrupted ? {}",
                CORE0_INT_FLAG, CORE1_INT_FLAG
            );
        }
        led_pin.set_high().unwrap();
        delay.delay_ms(100);
        led_pin.set_low().unwrap();
        delay.delay_ms(100);
    }
}

static mut ALARM0: Option<Alarm0> = None;

#[interrupt]
fn TIMER_IRQ_0() {
    unsafe {
        let cpuid = pac::Peripherals::steal().SIO.cpuid.read().bits();

        if cpuid == 0 {
            core::ptr::write_volatile(&mut CORE0_INT_FLAG, 1);
        } else {
            core::ptr::write_volatile(&mut CORE1_INT_FLAG, 1);
        }

        ALARM0.as_mut().unwrap().clear_interrupt();
    }
}
