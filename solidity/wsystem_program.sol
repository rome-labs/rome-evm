// SPDX-License-Identifier: MIT
pragma solidity =0.8.28;

import "./interface.sol";
import "./shared.sol";

contract WSystemProgram is Shared{
    event Message(string account);

    function program_id() public view returns (string memory) {
        bytes32 key = SystemProgram.program_id();
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);
        return string(b58);
    }

    function rome_evm_program_id() public view returns (string memory) {
        bytes32 key = SystemProgram.rome_evm_program_id();
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);
        return string(b58);
    }

    function pda(address user) public view returns (bytes32, uint8) {
        bytes32 key = SystemProgram.rome_evm_program_id();
        ISystemProgram.Seed[] memory seeds = Shared.balance_key_seeds(user, block.chainid);

        return SystemProgram.find_program_address(key, seeds);
    }

    function pda() public view returns (bytes32) {
        (bytes32 key, ) = pda(msg.sender);
        return key;
    }
    
    function pda_base58(address user) public view returns (string memory, uint8) {
        (bytes32 key, uint8 bump) = pda(user);
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);
        return (string(b58), bump);
    }

    function pda_base58() public view returns (string memory) {
        (string memory key, ) = pda_base58(msg.sender);
        return key;
    }

    function create_account(bytes32 owner, uint64 len, address user, bytes32 salt) public {
        (bool success, bytes memory result) = system_program_address.delegatecall(
            abi.encodeWithSignature("create_account(bytes32,uint64,address,bytes32)", owner, len, user, salt)
        );

        require (success, string(Shared.revert_msg(result)));

        bytes32 key = Shared.bytes_to_bytes32(result);
        bytes memory b58 = SystemProgram.bytes32_to_base58(key);
        
        emit Message(string(b58));
    }

    function create_account(string memory owner, uint64 len, address user, bytes32 salt) public {
        bytes32 owner_ = SystemProgram.base58_to_bytes32(bytes(owner)); 
        create_account(owner_, len, user, salt);    
    }

    function create_account(string memory owner, uint64 len, address user) public {
        bytes32 owner_ = SystemProgram.base58_to_bytes32(bytes(owner)); 
        bytes32 salt = bytes32(uint256(0));

        create_account(owner_, len, user, salt);    
    }

    function create_account(string memory owner, uint64 len) public {
        bytes32 owner_ = SystemProgram.base58_to_bytes32(bytes(owner)); 
        bytes32 salt = bytes32(uint256(0));

        create_account(owner_, len, msg.sender, salt);    
    }

    function allocate(string memory src, uint64 len) public {
        bytes32 src_ = SystemProgram.base58_to_bytes32(bytes(src));
        SystemProgram.allocate(src_, len);
    }

    function assign(string memory src, string memory owner) public {
        bytes32 src_ = SystemProgram.base58_to_bytes32(bytes(src));
        bytes32 owner_ = SystemProgram.base58_to_bytes32(bytes(owner));
        SystemProgram.assign(src_, owner_);    
    }

    function transfer_from(bytes32 to, uint64 lamports, bytes32 salt) public {
        (bool success, bytes memory result) = system_program_address.delegatecall(
            abi.encodeWithSignature("transfer(bytes32,uint64,bytes32)", to, lamports, salt)
        );

        require (success, string(Shared.revert_msg(result)));
    }

    function transfer_from(string memory to, uint64 lamports, bytes32 salt) public {
        bytes32 to_ = SystemProgram.base58_to_bytes32(bytes(to));
        transfer_from(to_, lamports, salt);
    }

    function transfer(string memory to, uint64 lamports) public {
        bytes32 salt = bytes32(uint256(0));
        bytes32 to_ = SystemProgram.base58_to_bytes32(bytes(to));
        transfer_from(to_, lamports, salt);    
    }    
}

