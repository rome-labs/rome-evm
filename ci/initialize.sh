#!/bin/bash

echo "Initializing Rome-EVM. Minting ${MINT_AMOUNT} to address ${MINT_TO}..."
if ! /usr/bin/initializer; then
        echo "Failed to initialize Rome-EVM"
        exit 1
fi

echo "Rome-EVM successfully initialized"
