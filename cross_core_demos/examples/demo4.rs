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

use cross_core_demos::{CrossCore, MessageQueue};
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

#[rp2040_hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();

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

    // make sure the task buffers are initialized (auto generated)
    unsafe {
        (*spawn::TASK0_QUEUE.as_mut_ptr()) = Default::default();
        (*spawn::TASK1_QUEUE.as_mut_ptr()) = Default::default();
    }

    // The single-cycle I/O block controls our GPIO pins
    let mut sio = hal::Sio::new(pac.SIO);
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _ = core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || {
        info!("core 1 running...");
        let pac = unsafe { pac::Peripherals::steal() };
        let mut core = unsafe { pac::CorePeripherals::steal() };

        // drain too ?
        // while pac.SIO.fifo_st.read().vld().bit() {
        //     let _ = pac.SIO.fifo_rd.read();
        // }
        // clear status bits before unpending the FIFO interrupt
        pac.SIO.fifo_st.write(|wr| unsafe { wr.bits(0xff) });
        pac::NVIC::unpend(pac::Interrupt::SIO_IRQ_PROC1);

        unsafe {
            // Set FIFO0 interrupts priority to MAX priority
            core.NVIC.set_priority(pac::Interrupt::SIO_IRQ_PROC1, 0);
            // unmask FIFO and TIMER0 interrupts
            pac::NVIC::unmask(pac::Interrupt::SIO_IRQ_PROC1);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
        }
        loop {
            asm::nop()
        }
    });

    // Draining the fifo must be done after starting the Core1, because the FIFO is used during waking up Core1
    // in order to pass the stack pointer and vector table
    sio.fifo.drain();

    pac::NVIC::unpend(pac::Interrupt::SIO_IRQ_PROC0);
    pac::NVIC::unpend(pac::Interrupt::TIMER_IRQ_0);
    unsafe {
        // Set FIFO0 interrupts priority to MAX priority
        core.NVIC.set_priority(pac::Interrupt::SIO_IRQ_PROC0, 0);
        // unmask SIO_IRQ_PROC0 From Core0 and expect Core1 to pend it
        pac::NVIC::unmask(pac::Interrupt::SIO_IRQ_PROC0);
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    }

    // start the ping pong...
    spawn::core1_task(3);

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

fn core0_task(ping: u32, cpuid: u32) {
    core::assert_eq!(cpuid, 0);

    let pong = ping + 1;
    info!("core0_task: Got Ping {}, Sending Pong {} ", ping, pong);
    asm::delay(12_000_000); //simulate some operation
                            // spawn core1_task on core 1 and pass the `pong` message to it
    spawn::core1_task(pong);
}

fn core1_task(pong: u32, cpuid: u32) {
    core::assert_eq!(cpuid, 1);

    let ping = pong + 1;
    info!("core1_task: Got Pong {}, Sending Ping {}", pong, ping);
    asm::delay(12_000_000); //simulate some operation
                            // spawn core1_task on core 1 and pass the `pong` message to it
    spawn::core0_task(ping);
}

// ======================================== Spawn API (auto generated) =======================================
mod spawn {

    use super::*;
    use core::mem::MaybeUninit;

    pub static mut TASK0_QUEUE: MaybeUninit<MessageQueue<u32, 3>> = MaybeUninit::uninit();
    pub static mut TASK1_QUEUE: MaybeUninit<MessageQueue<u32, 3>> = MaybeUninit::uninit();

    pub fn core0_task(ping: u32) {
        unsafe { TASK0_QUEUE.assume_init_mut().push(ping).unwrap() };
        CrossCore::pend_irq(pac::Interrupt::TIMER_IRQ_0, 1);
    }

    pub fn core1_task(pong: u32) {
        unsafe { TASK1_QUEUE.assume_init_mut().push(pong).unwrap() };
        CrossCore::pend_irq(pac::Interrupt::TIMER_IRQ_0, 0);
    }
}
// ========================================== Dispatchers (auto generated) =========================================
#[interrupt]
fn TIMER_IRQ_0() {
    let cpuid = unsafe { pac::Peripherals::steal().SIO.cpuid.read().bits() };
    if cpuid == 0 {
        // while is used as Core1 can produce signals much faster than Core0 can consume
        while let Some(data) = unsafe { spawn::TASK0_QUEUE.assume_init_mut().pop() } {
            core0_task(data, 0);
        }
    } else {
        // while is used as Core0 can produce signals much faster than Core1 can consume
        while let Some(data) = unsafe { spawn::TASK1_QUEUE.assume_init_mut().pop() } {
            core1_task(data, 1)
        }
    }
}

//================================== FIFO irqs acting as proxy ====================================

#[interrupt]
fn SIO_IRQ_PROC0() {
    if let Some(signal) = CrossCore::get_pended_irq() {
        // info!("SIO_IRQ_PROC0: forwarding irq {}", signal as u16);
        pac::NVIC::pend(signal);
    }
}

#[interrupt]
fn SIO_IRQ_PROC1() {
    if let Some(signal) = CrossCore::get_pended_irq() {
        // info!("SIO_IRQ_PROC1: forwarding irq {}", signal as u16);
        pac::NVIC::pend(signal);
    }
}
