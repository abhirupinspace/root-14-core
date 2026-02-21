# serialize

`r14_sdk::serialize` — Arkworks Groth16 types to hex for Soroban contracts.

## Types

### `SerializedVK`

```rust
pub struct SerializedVK {
    pub alpha_g1: String,   // 192-char hex (96 bytes uncompressed G1)
    pub beta_g2: String,    // 384-char hex (192 bytes uncompressed G2)
    pub gamma_g2: String,   // 384-char hex
    pub delta_g2: String,   // 384-char hex
    pub ic: Vec<String>,    // each 192-char hex; length = num_public_inputs + 1
}
```

### `SerializedProof`

```rust
pub struct SerializedProof {
    pub a: String,   // 192-char hex (G1)
    pub b: String,   // 384-char hex (G2)
    pub c: String,   // 192-char hex (G1)
}
```

## Functions

### `serialize_g1(point: &G1Affine) -> String`

Serialize a G1 point to 192-char uncompressed hex. Uses arkworks canonical LE form.

### `serialize_g2(point: &G2Affine) -> String`

Serialize a G2 point to 384-char uncompressed hex.

### `serialize_fr(fr: &Fr) -> String`

Serialize a field element to 64-char **big-endian** hex (no `0x` prefix).

> Arkworks serializes Fr as little-endian internally. This function reverses the bytes to match Soroban's `Fr::from_bytes` which expects big-endian.

### `serialize_vk_for_soroban(vk: &VerifyingKey<Bls12_381>) -> SerializedVK`

Convert a full verification key to hex-serialized form. Used during contract initialization.

```rust
let svk = r14_sdk::serialize::serialize_vk_for_soroban(&vk);
```

### `serialize_proof_for_soroban(proof, public_inputs) -> (SerializedProof, Vec<String>)`

Convert a proof and its public inputs to hex. Returns both the serialized proof and the hex-encoded public inputs.

```rust
let (sp, spi) = r14_sdk::serialize::serialize_proof_for_soroban(&proof, &pi_vec);
// sp.a, sp.b, sp.c — proof elements
// spi[0] = old_root, spi[1] = nullifier, spi[2] = cm_0, spi[3] = cm_1
```

## Byte order summary

| Type | Encoding | Size |
|------|----------|------|
| G1 | Uncompressed, arkworks canonical (LE) | 96 bytes = 192 hex chars |
| G2 | Uncompressed, arkworks canonical (LE) | 192 bytes = 384 hex chars |
| Fr | Big-endian (reversed from arkworks LE) | 32 bytes = 64 hex chars |
