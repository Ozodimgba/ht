#!/bin/bash

KEYPAIR_PATH="target/program-keypair.json"
LIB_RS_PATH="src/lib.rs"

# Create target directory if it doesn't exist
mkdir -p target

# Generate new keypair if it doesn't exist
if [ ! -f "$KEYPAIR_PATH" ]; then
    solana-keygen new -o "$KEYPAIR_PATH" --no-bip39-passphrase
    echo "Created new program keypair at $KEYPAIR_PATH"
fi

# Get public key from keypair
PROGRAM_ID=$(solana-keygen pubkey "$KEYPAIR_PATH")
echo "Program ID: $PROGRAM_ID"

# Check if lib.rs contains default program ID and update it
if grep -q "declare_id!(\"11111111111111111111111111111111\")" "$LIB_RS_PATH"; then
    # Replace the default program ID with the actual one
    sed -i "s/declare_id!(\"11111111111111111111111111111111\")/declare_id!(\"$PROGRAM_ID\")/" "$LIB_RS_PATH"
    echo "Updated program ID in lib.rs"
fi

# Build the program
cargo build-sbf

# Deploy the program
# solana program deploy --program-id "$KEYPAIR_PATH" target/sbf-solana-solana/release/ht.so