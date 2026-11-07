use crate::rpc_retry::RetryableRpcClient;
use anyhow::{Context, Result};
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiInstruction,
    UiTransactionStatusMeta,
};
use std::sync::Arc;

/// Fetches and parses transactions from Solana RPC. Just I/O, no business logic.
pub struct TransactionFetcher {
    rpc_client: Arc<RetryableRpcClient>,
}

impl TransactionFetcher {
    pub fn new(rpc_client: Arc<RetryableRpcClient>) -> Self {
        Self { rpc_client }
    }

    pub async fn fetch(
        &self,
        signature: Signature,
        pool_address: &str,
    ) -> Result<Option<ParsedTransaction>> {
        let tx = self
            .rpc_client
            .get_transaction(signature)
            .await
            .context("Failed to fetch transaction")?;

        Ok(parse_transaction(&tx, pool_address))
    }
}

/// The parts of a transaction worth processing.
pub struct ParsedTransaction {
    pub slot: u64,
    pub block_time: i64,
    pub meta: UiTransactionStatusMeta,
    pub account_keys: Vec<String>,
    pub instructions: Vec<UiInstruction>,
}

/// Extracts the usable parts of a transaction, or `None` when the transaction is
/// irrelevant (failed, unparseable, or not touching this pool).
fn parse_transaction(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
    pool_address: &str,
) -> Option<ParsedTransaction> {
    let meta = tx.transaction.meta.as_ref()?;
    if meta.err.is_some() {
        return None;
    }

    let EncodedTransaction::Json(transaction) = &tx.transaction.transaction else {
        return None;
    };

    let account_keys = extract_account_keys(&transaction.message);
    if !account_keys.iter().any(|key| key == pool_address) {
        return None;
    }

    let block_time = tx.block_time?;
    let instructions = flatten_instructions(&transaction.message, meta);

    Some(ParsedTransaction {
        slot: tx.slot,
        block_time,
        meta: meta.clone(),
        account_keys,
        instructions,
    })
}

fn extract_account_keys(message: &solana_transaction_status::UiMessage) -> Vec<String> {
    match message {
        solana_transaction_status::UiMessage::Parsed(parsed) => parsed
            .account_keys
            .iter()
            .map(|k| k.pubkey.clone())
            .collect(),
        solana_transaction_status::UiMessage::Raw(raw) => raw.account_keys.clone(),
    }
}

fn flatten_instructions(
    message: &solana_transaction_status::UiMessage,
    meta: &UiTransactionStatusMeta,
) -> Vec<UiInstruction> {
    let top_level: Vec<UiInstruction> = match message {
        solana_transaction_status::UiMessage::Parsed(parsed) => parsed.instructions.clone(),
        solana_transaction_status::UiMessage::Raw(raw) => raw
            .instructions
            .iter()
            .cloned()
            .map(UiInstruction::Compiled)
            .collect(),
    };

    let inner_groups: &[solana_transaction_status::UiInnerInstructions] =
        match &meta.inner_instructions {
            solana_transaction_status::option_serializer::OptionSerializer::Some(groups) => {
                groups.as_slice()
            }
            _ => &[],
        };

    let mut instructions = Vec::with_capacity(top_level.len());
    for (index, instruction) in top_level.into_iter().enumerate() {
        instructions.push(instruction);
        for group in inner_groups.iter().filter(|g| g.index as usize == index) {
            instructions.extend(group.instructions.iter().cloned());
        }
    }

    instructions
}
