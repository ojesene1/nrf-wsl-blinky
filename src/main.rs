#![no_std]
#![no_main]

// use defmt::info; // re-enable along with the info! lines below
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::pac;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _}; // Global logging and panic handlers

/// The GRTC timer lives in an always-on power domain, so its settings survive a
/// debugger reset. If firmware that ran before this one (e.g. the DK's factory
/// Zephyr demo) left GRTC interrupts enabled, Embassy's GRTC_2 handler fires
/// forever and `embassy_nrf::init` never returns. Wipe that state first.
fn reset_grtc_leftovers() {
    let grtc = pac::GRTC;
    // Domain 2 = application core, secure (GRTC_2 interrupt). Clear domain 1 too in case.
    grtc.intenclr(1).write(|w| w.0 = 0xFFFF_FFFF);
    grtc.intenclr(2).write(|w| w.0 = 0xFFFF_FFFF);
    for n in 0..12 {
        grtc.events_compare(n).write_value(0);
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // info!("1: booted");
    reset_grtc_leftovers();

    // Initialize the embassy-nrf HAL with default configuration
    // This hooks up the GRTC timer driver required by embassy-time
    let peripherals = embassy_nrf::init(Default::default());
    // info!("2: init done");

    // LED0 on the nRF54L15 DK is P2.09 (P1.04 is the DK's UART TX line, not an LED)
    let mut led = Output::new(peripherals.P2_09, Level::High, OutputDrive::Standard);
    // info!("3: LED0 on, starting blink loop");

    loop {
        Timer::after_millis(500).await;
        led.set_low();
        // info!("tick: LED off");

        Timer::after_millis(500).await;
        led.set_high();
        // info!("tick: LED on");
    }
}
