#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
pub mod axum;

pub mod balance;

#[cfg(feature = "channel")]
#[cfg_attr(docsrs, doc(cfg(feature = "channel")))]
pub mod channel;

#[cfg(feature = "iroh")]
#[cfg_attr(docsrs, doc(cfg(feature = "iroh")))]
pub mod iroh;

#[cfg(feature = "reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "reqwest")))]
pub mod reqwest;
