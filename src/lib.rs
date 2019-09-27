/*!
A platform agnostic Rust friver for the drv2605, based on the
[`embedded-hal`] traits.
*/
#![no_std]

use bitfield::bitfield;
use embedded_hal::blocking::i2c::{Write, WriteRead};

bitfield! {
    pub struct StatusReg(u8);
    impl Debug;
    /// Latching overcurrent detection flag.  If the load impedance is below
    /// the load-impedance threshold, the device shuts down and periodically
    /// attempts to restart until the impedance is above the threshold
    pub oc_detected, _: 0;
    /// Latching overtemperature detection flag. If the device becomes too hot,
    /// it shuts down. This bit clears upon read.
    pub over_temp, _: 1;
    /// Contains status for the feedback controller. This indicates when the ERM
    /// back-EMF has been zero for more than ~10 ms in ERM mode, and
    /// indicates when the LRA frequency tracking has lost frequency lock in LRA
    /// mode. This bit is for debug purposes only, and may sometimes be set
    /// under normal operation when extensive braking periods are used. This bit
    /// will clear upon read.
    pub feedback_controller_timed_out, _: 2;
    /// This flag stores the result of the auto-calibration routine and the diagnostic
    /// routine. The flag contains the result for whichever routine was executed
    /// last. The flag clears upon read. Test result is not valid until the GO bit self-
    /// clears at the end of the routine.
    /// Auto-calibration mode:
    /// 0: Auto-calibration passed (optimum result converged)
    /// 1: Auto-calibration failed (result did not converge)
    /// Diagnostic mode:
    /// 0: Actuator is functioning normally
    /// 1: Actuator is not present or is shorted, timing out, or giving
    /// out–of-range back-EMF.
    pub diagnostic_result, _: 3;
    /// Device identifier. The DEVICE_ID bit indicates the part number to the user.
    /// The user software can ascertain the device capabilities by reading this
    /// register.
    /// 4: DRV2604 (contains RAM, does not contain licensed ROM library)
    /// 3: DRV2605 (contains licensed ROM library, does not contain RAM)
    /// 6: DRV2604L (low-voltage version of the DRV2604 device)
    /// 7: DRV2605L (low-voltage version of the DRV2605 device)
    pub device_id, _: 7, 5;
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    /// Waveforms are fired by setting the GO bit in register 0x0C.
    InternalTrigger = 0,
    /// A rising edge on the IN/TRIG pin sets the GO Bit. A second rising
    /// edge on the IN/TRIG pin cancels the waveform if the second rising
    /// edge occurs before the GO bit has cleared.
    ExternalTriggerRisingEdge = 1,
    /// The GO bit follows the state of the external trigger. A rising edge on
    /// the IN/TRIG pin sets the GO bit, and a falling edge sends a cancel. If
    /// the GO bit is already in the appropriate state, no change occurs.
    ExternalTriggerLevelMode = 2,
    /// A PWM or analog signal is accepted at the IN/TRIG pin and used as
    /// the driving source. The device actively drives the actuator while in
    /// this mode. The PWM or analog input selection occurs by using the
    /// N_PWM_ANALOG bit.
    PwmInputAndAnalogInput = 3,
    /// An AC-coupled audio signal is accepted at the IN/TRIG pin. The
    /// device converts the audio signal into meaningful haptic vibration. The
    /// AC_COUPLE and N_PWM_ANALOG bits should also be set.
    AudioToVibe = 4,
    /// The device actively drives the actuator with the contents of the
    /// RTP_INPUT\[7:0\] bit in register 0x02.
    RealTimePlayback = 5,
    /// Set the device in this mode to perform a diagnostic test on the
    /// actuator. The user must set the GO bit to start the test. The test is
    /// complete when the GO bit self-clears. Results are stored in the
    /// DIAG_RESULT bit in register 0x00.
    Diagnostics = 6,
    /// Set the device in this mode to auto calibrate the device for the
    /// actuator. Before starting the calibration, the user must set the all
    /// required input parameters. The user must set the GO bit to start the
    /// calibration. Calibration is complete when the GO bit self-clears.
    AutoCalibration = 7,
}

impl From<u8> for Mode {
    fn from(val: u8) -> Mode {
        match val {
            0 => Mode::InternalTrigger,
            1 => Mode::ExternalTriggerRisingEdge,
            2 => Mode::ExternalTriggerLevelMode,
            3 => Mode::PwmInputAndAnalogInput,
            4 => Mode::AudioToVibe,
            5 => Mode::RealTimePlayback,
            6 => Mode::Diagnostics,
            7 => Mode::AutoCalibration,
            _ => unreachable!("impossible value read back from Mode register"),
        }
    }
}

bitfield! {
    pub struct ModeReg(u8);
    impl Debug;
    /// Device reset. Setting this bit performs the equivalent operation of power
    /// cycling the device. Any playback operations are immediately interrupted,
    /// and all registers are reset to the default values. The DEV_RESET bit self-
    /// clears after the reset operation is complete.
    pub dev_reset, set_dev_reset: 7;

    /// Software standby mode
    /// 0: Device ready
    /// 1: Device in software standby
    pub standby, set_standby: 6;

    /// The `Mode`
    pub into Mode, mode, set_mode: 2, 0;
}

#[derive(Debug, Clone, Copy)]
pub enum LibrarySelection {
    Empty = 0,
    A = 1,
    B = 2,
    C = 3,
    D = 4,
    E = 5,
    LRA = 6,
    Reserved = 7,
}

impl From<u8> for LibrarySelection {
    fn from(val: u8) -> LibrarySelection {
        match val {
            0 => LibrarySelection::Empty,
            1 => LibrarySelection::A,
            2 => LibrarySelection::B,
            3 => LibrarySelection::C,
            4 => LibrarySelection::D,
            5 => LibrarySelection::E,
            6 => LibrarySelection::LRA,
            7 => LibrarySelection::Reserved,
            _ => unreachable!("impossible LibrarySelection value"),
        }
    }
}

bitfield! {
    pub struct RegisterThree(u8);
    impl Debug;
    /// This bit sets the output driver into a true high-impedance state. The device
    /// must be enabled to go into the high-impedance state. When in hardware
    /// shutdown or standby mode, the output drivers have 15 kΩ to ground. When
    /// the HI_Z bit is asserted, the hi-Z functionality takes effect immediately, even
    /// if a transaction is taking place.
    pub hi_z, set_hi_z: 4;

    /// Waveform library selection value. This bit determines which library the
    /// playback engine selects when the GO bit is set.
    pub into LibrarySelection, library_selection, set_library_selection: 2, 0;
}

/// Identifies which of the waveforms from the ROM library that should
/// be played in a given waveform slot.
#[derive(Debug, Clone, Copy)]
pub enum Effect {
    /// Strong Click - 100%
    StrongClick100 = 1,
    /// Strong Click - 60%
    StrongClick60 = 2,
    /// Strong Click - 30%
    StrongClick30 = 3,
    /// Sharp Click - 100%
    SharpClick100 = 4,
    /// Sharp Click - 60%
    SharpClick60 = 5,
    /// Sharp Click - 30%
    SharpClick30 = 6,
    /// Soft Bump - 100%
    SoftBump100 = 7,
    /// Soft Bump - 60%
    SoftBump60 = 8,
    /// Soft Bump - 30%
    SoftBump30 = 9,
    /// Double Click - 100%
    DoubleClick100 = 10,
    /// Double Click - 60%
    DoubleClick60 = 11,
    /// Triple Click - 100%
    TripleClick100 = 12,
    /// Soft Fuzz - 60%
    SoftFuzz60 = 13,
    /// Strong Buzz - 100%
    StrongBuzz100 = 14,
    /// 750 ms Alert 100%
    Alert750ms = 15,
    /// 1000 ms Alert 100%
    Alert1000ms = 16,
    /// Strong Click 1 - 100%
    StrongClickOne100 = 17,
    /// Strong Click 2 - 80%
    StrongClickTwo80 = 18,
    /// Strong Click 3 - 60%
    StrongClickThree60 = 19,
    /// Strong Click 4 - 30%
    StrongClickFour30 = 20,
    /// Medium Click 1 - 100%
    MediumClickOne100 = 21,
    /// Medium Click 2 - 80%
    MediumClickTwo80 = 22,
    /// Medium Click 3 - 60%
    MediumClickThree60 = 23,
    /// Sharp Tick 1 - 100%
    SharpTickOne100 = 24,
    /// Sharp Tick 2 - 80%
    SharpTickTwo80 = 25,
    /// Sharp Tick 3 - 60%
    SharpTickThree60 = 26,
    /// Short Double Click Strong 1 - 100%
    ShortDoubleClickStrongOne100 = 27,
    /// Short Double Click Strong 2 - 80%
    ShortDoubleClickStrongTwo80 = 28,
    /// Short Double Click Strong 3 - 60%
    ShortDoubleClickStrongThree60 = 29,
    /// Short Double Click Strong 4 - 30%
    ShortDoubleClickStrongFour30 = 30,
    /// Short Double Click Medium 1 - 100%
    ShortDoubleClickMediumOne100 = 31,
    /// Short Double Click Medium 2 - 80%
    ShortDoubleClickMediumTwo80 = 32,
    /// Short Double Click Medium 3 - 60%
    ShortDoubleClickMediumThree60 = 33,
    /// Short Double Sharp Tick 1 - 100%
    ShortDoubleSharpTickOne100 = 34,
    /// Short Double Sharp Tick 2 - 80%
    ShortDoubleSharpTickTwo80 = 35,
    /// Short Double Sharp Tick 3 - 60%
    ShortDoubleSharpTickThree60 = 36,
    /// Long Double Sharp Click Strong 1 - 100%
    LongDoubleSharpClickStrongOne100 = 37,
    /// Long Double Sharp Click Strong 2 - 80%
    LongDoubleSharpClickStrongTwo80 = 38,
    /// Long Double Sharp Click Strong 3 - 60%
    LongDoubleSharpClickStrongThree60 = 39,
    /// Long Double Sharp Click Strong 4 - 30%
    LongDoubleSharpClickStrongFour30 = 40,
    /// Long Double Sharp Click Medium 1 - 100%
    LongDoubleSharpClickMediumOne100 = 41,
    /// Long Double Sharp Click Medium 2 - 80%
    LongDoubleSharpClickMediumTwo80 = 42,
    /// Long Double Sharp Click Medium 3 - 60%
    LongDoubleSharpClickMediumThree60 = 43,
    /// Long Double Sharp Tick 1 - 100%
    LongDoubleSharpTickOne100 = 44,
    /// Long Double Sharp Tick 2 - 80%
    LongDoubleSharpTickTwo80 = 45,
    /// Long Double Sharp Tick 3 - 60%
    LongDoubleSharpTickThree60 = 46,
    /// Buzz 1 - 100%
    BuzzOne100 = 47,
    /// Buzz 2 - 80%
    BuzzTwo80 = 48,
    /// Buzz 3 - 60%
    BuzzThree60 = 49,
    /// Buzz 4 - 40%
    BuzzFour40 = 50,
    /// Buzz 5 - 20%
    BuzzFive20 = 51,
    /// Pulsing Strong 1 - 100%
    PulsingStrongOne100 = 52,
    /// Pulsing Strong 2 - 60%
    PulsingStrongTwo60 = 53,
    /// Pulsing Medium 1 - 100%
    PulsingMediumOne100 = 54,
    /// Pulsing Medium 2 - 60%
    PulsingMediumTwo60 = 55,
    /// Pulsing Sharp 1 - 100%
    PulsingSharpOne100 = 56,
    /// Pulsing Sharp 2 - 60%
    PulsingSharpTwo60 = 57,
    /// Transition Click 1 - 100%
    TransitionClickOne100 = 58,
    /// Transition Click 2 - 80%
    TransitionClickTwo80 = 59,
    /// Transition Click 3 - 60%
    TransitionClickThree60 = 60,
    /// Transition Click 4 - 40%
    TransitionClickFour40 = 61,
    /// Transition Click 5 - 20%
    TransitionClickFive20 = 62,
    /// Transition Click 6 - 10%
    TransitionClickSix10 = 63,
    /// Transition Hum 1 - 100%
    TransitionHumOne100 = 64,
    /// Transition Hum 2 - 80%
    TransitionHumTwo80 = 65,
    /// Transition Hum 3 - 60%
    TransitionHumThree60 = 66,
    /// Transition Hum 4 - 40%
    TransitionHumFour40 = 67,
    /// Transition Hum 5 - 20%
    TransitionHumFive20 = 68,
    /// Transition Hum 6 - 10%
    TransitionHumSix10 = 69,
    /// Transition Ramp Down Long Smooth 1 - 100 to 0%
    TransitionRampDownLongSmoothOne100to0 = 70,
    /// Transition Ramp Down Long Smooth 2 - 100 to 0%
    TransitionRampDownLongSmoothTwo100to0 = 71,
    /// Transition Ramp Down Medium Smooth 1 - 100 to 0%
    TransitionRampDownMediumSmoothOne100to0 = 72,
    /// Transition Ramp Down Medium Smooth 2 - 100 to 0%
    TransitionRampDownMediumSmoothTwo100to0 = 73,
    /// Transition Ramp Down Short Smooth 1 - 100 to 0%
    TransitionRampDownShortSmoothOne100to0 = 74,
    /// Transition Ramp Down Short Smooth 2 - 100 to 0%
    TransitionRampDownShortSmoothTwo100to0 = 75,
    /// Transition Ramp Down Long Sharp 1 - 100 to 0%
    TransitionRampDownLongSharpOne100to0 = 76,
    /// Transition Ramp Down Long Sharp 2 - 100 to 0%
    TransitionRampDownLongSharpTwo100to0 = 77,
    /// Transition Ramp Down Medium Sharp 1 - 100 to 0%
    TransitionRampDownMediumSharpOne100to0 = 78,
    /// Transition Ramp Down Medium Sharp 2 - 100 to 0%
    TransitionRampDownMediumSharpTwo100to0 = 79,
    /// Transition Ramp Down Short Sharp 1 - 100 to 0%
    TransitionRampDownShortSharpOne100to0 = 80,
    /// Transition Ramp Down Short Sharp 2 - 100 to 0%
    TransitionRampDownShortSharpTwo100to0 = 81,
    /// Transition Ramp Up Long Smooth 1 - 0 to 100%
    TransitionRampUpLongSmoothOne0to100 = 82,
    /// Transition Ramp Up Long Smooth 2 - 0 to 100%
    TransitionRampUpLongSmoothTwo0to100 = 83,
    /// Transition Ramp Up Medium Smooth 1 - 0 to 100%
    TransitionRampUpMediumSmoothOne0to100 = 84,
    /// Transition Ramp Up Medium Smooth 2 - 0 to 100%
    TransitionRampUpMediumSmoothTwo0to100 = 85,
    /// Transition Ramp Up Short Smooth 1 - 0 to 100%
    TransitionRampUpShortSmoothOne0to100 = 86,
    /// Transition Ramp Up Short Smooth 2 - 0 to 100%
    TransitionRampUpShortSmoothTwo0to100 = 87,
    /// Transition Ramp Up Long Sharp 1 - 0 to 100%
    TransitionRampUpLongSharpOne0to100 = 88,
    /// Transition Ramp Up Long Sharp 2 - 0 to 100%
    TransitionRampUpLongSharpTwo0to100 = 89,
    /// Transition Ramp Up Medium Sharp 1 - 0 to 100%
    TransitionRampUpMediumSharpOne0to100 = 90,
    /// Transition Ramp Up Medium Sharp 2 - 0 to 100%
    TransitionRampUpMediumSharpTwo0to100 = 91,
    /// Transition Ramp Up Short Sharp 1 - 0 to 100%
    TransitionRampUpShortSharpOne0to100 = 92,
    /// Transition Ramp Up Short Sharp 2 - 0 to 100%
    TransitionRampUpShortSharpTwo0to100 = 93,
    /// Transition Ramp Down Long Smooth 1 - 50 to 0%
    TransitionRampDownLongSmoothOne50to0 = 94,
    /// Transition Ramp Down Long Smooth 2 - 50 to 0%
    TransitionRampDownLongSmoothTwo50to0 = 95,
    /// Transition Ramp Down Medium Smooth 1 - 50 to 0%
    TransitionRampDownMediumSmoothOne50to0 = 96,
    /// Transition Ramp Down Medium Smooth 2 - 50 to 0%
    TransitionRampDownMediumSmoothTwo50to0 = 97,
    /// Transition Ramp Down Short Smooth 1 - 50 to 0%
    TransitionRampDownShortSmoothOne50to0 = 98,
    /// Transition Ramp Down Short Smooth 2 - 50 to 0%
    TransitionRampDownShortSmoothTwo50to0 = 99,
    /// Transition Ramp Down Long Sharp 1 - 50 to 0%
    TransitionRampDownLongSharpOne50to0 = 100,
    /// Transition Ramp Down Long Sharp 2 - 50 to 0%
    TransitionRampDownLongSharpTwo50to0 = 101,
    /// Transition Ramp Down Medium Sharp 1 - 50 to 0%
    TransitionRampDownMediumSharpOne50to0 = 102,
    /// Transition Ramp Down Medium Sharp 2 - 50 to 0%
    TransitionRampDownMediumSharpTwo50to0 = 103,
    /// Transition Ramp Down Short Sharp 1 - 50 to 0%
    TransitionRampDownShortSharpOne50to0 = 104,
    /// Transition Ramp Down Short Sharp 2 - 50 to 0%
    TransitionRampDownShortSharpTwo50to0 = 105,
    /// Transition Ramp Up Long Smooth 1 - 0 to 50%
    TransitionRampUpLongSmoothOne0to50 = 106,
    /// Transition Ramp Up Long Smooth 2 - 0 to 50%
    TransitionRampUpLongSmoothTwo0to50 = 107,
    /// Transition Ramp Up Medium Smooth 1 - 0 to 50%
    TransitionRampUpMediumSmoothOne0to50 = 108,
    /// Transition Ramp Up Medium Smooth 2 - 0 to 50%
    TransitionRampUpMediumSmoothTwo0to50 = 109,
    /// Transition Ramp Up Short Smooth 1 - 0 to 50%
    TransitionRampUpShortSmoothOne0to50 = 110,
    /// Transition Ramp Up Short Smooth 2 - 0 to 50%
    TransitionRampUpShortSmoothTwo0to50 = 111,
    /// Transition Ramp Up Long Sharp 1 - 0 to 50%
    TransitionRampUpLongSharpOne0to50 = 112,
    /// Transition Ramp Up Long Sharp 2 - 0 to 50%
    TransitionRampUpLongSharpTwo0to50 = 113,
    /// Transition Ramp Up Medium Sharp 1 - 0 to 50%
    TransitionRampUpMediumSharpOne0to50 = 114,
    /// Transition Ramp Up Medium Sharp 2 - 0 to 50%
    TransitionRampUpMediumSharpTwo0to50 = 115,
    /// Transition Ramp Up Short Sharp 1 - 0 to 50%
    TransitionRampUpShortSharpOne0to50 = 116,
    /// Transition Ramp Up Short Sharp 2 - 0 to 50%
    TransitionRampUpShortSharpTwo0to50 = 117,
    /// Long Buzz For Programmatic Stopping - 100%
    LongBuzzForProgrammaticStopping100 = 118,
    /// Smooth Hum 1 (No kick or brake pulse) - 50%
    SmoothHumOne50 = 119,
    /// Smooth Hum 2 (No kick or brake pulse) - 40%
    SmoothHumTwo40 = 120,
    /// Smooth Hum 3 (No kick or brake pulse) - 30%
    SmoothHumThree30 = 121,
    /// Smooth Hum 4 (No kick or brake pulse) - 20%
    SmoothHumFour20 = 122,
    /// Smooth Hum 5 (No kick or brake pulse) - 10%
    SmoothHumFive10 = 123,
}

bitfield! {
    pub struct WaveformReg(u8);
    impl Debug;
    /// When this bit is set, the WAV_FRM_SEQ[6:0] bit is interpreted as a wait
    /// time in which the playback engine idles. This bit is used to insert timed
    /// delays between sequentially played waveforms.
    /// Delay time = 10 ms × WAV_FRM_SEQ[6:0]
    /// If WAIT = 0, then WAV_FRM_SEQ[6:0] is interpreted as a waveform
    /// identifier for sequence playback.
    wait, set_wait: 7;

    /// Waveform sequence value. This bit holds the waveform identifier of the
    /// waveform to be played. A waveform identifier is an integer value referring
    /// to the index position of a waveform in a ROM library. Playback begins at
    /// register address 0x04 when the user asserts the GO bit (register 0x0C).
    /// When playback of that waveform ends, the waveform sequencer plays the
    /// next waveform identifier held in register 0x05, if the next waveform
    /// identifier is non-zero. The waveform sequencer continues in this way until
    /// the sequencer reaches an identifier value of zero, or all eight identifiers are
    /// played (register addresses 0x04 through 0x0B), whichever comes first.
    waveform_seq, set_waveform_seq: 6, 0;
}

impl WaveformReg {
    /// Stops playing the sequence of effects
    pub fn new_stop() -> Self {
        let mut w = WaveformReg(0);
        w.set_wait(false);
        w.set_waveform_seq(0);
        w
    }

    /// Set the effect
    pub fn new_effect(effect: Effect) -> Self {
        let mut w = WaveformReg(0);
        w.set_wait(false);
        w.set_waveform_seq(effect as u8);
        w
    }

    /// Wait the specified amount of time (in 10ms intervals), before
    /// moving to the next effect and playing it.
    pub fn new_wait_time(tens_of_ms: u8) -> Self {
        let mut w = WaveformReg(0);
        w.set_wait(true);
        w.set_waveform_seq(tens_of_ms);
        w
    }
}

bitfield! {
    pub struct GoReg(u8);
    impl Debug;
    /// This bit is used to fire processes in the DRV2605 device. The process
    /// fired by the GO bit is selected by the MODE[2:0] bit (register 0x01). The
    /// primary function of this bit is to fire playback of the waveform identifiers in
    /// the waveform sequencer (registers 0x04 to 0x0B), in which case, this bit
    /// can be thought of a software trigger for haptic waveforms. The GO bit
    /// remains high until the playback of the haptic waveform sequence is
    /// complete. Clearing the GO bit during waveform playback cancels the
    /// waveform sequence. Using one of the external trigger modes can cause
    /// the GO bit to be set or cleared by the external trigger pin. This bit can also
    /// be used to fire the auto-calibration process or the diagnostic process.
    pub go, set_go: 0;
}

bitfield! {
    pub struct FeedbackControlReg(u8);
    impl Debug;

    /// This bit sets the DRV2605 device in ERM or LRA mode. This bit should be set
    /// prior to running auto calibration.
    /// 0: ERM Mode
    /// 1: LRA Mode
    pub n_erm_lra, set_n_erm_lra: 7;

    /// This bit selects the feedback gain ratio between braking gain and driving gain.
    /// In general, adding additional feedback gain while braking is desirable so that the
    /// actuator brakes as quickly as possible. Large ratios provide less-stable
    /// operation than lower ones. The advanced user can select to optimize this
    /// register. Otherwise, the default value should provide good performance for most
    /// actuators. This value should be set prior to running auto calibration.
    /// 0: 1x
    /// 1: 2x
    /// 2: 3x
    /// 3: 4x
    /// 4: 6x
    /// 5: 8x
    /// 6: 16x
    /// 7: Braking disabled
    pub fb_brake_factor, set_fb_brake_factor: 6, 4;

    /// This bit selects a loop gain for the feedback control. The LOOP_GAIN[1:0] bit
    /// sets how fast the loop attempts to make the back-EMF (and thus motor velocity)
    /// match the input signal level. Higher loop-gain (faster settling) options provide
    /// less-stable operation than lower loop gain (slower settling). The advanced user
    /// can select to optimize this register. Otherwise, the default value should provide
    /// good performance for most actuators. This value should be set prior to running
    /// auto calibration.
    /// 0: Low
    /// 1: Medium (default)
    /// 2: High
    /// 3: Very High
    pub loop_gain, set_loop_gain: 3, 2;

    /// This bit sets the analog gain of the back-EMF amplifier. This value is interpreted
    /// differently between ERM mode and LRA mode. Auto calibration automatically
    /// populates the BEMF_GAIN bit with the most appropriate value for the actuator.
    /// ERM Mode
    /// 0: 0.33x
    /// 1: 1.0x
    /// 2: 1.8x (default)
    /// 3: 4.0x
    /// LRA Mode
    /// 0: 5x
    /// 1: 10x
    /// 2: 20x (default)
    /// 3: 30x
    pub bemf_gain, set_bemf_gain: 1, 0;
}

bitfield! {
    pub struct Control1Reg(u8);
    impl Debug;
    /// This bit applies higher loop gain during overdrive to enhance actuator transient response.
    pub startup_boost, set_startup_boost: 7;
    /// This bit applies a 0.9-V common mode voltage to the IN/TRIG pin when an AC-
    /// coupling capacitor is used. This bit is only useful for analog input mode. This bit
    /// should not be asserted for PWM mode or external trigger mode.
    /// 0: Common-mode drive disabled for DC-coupling or digital inputs modes
    /// 1: Common-mode drive enabled for AC coupling
    pub ac_couple, set_ac_couple: 5;
    /// LRA Mode: Sets initial guess for LRA drive-time in LRA mode. Drive time is
    /// automatically adjusted for optimum drive in real time; however, this register
    /// should be optimized for the approximate LRA frequency. If the bit is set too low,
    /// it can affect the actuator startup time. If it is set too high, it can cause instability.
    /// Optimum drive time (ms) ≈ 0.5 × LRA Period
    /// Drive time (ms) = DRIVE_TIME[4:0] × 0.1 ms + 0.5 ms
    /// ERM Mode: Sets the sample rate for the back-EMF detection. Lower drive times
    /// cause higher peak-to-average ratios in the output signal, requiring more supply
    /// headroom. Higher drive times cause the feedback to react at a slower rate.
    /// Drive Time (ms) = DRIVE_TIME[4:0] × 0.2 ms + 1 ms
    pub drive_time, set_drive_time: 4, 0;
}

bitfield! {
    pub struct Control2Reg(u8);
    impl Debug;
    /// The BIDIR_INPUT bit selects how the engine interprets data.
    /// 0: Unidirectional input mode
    /// Braking is automatically determined by the feedback conditions and is
    /// applied when needed. Use of this mode also recovers an additional bit
    /// of vertical resolution. This mode should only be used for closed-loop
    /// operation.
    /// Examples::
    /// 0% Input -> No output signal
    /// 50% Input -> Half-scale output signal
    /// 100% Input -> Full-scale output signal
    /// 1: Bidirectional input mode (default)
    /// This mode is compatible with traditional open-loop signaling and also
    /// works well with closed-loop mode. When operating closed-loop, braking
    /// is automatically determined by the feedback conditions and applied
    /// when needed. When operating open-loop modes, braking is only
    /// applied when the input signal is less than 50%.
    /// Open-loop mode (ERM and LRA) examples:
    /// 0% Input -> Negative full-scale output signal (braking)
    /// 25% Input -> Negative half-scale output signal (braking)
    /// 50% Input -> No output signal
    /// 75% Input -> Positive half-scale output signal
    /// 100% Input -> Positive full-scale output signal
    /// Closed-loop mode (ERM and LRA) examples:
    /// 0% to 50% Input -> No output signal
    /// 50% Input -> No output signal
    /// 75% Input -> Half-scale output signal
    /// 100% Input -> Full-scale output signal
    pub bidir_input, set_bidir_input: 7;
    /// When this bit is set, loop gain is reduced when braking is almost complete to
    /// improve loop stability
    pub brake_stabilizer, set_brake_stabilizer: 6;
    /// LRA auto-resonance sampling time (Advanced use only)
    /// 0: 150 us
    /// 1: 200 us
    /// 2: 250 us
    /// 3: 300 us
    pub sample_time, set_sample_time: 5, 4;
    /// Blanking time before the back-EMF AD makes a conversion. (Advanced use only)
    pub blanking_time, set_blanking_time: 3, 2;
    /// Current dissipation time. This bit is the time allowed for the current to dissipate
    /// from the actuator between PWM cycles for flyback mitigation. (Advanced use
    /// only)
    pub idiss_time, set_idiss_time: 1, 0;
}

bitfield! {
    pub struct Control3Reg(u8);
    impl Debug;

    /// This bit is the noise-gate threshold for PWM and analog inputs.
    /// 0: Disabled
    /// 1: 2%
    /// 2: 4% (Default)
    /// 3: 8%
    pub ng_thresh, set_ng_thresh: 7, 6;
    /// This bit selects mode of operation while in ERM mode. Closed-loop operation is
    /// usually desired for because of automatic overdrive and braking properties.
    /// However, many existing waveform libraries were designed for open-loop
    /// operation, so open-loop operation may be required for compatibility.
    /// 0: Closed Loop
    /// 1: Open Loop
    pub erm_open_loop, set_erm_open_loop: 5;
    /// This bit disables supply compensation. The DRV2605 device generally provides
    /// constant drive output over variation in the power supply input (V DD ). In some
    /// systems, supply compensation may have already been implemented upstream,
    /// so disabling the DRV2605 supply compensation can be useful.
    /// 0: Supply compensation enabled
    /// 1: Supply compensation disabled
    pub supply_comp_dis, set_supply_comp_dis: 4;
    /// This bit selects the input data interpretation for RTP (Real-Time Playback)
    /// mode.
    /// 0: Signed
    /// 1: Unsigned
    pub data_format_rtp, set_data_format_rtp: 3;
    /// This bit selects the drive mode for the LRA algorithm. This bit determines how
    /// often the drive amplitude is updated. Updating once per cycle provides a
    /// symmetrical output signal, while updating twice per cycle provides more precise
    /// control.
    /// 0: Once per cycle
    /// 1: Twice per cycle
    pub lra_drive_mode, set_lra_drive_mode: 2;
    /// This bit selects the input mode for the IN/TRIG pin when MODE[2:0] = 3. In
    /// PWM input mode, the duty cycle of the input signal determines the amplitude of
    /// the waveform. In analog input mode, the amplitude of the input determines the
    /// amplitude of the waveform.
    /// 0: PWM Input
    /// 1: Analog Input
    pub n_pwm_analog, set_n_pwm_analog: 1;
    /// This bit selects an open-loop drive option for LRA Mode. When asserted, the
    /// playback engine drives the LRA at the selected frequency independently of the
    /// resonance frequency. In PWM input mode, the playback engine recovers the
    /// LRA commutation frequency from the PWM input, dividing the frequency by
    /// 128. Therefore the PWM input frequency must be equal to 128 times the
    /// resonant frequency of the LRA.
    /// 0: Auto-resonance mode
    /// 1: LRA open-loop mode
    pub lra_open_loop, set_lra_open_loop: 0;
}

bitfield! {
    pub struct Control4Reg(u8);
    impl Debug;

    /// This bit sets the length of the auto calibration time. The AUTO_CAL_TIME[1:0]
    /// bit should be enough time for the motor acceleration to settle when driven at the
    /// RATED_VOLTAGE[7:0] value.
    /// 0: 150 ms (minimum), 350 ms (maximum)
    /// 1: 250 ms (minimum), 450 ms (maximum)
    /// 2: 500 ms (minimum), 700 ms (maximum)
    /// 3: 1000 ms (minimum), 1200 ms (maximum)
    pub auto_cal_time, set_auto_cal_time: 5, 4;

    /// OTP Memory status
    /// 0: OTP Memory has not been programmed
    /// 1: OTP Memory has been programmed
    pub otp_status, set_otp_status: 2;

    /// This bit launches the programming process for one-time programmable (OTP)
    /// memory which programs the contents of register 0x16 through 0x1A into
    /// nonvolatile memory. This process can only be executed one time per device.
    /// See the Programming On-Chip OTP Memory section for details.
    pub otp_program, set_otp_program: 1;
}

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Register {
    Status = 0,
    Mode = 1,
    /// This field is the entry point for real-time playback (RTP) data. The DRV2605
    /// playback engine drives the RTP_INPUT[7:0] value to the load when
    /// MODE[2:0] = 5 (RTP mode). The RTP_INPUT[7:0] value can be updated in
    /// real-time by the host controller to create haptic waveforms. The
    /// RTP_INPUT[7:0] value is interpreted as signed by default, but can be set to
    /// unsigned by the DATA_FORMAT_RTP bit in register 0x1D. When the
    /// haptic waveform is complete, the user can idle the device by setting
    /// MODE[2:0] = 0, or alternatively by setting STANDBY = 1.
    RealTimePlaybackInput = 2,
    Register3 = 3,
    WaveformSequence0 = 4,
    WaveformSequence1 = 5,
    WaveformSequence2 = 6,
    WaveformSequence3 = 7,
    WaveformSequence4 = 8,
    WaveformSequence5 = 9,
    WaveformSequence6 = 0xa,
    WaveformSequence7 = 0xb,
    Go = 0xc,
    OverdriveTimeOffset = 0xd,
    SustainTimeOffsetPositive = 0xe,
    SustainTimeOffsetNegative = 0xf,
    BrakeTimeOffset = 0x10,

    /// This bit sets the reference voltage for full-scale output during closed-loop
    /// operation. The auto-calibration routine uses this register as an input, so this
    /// register must be written with the rated voltage value of the motor before
    /// calibration is performed. This register is ignored for open-loop operation
    /// because the overdrive voltage sets the reference for that case. Any
    /// modification of this register value should be followed by calibration to set
    /// A_CAL_BEMF appropriately.
    /// See the Rated Voltage Programming section for calculating the correct register
    /// value.
    RatedVoltage = 0x16,

    /// During closed-loop operation the actuator feedback allows the output voltage
    /// to go above the rated voltage during the automatic overdrive and automatic
    /// braking periods. This register sets a clamp so that the automatic overdrive is
    /// bounded. This bit also serves as the full-scale reference voltage for open-loop
    /// operation.
    /// See the Overdrive Voltage-Clamp Programming section for calculating the
    /// correct register value.
    OverdriveClampVoltage = 0x17,

    /// This register contains the voltage-compensation result after execution of auto
    /// calibration. The value stored in the A_CAL_COMP bit compensates for any
    /// resistive losses in the driver. The calibration routine checks the impedance of
    /// the actuator to automatically determine an appropriate value. The auto-
    /// calibration compensation-result value is multiplied by the drive gain during
    /// playback.
    /// Auto-calibration compensation coefficient = 1 + A_CAL_COMP[7:0] / 255
    AutoCalibrationCompensationResult = 0x18,

    /// This register contains the rated back-EMF result after execution of auto
    /// calibration. The A_CAL_BEMF[7:0] bit is the level of back-EMF voltage that the
    /// actuator gives when the actuator is driven at the rated voltage. The DRV2605
    /// playback engine uses this the value stored in this bit to automatically determine
    /// the appropriate feedback gain for closed-loop operation.
    /// Auto-calibration back-EMF (V) = (A_CAL_BEMF[7:0] / 255) × 1.22 V /
    /// BEMF_GAIN[1:0]
    AutoCalibrationBackEMFResult = 0x19,

    FeedbackControl = 0x1a,

    Control1 = 0x1b,
    Control2 = 0x1c,
    Control3 = 0x1d,
    Control4 = 0x1e,
}

/// The hardcoded address of the driver.  All drivers share the same
/// address so that it is possible to broadcast on the bus and have
/// multiple units emit the same waveform
pub const ADDRESS: u8 = 0x5a;

//we could encode open and closed loop in state as well?
pub struct Drv2605Erm;
pub struct Drv2605Lra;
pub struct Drv2604Lra;
pub struct Drv2604Erm;
pub struct Drv2605lErm;
pub struct Drv2605lLra;
pub struct Drv2604lErm;
pub struct Drv2604lLra;

pub trait DrvConfig {
    const ID: u8;
}

impl DrvConfig for Drv2605Erm {
    const ID: u8 = 3;
}
impl DrvConfig for Drv2605Lra {
    const ID: u8 = 3;
}
impl DrvConfig for Drv2604Erm {
    const ID: u8 = 4;
}
impl DrvConfig for Drv2604Lra {
    const ID: u8 = 4;
}
impl DrvConfig for Drv2604lErm {
    const ID: u8 = 6;
}
impl DrvConfig for Drv2604lLra {
    const ID: u8 = 6;
}
impl DrvConfig for Drv2605lErm {
    const ID: u8 = 7;
}
impl DrvConfig for Drv2605lLra {
    const ID: u8 = 7;
}

pub struct Drv2605<I2C, DEV> {
    i2c: I2C,
    marker: core::marker::PhantomData<DEV>,
}

#[derive(Debug)]
pub enum DrvError<E> {
    DeviceIdError,
    ConnectionError(E),
    DeviceDiagError,
    CalibrationError,
}

// todo, builder pattern
// you have to either provide .otp() which checks OTP_STATUS bit
// or provide calibration values and wait for a calibration
/// Operations that are valid only in Drv2605Erm state.
impl<I2C, E> Drv2605<I2C, Drv2605Erm>
where
    I2C: WriteRead<Error = E> + Write<Error = E>,
{
    fn check_id(&mut self) -> Result<(), DrvError<E>> {
        let reg = self.get_status()?;
        if reg.device_id() != Drv2605Erm::ID {
            return Err(DrvError::DeviceIdError);
        }

        Ok(())
    }

    pub fn set_open_loop(&mut self) -> Result<(), DrvError<E>> {
        let mut control3 = Control3Reg(self.read(Register::Control3)?);
        control3.set_erm_open_loop(true);
        self.write(Register::Control3, control3.0)
    }

    pub fn config(&mut self) -> Result<(), DrvError<E>> {
        self.check_id()?;

        let mut feedback = FeedbackControlReg(self.read(Register::FeedbackControl)?);
        feedback.set_n_erm_lra(false);
        self.write(Register::FeedbackControl, feedback.0)?;

        self.diagnostics()?;

        self.set_standby(true)
    }

    /// Selects the library the playback engine selects when the GO bit is set.
    pub fn set_library(&mut self, value: LibrarySelection) -> Result<(), DrvError<E>> {
        let mut register = RegisterThree(self.read(Register::Register3)?);
        register.set_library_selection(value as u8);
        self.write(Register::Register3, register.0)
    }

    /// Sets the waveform generation registers to the shape provided
    pub fn set_waveform(&mut self, waveform: &[WaveformReg; 8]) -> Result<(), DrvError<E>> {
        let buf: [u8; 9] = [
            Register::WaveformSequence0 as u8,
            waveform[0].0,
            waveform[1].0,
            waveform[2].0,
            waveform[3].0,
            waveform[4].0,
            waveform[5].0,
            waveform[6].0,
            waveform[7].0,
        ];
        self.i2c
            .write(ADDRESS, &buf)
            .map_err(DrvError::ConnectionError)
    }

    pub fn set_single_effect(&mut self, effect: Effect) -> Result<(), DrvError<E>> {
        let buf: [u8; 3] = [
            Register::WaveformSequence0 as u8,
            WaveformReg::new_effect(effect).0,
            WaveformReg::new_stop().0,
        ];
        self.i2c
            .write(ADDRESS, &buf)
            .map_err(DrvError::ConnectionError)
    }
}
/// Operations that are valid only in Drv2605Lra state.
impl<I2C, E> Drv2605<I2C, Drv2605Lra>
where
    I2C: WriteRead<Error = E> + Write<Error = E>,
{
    fn check_id(&mut self) -> Result<(), DrvError<E>> {
        let reg = self.get_status()?;
        if reg.device_id() != Drv2605Lra::ID {
            return Err(DrvError::DeviceIdError);
        }

        Ok(())
    }

    pub fn set_open_loop(&mut self) -> Result<(), DrvError<E>> {
        let mut control3 = Control3Reg(self.read(Register::Control3)?);
        control3.set_lra_open_loop(true);
        self.write(Register::Control3, control3.0)
    }

    pub fn config(&mut self) -> Result<(), DrvError<E>> {
        self.check_id()?;

        let mut feedback = FeedbackControlReg(self.read(Register::FeedbackControl)?);
        feedback.set_n_erm_lra(true);
        self.write(Register::FeedbackControl, feedback.0)?;

        self.diagnostics()?;

        self.set_standby(true)
    }
}

impl<DEV, I2C, E> Drv2605<I2C, DEV>
where
    DEV: DrvConfig,
    I2C: WriteRead<Error = E> + Write<Error = E>,
{
    /// Construct a driver instance, but don't do any initialization
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            marker: core::marker::PhantomData,
        }
    }

    /// Write `value` to `register`
    fn write(&mut self, register: Register, value: u8) -> Result<(), DrvError<E>> {
        self.i2c
            .write(ADDRESS, &[register as u8, value])
            .map_err(DrvError::ConnectionError)
    }

    /// Read an 8-bit value from the register
    fn read(&mut self, register: Register) -> Result<u8, DrvError<E>> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(ADDRESS, &[register as u8], &mut buf)
            .map_err(DrvError::ConnectionError)?;
        Ok(buf[0])
    }

    pub fn get_status(&mut self) -> Result<StatusReg, DrvError<E>> {
        self.read(Register::Status).map(StatusReg)
    }

    pub fn get_mode(&mut self) -> Result<ModeReg, DrvError<E>> {
        self.read(Register::Mode).map(ModeReg)
    }

    /// performs the equivalent operation of power
    /// cycling the device. Any playback operations are immediately interrupted,
    /// and all registers are reset to the default values.
    pub fn reset(&mut self) -> Result<(), DrvError<E>> {
        let mut mode = ModeReg(0);
        mode.set_dev_reset(true);
        self.write(Register::Mode, mode.0)
    }

    /// Put the device into standby mode, or wake it up from standby
    pub fn set_standby(&mut self, standby: bool) -> Result<(), DrvError<E>> {
        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_standby(standby);
        self.write(Register::Mode, mode.0)
    }

    /// This field is the entry point for real-time playback (RTP) data. The DRV2605
    /// playback engine drives the RTP_INPUT[7:0] value to the load when
    /// MODE[2:0] = 5 (RTP mode). The RTP_INPUT[7:0] value can be updated in
    /// real-time by the host controller to create haptic waveforms. The
    /// RTP_INPUT[7:0] value is interpreted as signed by default, but can be set to
    /// unsigned by the DATA_FORMAT_RTP bit in register 0x1D. When the
    /// haptic waveform is complete, the user can idle the device by setting
    /// MODE[2:0] = 0, or alternatively by setting STANDBY = 1.
    pub fn set_realtime_playback_input(&mut self, value: i8) -> Result<(), DrvError<E>> {
        self.write(Register::RealTimePlaybackInput, value as u8)
    }

    /// This bit sets the output driver into a true high-impedance state. The device
    /// must be enabled to go into the high-impedance state. When in hardware
    /// shutdown or standby mode, the output drivers have 15 kΩ to ground. When
    /// the HI_Z bit is asserted, the hi-Z functionality takes effect immediately, even
    /// if a transaction is taking place.
    pub fn set_high_impedance_state(&mut self, value: bool) -> Result<(), DrvError<E>> {
        let mut register = RegisterThree(self.read(Register::Register3)?);
        register.set_hi_z(value);
        self.write(Register::Register3, register.0)
    }

    /// This bit is used to fire processes in the DRV2605 device. The process
    /// fired by the GO bit is selected by the MODE[2:0] bit (register 0x01). The
    /// primary function of this bit is to fire playback of the waveform identifiers in
    /// the waveform sequencer (registers 0x04 to 0x0B), in which case, this bit
    /// can be thought of a software trigger for haptic waveforms. The GO bit
    /// remains high until the playback of the haptic waveform sequence is
    /// complete. Clearing the GO bit during waveform playback cancels the
    /// waveform sequence. Using one of the external trigger modes can cause
    /// the GO bit to be set or cleared by the external trigger pin. This bit can also
    /// be used to fire the auto-calibration process or the diagnostic process.
    pub fn set_go(&mut self, go: bool) -> Result<(), DrvError<E>> {
        let mut register = GoReg(self.read(Register::Go)?);
        register.set_go(go);
        self.write(Register::Go, register.0)
    }

    /// This bit adds a time offset to the overdrive portion of the library
    /// waveforms. Some motors require more overdrive time than others, so this
    /// register allows the user to add or remove overdrive time from the library
    /// waveforms. The maximum voltage value in the library waveform is
    /// automatically determined to be the overdrive portion. This register is only
    /// useful in open-loop mode. Overdrive is automatic for closed-loop mode.
    /// The offset is interpreted as 2s complement, so the time offset may be
    /// positive or negative.
    /// Overdrive Time Offset (ms) = ODT[7:0] × PLAYBACK_INTERVAL
    /// See the section for PLAYBACK_INTERVAL details.
    pub fn set_overdrive_time_offset(&mut self, value: i8) -> Result<(), DrvError<E>> {
        self.write(Register::OverdriveTimeOffset, value as u8)
    }

    /// This bit adds a time offset to the positive sustain portion of the library
    /// waveforms. Some motors have a faster or slower response time than
    /// others, so this register allows the user to add or remove positive sustain
    /// time from the library waveforms. Any positive voltage value other than the
    /// overdrive portion is considered as a sustain positive value. The offset is
    /// interpreted as 2s complement, so the time offset can positive or negative.
    /// Sustain-Time Positive Offset (ms) = SPT[7:0] × PLAYBACK_INTERVAL
    /// See the section for PLAYBACK_INTERVAL details.
    pub fn set_sustain_time_offset_positive(&mut self, value: i8) -> Result<(), DrvError<E>> {
        self.write(Register::SustainTimeOffsetPositive, value as u8)
    }

    /// This bit adds a time offset to the negative sustain portion of the library
    /// waveforms. Some motors have a faster or slower response time than
    /// others, so this register allows the user to add or remove negative sustain
    /// time from the library waveforms. Any negative voltage value other than the
    /// overdrive portion is considered as a sustaining negative value. The offset is
    /// interpreted as two’s complement, so the time offset can be positive or
    /// negative.
    /// Sustain-Time Negative Offset (ms) = SNT[7:0] × PLAYBACK_INTERVAL
    /// See the section for PLAYBACK_INTERVAL details.
    pub fn set_sustain_time_offset_negative(&mut self, value: i8) -> Result<(), DrvError<E>> {
        self.write(Register::SustainTimeOffsetNegative, value as u8)
    }

    /// This bit adds a time offset to the braking portion of the library waveforms.
    /// Some motors require more braking time than others, so this register allows
    /// the user to add or take away brake time from the library waveforms. The
    /// most negative voltage value in the library waveform is automatically
    /// determined to be the braking portion. This register is only useful in open-loop
    /// mode. Braking is automatic for closed-loop mode. The offset is interpreted as
    /// 2s complement, so the time offset can be positive or negative.
    /// Brake Time Offset (ms) = BRT[7:0] × PLAYBACK_INTERVAL
    /// See the section for PLAYBACK_INTERVAL details.
    pub fn set_brake_time_offset(&mut self, value: i8) -> Result<(), DrvError<E>> {
        self.write(Register::BrakeTimeOffset, value as u8)
    }

    pub fn diagnostics(&mut self) -> Result<(), DrvError<E>> {
        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_standby(false);
        mode.set_mode(1);
        self.write(Register::Mode, mode.0)?;

        self.set_go(true)?;

        //todo timeout
        while GoReg(self.read(Register::Go)?).go() {}

        let reg = self.get_status()?;
        if reg.diagnostic_result() {
            return Err(DrvError::DeviceDiagError);
        }

        Ok(())
    }
}
