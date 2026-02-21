# wallet

`r14_sdk::wallet` — Key/note persistence and hex-Fr conversion.

## Types

### `WalletData`

```rust
pub struct WalletData {
    pub secret_key: String,            // hex-encoded Fr
    pub owner_hash: String,            // hex-encoded Fr
    pub stellar_secret: String,        // Stellar secret key (S...)
    pub notes: Vec<NoteEntry>,         // all notes (spent + unspent)
    pub indexer_url: String,           // e.g. "http://localhost:3000"
    pub rpc_url: String,               // Soroban RPC endpoint
    pub core_contract_id: String,      // r14-core contract ID (C...)
    pub transfer_contract_id: String,  // r14-transfer contract ID (C...)
}
```

### `NoteEntry`

Serializable note record. Stores hex strings rather than `Fr` values.

```rust
pub struct NoteEntry {
    pub value: u64,
    pub app_tag: u32,
    pub owner: String,       // hex
    pub nonce: String,        // hex
    pub commitment: String,   // hex
    pub index: Option<u64>,   // on-chain leaf index, None if local-only
    pub spent: bool,
}
```

## Functions

### `wallet_path() -> Result<PathBuf>`

Returns `~/.r14/wallet.json`. Creates parent directories on save.

### `load_wallet() -> Result<WalletData>`

Deserialize wallet from `wallet_path()`. Fails if file doesn't exist.

### `save_wallet(wallet: &WalletData) -> Result<()>`

Serialize wallet as pretty-printed JSON. Creates `~/.r14/` if needed.

### `fr_to_hex(fr: &Fr) -> String`

Convert a field element to `0x`-prefixed big-endian hex (66 chars total).

```rust
let hex = fr_to_hex(&Fr::from(42u64));
// "0x000000000000000000000000000000000000000000000000000000000000002a"
```

### `hex_to_fr(s: &str) -> Result<Fr>`

Parse hex string to field element. Accepts with or without `0x` prefix. Zero-pads short inputs to 32 bytes.

```rust
let fr = hex_to_fr("0x2a")?;        // works
let fr = hex_to_fr("2a")?;          // also works
let fr = hex_to_fr("0x00...2a")?;   // also works
```

### `crypto_rng() -> StdRng`

Time-seeded RNG. Suitable for note creation and key generation.

```rust
let mut rng = crypto_rng();
let sk = SecretKey::random(&mut rng);
```
