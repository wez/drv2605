// Compile this with:
//  RUSTFLAGS="-C link-arg=-Tlink.x" cargo build --examples --target thumbv6m-none-eabi
// to build for the metro_m0
#![no_std]
#![no_main]

extern crate metro_m0 as hal;
extern crate panic_rtt;

use cortex_m_rt::entry;
use drv2605::{Drv2605l, Effect, ErmCalibration, ErmLibrary, LoadParams};
use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::{CorePeripherals, Peripherals};
use jlink_rtt;

macro_rules! dbgprint {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let mut stdout = jlink_rtt::Output::new();
            writeln!(stdout, $($arg)*).ok();
        }
    };
}

#[entry]
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

    // will vibrate on startup to autocalibrate, should probably use auto once and print those values to be used subsequently
    // let mut haptic = Drv2605l::erm(twi, ErmCalibration::Auto(GeneralParams::default())).unwrap();

    // Or lra would looks like this
    // let mut haptic = Drv2605l::lra(
    //     twi,
    //     LraCalibration::Auto(
    //         GeneralParams::default(),
    //         LraParams {
    //             rated: 0x3E,
    //             clamp: 0x89,
    //             drive_time: 19,
    //         },
    //     ),
    // );

    // let params = haptic.calibration().unwrap();
    // info!(
    //     "comp:{} bemf:{} gain:{}",
    //     params.comp, params.bemf, params.gain
    // );

    let mut haptic = Drv2605l::erm(
        i2c,
        ErmCalibration::Load(LoadParams {
            comp: 15,
            bemf: 134,
            gain: 2,
        }),
    )
    .unwrap();
    dbgprint!("device successfully init");

    // rom mode using built in effects, choose the correct ErmLibrary for your
    // motor characteristics
    haptic.set_mode_rom(ErmLibrary::B).unwrap();

    // set one effect to happen when go bit enabled
    haptic
        .set_rom_single(Effect::TransitionRampDownLongSmoothOne100to0)
        .unwrap();
    // or you could set several
    // let roms = [
    //     Effect::StrongClick100,
    //     Effect::BuzzOne100,
    //     Effect::StrongClick100,
    //     Effect::BuzzOne100,
    //     Effect::StrongClick100,
    //     Effect::BuzzOne100,
    //     Effect::TransitionRampDownLongSmoothOne100to0,
    //     Effect::None, //stop early
    // ];
    // haptic.set_rom(&roms).unwrap();

    haptic.set_standby(false).unwrap();
    loop {
        for _ in 0..10 {
            delay.delay_ms(200u8);
            red_led.set_high();
            delay.delay_ms(200u8);
            red_led.set_low();
        }

        dbgprint!("go: {:?}", haptic.set_go(true));
    }

    // or rtp mode would look like this instead
    // haptic.set_standby(false).unwrap();
    // haptic.set_mode_rtp().unwrap();
    // loop {
    //     haptic.set_standby(false).unwrap();

    //     for i in 180..255 {
    //         haptic.set_rtp(i).unwrap();
    //         delay.delay_ms(100u8);
    //     }
    //     for i in (180..255).rev() {
    //         haptic.set_rtp(i).unwrap();
    //         delay.delay_ms(100u8);
    //     }
    //     haptic.set_standby(true).unwrap();
    //     delay.delay_ms(255u8);
    //     delay.delay_ms(255u8);
    //     delay.delay_ms(255u8);
    //     delay.delay_ms(255u8);
    // }

    // or pwm mode, assuming pwm has previously been configured and is
    // outputting to the in/trig pin
    // haptic.set_mode_pwm().unwrap();
    // haptic.set_standby(false).unwrap();
}
