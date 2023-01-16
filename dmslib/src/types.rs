//! Primitive data types.

/// Data type for bus indices.
#[cfg(not(feature = "minmem"))]
pub type BusIndex = usize;
#[cfg(feature = "minmem")]
pub type BusIndex = u8;

/// Data type for state indices.
#[cfg(not(feature = "minmem"))]
pub type StateIndex = usize;
#[cfg(feature = "minmem")]
pub type StateIndex = u32;

/// Data type for measuring time.
#[cfg(not(feature = "minmem"))]
pub type Time = usize;
#[cfg(feature = "minmem")]
pub type Time = u8;

/// Data type for measuring transition costs.
#[cfg(not(feature = "minmem"))]
pub type Cost = f64;
#[cfg(feature = "minmem")]
pub type Cost = BusIndex;

/// Data type for probability.
#[cfg(not(feature = "minmem"))]
pub type Probability = f64;
#[cfg(feature = "minmem")]
pub type Probability = f32;

/// Data type for the value function.
#[cfg(not(feature = "minmem"))]
pub type Value = f64;
#[cfg(feature = "minmem")]
pub type Value = f32;

/// Data type for action indices in policy.
#[cfg(not(feature = "minmem"))]
pub type ActionIndex = usize;
#[cfg(feature = "minmem")]
pub type ActionIndex = u32;
