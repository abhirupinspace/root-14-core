// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! High-level integration client for Root14.
//!
//! Wraps wallet, merkle, soroban, and (optionally) proof generation into
//! a small surface area: construct → deposit → transfer → balance.
//!
//! ```rust,no_run
//! use r14_sdk::client::{R14Client, R14Contracts};
//!
//! # async fn example() -> r14_sdk::error::R14Result<()> {
//! let client = R14Client::new(
//!     "http://localhost:3000",
//!     R14Contracts { core: "C_CORE...".into(), transfer: "C_XFER...".into() },
//!     "S_SECRET...",
//!     "testnet",
//! )?;
//! # Ok(())
//! # }
//! ```

use ark_bls12_381::Fr;
use serde::Deserialize;

use crate::error::{R14Error, R14Result};
use crate::wallet::NoteEntry;
use crate::{commitment, Note};

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

pub struct R14Client {
    indexer_url: String,
    contracts: R14Contracts,
    stellar_secret: String,
    network: String,
    http: reqwest::Client,
}

pub struct R14Contracts {
    pub core: String,
    pub transfer: String,
}

pub struct DepositResult {
    pub commitment: String,
    pub value: u64,
    pub app_tag: u32,
    pub tx_result: String,
    pub note_entry: NoteEntry,
}

pub struct TransferResult {
    pub nullifier: String,
    pub out_commitment_0: String,
    pub out_commitment_1: String,
    pub tx_result: String,
    pub recipient_note: NoteEntry,
    pub change_note: NoteEntry,
    pub consumed_note_index: usize,
}

pub struct BalanceResult {
    pub total: u64,
    pub notes: Vec<NoteStatus>,
}

pub struct NoteStatus {
    pub value: u64,
    pub app_tag: u32,
    pub commitment: String,
    pub on_chain: bool,
}

pub struct InitResult {
    pub circuit_id: String,
    pub tx_result: String,
}

pub struct PrebuiltProof {
    pub proof_json: String,
    pub old_root: String,
    pub nullifier: String,
    pub cm_0: String,
    pub cm_1: String,
}

// ---------------------------------------------------------------------------
// Indexer response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LeafResponse {
    index: u64,
    #[allow(dead_code)]
    block_height: u64,
}

#[derive(Deserialize)]
#[cfg_attr(not(feature = "prove"), allow(dead_code))]
struct ProofResponse {
    siblings: Vec<String>,
    indices: Vec<bool>,
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

impl R14Client {
    pub fn new(
        indexer_url: &str,
        contracts: R14Contracts,
        stellar_secret: &str,
        network: &str,
    ) -> R14Result<Self> {
        Ok(Self {
            indexer_url: indexer_url.to_string(),
            contracts,
            stellar_secret: stellar_secret.to_string(),
            network: network.to_string(),
            http: reqwest::Client::new(),
        })
    }

    pub fn from_wallet(wallet: &crate::wallet::WalletData) -> R14Result<Self> {
        Ok(Self {
            indexer_url: wallet.indexer_url.clone(),
            contracts: R14Contracts {
                core: wallet.core_contract_id.clone(),
                transfer: wallet.transfer_contract_id.clone(),
            },
            stellar_secret: wallet.stellar_secret.clone(),
            network: "testnet".to_string(),
            http: reqwest::Client::new(),
        })
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn fr_to_raw_hex(fr: &Fr) -> String {
        crate::wallet::fr_to_raw_hex(fr)
    }

    async fn fetch_leaf_index(&self, cm_hex: &str) -> R14Result<Option<u64>> {
        let cm = cm_hex.strip_prefix("0x").unwrap_or(cm_hex);
        let url = format!("{}/v1/leaf/{}", self.indexer_url, cm);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| R14Error::Indexer(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok(None);
        }
        match resp.json::<LeafResponse>().await {
            Ok(leaf) => Ok(Some(leaf.index)),
            Err(_) => Ok(None),
        }
    }

    #[cfg_attr(not(feature = "prove"), allow(dead_code))]
    async fn fetch_merkle_proof(
        &self,
        leaf_index: u64,
    ) -> R14Result<(Vec<Fr>, Vec<bool>)> {
        let url = format!("{}/v1/proof/{}", self.indexer_url, leaf_index);
        let resp: ProofResponse = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| R14Error::Indexer(e.to_string()))?
            .json()
            .await
            .map_err(|e| R14Error::Indexer(format!("parse proof: {e}")))?;

        let siblings: Vec<Fr> = resp
            .siblings
            .iter()
            .map(|s| crate::wallet::hex_to_fr(s).map_err(R14Error::Other))
            .collect::<R14Result<_>>()?;

        Ok((siblings, resp.indices))
    }

    async fn invoke(
        &self,
        contract_id: &str,
        function: &str,
        args: &[(&str, &str)],
    ) -> R14Result<String> {
        crate::soroban::invoke_contract(
            contract_id,
            &self.network,
            &self.stellar_secret,
            function,
            args,
        )
        .await
        .map_err(|e| R14Error::Soroban(e.to_string()))
    }

    #[cfg_attr(not(feature = "prove"), allow(dead_code))]
    fn require_contracts(&self) -> R14Result<()> {
        if self.contracts.transfer == "PLACEHOLDER" || self.contracts.core == "PLACEHOLDER" {
            return Err(R14Error::Config(
                "contracts not configured — set core and transfer contract IDs".to_string(),
            ));
        }
        Ok(())
    }

    fn require_transfer_contract(&self) -> R14Result<()> {
        if self.contracts.transfer == "PLACEHOLDER" {
            return Err(R14Error::Config(
                "transfer_contract_id not configured".to_string(),
            ));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Public API — always available
    // -----------------------------------------------------------------------

    /// Create a note and submit deposit on-chain.
    pub async fn deposit(
        &self,
        value: u64,
        app_tag: u32,
        owner: &Fr,
    ) -> R14Result<DepositResult> {
        self.require_transfer_contract()?;

        let mut rng = crate::wallet::crypto_rng();
        let note = Note::new(value, app_tag, *owner, &mut rng);
        let cm = commitment(&note);

        let cm_hex = Self::fr_to_raw_hex(&cm);
        let new_root = crate::merkle::compute_new_root(&self.indexer_url, &[cm])
            .await
            .map_err(R14Error::Other)?;

        let tx_result = self
            .invoke(
                &self.contracts.transfer,
                "deposit",
                &[("cm", &cm_hex), ("new_root", &new_root)],
            )
            .await?;

        let note_entry = NoteEntry {
            value: note.value,
            app_tag: note.app_tag,
            owner: crate::wallet::fr_to_hex(&note.owner),
            nonce: crate::wallet::fr_to_hex(&note.nonce),
            commitment: crate::wallet::fr_to_hex(&cm),
            index: None,
            spent: false,
        };

        Ok(DepositResult {
            commitment: crate::wallet::fr_to_hex(&cm),
            value,
            app_tag,
            tx_result,
            note_entry,
        })
    }

    /// Sync note on-chain indices from the indexer.
    pub async fn sync_notes(&self, notes: &mut [NoteEntry]) -> R14Result<()> {
        for note in notes.iter_mut().filter(|n| !n.spent && n.index.is_none()) {
            if let Some(idx) = self.fetch_leaf_index(&note.commitment).await? {
                note.index = Some(idx);
            }
        }
        Ok(())
    }

    /// Sync notes and return balance summary.
    pub async fn balance(&self, notes: &mut [NoteEntry]) -> R14Result<BalanceResult> {
        self.sync_notes(notes).await?;

        let mut total = 0u64;
        let mut statuses = Vec::new();
        for note in notes.iter().filter(|n| !n.spent) {
            total += note.value;
            statuses.push(NoteStatus {
                value: note.value,
                app_tag: note.app_tag,
                commitment: note.commitment.clone(),
                on_chain: note.index.is_some(),
            });
        }

        Ok(BalanceResult {
            total,
            notes: statuses,
        })
    }

    /// Submit a pre-built proof on-chain (no ZK generation needed).
    pub async fn transfer_with_proof(
        &self,
        proof: &PrebuiltProof,
        recipient_note: NoteEntry,
        change_note: NoteEntry,
        consumed_idx: usize,
    ) -> R14Result<TransferResult> {
        self.require_transfer_contract()?;

        let cm_0_fr =
            crate::wallet::hex_to_fr(&recipient_note.commitment).map_err(R14Error::Other)?;
        let cm_1_fr =
            crate::wallet::hex_to_fr(&change_note.commitment).map_err(R14Error::Other)?;

        let new_root =
            crate::merkle::compute_new_root(&self.indexer_url, &[cm_0_fr, cm_1_fr])
                .await
                .map_err(R14Error::Other)?;

        let tx_result = self
            .invoke(
                &self.contracts.transfer,
                "transfer",
                &[
                    ("proof", &proof.proof_json),
                    ("old_root", &proof.old_root),
                    ("nullifier", &proof.nullifier),
                    ("cm_0", &proof.cm_0),
                    ("cm_1", &proof.cm_1),
                    ("new_root", &new_root),
                ],
            )
            .await?;

        Ok(TransferResult {
            nullifier: format!("0x{}", &proof.nullifier),
            out_commitment_0: recipient_note.commitment.clone(),
            out_commitment_1: change_note.commitment.clone(),
            tx_result,
            recipient_note,
            change_note,
            consumed_note_index: consumed_idx,
        })
    }

    // -----------------------------------------------------------------------
    // Public API — prove-gated
    // -----------------------------------------------------------------------

    /// Auto-select note, generate proof, submit transfer on-chain.
    #[cfg(feature = "prove")]
    pub async fn transfer(
        &self,
        notes: &mut [NoteEntry],
        sk: &Fr,
        owner: &Fr,
        recipient: &Fr,
        value: u64,
    ) -> R14Result<TransferResult> {
        use ark_std::rand::{rngs::StdRng, SeedableRng};

        self.require_transfer_contract()?;

        // find first unspent on-chain note with sufficient value
        let note_idx = notes
            .iter()
            .position(|n| !n.spent && n.value >= value && n.index.is_some())
            .ok_or_else(|| {
                let best = notes
                    .iter()
                    .filter(|n| !n.spent && n.index.is_some())
                    .map(|n| n.value)
                    .max()
                    .unwrap_or(0);
                R14Error::InsufficientBalance { needed: value, best }
            })?;

        let entry = &notes[note_idx];
        let consumed = Note::with_nonce(
            entry.value,
            entry.app_tag,
            crate::wallet::hex_to_fr(&entry.owner).map_err(R14Error::Other)?,
            crate::wallet::hex_to_fr(&entry.nonce).map_err(R14Error::Other)?,
        );
        let leaf_index = entry.index.ok_or(R14Error::NoteNotOnChain)?;
        let app_tag = entry.app_tag;
        let consumed_value = entry.value;

        // fetch merkle proof
        let (siblings, indices) = self.fetch_merkle_proof(leaf_index).await?;
        let merkle_path = crate::MerklePath { siblings, indices };

        // build output notes
        let mut rng = crate::wallet::crypto_rng();
        let change = consumed_value - value;
        let note_0 = Note::new(value, app_tag, *recipient, &mut rng);
        let note_1 = Note::new(change, app_tag, *owner, &mut rng);

        // Deterministic setup — same seed=42 reproduces VK matching on-chain
        let setup_rng = &mut StdRng::seed_from_u64(42);
        let (pk, _vk) = crate::prove::setup(setup_rng);
        let (proof, pi) = crate::prove::prove(
            &pk,
            *sk,
            consumed,
            merkle_path,
            [note_0.clone(), note_1.clone()],
            &mut rng,
        );

        let (serialized_proof, serialized_pi) =
            crate::prove::serialize_proof_for_soroban(&proof, &pi.to_vec());

        let cm_0 = commitment(&note_0);
        let cm_1 = commitment(&note_1);

        let proof_json = format!(
            r#"{{"a":"{}","b":"{}","c":"{}"}}"#,
            serialized_proof.a, serialized_proof.b, serialized_proof.c
        );

        let prebuilt = PrebuiltProof {
            proof_json,
            old_root: crate::wallet::strip_0x(&serialized_pi[0]),
            nullifier: crate::wallet::strip_0x(&serialized_pi[1]),
            cm_0: crate::wallet::strip_0x(&serialized_pi[2]),
            cm_1: crate::wallet::strip_0x(&serialized_pi[3]),
        };

        let recipient_entry = NoteEntry {
            value: note_0.value,
            app_tag: note_0.app_tag,
            owner: crate::wallet::fr_to_hex(&note_0.owner),
            nonce: crate::wallet::fr_to_hex(&note_0.nonce),
            commitment: crate::wallet::fr_to_hex(&cm_0),
            index: None,
            spent: false,
        };

        let change_entry = NoteEntry {
            value: note_1.value,
            app_tag: note_1.app_tag,
            owner: crate::wallet::fr_to_hex(&note_1.owner),
            nonce: crate::wallet::fr_to_hex(&note_1.nonce),
            commitment: crate::wallet::fr_to_hex(&cm_1),
            index: None,
            spent: false,
        };

        let result = self
            .transfer_with_proof(&prebuilt, recipient_entry, change_entry, note_idx)
            .await?;

        // mark consumed note spent
        notes[note_idx].spent = true;

        Ok(result)
    }

    /// Register VK on core contract and initialize transfer contract.
    #[cfg(feature = "prove")]
    pub async fn init_contracts(&self) -> R14Result<InitResult> {
        use ark_std::rand::{rngs::StdRng, SeedableRng};

        self.require_contracts()?;

        let mut rng = StdRng::seed_from_u64(42);
        let (_pk, vk) = crate::prove::setup(&mut rng);
        let svk = crate::prove::serialize_vk_for_soroban(&vk);

        let ic_entries: Vec<String> = svk.ic.iter().map(|s| format!("\"{}\"", s)).collect();
        let vk_json = format!(
            r#"{{"alpha_g1":"{}","beta_g2":"{}","gamma_g2":"{}","delta_g2":"{}","ic":[{}]}}"#,
            svk.alpha_g1, svk.beta_g2, svk.gamma_g2, svk.delta_g2, ic_entries.join(",")
        );

        let caller = crate::soroban::get_public_key(&self.stellar_secret)
            .await
            .map_err(|e| R14Error::Soroban(e.to_string()))?;

        let circuit_id = self
            .invoke(
                &self.contracts.core,
                "register",
                &[("caller", &caller), ("vk", &vk_json)],
            )
            .await?;

        let empty_root = crate::merkle::empty_root_hex();

        let tx_result = self
            .invoke(
                &self.contracts.transfer,
                "init",
                &[
                    ("core_contract", &self.contracts.core),
                    ("circuit_id", &circuit_id),
                    ("empty_root", &empty_root),
                ],
            )
            .await?;

        Ok(InitResult {
            circuit_id,
            tx_result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::WalletData;

    #[test]
    fn from_wallet_accepts_placeholder() {
        let wallet = WalletData {
            secret_key: "0x01".to_string(),
            owner_hash: "0x02".to_string(),
            stellar_secret: "PLACEHOLDER".to_string(),
            notes: vec![],
            indexer_url: "http://localhost:3000".to_string(),
            rpc_url: "https://soroban-testnet.stellar.org:443".to_string(),
            core_contract_id: "PLACEHOLDER".to_string(),
            transfer_contract_id: "PLACEHOLDER".to_string(),
        };
        let client = R14Client::from_wallet(&wallet);
        assert!(client.is_ok());
    }

    #[test]
    fn require_contracts_rejects_placeholder() {
        let client = R14Client::new(
            "http://localhost:3000",
            R14Contracts {
                core: "PLACEHOLDER".to_string(),
                transfer: "PLACEHOLDER".to_string(),
            },
            "S_SECRET",
            "testnet",
        )
        .unwrap();
        assert!(client.require_contracts().is_err());
    }

    #[test]
    fn require_transfer_rejects_placeholder() {
        let client = R14Client::new(
            "http://localhost:3000",
            R14Contracts {
                core: "C_CORE".to_string(),
                transfer: "PLACEHOLDER".to_string(),
            },
            "S_SECRET",
            "testnet",
        )
        .unwrap();
        assert!(client.require_transfer_contract().is_err());
    }

    #[test]
    fn require_contracts_accepts_real_ids() {
        let client = R14Client::new(
            "http://localhost:3000",
            R14Contracts {
                core: "C_CORE_REAL".to_string(),
                transfer: "C_XFER_REAL".to_string(),
            },
            "S_SECRET",
            "testnet",
        )
        .unwrap();
        assert!(client.require_contracts().is_ok());
        assert!(client.require_transfer_contract().is_ok());
    }

    #[test]
    fn fr_to_raw_hex_no_prefix() {
        let fr = Fr::from(42u64);
        let hex = R14Client::fr_to_raw_hex(&fr);
        assert!(!hex.starts_with("0x"));
        assert_eq!(hex.len(), 64);
    }

    #[test]
    fn balance_result_empty() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let client = R14Client::new(
                "http://localhost:3000",
                R14Contracts {
                    core: "PLACEHOLDER".to_string(),
                    transfer: "PLACEHOLDER".to_string(),
                },
                "S_SECRET",
                "testnet",
            )
            .unwrap();
            // balance with no notes should work even without indexer
            let mut notes: Vec<NoteEntry> = vec![];
            let result = client.balance(&mut notes).await.unwrap();
            assert_eq!(result.total, 0);
            assert!(result.notes.is_empty());
        });
    }
}
