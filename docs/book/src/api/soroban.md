# soroban

`r14_sdk::soroban` — Stellar CLI wrapper for on-chain contract invocation.

Requires the [Stellar CLI](https://github.com/stellar/stellar-cli) installed and on `$PATH`.

## Functions

### `get_public_key(secret: &str) -> Result<String>` *(async)*

Derive the Stellar public key (`G...`) from a secret key (`S...`).

Shells out to `stellar keys address <secret>`.

```rust
let pubkey = r14_sdk::soroban::get_public_key("S_SECRET...").await?;
// "G..."
```

### `invoke_contract(contract_id, network, source_secret, function, args) -> Result<String>` *(async)*

Invoke a Soroban contract function.

```rust
let result = r14_sdk::soroban::invoke_contract(
    "C_CONTRACT_ID",  // contract address
    "testnet",        // network name
    "S_SECRET...",    // source account secret
    "deposit",        // function name
    &[                // named arguments
        ("cm", "deadbeef..."),
        ("new_root", "cafebabe..."),
    ],
).await?;
```

Translates to:

```bash
stellar contract invoke \
  --id C_CONTRACT_ID \
  --network testnet \
  --source S_SECRET... \
  -- deposit \
  --cm deadbeef... \
  --new_root cafebabe...
```

Returns the stdout output on success, or an error with stderr on failure.

## Error handling

Both functions return `anyhow::Result`. Common failure cases:

- `stellar` CLI not installed or not on `$PATH`
- Invalid secret key
- Contract invocation failure (insufficient funds, invalid proof, etc.)
- Network issues
