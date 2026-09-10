//! # Library for USB PD
//!
//! Modeled after the Universal Serial Bus Power Delivery Specification: USB PD R3.2 v1.1 (2024/10).
//!
//! The library implements:
//! - A policy engine for each supported mode,
//! - the protocol layer, and
//! - the `DevicePolicyManager` trait, which allows a device user application to talk to the policy engine, and control it.
//!
//! ## Currently supported modes
//!
//! - SPR Sink with helpers for requesting
//! - A fixed supply
//! - A Programmable Power Supply (PPS)
//!

#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

#[macro_use]
extern crate uom;

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub(crate) mod counters;
pub mod protocol_layer;
pub mod sink;
pub mod source;
pub mod timers;

#[cfg(test)]
#[allow(missing_docs)] // FIXME: Docs for the dummy?
pub mod dummy;

/// Units of measurement for the USB Power Delivery protocol layer, based on `uom`.
///
/// Quantities are stored as `u32` in the base units of this system: millivolt (mV) for
/// [`ElectricPotential`](units::ElectricPotential), milliampere (mA) for
/// [`ElectricCurrent`](units::ElectricCurrent) and microwatt (µW) for [`Power`](units::Power).
/// Multiplying a current by a potential yields a power, and dividing a power by a potential yields a
/// current, exactly and without any unit conversion.
///
/// The generic `uom` conversions `Quantity::new::<unit>()` and `Quantity::get::<unit>()` build their
/// conversion factors from `f64` at runtime, which pulls in software floating point and rational
/// arithmetic on targets without an FPU. Prefer the float-free constructors and getters of the
/// [`ElectricPotentialExt`](units::ElectricPotentialExt), [`ElectricCurrentExt`](units::ElectricCurrentExt)
/// and [`PowerExt`](units::PowerExt) traits, which operate directly on the stored base unit value.
pub mod units {
    use core::marker::PhantomData;

    ISQ!(
        uom::si,
        u32,
        (millimeter, kilogram, second, milliampere, kelvin, mole, candela)
    );

    /// Float-free construction and readout of an [`ElectricPotential`] in millivolts.
    pub trait ElectricPotentialExt {
        /// Create an electric potential from a value in millivolts.
        fn from_millivolts(millivolts: u32) -> Self;
        /// Get the electric potential in millivolts.
        fn millivolts(&self) -> u32;
    }

    impl ElectricPotentialExt for ElectricPotential {
        #[inline]
        fn from_millivolts(millivolts: u32) -> Self {
            Self {
                dimension: PhantomData,
                units: PhantomData,
                value: millivolts,
            }
        }

        #[inline]
        fn millivolts(&self) -> u32 {
            self.value
        }
    }

    /// Float-free construction and readout of an [`ElectricCurrent`] in milliamperes.
    pub trait ElectricCurrentExt {
        /// Create an electric current from a value in milliamperes.
        fn from_milliamperes(milliamperes: u32) -> Self;
        /// Get the electric current in milliamperes.
        fn milliamperes(&self) -> u32;
    }

    impl ElectricCurrentExt for ElectricCurrent {
        #[inline]
        fn from_milliamperes(milliamperes: u32) -> Self {
            Self {
                dimension: PhantomData,
                units: PhantomData,
                value: milliamperes,
            }
        }

        #[inline]
        fn milliamperes(&self) -> u32 {
            self.value
        }
    }

    /// Float-free construction and readout of a [`Power`] in milliwatts.
    pub trait PowerExt {
        /// Create a power from a value in milliwatts.
        fn from_milliwatts(milliwatts: u32) -> Self;
        /// Get the power in milliwatts.
        fn milliwatts(&self) -> u32;
    }

    impl PowerExt for Power {
        #[inline]
        fn from_milliwatts(milliwatts: u32) -> Self {
            Self {
                dimension: PhantomData,
                units: PhantomData,
                value: milliwatts * 1000,
            }
        }

        #[inline]
        fn milliwatts(&self) -> u32 {
            self.value / 1000
        }
    }
}

use core::fmt::Debug;

/// The power role of the port.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PowerRole {
    /// The port is a source.
    Source,
    /// The port is a sink.
    Sink,
}

impl From<bool> for PowerRole {
    fn from(value: bool) -> Self {
        match value {
            false => Self::Sink,
            true => Self::Source,
        }
    }
}

impl From<PowerRole> for bool {
    fn from(role: PowerRole) -> bool {
        match role {
            PowerRole::Sink => false,
            PowerRole::Source => true,
        }
    }
}

/// The data role of the port.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DataRole {
    /// The port is an upstream-facing port.
    Ufp,
    /// The port is a downstream-facing port.
    Dfp,
}

impl From<bool> for DataRole {
    fn from(value: bool) -> Self {
        match value {
            false => Self::Ufp,
            true => Self::Dfp,
        }
    }
}

impl From<DataRole> for bool {
    fn from(role: DataRole) -> bool {
        match role {
            DataRole::Ufp => false,
            DataRole::Dfp => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use uom::si::electric_current::{centiampere, milliampere};
    use uom::si::electric_potential::{millivolt, volt};
    use uom::si::power::{milliwatt, watt};

    use crate::units::{ElectricCurrent, ElectricCurrentExt, ElectricPotential, ElectricPotentialExt, Power, PowerExt};

    /// The float-free extension traits must agree with the generic `uom` conversions.
    #[test]
    fn test_units() {
        let current = ElectricCurrent::from_milliamperes(1230);
        let potential = ElectricPotential::from_millivolts(4560);
        let power = Power::from_milliwatts(140_000);

        core::assert_eq!(current, ElectricCurrent::new::<centiampere>(123));
        core::assert_eq!(potential, ElectricPotential::new::<millivolt>(4560));
        core::assert_eq!(power, Power::new::<watt>(140));

        core::assert_eq!(current.milliamperes(), current.get::<milliampere>());
        core::assert_eq!(potential.millivolts(), potential.get::<millivolt>());
        core::assert_eq!(power.milliwatts(), power.get::<milliwatt>());
        core::assert_eq!(ElectricPotential::from_millivolts(20_000).get::<volt>(), 20);
    }

    /// Multiplication and division of quantities is exact in the base units of the system.
    #[test]
    fn test_units_arithmetic() {
        let current = ElectricCurrent::from_milliamperes(3000);
        let potential = ElectricPotential::from_millivolts(20_000);
        let power = current * potential;

        core::assert_eq!(power, Power::from_milliwatts(60_000));
        core::assert_eq!(power / potential, current);
        core::assert_eq!(power / current, potential);
    }
}
