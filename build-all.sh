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

# Determine all contracts under `$CONTRACTS_DIR`
contracts=($(find $CONTRACTS_DIR -maxdepth 1 -type d -exec test -f {}/Cargo.toml \; -print | xargs -n 1 basename))
echo $contracts
# Build all contracts
for i in "${contracts[@]}"
do
  echo -e "\nBuilding '$CONTRACTS_DIR/$i/Cargo.toml'…"
  cargo +stable-2023-12-28 contract build --release --quiet --manifest-path $CONTRACTS_DIR/$i/Cargo.toml

  echo "Copying build files to '$DEPLOYMENTS_DIR/$CHAIN/$i'…"
  mkdir -p $DEPLOYMENTS_DIR/$CHAIN/$i
  $CP_CMD ./target/ink/$i/$i.contract $DEPLOYMENTS_DIR/$CHAIN/$i/
  $CP_CMD ./target/ink/$i/$i.wasm $DEPLOYMENTS_DIR/$CHAIN/$i/
  $CP_CMD ./target/ink/$i/$i.json $DEPLOYMENTS_DIR/$CHAIN/$i/
done