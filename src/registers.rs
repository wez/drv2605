use bitfield::bitfield;

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
    ExternalTriggerLevel = 2,
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
            2 => Mode::ExternalTriggerLevel,
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

#[derive(Debug)]
pub struct RatedVoltageReg(pub u8);

impl Default for RatedVoltageReg {
    fn default() -> Self {
        Self(0x3E)
    }
}

#[derive(Debug)]
pub struct OverdriveClampReg(pub u8);

impl Default for OverdriveClampReg {
    fn default() -> Self {
        Self(0x8C)
    }
}

#[derive(Debug)]
pub struct AutoCalibrationCompensationReg(pub u8);

impl Default for AutoCalibrationCompensationReg {
    fn default() -> Self {
        Self(0x0C)
    }
}

#[derive(Debug)]
pub struct AutoCalibrationCompensationBackEmfReg(pub u8);

impl Default for AutoCalibrationCompensationBackEmfReg {
    fn default() -> Self {
        Self(0x6C)
    }
}

#[derive(Debug)]
pub struct OverdriveTimeOffsetReg(pub u8);

impl Default for OverdriveTimeOffsetReg {
    fn default() -> Self {
        Self(0x0)
    }
}

#[derive(Debug)]
pub struct SustainTimeOffsetPositiveReg(pub u8);

impl Default for SustainTimeOffsetPositiveReg {
    fn default() -> Self {
        Self(0x0)
    }
}

#[derive(Debug)]
pub struct SustainTimeOffsetNegativeReg(pub u8);

impl Default for SustainTimeOffsetNegativeReg {
    fn default() -> Self {
        Self(0x0)
    }
}

#[derive(Debug)]
pub struct BrakeTimeOffsetReg(pub u8);

impl Default for BrakeTimeOffsetReg {
    fn default() -> Self {
        Self(0x0)
    }
}

impl Default for ModeReg {
    fn default() -> Self {
        let mut reg = Self(0);
        reg.set_dev_reset(false);
        reg.set_standby(true);
        reg.set_mode(Mode::InternalTrigger as u8);
        reg
    }
}

/// Selection of Library of built-in waveforms. Each library offers all the same
/// waveforms, but is tuned to work for different motors so it is important to
/// choose the correct library for your motor characteristics
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Library {
    /// No library selected
    Empty = 0,
    /// Rated Voltage 1.3V Overdrive Voltage 3V Rise Time 40-60ms Brake Time 20-40ms
    A = 1,
    /// Rated Voltage 3V Overdrive Voltage 3V Rise Time 40-60ms Brake Time 5-15ms
    B = 2,
    /// Rated Voltage 3V Overdrive Voltage 3V Rise Time 60-80ms Brake Time 10-20ms
    C = 3,
    /// Rated Voltage 3V Overdrive Voltage 3V Rise Time 100-140ms Brake Time 15-25ms
    D = 4,
    /// Rated Voltage 3V Overdrive Voltage 3V Rise Time >140ms Brake Time >30ms
    E = 5,
    /// Rated Voltage 3V Overdrive Voltage 3V Rise Time >140ms Brake Time >30ms
    Lra = 6,
    /// Rated Voltage 4.5V Overdrive Voltage 5V Rise Time 35-45ms Brake Time 10-20ms
    F = 7,
}

impl From<u8> for Library {
    fn from(val: u8) -> Library {
        match val {
            0 => Library::Empty,
            1 => Library::A,
            2 => Library::B,
            3 => Library::C,
            4 => Library::D,
            5 => Library::E,
            6 => Library::Lra,
            7 => Library::F,
            _ => unreachable!("impossible Library value"),
        }
    }
}

bitfield! {
    pub struct LibrarySelectionReg(u8);
    impl Debug;
    /// This bit sets the output driver into a true high-impedance state. The device
    /// must be enabled to go into the high-impedance state. When in hardware
    /// shutdown or standby mode, the output drivers have 15 kΩ to ground. When
    /// the HI_Z bit is asserted, the hi-Z functionality takes effect immediately, even
    /// if a transaction is taking place.
    pub hi_z, set_hi_z: 4;

    /// Waveform library selection value. This bit determines which library the
    /// playback engine selects when the GO bit is set.
    pub into Library, library_selection, set_library_selection: 2, 0;
}

impl From<Effect> for u8 {
    fn from(val: Effect) -> Self {
        match val {
            Effect::Delays(n) => n | 0x80,
            Effect::Stop => 0,
            Effect::StrongClick100 => 1,
            Effect::StrongClick60 => 2,
            Effect::StrongClick30 => 3,
            Effect::SharpClick100 => 4,
            Effect::SharpClick60 => 5,
            Effect::SharpClick30 => 6,
            Effect::SoftBump100 => 7,
            Effect::SoftBump60 => 8,
            Effect::SoftBump30 => 9,
            Effect::DoubleClick100 => 10,
            Effect::DoubleClick60 => 11,
            Effect::TripleClick100 => 12,
            Effect::SoftFuzz60 => 13,
            Effect::StrongBuzz100 => 14,
            Effect::Alert750ms => 15,
            Effect::Alert1000ms => 16,
            Effect::StrongClickOne100 => 17,
            Effect::StrongClickTwo80 => 18,
            Effect::StrongClickThree60 => 19,
            Effect::StrongClickFour30 => 20,
            Effect::MediumClickOne100 => 21,
            Effect::MediumClickTwo80 => 22,
            Effect::MediumClickThree60 => 23,
            Effect::SharpTickOne100 => 24,
            Effect::SharpTickTwo80 => 25,
            Effect::SharpTickThree60 => 26,
            Effect::ShortDoubleClickStrongOne100 => 27,
            Effect::ShortDoubleClickStrongTwo80 => 28,
            Effect::ShortDoubleClickStrongThree60 => 29,
            Effect::ShortDoubleClickStrongFour30 => 30,
            Effect::ShortDoubleClickMediumOne100 => 31,
            Effect::ShortDoubleClickMediumTwo80 => 32,
            Effect::ShortDoubleClickMediumThree60 => 33,
            Effect::ShortDoubleSharpTickOne100 => 34,
            Effect::ShortDoubleSharpTickTwo80 => 35,
            Effect::ShortDoubleSharpTickThree60 => 36,
            Effect::LongDoubleSharpClickStrongOne100 => 37,
            Effect::LongDoubleSharpClickStrongTwo80 => 38,
            Effect::LongDoubleSharpClickStrongThree60 => 39,
            Effect::LongDoubleSharpClickStrongFour30 => 40,
            Effect::LongDoubleSharpClickMediumOne100 => 41,
            Effect::LongDoubleSharpClickMediumTwo80 => 42,
            Effect::LongDoubleSharpClickMediumThree60 => 43,
            Effect::LongDoubleSharpTickOne100 => 44,
            Effect::LongDoubleSharpTickTwo80 => 45,
            Effect::LongDoubleSharpTickThree60 => 46,
            Effect::BuzzOne100 => 47,
            Effect::BuzzTwo80 => 48,
            Effect::BuzzThree60 => 49,
            Effect::BuzzFour40 => 50,
            Effect::BuzzFive20 => 51,
            Effect::PulsingStrongOne100 => 52,
            Effect::PulsingStrongTwo60 => 53,
            Effect::PulsingMediumOne100 => 54,
            Effect::PulsingMediumTwo60 => 55,
            Effect::PulsingSharpOne100 => 56,
            Effect::PulsingSharpTwo60 => 57,
            Effect::TransitionClickOne100 => 58,
            Effect::TransitionClickTwo80 => 59,
            Effect::TransitionClickThree60 => 60,
            Effect::TransitionClickFour40 => 61,
            Effect::TransitionClickFive20 => 62,
            Effect::TransitionClickSix10 => 63,
            Effect::TransitionHumOne100 => 64,
            Effect::TransitionHumTwo80 => 65,
            Effect::TransitionHumThree60 => 66,
            Effect::TransitionHumFour40 => 67,
            Effect::TransitionHumFive20 => 68,
            Effect::TransitionHumSix10 => 69,
            Effect::TransitionRampDownLongSmoothOne100to0 => 70,
            Effect::TransitionRampDownLongSmoothTwo100to0 => 71,
            Effect::TransitionRampDownMediumSmoothOne100to0 => 72,
            Effect::TransitionRampDownMediumSmoothTwo100to0 => 73,
            Effect::TransitionRampDownShortSmoothOne100to0 => 74,
            Effect::TransitionRampDownShortSmoothTwo100to0 => 75,
            Effect::TransitionRampDownLongSharpOne100to0 => 76,
            Effect::TransitionRampDownLongSharpTwo100to0 => 77,
            Effect::TransitionRampDownMediumSharpOne100to0 => 78,
            Effect::TransitionRampDownMediumSharpTwo100to0 => 79,
            Effect::TransitionRampDownShortSharpOne100to0 => 80,
            Effect::TransitionRampDownShortSharpTwo100to0 => 81,
            Effect::TransitionRampUpLongSmoothOne0to100 => 82,
            Effect::TransitionRampUpLongSmoothTwo0to100 => 83,
            Effect::TransitionRampUpMediumSmoothOne0to100 => 84,
            Effect::TransitionRampUpMediumSmoothTwo0to100 => 85,
            Effect::TransitionRampUpShortSmoothOne0to100 => 86,
            Effect::TransitionRampUpShortSmoothTwo0to100 => 87,
            Effect::TransitionRampUpLongSharpOne0to100 => 88,
            Effect::TransitionRampUpLongSharpTwo0to100 => 89,
            Effect::TransitionRampUpMediumSharpOne0to100 => 90,
            Effect::TransitionRampUpMediumSharpTwo0to100 => 91,
            Effect::TransitionRampUpShortSharpOne0to100 => 92,
            Effect::TransitionRampUpShortSharpTwo0to100 => 93,
            Effect::TransitionRampDownLongSmoothOne50to0 => 94,
            Effect::TransitionRampDownLongSmoothTwo50to0 => 95,
            Effect::TransitionRampDownMediumSmoothOne50to0 => 96,
            Effect::TransitionRampDownMediumSmoothTwo50to0 => 97,
            Effect::TransitionRampDownShortSmoothOne50to0 => 98,
            Effect::TransitionRampDownShortSmoothTwo50to0 => 99,
            Effect::TransitionRampDownLongSharpOne50to0 => 100,
            Effect::TransitionRampDownLongSharpTwo50to0 => 101,
            Effect::TransitionRampDownMediumSharpOne50to0 => 102,
            Effect::TransitionRampDownMediumSharpTwo50to0 => 103,
            Effect::TransitionRampDownShortSharpOne50to0 => 104,
            Effect::TransitionRampDownShortSharpTwo50to0 => 105,
            Effect::TransitionRampUpLongSmoothOne0to50 => 106,
            Effect::TransitionRampUpLongSmoothTwo0to50 => 107,
            Effect::TransitionRampUpMediumSmoothOne0to50 => 108,
            Effect::TransitionRampUpMediumSmoothTwo0to50 => 109,
            Effect::TransitionRampUpShortSmoothOne0to50 => 110,
            Effect::TransitionRampUpShortSmoothTwo0to50 => 111,
            Effect::TransitionRampUpLongSharpOne0to50 => 112,
            Effect::TransitionRampUpLongSharpTwo0to50 => 113,
            Effect::TransitionRampUpMediumSharpOne0to50 => 114,
            Effect::TransitionRampUpMediumSharpTwo0to50 => 115,
            Effect::TransitionRampUpShortSharpOne0to50 => 116,
            Effect::TransitionRampUpShortSharpTwo0to50 => 117,
            Effect::LongBuzzForProgrammaticStopping100 => 118,
            Effect::SmoothHumOne50 => 119,
            Effect::SmoothHumTwo40 => 120,
            Effect::SmoothHumThree30 => 121,
            Effect::SmoothHumFour20 => 122,
            Effect::SmoothHumFive10 => 123,
        }
    }
}

/// Selection of built-in waveforms that can be sequenced using the `set_rom`
/// and `set_rom_single` function and triggered using the `set_go` function
#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// No effect, or Stop playing
    Stop,
    /// Use the effect period as (up to 127) counts of 10ms delays
    Delays(u8),
    /// Strong Click - 100%
    StrongClick100,
    /// Strong Click - 60%
    StrongClick60,
    /// Strong Click - 30%
    StrongClick30,
    /// Sharp Click - 100%
    SharpClick100,
    /// Sharp Click - 60%
    SharpClick60,
    /// Sharp Click - 30%
    SharpClick30,
    /// Soft Bump - 100%
    SoftBump100,
    /// Soft Bump - 60%
    SoftBump60,
    /// Soft Bump - 30%
    SoftBump30,
    /// Double Click - 100%
    DoubleClick100,
    /// Double Click - 60%
    DoubleClick60,
    /// Triple Click - 100%
    TripleClick100,
    /// Soft Fuzz - 60%
    SoftFuzz60,
    /// Strong Buzz - 100%
    StrongBuzz100,
    /// 750 ms Alert 100%
    Alert750ms,
    /// 1000 ms Alert 100%
    Alert1000ms,
    /// Strong Click 1 - 100%
    StrongClickOne100,
    /// Strong Click 2 - 80%
    StrongClickTwo80,
    /// Strong Click 3 - 60%
    StrongClickThree60,
    /// Strong Click 4 - 30%
    StrongClickFour30,
    /// Medium Click 1 - 100%
    MediumClickOne100,
    /// Medium Click 2 - 80%
    MediumClickTwo80,
    /// Medium Click 3 - 60%
    MediumClickThree60,
    /// Sharp Tick 1 - 100%
    SharpTickOne100,
    /// Sharp Tick 2 - 80%
    SharpTickTwo80,
    /// Sharp Tick 3 - 60%
    SharpTickThree60,
    /// Short Double Click Strong 1 - 100%
    ShortDoubleClickStrongOne100,
    /// Short Double Click Strong 2 - 80%
    ShortDoubleClickStrongTwo80,
    /// Short Double Click Strong 3 - 60%
    ShortDoubleClickStrongThree60,
    /// Short Double Click Strong 4 - 30%
    ShortDoubleClickStrongFour30,
    /// Short Double Click Medium 1 - 100%
    ShortDoubleClickMediumOne100,
    /// Short Double Click Medium 2 - 80%
    ShortDoubleClickMediumTwo80,
    /// Short Double Click Medium 3 - 60%
    ShortDoubleClickMediumThree60,
    /// Short Double Sharp Tick 1 - 100%
    ShortDoubleSharpTickOne100,
    /// Short Double Sharp Tick 2 - 80%
    ShortDoubleSharpTickTwo80,
    /// Short Double Sharp Tick 3 - 60%
    ShortDoubleSharpTickThree60,
    /// Long Double Sharp Click Strong 1 - 100%
    LongDoubleSharpClickStrongOne100,
    /// Long Double Sharp Click Strong 2 - 80%
    LongDoubleSharpClickStrongTwo80,
    /// Long Double Sharp Click Strong 3 - 60%
    LongDoubleSharpClickStrongThree60,
    /// Long Double Sharp Click Strong 4 - 30%
    LongDoubleSharpClickStrongFour30,
    /// Long Double Sharp Click Medium 1 - 100%
    LongDoubleSharpClickMediumOne100,
    /// Long Double Sharp Click Medium 2 - 80%
    LongDoubleSharpClickMediumTwo80,
    /// Long Double Sharp Click Medium 3 - 60%
    LongDoubleSharpClickMediumThree60,
    /// Long Double Sharp Tick 1 - 100%
    LongDoubleSharpTickOne100,
    /// Long Double Sharp Tick 2 - 80%
    LongDoubleSharpTickTwo80,
    /// Long Double Sharp Tick 3 - 60%
    LongDoubleSharpTickThree60,
    /// Buzz 1 - 100%
    BuzzOne100,
    /// Buzz 2 - 80%
    BuzzTwo80,
    /// Buzz 3 - 60%
    BuzzThree60,
    /// Buzz 4 - 40%
    BuzzFour40,
    /// Buzz 5 - 20%
    BuzzFive20,
    /// Pulsing Strong 1 - 100%
    PulsingStrongOne100,
    /// Pulsing Strong 2 - 60%
    PulsingStrongTwo60,
    /// Pulsing Medium 1 - 100%
    PulsingMediumOne100,
    /// Pulsing Medium 2 - 60%
    PulsingMediumTwo60,
    /// Pulsing Sharp 1 - 100%
    PulsingSharpOne100,
    /// Pulsing Sharp 2 - 60%
    PulsingSharpTwo60,
    /// Transition Click 1 - 100%
    TransitionClickOne100,
    /// Transition Click 2 - 80%
    TransitionClickTwo80,
    /// Transition Click 3 - 60%
    TransitionClickThree60,
    /// Transition Click 4 - 40%
    TransitionClickFour40,
    /// Transition Click 5 - 20%
    TransitionClickFive20,
    /// Transition Click 6 - 10%
    TransitionClickSix10,
    /// Transition Hum 1 - 100%
    TransitionHumOne100,
    /// Transition Hum 2 - 80%
    TransitionHumTwo80,
    /// Transition Hum 3 - 60%
    TransitionHumThree60,
    /// Transition Hum 4 - 40%
    TransitionHumFour40,
    /// Transition Hum 5 - 20%
    TransitionHumFive20,
    /// Transition Hum 6 - 10%
    TransitionHumSix10,
    /// Transition Ramp Down Long Smooth 1 - 100 to 0%
    TransitionRampDownLongSmoothOne100to0,
    /// Transition Ramp Down Long Smooth 2 - 100 to 0%
    TransitionRampDownLongSmoothTwo100to0,
    /// Transition Ramp Down Medium Smooth 1 - 100 to 0%
    TransitionRampDownMediumSmoothOne100to0,
    /// Transition Ramp Down Medium Smooth 2 - 100 to 0%
    TransitionRampDownMediumSmoothTwo100to0,
    /// Transition Ramp Down Short Smooth 1 - 100 to 0%
    TransitionRampDownShortSmoothOne100to0,
    /// Transition Ramp Down Short Smooth 2 - 100 to 0%
    TransitionRampDownShortSmoothTwo100to0,
    /// Transition Ramp Down Long Sharp 1 - 100 to 0%
    TransitionRampDownLongSharpOne100to0,
    /// Transition Ramp Down Long Sharp 2 - 100 to 0%
    TransitionRampDownLongSharpTwo100to0,
    /// Transition Ramp Down Medium Sharp 1 - 100 to 0%
    TransitionRampDownMediumSharpOne100to0,
    /// Transition Ramp Down Medium Sharp 2 - 100 to 0%
    TransitionRampDownMediumSharpTwo100to0,
    /// Transition Ramp Down Short Sharp 1 - 100 to 0%
    TransitionRampDownShortSharpOne100to0,
    /// Transition Ramp Down Short Sharp 2 - 100 to 0%
    TransitionRampDownShortSharpTwo100to0,
    /// Transition Ramp Up Long Smooth 1 - 0 to 100%
    TransitionRampUpLongSmoothOne0to100,
    /// Transition Ramp Up Long Smooth 2 - 0 to 100%
    TransitionRampUpLongSmoothTwo0to100,
    /// Transition Ramp Up Medium Smooth 1 - 0 to 100%
    TransitionRampUpMediumSmoothOne0to100,
    /// Transition Ramp Up Medium Smooth 2 - 0 to 100%
    TransitionRampUpMediumSmoothTwo0to100,
    /// Transition Ramp Up Short Smooth 1 - 0 to 100%
    TransitionRampUpShortSmoothOne0to100,
    /// Transition Ramp Up Short Smooth 2 - 0 to 100%
    TransitionRampUpShortSmoothTwo0to100,
    /// Transition Ramp Up Long Sharp 1 - 0 to 100%
    TransitionRampUpLongSharpOne0to100,
    /// Transition Ramp Up Long Sharp 2 - 0 to 100%
    TransitionRampUpLongSharpTwo0to100,
    /// Transition Ramp Up Medium Sharp 1 - 0 to 100%
    TransitionRampUpMediumSharpOne0to100,
    /// Transition Ramp Up Medium Sharp 2 - 0 to 100%
    TransitionRampUpMediumSharpTwo0to100,
    /// Transition Ramp Up Short Sharp 1 - 0 to 100%
    TransitionRampUpShortSharpOne0to100,
    /// Transition Ramp Up Short Sharp 2 - 0 to 100%
    TransitionRampUpShortSharpTwo0to100,
    /// Transition Ramp Down Long Smooth 1 - 50 to 0%
    TransitionRampDownLongSmoothOne50to0,
    /// Transition Ramp Down Long Smooth 2 - 50 to 0%
    TransitionRampDownLongSmoothTwo50to0,
    /// Transition Ramp Down Medium Smooth 1 - 50 to 0%
    TransitionRampDownMediumSmoothOne50to0,
    /// Transition Ramp Down Medium Smooth 2 - 50 to 0%
    TransitionRampDownMediumSmoothTwo50to0,
    /// Transition Ramp Down Short Smooth 1 - 50 to 0%
    TransitionRampDownShortSmoothOne50to0,
    /// Transition Ramp Down Short Smooth 2 - 50 to 0%
    TransitionRampDownShortSmoothTwo50to0,
    /// Transition Ramp Down Long Sharp 1 - 50 to 0%
    TransitionRampDownLongSharpOne50to0,
    /// Transition Ramp Down Long Sharp 2 - 50 to 0%
    TransitionRampDownLongSharpTwo50to0,
    /// Transition Ramp Down Medium Sharp 1 - 50 to 0%
    TransitionRampDownMediumSharpOne50to0,
    /// Transition Ramp Down Medium Sharp 2 - 50 to 0%
    TransitionRampDownMediumSharpTwo50to0,
    /// Transition Ramp Down Short Sharp 1 - 50 to 0%
    TransitionRampDownShortSharpOne50to0,
    /// Transition Ramp Down Short Sharp 2 - 50 to 0%
    TransitionRampDownShortSharpTwo50to0,
    /// Transition Ramp Up Long Smooth 1 - 0 to 50%
    TransitionRampUpLongSmoothOne0to50,
    /// Transition Ramp Up Long Smooth 2 - 0 to 50%
    TransitionRampUpLongSmoothTwo0to50,
    /// Transition Ramp Up Medium Smooth 1 - 0 to 50%
    TransitionRampUpMediumSmoothOne0to50,
    /// Transition Ramp Up Medium Smooth 2 - 0 to 50%
    TransitionRampUpMediumSmoothTwo0to50,
    /// Transition Ramp Up Short Smooth 1 - 0 to 50%
    TransitionRampUpShortSmoothOne0to50,
    /// Transition Ramp Up Short Smooth 2 - 0 to 50%
    TransitionRampUpShortSmoothTwo0to50,
    /// Transition Ramp Up Long Sharp 1 - 0 to 50%
    TransitionRampUpLongSharpOne0to50,
    /// Transition Ramp Up Long Sharp 2 - 0 to 50%
    TransitionRampUpLongSharpTwo0to50,
    /// Transition Ramp Up Medium Sharp 1 - 0 to 50%
    TransitionRampUpMediumSharpOne0to50,
    /// Transition Ramp Up Medium Sharp 2 - 0 to 50%
    TransitionRampUpMediumSharpTwo0to50,
    /// Transition Ramp Up Short Sharp 1 - 0 to 50%
    TransitionRampUpShortSharpOne0to50,
    /// Transition Ramp Up Short Sharp 2 - 0 to 50%
    TransitionRampUpShortSharpTwo0to50,
    /// Long Buzz For Programmatic Stopping - 100%
    LongBuzzForProgrammaticStopping100,
    /// Smooth Hum 1 (No kick or brake pulse) - 50%
    SmoothHumOne50,
    /// Smooth Hum 2 (No kick or brake pulse) - 40%
    SmoothHumTwo40,
    /// Smooth Hum 3 (No kick or brake pulse) - 30%
    SmoothHumThree30,
    /// Smooth Hum 4 (No kick or brake pulse) - 20%
    SmoothHumFour20,
    /// Smooth Hum 5 (No kick or brake pulse) - 10%
    SmoothHumFive10,
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

impl Default for FeedbackControlReg {
    fn default() -> Self {
        let mut reg = Self(0);
        reg.set_n_erm_lra(false);
        reg.set_fb_brake_factor(0x3);
        reg.set_loop_gain(0x1);
        reg.set_bemf_gain(0x2);
        reg
    }
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

impl Default for Control1Reg {
    fn default() -> Self {
        let mut reg = Self(0);
        reg.set_startup_boost(true);
        reg.set_ac_couple(false);
        reg.set_drive_time(0x13);
        reg
    }
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

impl Default for Control2Reg {
    fn default() -> Self {
        let mut reg = Self(0);
        reg.set_bidir_input(true);
        reg.set_brake_stabilizer(true);
        reg.set_sample_time(0x3);
        reg.set_blanking_time(0x1);
        reg.set_idiss_time(0x1);
        reg
    }
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

impl Default for Control3Reg {
    fn default() -> Self {
        let mut reg = Self(0);
        reg.set_ng_thresh(0x2);
        reg.set_erm_open_loop(true);
        reg.set_supply_comp_dis(false);
        reg.set_data_format_rtp(false);
        reg.set_lra_drive_mode(false);
        reg.set_n_pwm_analog(false);
        reg.set_lra_open_loop(false);
        reg
    }
}

bitfield! {
    pub struct Control4Reg(u8);
    impl Debug;

    /// This bit sets the minimum length of time devoted for detecting a zero crossing.
    /// (advanced use only). Only documented on l models?
    /// 0: 100 us (Default)
    /// 1: 200 us
    /// 2: 300 us
    /// 3: 390 us
    pub zc_det_time, set_zc_det_time: 7, 6;

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

    /// This bit launches the programming process for one-time programmable
    /// (OTP) memory which programs the contents of register 0x16 through 0x1A
    /// into nonvolatile memory. This process can only be executed one time per
    /// device. See the Programming On-Chip OTP Memory section for details.
    pub otp_program, set_otp_program: 1;
}

impl Default for Control4Reg {
    fn default() -> Self {
        let mut reg = Self(0);
        reg.set_auto_cal_time(0x2);
        reg.set_otp_program(false);
        reg
    }
}

bitfield! {
    pub struct Control5Reg(u8);
    impl Debug;

    /// This bit selects number of cycles required to attempt synchronization
    /// before transitioning to open loop when the LRA_AUTO_OPEN_LOOP bit is
    /// asserted,
    /// 0: 3 attempts (Default)
    /// 1: 4 attempts
    /// 2: 5 attempts
    /// 3: 6 attempts
    pub auto_ol_cnt, set_auto_ol_cnt: 7, 6;

    /// This bit selects the automatic transition to open-loop drive when a
    /// back-EMF signal is not detected (LRA only).
    ///
    /// 0: Never transitions to open loop (Default)
    /// 1: Automatically transitions to open loop
    pub lra_auto_open_loop, set_lra_auto_open_loop: 5;

    /// This bit selects the memory playback interval
    /// 0: 5ms (Default)
    /// 1: 1ms
    pub playback_interval, set_playback_interval: 4;

    /// Thhis bit sets the MSB for the BLANKING_TIME[3:0]. See the
    /// BLANKING_TIME[3:0] bit in the Control2 (Address: 0x1C) section for details.
    /// Advanced use only.
    pub blanking_time_msb, set_blanking_time_mss: 3,2;

    /// This bit sets the MSB for IDISS_TIME[3:0]. See the IDISS_TIME[1:0] bit
    /// in the Control2 section for details. Advanced use only
    pub idiss_time_msb, set_idiss_time_msb: 1;

}

impl Default for Control5Reg {
    fn default() -> Self {
        let mut reg = Self(0);
        reg.set_auto_ol_cnt(0x2);
        reg
    }
}

#[allow(unused)]
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Register {
    Status = 0x00,
    Mode = 0x01,
    /// This field is the entry point for real-time playback (RTP) data. The
    /// DRV2605 playback engine drives the RTP_INPUT[7:0] value to the load when
    /// MODE[2:0] = 5 (RTP mode). The RTP_INPUT[7:0] value can be updated in
    /// real-time by the host controller to create haptic waveforms. The
    /// RTP_INPUT[7:0] value is interpreted as signed by default, but can be set
    /// to unsigned by the DATA_FORMAT_RTP bit in register 0x1D. When the haptic
    /// waveform is complete, the user can idle the device by setting MODE[2:0]
    /// = 0, or alternatively by setting STANDBY = 1.
    RealTimePlaybackInput = 0x02,
    LibrarySelection = 0x03,
    WaveformSequence0 = 0x04,
    WaveformSequence1 = 0x05,
    WaveformSequence2 = 0x06,
    WaveformSequence3 = 0x07,
    WaveformSequence4 = 0x08,
    WaveformSequence5 = 0x09,
    WaveformSequence6 = 0x0a,
    WaveformSequence7 = 0x0b,
    Go = 0x0c,
    OverdriveTimeOffset = 0x0d,
    SustainTimeOffsetPositive = 0x0e,
    SustainTimeOffsetNegative = 0x0f,
    BrakeTimeOffset = 0x10,

    // todo
    AudioToVibeControl = 0x11,
    AudioToVibeMinimumInputLevel = 0x12,
    AudioToVibeMaximumInputLevel = 0x13,
    AudioToVibeMinimumOutputDrive = 0x14,
    AudioToVibeMaximumOutputDrive = 0x15,

    /// This bit sets the reference voltage for full-scale output during
    /// closed-loop operation. The auto-calibration routine uses this register
    /// as an input, so this register must be written with the rated voltage
    /// value of the motor before calibration is performed. This register is
    /// ignored for open-loop operation because the overdrive voltage sets the
    /// reference for that case. Any modification of this register value should
    /// be followed by calibration to set A_CAL_BEMF appropriately.
    ///
    /// See the Rated Voltage Programming section for calculating the correct
    /// register value.
    RatedVoltage = 0x16,

    /// During closed-loop operation the actuator feedback allows the output
    /// voltage to go above the rated voltage during the automatic overdrive and
    /// automatic braking periods. This register sets a clamp so that the
    /// automatic overdrive is bounded. This bit also serves as the full-scale
    /// reference voltage for open-loop operation.
    ///
    /// See the Overdrive Voltage-Clamp Programming section for calculating the
    /// correct register value.
    ///
    /// 8.5.2.2 Overdrive Voltage-Clamp Programming LRA and ERM are swapped,
    /// confirmed https://e2e.ti.com/support/other_analog/haptics/f/927/t/655886
    /// (21.64x10-3 x OD_CLAMP[7:0] x (tDRIVE_TIME - 300x10^-6)) / (tDRIVE_TIME
    /// + tIDISS_TIME + tBLANKING_TIME)
    OverdriveClampVoltage = 0x17,

    /// This register contains the voltage-compensation result after execution
    /// of auto calibration. The value stored in the A_CAL_COMP bit compensates
    /// for any resistive losses in the driver. The calibration routine checks
    /// the impedance of the actuator to automatically determine an appropriate
    /// value. The auto- calibration compensation-result value is multiplied by
    /// the drive gain during playback.
    ///
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

    Control5 = 0x1f,

    LRAOpenLoopPeriod = 0x20,

    //todo
    VBatVoltageMonitor = 0x21,
    LraResonancePeriod = 0x22,
}
