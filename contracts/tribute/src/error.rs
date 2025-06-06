use cosmwasm_std::{StdError, VerificationError};
use outbe_nft::error::Cw721ContractError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Cw721ContractError(#[from] Cw721ContractError),
    #[error("WrongInput")]
    WrongInput {},
    #[error("WrongDigest")]
    WrongDigest {},
    #[error("{0}")]
    VerificationError(#[from] VerificationError),
    #[error("HashAlreadyExists")]
    HashAlreadyExists {},
    #[error("Submission failed: tribute already exists on the network")]
    AlreadyExists {},
}
