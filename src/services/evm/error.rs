use std::{
    error::Error,
    fmt::{self, Display},
};

use crate::repositories::RepoError;
use actix_web::error::BlockingError;
use alloy::transports::TransportError;

#[derive(Debug)]
pub enum EvmTokenServiceError {
    Repository(RepoError),
    Chain(TransportError),
    Multicall(String),
    ChainIdMismatch(u64, u64),
    CaipIdBuildFailed(tap_caip::error::Error),
    BlockingError(BlockingError),
}

impl Display for EvmTokenServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvmTokenServiceError: {}", self)
    }
}

impl Error for EvmTokenServiceError {}

impl From<RepoError> for EvmTokenServiceError {
    fn from(error: RepoError) -> Self {
        EvmTokenServiceError::Repository(error)
    }
}

impl From<TransportError> for EvmTokenServiceError {
    fn from(error: TransportError) -> Self {
        EvmTokenServiceError::Chain(error)
    }
}

impl From<tap_caip::error::Error> for EvmTokenServiceError {
    fn from(error: tap_caip::error::Error) -> Self {
        EvmTokenServiceError::CaipIdBuildFailed(error)
    }
}

impl From<BlockingError> for EvmTokenServiceError {
    fn from(error: BlockingError) -> Self {
        EvmTokenServiceError::BlockingError(error)
    }
}
