package blockchain

import (
	"math/big"
	"testing"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
)

func TestEventTopicHashes(t *testing.T) {
	// Test that event topic hashes match expected Keccak256 hashes
	tests := []struct {
		name      string
		signature string
		expected  common.Hash
	}{
		{
			"Swap",
			"Swap(address,uint256,uint256,uint256,uint256,address)",
			crypto.Keccak256Hash([]byte("Swap(address,uint256,uint256,uint256,uint256,address)")),
		},
		{
			"Mint",
			"Mint(address,uint256,uint256)",
			crypto.Keccak256Hash([]byte("Mint(address,uint256,uint256)")),
		},
		{
			"Burn",
			"Burn(address,uint256,uint256,address)",
			crypto.Keccak256Hash([]byte("Burn(address,uint256,uint256,address)")),
		},
		{
			"Transfer",
			"Transfer(address,address,uint256)",
			crypto.Keccak256Hash([]byte("Transfer(address,address,uint256)")),
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var actual common.Hash
			switch tt.name {
			case "Swap":
				actual = SwapEventTopic
			case "Mint":
				actual = MintEventTopic
			case "Burn":
				actual = BurnEventTopic
			case "Transfer":
				actual = TransferEventTopic
			}

			if actual != tt.expected {
				t.Errorf("%s topic hash mismatch.\nExpected: %s\nGot: %s",
					tt.name, tt.expected.Hex(), actual.Hex())
			}
		})
	}
}

func TestParseSwapLog_ValidLog(t *testing.T) {
	// Create a valid swap log
	sender := common.HexToAddress("0x1111111111111111111111111111111111111111")
	recipient := common.HexToAddress("0x2222222222222222222222222222222222222222")

	logEntry := types.Log{
		Topics: []common.Hash{
			SwapEventTopic,
			common.BytesToHash(sender.Bytes()),
			common.BytesToHash(recipient.Bytes()),
		},
		Data: encodeSwapData(
			big.NewInt(100), // amount0In
			big.NewInt(0),   // amount1In
			big.NewInt(0),   // amount0Out
			big.NewInt(85),  // amount1Out
		),
	}

	parsedSender, parsedRecipient, amount0In, amount1In, amount0Out, amount1Out, err := parseSwapLog(logEntry)

	if err != nil {
		t.Fatalf("parseSwapLog failed: %v", err)
	}

	if parsedSender != sender {
		t.Errorf("Expected sender %s, got %s", sender.Hex(), parsedSender.Hex())
	}

	if parsedRecipient != recipient {
		t.Errorf("Expected recipient %s, got %s", recipient.Hex(), parsedRecipient.Hex())
	}

	if amount0In.Cmp(big.NewInt(100)) != 0 {
		t.Errorf("Expected amount0In 100, got %s", amount0In.String())
	}

	if amount1In.Cmp(big.NewInt(0)) != 0 {
		t.Errorf("Expected amount1In 0, got %s", amount1In.String())
	}

	if amount0Out.Cmp(big.NewInt(0)) != 0 {
		t.Errorf("Expected amount0Out 0, got %s", amount0Out.String())
	}

	if amount1Out.Cmp(big.NewInt(85)) != 0 {
		t.Errorf("Expected amount1Out 85, got %s", amount1Out.String())
	}
}

func TestParseSwapLog_MissingTopics(t *testing.T) {
	// Test with insufficient topics
	logEntry := types.Log{
		Topics: []common.Hash{SwapEventTopic}, // Missing sender and recipient
		Data:   []byte{},
	}

	_, _, _, _, _, _, err := parseSwapLog(logEntry)

	if err == nil {
		t.Error("Expected error for missing topics, got nil")
	}
}

func TestParseMintLog_ValidLog(t *testing.T) {
	sender := common.HexToAddress("0x3333333333333333333333333333333333333333")

	logEntry := types.Log{
		Topics: []common.Hash{
			MintEventTopic,
			common.BytesToHash(sender.Bytes()),
		},
		Data: encodeMintData(
			big.NewInt(1000),
			big.NewInt(850),
		),
	}

	parsedSender, amount0, amount1, err := parseMintLog(logEntry)

	if err != nil {
		t.Fatalf("parseMintLog failed: %v", err)
	}

	if parsedSender != sender {
		t.Errorf("Expected sender %s, got %s", sender.Hex(), parsedSender.Hex())
	}

	if amount0.Cmp(big.NewInt(1000)) != 0 {
		t.Errorf("Expected amount0 1000, got %s", amount0.String())
	}

	if amount1.Cmp(big.NewInt(850)) != 0 {
		t.Errorf("Expected amount1 850, got %s", amount1.String())
	}
}

func TestParseBurnLog_ValidLog(t *testing.T) {
	sender := common.HexToAddress("0x4444444444444444444444444444444444444444")
	recipient := common.HexToAddress("0x5555555555555555555555555555555555555555")

	logEntry := types.Log{
		Topics: []common.Hash{
			BurnEventTopic,
			common.BytesToHash(sender.Bytes()),
			common.BytesToHash(recipient.Bytes()),
		},
		Data: encodeBurnData(
			big.NewInt(500),
			big.NewInt(425),
		),
	}

	parsedSender, parsedRecipient, amount0, amount1, err := parseBurnLog(logEntry)

	if err != nil {
		t.Fatalf("parseBurnLog failed: %v", err)
	}

	if parsedSender != sender {
		t.Errorf("Expected sender %s, got %s", sender.Hex(), parsedSender.Hex())
	}

	if parsedRecipient != recipient {
		t.Errorf("Expected recipient %s, got %s", recipient.Hex(), parsedRecipient.Hex())
	}

	if amount0.Cmp(big.NewInt(500)) != 0 {
		t.Errorf("Expected amount0 500, got %s", amount0.String())
	}

	if amount1.Cmp(big.NewInt(425)) != 0 {
		t.Errorf("Expected amount1 425, got %s", amount1.String())
	}
}

func TestParseTransferLog_ValidLog(t *testing.T) {
	from := common.HexToAddress("0x6666666666666666666666666666666666666666")
	to := common.HexToAddress("0x7777777777777777777777777777777777777777")

	logEntry := types.Log{
		Topics: []common.Hash{
			TransferEventTopic,
			common.BytesToHash(from.Bytes()),
			common.BytesToHash(to.Bytes()),
		},
		Data: common.LeftPadBytes(big.NewInt(123456).Bytes(), 32),
	}

	parsedFrom, parsedTo, value, err := parseTransferLog(logEntry)
	if err != nil {
		t.Fatalf("parseTransferLog failed: %v", err)
	}

	if parsedFrom != from {
		t.Errorf("Expected from %s, got %s", from.Hex(), parsedFrom.Hex())
	}

	if parsedTo != to {
		t.Errorf("Expected to %s, got %s", to.Hex(), parsedTo.Hex())
	}

	if value.Cmp(big.NewInt(123456)) != 0 {
		t.Errorf("Expected value 123456, got %s", value.String())
	}
}

func TestReadBigInt_Success(t *testing.T) {
	value := big.NewInt(12345)
	values := map[string]any{
		"testKey": value,
	}

	result, err := readBigInt(values, "testKey")

	if err != nil {
		t.Fatalf("readBigInt failed: %v", err)
	}

	if result.Cmp(value) != 0 {
		t.Errorf("Expected %s, got %s", value.String(), result.String())
	}
}

func TestReadBigInt_NotFound(t *testing.T) {
	values := map[string]any{}

	_, err := readBigInt(values, "missingKey")

	if err == nil {
		t.Error("Expected error for missing key, got nil")
	}
}

func TestReadBigInt_WrongType(t *testing.T) {
	values := map[string]any{
		"testKey": "not a bigint",
	}

	_, err := readBigInt(values, "testKey")

	if err == nil {
		t.Error("Expected error for wrong type, got nil")
	}
}

// Helper functions to encode test data
func encodeSwapData(amount0In, amount1In, amount0Out, amount1Out *big.Int) []byte {
	// Pack the swap event data according to Uniswap V2 ABI
	data := make([]byte, 128) // 4 uint256 values = 128 bytes

	copy(data[0:32], common.LeftPadBytes(amount0In.Bytes(), 32))
	copy(data[32:64], common.LeftPadBytes(amount1In.Bytes(), 32))
	copy(data[64:96], common.LeftPadBytes(amount0Out.Bytes(), 32))
	copy(data[96:128], common.LeftPadBytes(amount1Out.Bytes(), 32))

	return data
}

func encodeMintData(amount0, amount1 *big.Int) []byte {
	data := make([]byte, 64) // 2 uint256 values = 64 bytes

	copy(data[0:32], common.LeftPadBytes(amount0.Bytes(), 32))
	copy(data[32:64], common.LeftPadBytes(amount1.Bytes(), 32))

	return data
}

func encodeBurnData(amount0, amount1 *big.Int) []byte {
	return encodeMintData(amount0, amount1) // Same structure
}
