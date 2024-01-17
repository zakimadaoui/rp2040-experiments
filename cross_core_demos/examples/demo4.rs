// DEMO4:  Ping pong example using cross-pending Interrupts

#![no_std]
#![no_main]

use cortex_m::asm;
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

use cross_core_demos::CoreBridge;
use hal::clocks::Clock;
use hal::multicore::{Multicore, Stack};
use hal::pac::interrupt;

static mut CORE1_STACK: Stack<4096> = Stack::new();

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// simplest buffer possible
static mut TASK0_SIMPLE_BUFF: u32 = 2;
static mut TASK1_SIMPLE_BUFF: u32 = 0;

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
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _ = core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || {
        info!("core 1 running...");
        let pac = unsafe { pac::Peripherals::steal() };

        // drain too ?
        // while pac.SIO.fifo_st.read().vld().bit() {
        //     let _ = pac.SIO.fifo_rd.read();
        // }
        // clear status bits before unpending the FIFO interrupt
        pac.SIO.fifo_st.write(|wr| unsafe { wr.bits(0xff) });
        pac::NVIC::unpend(pac::Interrupt::SIO_IRQ_PROC1);

        // unmask FIFO and TIMER1 interrupts
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::SIO_IRQ_PROC1);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_1);
        }
        loop {
            asm::nop()
        }
    });

    // Draining the fifo must be done after starting the Core1, because the FIFO is used during waking up Core1
    // in order to pass the stack pointer and vector table
    sio.fifo.drain();

    // unmask SIO_IRQ_PROC0 From Core0 and expect Core1 to pend it
    pac::NVIC::unpend(pac::Interrupt::SIO_IRQ_PROC0);
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::SIO_IRQ_PROC0);
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    }

    // trigger TIMER1 interrupt which is unmasked in core 1
    CoreBridge::send_signal(pac::Interrupt::TIMER_IRQ_1);

    // Configure GPIO25 as an output
    // we need to toggle this led as a sign of life :P !

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut led_pin = pins.gpio25.into_push_pull_output();
    loop {
        led_pin.set_high().unwrap();
        delay.delay_ms(100);
        led_pin.set_low().unwrap();
        delay.delay_ms(100);
    }
}

// ============================================ Tasks =============================================

#[interrupt]
fn TIMER_IRQ_0() {
    let ping = unsafe { core::ptr::read_volatile(&TASK1_SIMPLE_BUFF) };

    let pong = ping + 2;
    info!("TIMER_IRQ_0: Got Ping {}, Sending Pong {} ", ping, pong);
    asm::delay(1_200_000); //simulate some operation

    // write to buffer and signal to TIMER1 task in core1
    unsafe { core::ptr::write_volatile(&mut TASK0_SIMPLE_BUFF, pong) };
    CoreBridge::send_signal(pac::Interrupt::TIMER_IRQ_1);
}

#[interrupt]
fn TIMER_IRQ_1() {
    let pong = unsafe { core::ptr::read_volatile(&TASK0_SIMPLE_BUFF) };

    // assertion to verify no race condition happend
    core::assert_eq!(pong, unsafe { TASK1_SIMPLE_BUFF + 2 });

    let ping = pong + 1;
    info!("TIMER_IRQ_1: Got Pong {}, Sending Ping {}", pong, ping);
    asm::delay(1_200_000); //simulate some operation

    // write to buffer and signal to TIMER0 task in core0
    unsafe { core::ptr::write_volatile(&mut TASK1_SIMPLE_BUFF, ping) };
    CoreBridge::send_signal(pac::Interrupt::TIMER_IRQ_0);
}

//================================== FIFO irqs acting as proxy ====================================

#[interrupt]
fn SIO_IRQ_PROC0() {
    if let Some(signal) = CoreBridge::read_signal() {
        info!("SIO_IRQ_PROC0: forwarding irq {}", signal as u16);
        pac::NVIC::pend(signal);
    }
}

#[interrupt]
fn SIO_IRQ_PROC1() {
    if let Some(signal) = CoreBridge::read_signal() {
        info!("SIO_IRQ_PROC1: forwarding irq {}", signal as u16);
        pac::NVIC::pend(signal);
    }
}
