// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import "./wassociated_spl_token.sol";
import "./wsystem_program.sol";
import "./wspl_token.sol";
import "./shared.sol";

contract SPL_ERC20 is ERC20, Shared {

    bytes32 _spl_pool;
    bytes32 _mint32;

    constructor(string memory mint_b58, string memory symbol) ERC20(mint_b58, symbol) {
        _mint32 = SystemProgram.base58_to_bytes32(bytes(mint_b58));
        (bytes32 pda_,) = pda(address(this)); 
        _spl_pool = AssociatedSplToken.create_associated_token_account(pda_, _mint32);

    }

    function deposit(address user, uint256 amount) public {
        _mint(user, amount);
        require(totalSupply() == total_supply_spl(), "Inconsistency SPL and ERC-20 balances");
    }

    function withdraw(bytes32 spl, uint256 amount) public {
        _burn(msg.sender, amount);
        transfer(_spl_pool, spl, toUint64(amount));
        require(totalSupply() == total_supply_spl(), "Inconsistency SPL and ERC-20 balances");
    }

    function decimals() override public pure  returns (uint8) {
        return 9;
    }

    function spl_mint() public view returns (bytes32) {
        return _mint32;
    }

    function spl_pool() public view returns (bytes32) {
        return _spl_pool;
    }

    function pda(address user) public view returns (bytes32, uint8) {
        bytes32 key = SystemProgram.rome_evm_program_id();
        ISystemProgram.Seed[] memory seeds = Shared.balance_key_seeds(user, block.chainid);

        return SystemProgram.find_program_address(key, seeds);
    }
    
    function total_supply_spl() public view returns(uint256) {
        ISplToken.Account memory acc = SplToken.account_state(_spl_pool);

        return uint256(acc.amount);
    } 

    function transfer(bytes32 from, bytes32 to, uint64 amount) public {
        ISplToken.Seed[] memory seeds = new ISplToken.Seed[](0);
        (bool success, bytes memory result) = spl_token_address.call(
            abi.encodeWithSignature("transfer(bytes32,bytes32,uint64,(bytes)[])", from, to, amount, seeds)
        );

        require (success, string(Shared.revert_msg(result)));
    }        
}


contract SplHolder is Ownable, Shared {
    address waspl;
    address wsys;
    address wspl;
    mapping(bytes32 mint => bytes32) private spl_accounts;

    event Message(string account);


    constructor(address aspl, address sys, address spl) Ownable(msg.sender) {
        waspl = aspl;
        wsys = sys;
        wspl = spl;
    }

    function create_spl_account(string memory mint_b58 ) public onlyOwner {
        bytes32 mint32 = SystemProgram.base58_to_bytes32(bytes(mint_b58));

        WSystemProgram wsystem = WSystemProgram(wsys);  
        WAssociatedSplToken waspl_token = WAssociatedSplToken(waspl);

        bytes32 pda = wsystem.pda();
        spl_accounts[mint32] = waspl_token.create_associated_token_account(pda, mint32);

        bytes memory b58 = SystemProgram.bytes32_to_base58(spl_accounts[mint32]);

        emit Message(string(b58));
    }

    function spl_account(bytes32 mint32) internal view returns (bytes32) {
        return spl_accounts[mint32];
    }

    function deposit(address erc20, uint64 amount) public onlyOwner {
        SPL_ERC20 erc20_ = SPL_ERC20(erc20);

        transfer(spl_accounts[erc20_.spl_mint()], erc20_.spl_pool(), amount);
        erc20_.deposit(msg.sender, uint256(amount));
    }

    function transfer(bytes32 from, bytes32 to, uint64 amount) onlyOwner internal {
        ISplToken.Seed[] memory seeds = new ISplToken.Seed[](0);
        (bool success, bytes memory result) = spl_token_address.call(
            abi.encodeWithSignature("transfer(bytes32,bytes32,uint64,(bytes)[])", from, to, amount, seeds)
        );

        require (success, string(Shared.revert_msg(result)));
    }
}
