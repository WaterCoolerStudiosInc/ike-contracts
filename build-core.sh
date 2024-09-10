#!/usr/bin/env bash
set -eu

# ENVIRONMENT VARIABLES
CONTRACTS_DIR="./src" # Base contract directory
DEPLOYMENTS_DIR="./deployments" # Output directory for build files

# Default CHAIN to "development" if not set
: "${CHAIN:=development}"
echo "Using CHAIN: $CHAIN"

# Copy command helper (cross-platform)
CP_CMD=$(command -v cp &> /dev/null && echo "cp" || echo "copy")

core_contracts=("mock_nominator" "nomination_agent" "registry" "share_token" "vault")

# Build core contracts
for i in "${core_contracts[@]}"
do
  echo -e "\nBuilding '$CONTRACTS_DIR/$i/Cargo.toml'…"
  cargo +stable-2023-12-28 contract build --release --quiet --manifest-path $CONTRACTS_DIR/$i/Cargo.toml

  echo "Copying build files to '$DEPLOYMENTS_DIR/$CHAIN/$i'…"
  mkdir -p $DEPLOYMENTS_DIR/$CHAIN/$i
  $CP_CMD ./target/ink/$i/$i.contract $DEPLOYMENTS_DIR/$CHAIN/$i/
  $CP_CMD ./target/ink/$i/$i.wasm $DEPLOYMENTS_DIR/$CHAIN/$i/
  $CP_CMD ./target/ink/$i/$i.json $DEPLOYMENTS_DIR/$CHAIN/$i/
done