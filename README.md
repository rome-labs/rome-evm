# rome-evm

build contract:
- cd program
- cargo build-sbf

deploy contract:
- solana program deploy --program-id upgradeable-rome-keypair.json --upgrade-authority upgrade-authority-keypair.json ../target/deploy/rome_evm.so 

extend program account data:
- solana program extend CaQC27sVhdPyZF7defivoTQ48E8ws4tXvJfXYPRXboaH 5000

where CaQC27sVhdPyZF7defivoTQ48E8ws4tXvJfXYPRXboaH - is Pubkey of the keypair upgradeable-rome-keypair.json


