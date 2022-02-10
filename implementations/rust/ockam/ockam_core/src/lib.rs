//! Core types of the Ockam library.
//!
//! This crate contains the core types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!(r#"The "no_std" feature currently requires the "alloc" feature"#);

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

pub use async_trait::async_trait;

pub extern crate hashbrown;

#[allow(unused_imports)]
#[macro_use]
pub extern crate hex;

#[allow(unused_imports)]
#[macro_use]
pub extern crate async_trait;
pub use async_trait::async_trait as worker;

extern crate ockam_macros;
pub use ockam_macros::{AsyncTryClone, Message};

extern crate futures_util;

mod access_control;
pub mod compat;
mod error;
mod message;
mod processor;
mod routing;
pub mod vault;
mod worker;

pub use access_control::*;
pub use error::*;
pub use message::*;
pub use processor::*;
pub use routing::*;
pub use traits::*;
pub use worker::*;

#[cfg(feature = "std")]
pub use std::println;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
/// println macro for no_std
pub mod println_no_std {
    #[macro_export]
    /// implements println for no_std by wrapping the tracing::info! macro
    macro_rules! println {
        ($($arg:tt)*) => {{
            tracing::info!($($arg)*);
        }};
    }
}

/// Module for custom implementation of standard traits.
pub mod traits {
    use crate::compat::boxed::Box;
    use crate::error::Result;

    /// Clone trait for async structs.
    #[async_trait]
    pub trait AsyncTryClone: Sized {
        /// Try cloning a object and return an `Err` in case of failure.
        async fn async_try_clone(&self) -> Result<Self>;
    }
    #[async_trait]
    impl<D> AsyncTryClone for D
    where
        D: Clone + Sync,
    {
        async fn async_try_clone(&self) -> Result<Self> {
            Ok(self.clone())
        }
    }
}

/// Implement encode/decode functionality for `hashbrown::HashMap` which
/// does not have impls for `minicbor::{Encode, Decode}`.
pub mod hashbrown_cbor {
    use hashbrown::HashMap;
    use minicbor::{Encode, Decode, Encoder, Decoder};
    use minicbor::encode::Write;

    /// Encode this `HashMap` to CBOR.
    pub fn encode<K, V, S, W>(map: &HashMap<K, V, S>, e: &mut Encoder<W>) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        K: Encode,
        V: Encode,
        W: Write
    {
        e.map(map.len() as u64)?;
        for (k, v) in map {
            k.encode(e)?;
            v.encode(e)?;
        }
        Ok(())
    }

    /// Decode a `HashMap` from CBOR.
    pub fn decode<'b, K, V, S>(d: &mut Decoder<'b>) -> Result<HashMap<K, V, S>, minicbor::decode::Error>
    where
        K: Decode<'b> + core::hash::Hash + Eq,
        V: Decode<'b>,
        S: core::hash::BuildHasher + Default
    {
        let mut m = HashMap::default();
        let iter: minicbor::decode::MapIter<K, V> = d.map_iter()?;
        for x in iter {
            let (k, v) = x?;
            m.insert(k, v);
        }
        Ok(m)
    }

}
