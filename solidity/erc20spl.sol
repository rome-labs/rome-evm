// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "./interface.sol";

contract SPL_ERC20 is ERC20 {

    bytes32 mint_id;

    constructor(bytes32 _mint_id, string memory name, string memory symbol) ERC20(name, symbol) {
        ASplProgram.create_associated_token_account(address(this), _mint_id);
        mint_id = _mint_id;
    }

    function mint(uint256 amount) public {
        _mint(msg.sender, amount);
        SplProgram.balance_ge(address(this), mint_id, totalSupply());
    }

    function withdraw(bytes32 to, uint256 amount) public {
        _burn(msg.sender, amount);
        SplProgram.transfer(to, mint_id, amount);
    }

    function decimals() override public pure  returns (uint8) {
        return 9;
    }

    function mint_to(address target_wallet, uint256 amount) public {
        mint(amount);
        transfer(target_wallet, amount);
    }
}

