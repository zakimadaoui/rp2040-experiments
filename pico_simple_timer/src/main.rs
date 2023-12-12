//! # GPIO 'Blinky' Example
//!
//! This application demonstrates how to control a GPIO pin on the RP2040.
//!
//! It may need to be adapted to your particular board layout and/or pin assignment.
//!
//! See the `Cargo.toml` file for Copyright and license details.

#![no_std]
#![no_main]

use hal::fugit::MicrosDurationU32;
use hal::gpio::bank0::Gpio25;
use hal::gpio::{FunctionSio, Pin, PullDown, SioOutput};
use hal::pac::interrupt;
use hal::timer::{Alarm, Alarm0};
// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// Alias for our HAL crate
use rp2040_hal as hal;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use hal::pac::{self};

// Some traits we need
use embedded_hal::digital::v2::{InputPin, OutputPin};

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
/// Note: This boot block is not necessary when using a rp-hal based BSP
/// as the BSPs already perform this step.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
/// if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Entry point to our bare-metal application.
///
/// The `#[rp2040_hal::entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables and the spinlock are initialised.
///
/// The function configures the RP2040 peripherals, then toggles a GPIO pin in
/// an infinite loop. If there is an LED connected to that pin, it will blink.
///

static mut LED_PIN: Option<Pin<Gpio25, FunctionSio<SioOutput>, PullDown>> = None;
static mut ALARM0: Option<Alarm0> = None;

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

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Configure GPIO25 as an output
    unsafe {
        LED_PIN = Some(pins.gpio25.into_push_pull_output());

        let mut timer = rp2040_hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
        let mut alarm0 = timer.alarm_0().unwrap();
        let _ = alarm0.schedule(MicrosDurationU32::millis(100));
        alarm0.enable_interrupt();
        ALARM0 = Some(alarm0);
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    }

    loop {}
}

// End of file

#[interrupt]
fn TIMER_IRQ_0() {
    unsafe {
        if let Some(led_pin) = LED_PIN.as_mut() {
            if led_pin.is_high().unwrap() {
                let _ = led_pin.set_low();
            } else {
                let _ = led_pin.set_high();
            }
        }

        if let Some(alarm0) = ALARM0.as_mut() {
            let _ = alarm0.schedule(MicrosDurationU32::millis(100));
            alarm0.clear_interrupt();
            alarm0.enable_interrupt();
        }
    }
}
