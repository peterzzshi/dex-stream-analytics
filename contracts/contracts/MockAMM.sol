// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title MockAMM
 * @dev Simple Automated Market Maker for testing purposes
 * @notice This contract mimics Uniswap V2 pair behavior for local testing
 */
contract MockAMM {
    // State variables
    address public token0;
    address public token1;
    uint112 private reserve0;
    uint112 private reserve1;
    uint32 private blockTimestampLast;

    // Events matching Uniswap V2 Pair interface
    event Sync(uint112 reserve0, uint112 reserve1);
    
    event Swap(
        address indexed sender,
        uint256 amount0In,
        uint256 amount1In,
        uint256 amount0Out,
        uint256 amount1Out,
        address indexed to
    );

    event Mint(address indexed sender, uint256 amount0, uint256 amount1);
    
    event Burn(
        address indexed sender,
        uint256 amount0,
        uint256 amount1,
        address indexed to
    );

    /**
     * @dev Constructor sets the token pair
     * @param _token0 Address of first token
     * @param _token1 Address of second token
     */
    constructor(address _token0, address _token1) {
        require(_token0 != address(0) && _token1 != address(0), "Invalid token addresses");
        token0 = _token0;
        token1 = _token1;
    }

    /**
     * @notice Get current reserves
     * @return _reserve0 Reserve of token0
     * @return _reserve1 Reserve of token1
     * @return _blockTimestampLast Last block timestamp
     */
    function getReserves() 
        public 
        view 
        returns (uint112 _reserve0, uint112 _reserve1, uint32 _blockTimestampLast) 
    {
        _reserve0 = reserve0;
        _reserve1 = reserve1;
        _blockTimestampLast = blockTimestampLast;
    }

    /**
     * @notice Simulate a swap for testing
     * @param amount0In Amount of token0 going in
     * @param amount1In Amount of token1 going in
     * @param amount0Out Amount of token0 coming out
     * @param amount1Out Amount of token1 coming out
     * @param to Recipient address
     */
    function swap(
        uint256 amount0In,
        uint256 amount1In,
        uint256 amount0Out,
        uint256 amount1Out,
        address to
    ) external {
        require(to != address(0), "Invalid recipient");
        require(amount0In > 0 || amount1In > 0, "Insufficient input amount");
        require(amount0Out > 0 || amount1Out > 0, "Insufficient output amount");

        // Update reserves
        if (amount0In > 0) {
            reserve0 += uint112(amount0In);
        }
        if (amount1In > 0) {
            reserve1 += uint112(amount1In);
        }
        if (amount0Out > 0) {
            reserve0 -= uint112(amount0Out);
        }
        if (amount1Out > 0) {
            reserve1 -= uint112(amount1Out);
        }

        _update(reserve0, reserve1);

        emit Swap(msg.sender, amount0In, amount1In, amount0Out, amount1Out, to);
    }

    /**
     * @notice Simulate liquidity addition
     * @param amount0 Amount of token0 to add
     * @param amount1 Amount of token1 to add
     */
    function mint(uint256 amount0, uint256 amount1) external {
        require(amount0 > 0 && amount1 > 0, "Insufficient amounts");

        reserve0 += uint112(amount0);
        reserve1 += uint112(amount1);

        _update(reserve0, reserve1);

        emit Mint(msg.sender, amount0, amount1);
    }

    /**
     * @notice Simulate liquidity removal
     * @param amount0 Amount of token0 to remove
     * @param amount1 Amount of token1 to remove
     * @param to Recipient address
     */
    function burn(uint256 amount0, uint256 amount1, address to) external {
        require(to != address(0), "Invalid recipient");
        require(amount0 <= reserve0 && amount1 <= reserve1, "Insufficient liquidity");

        reserve0 -= uint112(amount0);
        reserve1 -= uint112(amount1);

        _update(reserve0, reserve1);

        emit Burn(msg.sender, amount0, amount1, to);
    }

    /**
     * @dev Internal function to update reserves and emit Sync event
     */
    function _update(uint112 _reserve0, uint112 _reserve1) private {
        blockTimestampLast = uint32(block.timestamp % 2**32);
        emit Sync(_reserve0, _reserve1);
    }

    /**
     * @notice Force reserves to specific values (for testing)
     * @param _reserve0 New reserve0 value
     * @param _reserve1 New reserve1 value
     */
    function setReserves(uint112 _reserve0, uint112 _reserve1) external {
        reserve0 = _reserve0;
        reserve1 = _reserve1;
        _update(_reserve0, _reserve1);
    }
}
