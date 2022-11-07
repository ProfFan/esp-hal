use core::marker::PhantomData;

use fugit::HertzU32;
use operator::Operator;
use timer::Timer;

use crate::{
    clock::Clocks,
    system::{Peripheral, PeripheralClockControl},
    types::OutputSignal,
};

pub mod operator;
pub mod timer;

// TODO provide a getter
pub struct PwmClock(HertzU32);

pub struct MCPWM<'a, PWM> {
    pub timer0: Timer<0, PWM>,
    pub timer1: Timer<1, PWM>,
    pub timer2: Timer<2, PWM>,
    pub operator0: Operator<0, PWM>,
    pub operator1: Operator<1, PWM>,
    pub operator2: Operator<2, PWM>,
    pub pwm_clk: PwmClock,
    phantom: PhantomData<(&'a Clocks, PWM)>,
}

impl<'a, PWM: PwmPeripheral> MCPWM<'a, PWM> {
    /// `pwm_clk = clocks.crypto_pwm_clock / (prescaler + 1)`
    // clocks.crypto_pwm_clock normally is 160 MHz
    pub fn new(
        peripheral: PWM,
        clocks: &'a Clocks,
        prescaler: u8,
        system: &mut PeripheralClockControl,
    ) -> Self {
        let _ = peripheral;

        PWM::enable(system);

        let block = unsafe { &*PWM::block() };
        // set prescaler
        block.clk_cfg.write(|w| w.clk_prescale().variant(prescaler));
        // enable clock
        block.clk.write(|w| w.en().set_bit());

        // sync comparator updates when timer counter is equal to zero.
        Self::set_comparator_update_method(block, 0b0001);

        #[cfg(esp32)]
        // TODO Docs are unclear here, need to test this
        let pwm_clk = PwmClock(clocks.apb_clock / (prescaler as u32 + 1));
        #[cfg(esp32s3)]
        let pwm_clk = PwmClock(clocks.crypto_pwm_clock / (prescaler as u32 + 1));

        MCPWM {
            timer0: Timer::new(),
            timer1: Timer::new(),
            timer2: Timer::new(),
            operator0: Operator::new(),
            operator1: Operator::new(),
            operator2: Operator::new(),
            pwm_clk,
            phantom: PhantomData,
        }
    }

    pub fn timer_freq(&self, mode: timer::PwmWorkingMode, prescaler: u8, period: u16) -> HertzU32 {
        let period = match mode {
            timer::PwmWorkingMode::Increase | timer::PwmWorkingMode::Decrease => period as u32 + 1,
            timer::PwmWorkingMode::UpDown => period as u32 * 2,
        };
        self.pwm_clk.0 / (prescaler as u32 + 1) / period
    }

    #[cfg(esp32)]
    fn set_comparator_update_method(block: &crate::pac::pwm0::RegisterBlock, bits: u8) {
        block.gen0_stmp_cfg.write(|w| {
            w.gen0_a_upmethod()
                .variant(bits)
                .gen0_b_upmethod()
                .variant(bits)
        });
        block.gen1_stmp_cfg.write(|w| {
            w.gen1_a_upmethod()
                .variant(bits)
                .gen1_b_upmethod()
                .variant(bits)
        });
        block.gen2_stmp_cfg.write(|w| {
            w.gen2_a_upmethod()
                .variant(bits)
                .gen2_b_upmethod()
                .variant(bits)
        });
    }

    #[cfg(esp32s3)]
    fn set_comparator_update_method(block: &crate::pac::pwm0::RegisterBlock, bits: u8) {
        block.cmpr0_cfg.write(|w| {
            w.cmpr0_a_upmethod()
                .variant(bits)
                .cmpr0_b_upmethod()
                .variant(bits)
        });
        block.cmpr1_cfg.write(|w| {
            w.cmpr1_a_upmethod()
                .variant(bits)
                .cmpr1_b_upmethod()
                .variant(bits)
        });
        block.cmpr2_cfg.write(|w| {
            w.cmpr2_a_upmethod()
                .variant(bits)
                .cmpr2_b_upmethod()
                .variant(bits)
        });
    }
}

/// A MCPWM peripheral
pub unsafe trait PwmPeripheral {
    /// Enable peripheral
    fn enable(system: &mut PeripheralClockControl);
    /// Get a pointer to the peripheral RegisterBlock
    fn block() -> *const crate::pac::pwm0::RegisterBlock;
    /// Get operator GPIO mux output signal
    fn output_signal<const O: u8, const IS_A: bool>() -> OutputSignal;
}

unsafe impl PwmPeripheral for crate::pac::PWM0 {
    fn enable(system: &mut PeripheralClockControl) {
        system.enable(Peripheral::Mcpwm0)
    }

    fn block() -> *const crate::pac::pwm0::RegisterBlock {
        Self::ptr()
    }

    fn output_signal<const O: u8, const IS_A: bool>() -> OutputSignal {
        match (O, IS_A) {
            (0, true) => OutputSignal::PWM0_0A,
            (1, true) => OutputSignal::PWM0_1A,
            (2, true) => OutputSignal::PWM0_1A,
            (0, false) => OutputSignal::PWM0_0B,
            (1, false) => OutputSignal::PWM0_1B,
            (2, false) => OutputSignal::PWM0_1B,
            _ => unreachable!(),
        }
    }
}

unsafe impl PwmPeripheral for crate::pac::PWM1 {
    fn enable(system: &mut PeripheralClockControl) {
        system.enable(Peripheral::Mcpwm1)
    }

    fn block() -> *const crate::pac::pwm0::RegisterBlock {
        Self::ptr()
    }

    fn output_signal<const O: u8, const IS_A: bool>() -> OutputSignal {
        match (O, IS_A) {
            (0, true) => OutputSignal::PWM1_0A,
            (1, true) => OutputSignal::PWM1_1A,
            (2, true) => OutputSignal::PWM1_1A,
            (0, false) => OutputSignal::PWM1_0B,
            (1, false) => OutputSignal::PWM1_1B,
            (2, false) => OutputSignal::PWM1_1B,
            _ => unreachable!(),
        }
    }
}
