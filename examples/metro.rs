// Compile this with:
//  RUSTFLAGS="-C link-arg=-Tlink.x" cargo build --examples --target thumbv6m-none-eabi
// to build for the metro_m0
#![no_std]
#![no_main]

extern crate metro_m0 as hal;
extern crate panic_rtt;

use cortex_m_rt::entry;
use drv2605::{Calibration, CalibrationParams, Drv2605l, Effect, Library};
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

    // Note secure motor to a mass or calibration will fail!
    // might get away with defaults for an erm motor, but ideally compute these
    let calib = CalibrationParams::default();
    let mut haptic = Drv2605l::new(i2c, Calibration::Auto(calib), false).unwrap();

    // Or lra autocalibration would look like this.
    // let mut calib = CalibrationParams::default();
    // these are tricky and are computed from the lra motor and drv2605l datasheets
    // calib.rated = 0x3E;
    // calib.clamp = 0x8C;
    // calib.drive_time = 0x13;
    // let mut haptic = Drv2605l::new(i2c, Calibration::Auto(calib), false).unwrap();

    // print the sucessful calibration values so you can hardcode them
    // let params = haptic.calibration().unwrap();
    // info!(
    //     "comp:{} bemf:{} gain:{}",
    //     params.comp, params.bemf, params.gain
    // );

    // and hardcode them instead of using calibration like this
    // let mut haptic = Drv2605l::new(
    //     i2c,
    //     //from the
    //     Calibration::Load(drv2605::LoadParams {
    //         comp: 0x3E,
    //         bemf: 0x89,
    //         gain: 0x25,
    //     }),
    //     false,
    // )
    // .unwrap();
    dbgprint!("device successfully init");

    // rom mode using built in effects. Each library has all the same
    // vibrations, but is tuned to work for certain motor characteristics so its
    // important to choose Library for for your motor characteristics
    haptic.set_mode_rom(Library::B).unwrap();

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

        dbgprint!("go: {:?}", haptic.set_go());
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
