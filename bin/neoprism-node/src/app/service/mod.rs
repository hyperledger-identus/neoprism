pub mod error;
mod prism;

pub use prism::PrismDidService;

#[cfg(feature = "midnight")]
mod midnight;

#[cfg(feature = "midnight")]
pub use midnight::MidnightDidService;
