#!/usr/bin/env bash
set -eu

# ENVIRONMENT VARIABLES
CONTRACTS_DIR="./src" # Base contract directory
DEPLOYMENTS_DIR="./deployments" # Output directory for build files

# Copy command helper (cross-platform)
CP_CMD=$(command -v cp &> /dev/null && echo "cp" || echo "copy")

# Determine all contracts under `$CONTRACTS_DIR`
contracts=$1
# Build all contracts
for i in "${contracts[@]}"
do
  echo -e "\nBuilding '$CONTRACTS_DIR/$i/Cargo.toml'…"
  cargo  contract build --release --quiet --manifest-path $CONTRACTS_DIR/$i/Cargo.toml

  echo "Copying build files to '$DEPLOYMENTS_DIR/development/$i'…"
  mkdir -p $DEPLOYMENTS_DIR/development/$i
  $CP_CMD ./target/ink/$i/$i.contract $DEPLOYMENTS_DIR/development/$i/
  $CP_CMD ./target/ink/$i/$i.wasm $DEPLOYMENTS_DIR/development/$i/
  $CP_CMD ./target/ink/$i/$i.json $DEPLOYMENTS_DIR/development/$i/
done
