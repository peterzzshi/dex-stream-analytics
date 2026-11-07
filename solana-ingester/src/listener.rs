use crate::config::PoolConfig;
use crate::instruction_processor::{InstructionProcessor, TransactionContext};
use crate::rpc_retry::RetryableRpcClient;
use anyhow::{Context, Result};
use dex_ingester_common::Event;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiInstruction,
    UiTransactionStatusMeta,
};
use std::sync::Arc;
use tracing::{debug, warn};

pub struct Listener {
    rpc_client: Arc<RetryableRpcClient>,
    processor: InstructionProcessor,
    pool: PoolConfig,
}

impl Listener {
    pub fn new(
        rpc_client: Arc<RetryableRpcClient>,
        processor: InstructionProcessor,
        pool: PoolConfig,
    ) -> Self {
        Self {
            rpc_client,
            processor,
            pool,
        }
    }

    pub async fn fetch_and_process_transaction(
        &self,
        signature: Signature,
        pool_address: &str,
    ) -> Result<Vec<Event>> {
        let tx = self
            .rpc_client
            .get_transaction(signature)
            .await
            .context("Failed to fetch transaction")?;

        let Some((slot, block_time, meta, account_keys, instructions)) =
            usable_transaction(&tx, pool_address)
        else {
            return Ok(Vec::new());
        };

        let tx_ctx = TransactionContext {
            account_keys: &account_keys,
            signature: &signature,
            slot,
            block_time,
            meta,
            pool_address,
        };

        let mut events = Vec::new();
        for (index, instruction) in instructions.iter().enumerate() {
            match self.processor.process(instruction, index, &tx_ctx).await {
                Ok(Some(event)) => events.push(event),
                Ok(None) => {}
                Err(e) => {
                    warn!(
                        "[{}] Failed to process instruction {} in {}: {:#}",
                        self.pool.name, index, signature, e
                    );
                }
            }
        }

        Ok(events)
    }
}

/// The parts of a transaction worth processing, or `None` when the transaction is
/// irrelevant (failed, unparseable, or not touching this pool).
fn usable_transaction<'a>(
    tx: &'a EncodedConfirmedTransactionWithStatusMeta,
    pool_address: &str,
) -> Option<(
    u64,
    i64,
    &'a UiTransactionStatusMeta,
    Vec<String>,
    Vec<UiInstruction>,
)> {
    let meta = tx.transaction.meta.as_ref()?;
    if meta.err.is_some() {
        debug!("Skipping failed transaction");
        return None;
    }

    let EncodedTransaction::Json(transaction) = &tx.transaction.transaction else {
        debug!("Skipping non-JSON encoded transaction");
        return None;
    };

    let account_keys = extract_account_keys(&transaction.message);
    if !account_keys.iter().any(|key| key == pool_address) {
        return None;
    }

    let block_time = tx.block_time?;
    let instructions = flatten_instructions(&transaction.message, meta);

    Some((tx.slot, block_time, meta, account_keys, instructions))
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
