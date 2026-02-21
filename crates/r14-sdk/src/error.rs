// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Typed errors for [`R14Client`](crate::client::R14Client) operations.

#[derive(Debug, thiserror::Error)]
pub enum R14Error {
    #[error("insufficient balance: need {needed}, best {best}")]
    InsufficientBalance { needed: u64, best: u64 },

    #[error("note not on-chain — deposit or sync first")]
    NoteNotOnChain,

    #[error("indexer: {0}")]
    Indexer(String),

    #[error("soroban: {0}")]
    Soroban(String),

    #[error("config: {0}")]
    Config(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type R14Result<T> = Result<T, R14Error>;
