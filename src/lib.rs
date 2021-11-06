#![no_std]

mod registers;
use embedded_hal::blocking::i2c::{Write, WriteRead};
pub use registers::{
    Control1Reg, Control2Reg, Control3Reg, Control4Reg, Effect, FeedbackControlReg, GoReg, Library,
    LibrarySelectionReg, ModeReg, Register, StatusReg, WaveformReg,
};

pub struct Drv2605l<I2C, E>
where
    I2C: WriteRead<Error = E> + Write<Error = E>,
{
    i2c: I2C,
    lra: bool,
}

#[allow(unused)]
impl<I2C, E> Drv2605l<I2C, E>
where
    I2C: WriteRead<Error = E> + Write<Error = E>,
{
    /// Returns a calibrated Drv2605l device configured to standby mode for
    /// power savings. Closed loop is hardcoded for all motors and modes except
    /// ERM motors in rom mode where open loop is automatically enabled.
    ///
    /// Use a `set_mode` and `set_go` to trigger a vibration.
    pub fn new(i2c: I2C, calibration: Calibration, lra: bool) -> Result<Self, DrvError> {
        let mut haptic = Self { i2c, lra };
        haptic.check_id(7)?;

        // todo reset so registers are defaulted. Timing out..  need a solution
        // for delaying and retrying
        // haptic.reset()?;

        match calibration {
            // device will get calibration values out of the otp if the otp bit is set
            Calibration::Otp => {
                if !haptic.is_otp()? {
                    return Err(DrvError::OTPNotProgrammed);
                }
            }
            // load up previously calibrated values
            Calibration::Load(c) => haptic.set_calibration(c)?,
            Calibration::Auto(c) => {
                let mut feedback: FeedbackControlReg = Default::default();
                let mut ctrl2: Control2Reg = Default::default();
                let mut ctrl4: Control4Reg = Default::default();
                let mut ctrl1: Control1Reg = Default::default();

                feedback.set_fb_brake_factor(c.brake_factor);
                feedback.set_loop_gain(c.loop_gain);
                if (lra) {
                    feedback.set_n_erm_lra(true);
                }
                ctrl2.set_sample_time(c.lra_sample_time);
                ctrl2.set_blanking_time(c.lra_blanking_time);
                ctrl2.set_idiss_time(c.lra_idiss_time);
                ctrl4.set_auto_cal_time(c.auto_cal_time);
                ctrl4.set_zc_det_time(c.lra_zc_det_time);
                ctrl1.set_drive_time(c.drive_time);

                haptic.write(Register::FeedbackControl, feedback.0)?;
                haptic.write(Register::Control2, ctrl2.0)?;
                haptic.write(Register::Control4, ctrl4.0)?;
                haptic.write(Register::RatedVoltage, c.rated)?;
                haptic.write(Register::OverdriveClampVoltage, c.clamp)?;
                haptic.write(Register::Control1, ctrl1.0)?;
                haptic.calibrate()?;
            }
        }

        haptic.set_standby(true)?;

        Ok(haptic)
    }

    pub fn set_mode(&mut self, mode: Mode) -> Result<(), DrvError> {
        let mut m = ModeReg(self.read(Register::Mode)?);
        let mut ctrl3 = Control3Reg(self.read(Register::Control3)?);

        match mode {
            Mode::Pwm => {
                // unset in case coming from rom mode
                if !self.lra {
                    ctrl3.set_erm_open_loop(false);
                }
                ctrl3.set_n_pwm_analog(false);
                self.write(Register::Control3, ctrl3.0)?;

                m.set_mode(registers::Mode::PwmInputAndAnalogInput as u8);
                self.write(Register::Mode, m.0)
            }
            Mode::Rom(library) => {
                // erm requires open loop mode
                if !self.lra {
                    ctrl3.set_erm_open_loop(true);
                }
                self.write(Register::Control3, ctrl3.0)?;

                let mut lib = LibrarySelectionReg(self.read(Register::LibrarySelection)?);
                lib.set_library_selection(library as u8);
                self.write(Register::LibrarySelection, lib.0)?;

                m.set_mode(registers::Mode::InternalTrigger as u8);
                self.write(Register::Mode, m.0)
            }
            Mode::Analog => {
                // unset in case coming from rom mode
                if !self.lra {
                    ctrl3.set_erm_open_loop(false);
                }
                ctrl3.set_n_pwm_analog(true);
                self.write(Register::Control3, ctrl3.0)?;

                m.set_mode(registers::Mode::PwmInputAndAnalogInput as u8);
                self.write(Register::Mode, m.0)
            }
            Mode::RealTimePlayback => {
                // We won't need to unset as no other modes use this bit
                ctrl3.set_data_format_rtp(true);
                // unset in case coming from rom mode
                if !self.lra {
                    ctrl3.set_erm_open_loop(false);
                }
                self.write(Register::Control3, ctrl3.0)?;

                m.set_mode(registers::Mode::RealTimePlayback as u8);
                self.write(Register::Mode, m.0)
            }
        }
    }

    /// Sets up to 8 Effects to play in order when `set_go` is called. Stops
    /// playing early if `Effect::None` is used.
    pub fn set_rom(&mut self, roms: &[Effect; 8]) -> Result<(), DrvError> {
        // Todo The MSB of each sequence register can implement a delay between
        // sequence waveforms. When the MSB is high, bits [6:0] indicate the
        // length of the wait time. The wait time for that step then becomes
        // WAV_FRM_SEQ[6:0] × 10 ms
        let buf: [u8; 9] = [
            Register::WaveformSequence0 as u8,
            roms[0] as u8,
            roms[1] as u8,
            roms[2] as u8,
            roms[3] as u8,
            roms[4] as u8,
            roms[5] as u8,
            roms[6] as u8,
            roms[7] as u8,
        ];
        self.i2c
            .write(ADDRESS, &buf)
            .map_err(|_| DrvError::ConnectionError)
    }

    /// Set a single Rom to play during rom mode when `set_go` is called
    pub fn set_rom_single(&mut self, effect: Effect) -> Result<(), DrvError> {
        let buf: [u8; 3] = [
            Register::WaveformSequence0 as u8,
            WaveformReg::new_effect(effect).0,
            WaveformReg::new_stop().0,
        ];
        self.i2c
            .write(ADDRESS, &buf)
            .map_err(|_| DrvError::ConnectionError)
    }

    /// Change the duty cycle for rtp mode
    pub fn set_rtp(&mut self, duty: u8) -> Result<(), DrvError> {
        self.write(Register::RealTimePlaybackInput, duty)
    }

    /// Get the current rtp duty cycle
    pub fn rtp(&mut self) -> Result<u8, DrvError> {
        self.read(Register::RealTimePlaybackInput)
    }

    /// Trigger a GO for whatever mode is enabled
    pub fn set_go(&mut self) -> Result<(), DrvError> {
        let mut register = GoReg(self.read(Register::Go)?);
        register.set_go(true);
        self.write(Register::Go, register.0)
    }

    /// Get the go bit. For some modes the go bit can be polled to see when it
    /// clears indicating a waveform has completed playback.
    pub fn go(&mut self) -> Result<bool, DrvError> {
        Ok(GoReg(self.read(Register::Go)?).go())
    }

    /// Enabling standby goes into a low power state but maintains all mode
    /// configuration
    pub fn set_standby(&mut self, enable: bool) -> Result<(), DrvError> {
        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_standby(enable);
        self.write(Register::Mode, mode.0)
    }

    /// Get the status bits
    pub fn status(&mut self) -> Result<StatusReg, DrvError> {
        self.read(Register::Status).map(StatusReg)
    }

    /// Get the LoadParams that were loaded at startup or calculated via
    /// Calibration
    pub fn calibration(&mut self) -> Result<LoadParams, DrvError> {
        let feedback = self
            .read(Register::FeedbackControl)
            .map(FeedbackControlReg)?;

        let comp = self.read(Register::AutoCalibrationCompensationResult)?;
        let bemf = self.read(Register::AutoCalibrationBackEMFResult)?;

        Ok(LoadParams {
            gain: feedback.bemf_gain(),
            comp,
            bemf,
        })
    }

    /* Private calls */

    /// Write `value` to `register`
    fn write(&mut self, register: Register, value: u8) -> Result<(), DrvError> {
        self.i2c
            .write(ADDRESS, &[register as u8, value])
            .map_err(|_| DrvError::ConnectionError)
    }

    /// Read an 8-bit value from the register
    fn read(&mut self, register: Register) -> Result<u8, DrvError> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(ADDRESS, &[register as u8], &mut buf)
            .map_err(|_| DrvError::ConnectionError)?;
        Ok(buf[0])
    }

    fn check_id(&mut self, id: u8) -> Result<(), DrvError> {
        let reg = self.status()?;
        if reg.device_id() != id {
            return Err(DrvError::WrongDeviceId);
        }

        Ok(())
    }

    /// performs the equivalent operation of power cycling the device. Any
    /// playback operations are immediately interrupted, and all registers are
    /// reset to the default values.
    fn reset(&mut self) -> Result<(), DrvError> {
        let mut mode = ModeReg::default();
        mode.set_dev_reset(true);
        self.write(Register::Mode, mode.0)?;

        while ModeReg(self.read(Register::Mode)?).dev_reset() {}

        Ok(())
    }

    fn set_calibration(&mut self, load: LoadParams) -> Result<(), DrvError> {
        let mut fbcr = FeedbackControlReg(self.read(Register::FeedbackControl)?);
        fbcr.set_bemf_gain(load.gain);
        self.write(Register::FeedbackControl, fbcr.0)?;

        self.write(Register::AutoCalibrationCompensationResult, load.comp)?;

        self.write(Register::AutoCalibrationBackEMFResult, load.bemf)
    }

    /// Run diagnostics
    fn diagnostics(&mut self) -> Result<(), DrvError> {
        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_standby(false);
        mode.set_mode(registers::Mode::Diagnostics as u8);
        self.write(Register::Mode, mode.0)?;

        self.set_go()?;

        //todo timeout
        while GoReg(self.read(Register::Go)?).go() {}

        let reg = self.status()?;
        if reg.diagnostic_result() {
            return Err(DrvError::DeviceDiagnosticFailed);
        }

        Ok(())
    }

    /// Run auto calibration which updates the calibration registers and returns
    /// the resulting LoadParams
    fn calibrate(&mut self) -> Result<LoadParams, DrvError> {
        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_standby(false);
        mode.set_mode(registers::Mode::AutoCalibration as u8);
        self.write(Register::Mode, mode.0)?;

        self.set_go()?;

        //todo timeout
        while GoReg(self.read(Register::Go)?).go() {}

        let reg = self.status()?;
        if reg.diagnostic_result() {
            return Err(DrvError::CalibrationFailed);
        }

        self.calibration()
    }

    /// Check if the device's OTP has been set
    fn is_otp(&mut self) -> Result<bool, DrvError> {
        let reg4 = Control4Reg(self.read(Register::Control4)?);
        Ok(reg4.otp_status())
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum DrvError {
    WrongMotorType,
    WrongDeviceId,
    ConnectionError,
    DeviceDiagnosticFailed,
    CalibrationFailed,
    OTPNotProgrammed,
    WrongCalibrationEnum,
}

/// The hardcoded address of the driver.  All drivers share the same address so
/// that it is possible to broadcast on the bus and have multiple units emit the
/// same waveform
const ADDRESS: u8 = 0x5a;

// Choose calibration method during driver construction
pub enum Calibration {
    /// Many calibration params can be defaulted, and maybe the entire thing for
    /// some ERM motors. Required params for LRA motors especially though should
    /// calculated from the drv2605l and motor datasheet.
    ///
    /// NOTE: In general, but when doing autocalibration, be sure to secure the
    /// motor to some kind of mass. It can't calibrate if its jumping around on
    /// a board or a desk.
    Auto(CalibrationParams),
    // Load previously calibrated values. It is common to do an autocalibration
    // and then read back the calibration parameters so you can hardcode them
    Load(LoadParams),
    // Values were previously programmed into nonvolatile memory. This is not common.
    Otp,
}

// Computed calibration parameters. Provide previously calculated parameters
// during construction, or after read back the calibrated values for hardcoding
// after succsesfully Auto calibration.s
pub struct LoadParams {
    /// Automatic Compensation for Resistive Losses
    pub comp: u8,
    /// Auto-Calibration Back-EMF Result
    pub bemf: u8,
    /// Auto-Calibration BEMF_GAIN Result
    pub gain: u8,
}

/// Calibration Parameters for both motor ERM and LRA motor types. Some params
/// really need to be computed from the drv2605l and motor datashets, especially
/// for LRA motors.
#[non_exhaustive]
pub struct CalibrationParams {
    /// Required: Datasheet 8.5.2.1 Rated Voltage Programming
    pub rated: u8,
    /// Required: Datasheet 8.5.2.2 Overdrive Voltage-Clamp Programming
    pub clamp: u8,
    /// Required: Datasheet 8.5.1.1 Drive-Time Programming
    pub drive_time: u8,
    /// Default advised: Brake Factor
    pub brake_factor: u8,
    /// Default advised: Loop-Gain Control
    pub loop_gain: u8,
    /// Default advised: Auto Calibration Time Adjustment
    pub auto_cal_time: u8,
    /// Default advised: LRA auto-resonance sampling time
    pub lra_sample_time: u8,
    /// Default advised: LRA auto-resonance sampling time
    pub lra_blanking_time: u8,
    /// Default advised: LRA Current dissipation time
    pub lra_idiss_time: u8,
    /// Default advised: LRA Zero Crossing Detect
    pub lra_zc_det_time: u8,
}

impl Default for CalibrationParams {
    fn default() -> Self {
        Self {
            brake_factor: 2,
            loop_gain: 2,
            lra_sample_time: 3,
            lra_blanking_time: 1,
            lra_idiss_time: 1,
            auto_cal_time: 3,
            lra_zc_det_time: 0,
            rated: 0x3E,
            clamp: 0x8C,
            drive_time: 0x13,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    /// Select the Immersion TS2200 library that matches your motor
    /// characteristic. For ERM Motors, open loop operation will be enabled as
    /// all ERM libraries are tuned for open loop.
    ///
    /// Use set rom setters and then GO bit to play an `Effect`
    Rom(Library),
    /// Enable Pulse Width Modulated mod (closed loop unidirectional )
    ///
    /// 0% full braking, 50% 1/2 Rated Voltage, 100% Rated Voltage
    Pwm,
    /// Set analog input mode.
    ///
    /// Send an analog voltage to the IN/TRIG to set a duty cycle which will
    /// persist until mode change or standby. The reference voltage in standby
    /// mode is 1.8 V thus 100% is 1.8V, 50% is .9V, 0% is 0V analogous to the
    /// duty-cycle percentage in PWM mode
    Analog,
    /// Enable Real Time Playback (closed loop unidirectional unsigned )
    ///
    /// Use `set_rtp` to update the duty cycle which will persist until another
    /// call to `set_rtp`, change to standby, or mode change.
    /// 0x00 full braking, 0x7F 1/2 Rated Voltage, 0xFF Rated Voltage
    RealTimePlayback,
}
