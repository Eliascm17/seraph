use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Custom error message")]
    CustomError,

    #[msg("Validator cannot be scored yet: less than 5 epochs have passed")]
    NotEnoughEpochs,
}
