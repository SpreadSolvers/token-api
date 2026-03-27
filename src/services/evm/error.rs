use crate::repositories::RepoError;
use actix_web::error::BlockingError;
use alloy::transports::TransportError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvmTokenServiceError {
    #[error("Repository error: {0}")]
    Repository(RepoError),

    #[error("Chain error: {0}")]
    Chain(TransportError),

    #[error("Multicall error: {0}")]
    Multicall(String),

    #[error("Chain ID mismatch: {0} != {1}")]
    ChainIdMismatch(u64, u64),

    #[error("CAIP ID build failed: {0}")]
    CaipIdBuildFailed(tap_caip::error::Error),

    #[error("Blocking error: {0}")]
    BlockingError(BlockingError),
}

// impl Display for EvmTokenServiceError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "EvmTokenServiceError: {}", self)
//     }
// }

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
