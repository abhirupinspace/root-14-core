# Hex Conventions

Different modules use different hex formats. This page clarifies the conventions.

## Format table

| Context | Prefix | Length | Example |
|---------|--------|--------|---------|
| `wallet::fr_to_hex` | `0x` | 66 chars | `0x00ab...ef` |
| `wallet::hex_to_fr` (input) | optional `0x` | any | `0xab`, `ab`, `00ab...ef` |
| `merkle` module (output) | none | 64 chars | `00ab...ef` |
| `serialize::serialize_fr` | none | 64 chars | `00ab...ef` |
| `serialize::serialize_g1` | none | 192 chars | `aabb...` |
| `serialize::serialize_g2` | none | 384 chars | `aabb...` |
| Soroban contract args | none | varies | strip `0x` before passing |

## Converting between formats

```rust
use r14_sdk::wallet::fr_to_hex;

let hex_with_prefix = fr_to_hex(&fr);        // "0x00ab...ef"
let hex_raw = &hex_with_prefix[2..];          // "00ab...ef" — for Soroban
let hex_raw = hex_with_prefix.strip_prefix("0x").unwrap_or(&hex_with_prefix);
```

## Why two formats?

- **`0x` prefix** (wallet): human-readable, matches Ethereum convention, easy to identify as hex in JSON
- **No prefix** (merkle, serialize, Soroban): matches Soroban's `BytesN<N>::from_hex` expectation and avoids parsing overhead
