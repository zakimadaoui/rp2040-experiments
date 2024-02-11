//

#![no_std]
#![no_main]
#![allow(non_snake_case)]

use core::ops::{Add, Sub};

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
use hal::{pac, vector_table::VectorTable};

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;
/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[link_section = ".sram2_code"]
static mut TIMER_IRQ_ACK_TIME: u32 = 0;
// vector tables for the two cores stored in different memeory regions to avoid any concurrent
// access to the same memory bank when the two cores both receive an interrupt at the same time
#[link_section = ".sram2_code"]
static mut CORE0_VECTOR_TABLE: VectorTable = VectorTable::new();

#[rp2040_hal::entry]
#[link_section = ".sram2_code"]
fn main1() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();
    let sio = hal::Sio::new(pac.SIO);

    // configure priority for core 0 to be MAX
    pac.RESETS.reset.modify(|_, w| w.busctrl().clear_bit()); // take BUSCTRL out of reset mode
    pac.BUSCTRL.bus_priority.write(|w| w.proc0().set_bit());

    // configure systic to prepare for measurements
    systic_init();

    // configure a vector table in RAM
    unsafe {
        // let mut CORE0_VECTOR_TABLE: VectorTable = VectorTable::new();
        CORE0_VECTOR_TABLE.init(&mut pac.PPB);
        CORE0_VECTOR_TABLE.register_handler(pac::Interrupt::TIMER_IRQ_0 as usize, core0_timer_irq);
        CORE0_VECTOR_TABLE.activate(&mut pac.PPB);
    }

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

    // unpend and unmask timer interrupts
    pac::NVIC::unpend(pac::Interrupt::TIMER_IRQ_0);
    unsafe { core.NVIC.set_priority(pac::Interrupt::TIMER_IRQ_0, 1) };
    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };

    // prepare some constants
    const SYST_CVR: *const u32 = 0xE000_E018 as *const u32;
    let TIMER_INTF = 0x4005_403c as *mut u32;

    // enable timer interrupts
    pac.TIMER.inte.write(|wr| wr.alarm_0().set_bit());

    // read the systick timer and immediately trigger the timer interrupt, then immediately read the systic timer again
    // to allow us to determine the time needed to assert the ISR
    let start_time: u32;
    let intf_assersion_time: u32;
    unsafe {
        core::arch::asm!(
            "ldr {0}, [{1}]", // read systic right before asserting the interrupt line
            "str {2}, [{3}]", // force trigger interrupts
            "ldr {4}, [{1}]", // read systic right after asserting the interrupt line
            out(reg) start_time,
            in(reg) SYST_CVR,
            in(reg) 0x0000_0001, // enable the alarm0 interrupt line (first bit in the register)
            in(reg) TIMER_INTF,
            out(reg) intf_assersion_time,

        );
    }

    // wait a bit to make sure the timer interrupts have been executed
    asm::delay(12_500_000u32);

    //===============  calculate and print the irq latency ====================

    // this delay is the overhead of readying twice the value CVR of SYSTICK (once at start of the measurement and once at the end)
    // see: https://developer.arm.com/documentation/ka001406/latest/
    let systick_measurment_delay = 2;

    // time needed to execute the STR instruction to store 1 ( assert the ISR) to the TIMER INTF register
    let intf_write_delay = start_time - intf_assersion_time - systick_measurment_delay;

    // this delay is due to the assembly instructions that must be executed at the start of the interrupt handler
    // before the systick timer value can be read to a register. The assembly instructions are:
    // push	{r7, lr} (Takes 3 clock cycles)
    // add	r7, sp, #0 (Takes 1 clock cycles)
    // sub	sp, #8 (Takes 1 clock cycles)
    // ldr	r1, .LCPI43_0 (Takes 2 clock cycles)
    let irq_handler_dely = 7;

    println!(
        "Total measurement time           = {} clock cycles",
        start_time.sub(unsafe { TIMER_IRQ_ACK_TIME }) // TIMER_IRQ_ACK_TIME holds the value of systick timer as soon as the TIMER_IRQ_0 isr starts executing
    );
    println!("systick measurment delay         = 02 clock cycles");
    println!("irq handler delay                = 07 clock cycles");
    println!(
        "Alarm0 interrupt assertion delay = {:02} clock cycles",
        intf_write_delay
    );
    println!(
        "total measurment delay           = {} clock cycles",
        intf_write_delay.add(9)
    );
    println!(
        "irq latency on core0             = {} clock cycles",
        start_time
            .sub(unsafe { TIMER_IRQ_ACK_TIME }) // TIMER_IRQ_ACK_TIME holds the value of systick timer as soon as the TIMER_IRQ_0 isr starts executing
            .sub(intf_write_delay) //subtract measument delay1
            .sub(irq_handler_dely) //subtract measument delay2
            .sub(systick_measurment_delay) //subtract measument delay3
    );

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

#[link_section = ".sram3_code"]
#[no_mangle]
pub extern "C" fn core0_timer_irq() {
    unsafe {
        const SYST_CVR: *const u32 = 0xE000_E018 as *const u32;
        let ack_time: u32;
        core::arch::asm!(
            "ldr {0}, [{1}]", // read systick current value register CVR
            out(reg) ack_time,
            in(reg) SYST_CVR,
        );

        core::ptr::write_volatile(&mut TIMER_IRQ_ACK_TIME, ack_time);
        // stop this triggering interrupt
        pac::Peripherals::steal().TIMER.intf.reset();
    }
}
