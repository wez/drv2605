#![no_std]

mod registers;
use crate::registers::*;
pub use crate::registers::{Effect, Library};
use embedded_hal::blocking::i2c::{Write, WriteRead};

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
    /// Returns a calibrated Drv2605l Erm device configured to standby mode for
    /// power savings. Use a `set_mode` and `set_go` to trigger a vibration.
    pub fn erm(i2c: I2C, calibration: ErmCalibration) -> Result<Self, DrvError> {
        let mut haptic = Self { i2c, lra: false };
        haptic.check_id(7)?;

        // todo reset so registers are defaulted. Timing out..  need a solution
        // for delaying and retrying
        // haptic.reset()?;

        match calibration {
            // device will get calibration values out of the otp if the otp bit is set
            ErmCalibration::Otp => {
                if !haptic.is_otp()? {
                    return Err(DrvError::OTPNotProgrammed);
                }
            }
            // load up previously calibrated values
            ErmCalibration::Load(c) => haptic.set_calibration(c)?,
            ErmCalibration::Auto(c) => {
                let mut feedback: FeedbackControlReg = Default::default();
                let mut ctrl2: Control2Reg = Default::default();
                let mut ctrl4: Control4Reg = Default::default();

                feedback.set_fb_brake_factor(c.brake_factor);
                feedback.set_loop_gain(c.loop_gain);
                ctrl2.set_sample_time(c.lra_sample_time);
                ctrl2.set_blanking_time(c.lra_blanking_time);
                ctrl2.set_idiss_time(c.lra_idiss_time);
                ctrl4.set_auto_cal_time(c.auto_cal_time);
                ctrl4.set_zc_det_time(c.lra_zc_det_time);

                haptic.write(Register::FeedbackControl, feedback.0)?;
                haptic.write(Register::Control2, ctrl2.0)?;
                haptic.write(Register::Control4, ctrl4.0)?;
                haptic.calibrate()?;
            }
        }

        haptic.set_standby(true)?;

        Ok(haptic)
    }

    /// Returns a calibrated Drv2605l Lra device configured to standby mode for
    /// power savings. Use a `set_mode` and `set_go` to trigger a vibration.
    pub fn lra(i2c: I2C, calibration: LraCalibration) -> Result<Self, DrvError> {
        let mut haptic = Self { i2c, lra: true };
        haptic.check_id(7)?;

        // todo reset so registers are defaulted. Timing out..  need a solution
        // for delaying and retrying
        // haptic.reset()?;

        match calibration {
            // device will get calibration values out of the otp if the otp bit
            // is set
            LraCalibration::Otp => {
                if !haptic.is_otp()? {
                    return Err(DrvError::OTPNotProgrammed);
                }
            }
            // load up previously calibrated values
            LraCalibration::Load(c) => haptic.set_calibration(c)?,
            LraCalibration::Auto(c, c_lra) => {
                let mut feedback: FeedbackControlReg = Default::default();
                let mut ctrl2: Control2Reg = Default::default();
                let mut ctrl4: Control4Reg = Default::default();
                let mut ctrl1: Control1Reg = Default::default();

                feedback.set_fb_brake_factor(c.brake_factor);
                feedback.set_loop_gain(c.loop_gain);
                ctrl1.set_drive_time(c_lra.drive_time);
                ctrl2.set_sample_time(c.lra_sample_time);
                ctrl2.set_blanking_time(c.lra_blanking_time);
                ctrl2.set_idiss_time(c.lra_idiss_time);
                ctrl4.set_auto_cal_time(c.auto_cal_time);
                ctrl4.set_zc_det_time(c.lra_zc_det_time);

                haptic.write(Register::FeedbackControl, feedback.0)?;
                haptic.write(Register::RatedVoltage, c_lra.rated)?;
                haptic.write(Register::OverdriveClampVoltage, c_lra.clamp)?;
                haptic.write(Register::Control1, ctrl1.0)?;
                haptic.write(Register::Control2, ctrl2.0)?;
                haptic.write(Register::Control4, ctrl4.0)?;
                haptic.calibrate()?;
            }
        }

        haptic.set_standby(true)?;

        Ok(haptic)
    }

    /// Select the Immersion TS2200 library that matches your motor
    /// characteristic. Afterwards set the rom(s) and thn GO bit to play
    pub fn set_mode_rom(&mut self, library: Library) -> Result<(), DrvError> {
        if !self.lra && library == Library::Lra {
            return Err(DrvError::WrongMotorType);
        }
        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_mode(Mode::InternalTrigger as u8);
        self.write(Register::Mode, mode.0)?;

        if !self.lra {
            // Library A is open loop ONLY, but also says all seem to prefer open
            // loop for ERM
            self.set_open_loop(true)?;
        } else {
            self.set_open_loop(false)?;
        }

        let mut register = RegisterThree(self.read(Register::Register3)?);
        register.set_library_selection(library as u8);
        self.write(Register::Register3, register.0)
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

    /// Device accepts an analog voltage at the IN/TRIG pin until mode change or
    /// standby. The reference voltage in standby mode is 1.8 V thus 100% is
    /// 1.8V, 50% is .9V, 0% is 0V analogous to the duty-cycle percentage in
    /// PWM mode
    pub fn set_mode_analog(&mut self) -> Result<(), DrvError> {
        self.set_open_loop(false)?;

        let mut ctrl3 = Control3Reg(self.read(Register::Control3)?);
        ctrl3.set_n_pwm_analog(true);
        self.write(Register::Control3, ctrl3.0)?;

        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_mode(Mode::PwmInputAndAnalogInput as u8);
        self.write(Register::Mode, mode.0)
    }

    /// Device accepts PWM data at the IN/TRIG pin
    pub fn set_mode_pwm(&mut self) -> Result<(), DrvError> {
        self.set_open_loop(false)?;

        let mut ctrl3 = Control3Reg(self.read(Register::Control3)?);
        ctrl3.set_n_pwm_analog(false);
        self.write(Register::Control3, ctrl3.0)?;

        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_mode(Mode::PwmInputAndAnalogInput as u8);
        self.write(Register::Mode, mode.0)
    }

    /// Plays duty cycle set with `set_rtp` until another call to `set_rtp`, or
    /// mode change
    pub fn set_mode_rtp(&mut self) -> Result<(), DrvError> {
        self.set_open_loop(false)?;

        let mut ctrl3 = Control3Reg(self.read(Register::Control3)?);
        // unsigned. todo do we need to unset?
        ctrl3.set_data_format_rtp(true);
        self.write(Register::Control3, ctrl3.0)?;

        let mut mode = ModeReg(self.read(Register::Mode)?);
        mode.set_mode(Mode::RealTimePlayback as u8);
        self.write(Register::Mode, mode.0)
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
    pub fn set_go(&mut self, go: bool) -> Result<(), DrvError> {
        let mut register = GoReg(self.read(Register::Go)?);
        register.set_go(go);
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

    /// Closed-loop operation is usually desired for because of automatic
    /// overdrive and braking properties.
    fn set_open_loop(&mut self, enable: bool) -> Result<(), DrvError> {
        let mut reg = Control3Reg(self.read(Register::Control3)?);
        if self.lra {
            reg.set_lra_open_loop(enable);
        } else {
            reg.set_erm_open_loop(enable);
        }
        self.write(Register::Control3, reg.0)
    }

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

    fn mode(&mut self) -> Result<ModeReg, DrvError> {
        self.read(Register::Mode).map(ModeReg)
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

    /// This bit sets the output driver into a true high-impedance state. The
    /// device must be enabled to go into the high-impedance state. When in
    /// hardware shutdown or standby mode, the output drivers have 15 kΩ to
    /// ground. When the HI_Z bit is asserted, the hi-Z functionality takes
    /// effect immediately, even if a transaction is taking place.
    fn set_high_impedance_state(&mut self, value: bool) -> Result<(), DrvError> {
        let mut register = RegisterThree(self.read(Register::Register3)?);
        register.set_hi_z(value);
        self.write(Register::Register3, register.0)
    }

    /// This bit adds a time offset to the overdrive portion of the library
    /// waveforms. Some motors require more overdrive time than others, so this
    /// register allows the user to add or remove overdrive time from the
    /// library waveforms. The maximum voltage value in the library waveform is
    /// automatically determined to be the overdrive portion. This register is
    /// only useful in open-loop mode. Overdrive is automatic for closed-loop
    /// mode. The offset is interpreted as 2s complement, so the time offset may
    /// be positive or negative. Overdrive Time Offset (ms) = ODT[7:0] ×
    /// PLAYBACK_INTERVAL See the section for PLAYBACK_INTERVAL details.
    fn set_overdrive_time_offset(&mut self, value: i8) -> Result<(), DrvError> {
        self.write(Register::OverdriveTimeOffset, value as u8)
    }

    /// This bit adds a time offset to the positive sustain portion of the
    /// library waveforms. Some motors have a faster or slower response time
    /// than others, so this register allows the user to add or remove positive
    /// sustain time from the library waveforms. Any positive voltage value
    /// other than the overdrive portion is considered as a sustain positive
    /// value. The offset is interpreted as 2s complement, so the time offset
    /// can positive or negative. Sustain-Time Positive Offset (ms) = SPT[7:0] ×
    /// PLAYBACK_INTERVAL See the section for PLAYBACK_INTERVAL details.
    fn set_sustain_time_offset_positive(&mut self, value: i8) -> Result<(), DrvError> {
        self.write(Register::SustainTimeOffsetPositive, value as u8)
    }

    /// This bit adds a time offset to the negative sustain portion of the
    /// library waveforms. Some motors have a faster or slower response time
    /// than others, so this register allows the user to add or remove negative
    /// sustain time from the library waveforms. Any negative voltage value
    /// other than the overdrive portion is considered as a sustaining negative
    /// value. The offset is interpreted as two’s complement, so the time offset
    /// can be positive or negative. Sustain-Time Negative Offset (ms) =
    /// SNT[7:0] × PLAYBACK_INTERVAL See the section for PLAYBACK_INTERVAL
    /// details.
    fn set_sustain_time_offset_negative(&mut self, value: i8) -> Result<(), DrvError> {
        self.write(Register::SustainTimeOffsetNegative, value as u8)
    }

    /// This bit adds a time offset to the braking portion of the library
    /// waveforms. Some motors require more braking time than others, so this
    /// register allows the user to add or take away brake time from the library
    /// waveforms. The most negative voltage value in the library waveform is
    /// automatically determined to be the braking portion. This register is
    /// only useful in open-loop mode. Braking is automatic for closed-loop
    /// mode. The offset is interpreted as 2s complement, so the time offset can
    /// be positive or negative. Brake Time Offset (ms) = BRT[7:0] ×
    /// PLAYBACK_INTERVAL See the section for PLAYBACK_INTERVAL details.
    fn set_brake_time_offset(&mut self, value: i8) -> Result<(), DrvError> {
        self.write(Register::BrakeTimeOffset, value as u8)
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
        mode.set_mode(Mode::Diagnostics as u8);
        self.write(Register::Mode, mode.0)?;

        self.set_go(true)?;

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
        mode.set_mode(Mode::AutoCalibration as u8);
        self.write(Register::Mode, mode.0)?;

        self.set_go(true)?;

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
pub const ADDRESS: u8 = 0x5a;

#[allow(unused)]
pub enum LraCalibration {
    Otp,
    /// When using autocalibration be sure to secure the motor to mass. It can't
    /// calibrate if its jumping around on a board or a desk.
    Auto(GeneralParams, LraParams),
    Load(LoadParams),
}

#[allow(unused)]
pub enum ErmCalibration {
    Otp,
    /// When using autocalibration be sure to secure the motor to mass. It can't
    /// calibrate if its jumping around on a board or a desk.
    Auto(GeneralParams),
    Load(LoadParams),
}
pub struct LoadParams {
    pub comp: u8,
    pub bemf: u8,
    pub gain: u8,
}

pub struct GeneralParams {
    pub brake_factor: u8,
    pub loop_gain: u8,
    pub lra_sample_time: u8,
    pub lra_blanking_time: u8,
    pub lra_idiss_time: u8,
    pub auto_cal_time: u8,
    pub lra_zc_det_time: u8,
}

/// additional fields requited for LRA calibration
pub struct LraParams {
    pub rated: u8,
    pub clamp: u8,
    pub drive_time: u8,
}

impl Default for GeneralParams {
    /// general best fit values from datasheet
    fn default() -> Self {
        Self {
            brake_factor: 2,
            loop_gain: 2,
            lra_sample_time: 3,
            lra_blanking_time: 1, //not on drv2605
            lra_idiss_time: 1,    //not on drv2605
            auto_cal_time: 3,
            lra_zc_det_time: 0, //not on drv2605
        }
    }
}
