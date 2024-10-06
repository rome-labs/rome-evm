#!/bin/bash

if [ -z "${CHAIN_ID}" ]; then
  echo "CHAIN_ID is not defined"
  exit 1
fi;

if [ -z "${CONTRACT_OWNER_KEYPAIR}" ]; then
  echo "CONTRACT_OWNER_KEYPAIR is not defined"
  exit 1
fi;

CONTRACT_OWNER=$(/usr/bin/solana address -k "${CONTRACT_OWNER_KEYPAIR}")
export CONTRACT_OWNER

echo "Building Rome-EVM with CHAIN_ID ${CHAIN_ID} and owner ${CONTRACT_OWNER}..."
sleep 2

cd /opt/rome-evm-private/program
if ! cargo build-sbf; then
  echo "Failed to build Rome-EVM"
  exit 1
fi

echo "Rome-EVM successfully built"
