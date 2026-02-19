#!/bin/bash
# Phase 0 Deployment Script - Groth16 Feasibility Spike

set -e

echo "=== Phase 0: Deploying Groth16 Verifier to Testnet ==="

# Build contract
echo "Building contract..."
cargo build --target wasm32-unknown-unknown --release --package r14-kernel

WASM_PATH="target/wasm32-unknown-unknown/release/r14_kernel.wasm"
echo "WASM built: $(ls -lh $WASM_PATH | awk '{print $5}')"

# Deploy
echo ""
echo "Deploying to testnet..."
echo "NOTE: Set STELLAR_ACCOUNT environment variable or pass --source flag"
echo ""

# Uncomment and configure with your account:
# CONTRACT_ID=$(stellar contract deploy \
#   --wasm $WASM_PATH \
#   --network testnet \
#   --source <YOUR_ACCOUNT> \
#   2>&1 | grep -oE 'C[A-Z0-9]{55}')

# echo "Contract deployed: $CONTRACT_ID"
# echo ""

# Test invocation
# echo "Invoking verify_dummy_proof..."
# stellar contract invoke \
#   --id $CONTRACT_ID \
#   --network testnet \
#   -- verify_dummy_proof

echo "To deploy manually:"
echo ""
echo "1. Deploy:"
echo "   stellar contract deploy --wasm $WASM_PATH --network testnet --source <ACCOUNT>"
echo ""
echo "2. Invoke:"
echo "   stellar contract invoke --id <CONTRACT_ID> --network testnet -- verify_dummy_proof"
echo ""
echo "3. Check transaction for instruction count in metadata"
echo ""
echo "Decision criteria:"
echo "  < 80M instructions  → GO (proceed to Phase 1)"
echo "  80-100M             → CAUTION (proceed, flag optimization)"
echo "  > 100M              → NO-GO (implement Poseidon compression)"
