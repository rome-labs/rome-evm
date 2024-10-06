#!/bin/bash

if [ -z "${SOLANA_RPC}" ]; then
  echo "SOLANA_RPC is not defined"
  exit 1
fi;

if [ -z "${CONTRACT_OWNER_KEYPAIR}" ]; then
  echo "CONTRACT_OWNER_KEYPAIR is not defined"
  exit 1
fi;

if [ -z "${ROME_EVM_KEYPAIR}" ]; then
  echo "ROME_EVM_KEYPAIR is not defined"
  exit 1
fi;

CONTRACT_OWNER=$(/usr/bin/solana address -k "${CONTRACT_OWNER_KEYPAIR}")
export CONTRACT_OWNER

PROGRAM_ADDRESS=$(/usr/bin/solana address -k "${ROME_EVM_KEYPAIR}")
export PROGRAM_ADDRESS

/usr/bin/solana config set --url "${SOLANA_RPC}" --keypair "${CONTRACT_OWNER_KEYPAIR}"

echo "Deploying Rome-EVM to ${PROGRAM_ADDRESS} with upgrade authority ${CONTRACT_OWNER}..."
sleep 2

if ! /usr/bin/solana program deploy \
  --program-id "${ROME_EVM_KEYPAIR}" /opt/rome-evm-private/target/deploy/rome_evm.so \
  --upgrade-authority "${CONTRACT_OWNER_KEYPAIR}" \
  --fee-payer "${CONTRACT_OWNER_KEYPAIR}"; then
    echo "Failed to deploy Rome-EVM"
    exit 1
fi

echo "Rome-EVM successfully deployed"
