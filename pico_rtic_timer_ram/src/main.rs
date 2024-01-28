#![no_std]
#![no_main]

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
/// Note: This boot block is not necessary when using a rp-hal based BSP
/// as the BSPs already perform this step.
#[link_section = ".boot2"]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_RAM_MEMCPY;

#[rtic::app(device = rp_pico::hal::pac, peripherals = true)]
mod app {

    static DELAY: u32 = 50;

    /// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
    /// if your board has a different frequency
    const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    use rp2040_hal::fugit::MicrosDurationU32;
    use rp2040_hal::gpio::bank0::Gpio25;
    use rp2040_hal::gpio::{FunctionSio, Pin, PullDown, SioOutput};
    use rp2040_hal::timer::{Alarm, Alarm0};
    // Ensure we halt the program on panic (if we don't mention this crate it won't
    // be linked)
    use panic_halt as _;

    // Alias for our HAL crate

    // A shorter alias for the Peripheral Access Crate, which provides low-level
    // register access
    use rp2040_hal::pac::{self};

    // Some traits we need
    use embedded_hal::digital::v2::{InputPin, OutputPin};

    #[shared]
    struct Shared {
        alarm: Alarm0,
        led: Pin<Gpio25, FunctionSio<SioOutput>, PullDown>,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Initialization of the system clock.
        let mut watchdog = rp2040_hal::watchdog::Watchdog::new(cx.device.WATCHDOG);

        // Configure the clocks - The default is to generate a 125 MHz system clock
        let clocks = rp2040_hal::clocks::init_clocks_and_plls(
            XTAL_FREQ_HZ,
            cx.device.XOSC,
            cx.device.CLOCKS,
            cx.device.PLL_SYS,
            cx.device.PLL_USB,
            &mut cx.device.RESETS,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        // set (ADDR(.vector_table) + 4) in VTOR to point to the RAM vector table offset.
        // the +4 is because _stack_start location is stored at the beginning of .vector_table section.
        unsafe {
            cx.core.SCB.vtor.write(0x20000000 + 4);
        }

        // The single-cycle I/O block controls our GPIO pins
        let sio = rp2040_hal::Sio::new(cx.device.SIO);

        // Set the pins to their default state
        let pins = rp2040_hal::gpio::Pins::new(
            cx.device.IO_BANK0,
            cx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut cx.device.RESETS,
        );

        // Configure GPIO25 as an output
        let led_pin = pins.gpio25.into_push_pull_output();
        let mut timer = rp2040_hal::Timer::new(cx.device.TIMER, &mut cx.device.RESETS, &clocks);
        let mut alarm0 = timer.alarm_0().unwrap();
        alarm0.schedule(MicrosDurationU32::millis(DELAY)).unwrap();
        alarm0.enable_interrupt();

        unsafe {
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
        }

        // Return the Shared variables struct, the Local variables struct and the XPTO Monitonics
        //    (Note: Read again the RTIC book in the section of Monotonics timers)
        (
            Shared {
                alarm: alarm0,
                led: led_pin,
            },
            Local {},
            init::Monotonics(),
        )
    }

    /// Task that blinks the rp-pico onboard LED and that send a message "LED ON!" and "LED OFF!" do USB-Serial.
    #[task(
        binds = TIMER_IRQ_0,
        priority = 1,
        shared = [alarm, led],
        local = [tog: bool = true],
    )]
    fn timer_irq(mut cx: timer_irq::Context) {
        cx.shared.led.lock(|led_pin| {
            if led_pin.is_high().unwrap() {
                let _ = led_pin.set_low();
            } else {
                let _ = led_pin.set_high();
            }
        });

        cx.shared.alarm.lock(|alarm0| {
            let _ = alarm0.schedule(MicrosDurationU32::millis(DELAY));
            alarm0.clear_interrupt();
        });
    }

    // Task with least priority that only runs when nothing else is running.
    #[idle(local = [x: u32 = 0])]
    fn idle(_cx: idle::Context) -> ! {
        // Locals in idle have lifetime 'static
        // let _x: &'static mut u32 = cx.local.x;

        //hprintln!("idle").unwrap();

        loop {
            cortex_m::asm::nop();
        }
    }
}
