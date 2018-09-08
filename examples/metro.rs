#![no_std]
#![no_main]
#![feature(used)]

extern crate cortex_m;
extern crate jlink_rtt;
extern crate metro_m0 as hal;
extern crate panic_rtt;

#[macro_use(entry)]
extern crate cortex_m_rt;

extern crate drv2605;
use drv2605::{Drv2605, Effect};

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::{CorePeripherals, Peripherals};

macro_rules! dbgprint {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let mut stdout = jlink_rtt::Output::new();
            writeln!(stdout, $($arg)*).ok();
        }
    };
}

entry!(main);

fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut pins = hal::Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);
    let mut delay = Delay::new(core.SYST, &mut clocks);

    let i2c = hal::i2c_master(
        &mut clocks,
        400.khz(),
        peripherals.SERCOM3,
        &mut peripherals.PM,
        pins.sda,
        pins.scl,
        &mut pins.port,
    );

    let mut haptic = Drv2605::new(i2c);
    dbgprint!("about to init device");
    dbgprint!("init say: {:?}", haptic.init_open_loop_erm());

    dbgprint!(
        "set effect: {:?}",
        haptic.set_single_effect(Effect::TransitionRampDownLongSmoothOne100to0)
    );

    loop {
        for _ in 0..10 {
            delay.delay_ms(200u8);
            red_led.set_high();
            delay.delay_ms(200u8);
            red_led.set_low();
        }

        dbgprint!("go: {:?}", haptic.set_go(true));
    }
}
