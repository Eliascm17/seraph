# Seraph

## Overview

Seraph is a Solana smart contract inspired by the concept of dynamically managing staking accounts to optimize staking rewards. The primary function of Seraph is to delegate stake accounts to validators performing above the 90th percentile in staking rewards over the last five epochs. It achieves this by utilizing data from the Validator History program to calculate staking rewards, using a formula based on epoch credits and validator commission.

The Seraph smart contract is designed to be a naive equivalent of advanced staking management systems, providing a simplified yet effective approach to maximizing staking rewards on the Solana network.

### Important Files

- `programs/seraph/*`: Directory containing the Seraph smart contract.
- `tests/tests/test_seraph.rs`: Tests for the smart contract, demonstrating delegate, redelegate, and deactivate operations of stake accounts.

## Test

Tests are in `tests/` written with solana-program-test, and can be run with `cargo test`. To run the seraph specific tests run `cargo test --test test_seraph`

## Build

`anchor build`