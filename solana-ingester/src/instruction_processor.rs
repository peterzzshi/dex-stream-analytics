use crate::balances;
use crate::config::PoolConfig;
use crate::decoder::{self, DecodedInstruction};
use anyhow::{Context, Result};
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    option_serializer::OptionSerializer, UiInstruction, UiParsedInstruction,
    UiTransactionStatusMeta, UiTransactionTokenBalance,
};
use tracing::warn;

/// Decodes Raydium instructions. Pure instruction processing, no I/O.
pub struct InstructionProcessor {
    program_id: String,
    pool_config: PoolConfig,
}

impl InstructionProcessor {
    pub fn new(program_id: String, pool_config: PoolConfig) -> Self {
        Self {
            program_id,
            pool_config,
        }
    }

    /// Decodes all instructions in a transaction that touch this pool.
    /// Returns decoded instructions with their context, ready for event building.
    pub fn decode_instructions(
        &self,
        tx: &TransactionData<'_>,
        pool_address: &str,
    ) -> Result<Vec<DecodedInstructionContext>> {
        let Some((pre_balances, post_balances)) = token_balances(tx.meta) else {
            return Ok(Vec::new());
        };

        let mut results = Vec::new();

        for (index, instruction) in tx.instructions.iter().enumerate() {
            let Some(view) = InstructionView::parse(instruction, tx.account_keys)? else {
                continue;
            };

            if view.program_id != self.program_id.as_str() {
                continue;
            }

            if !view.touches(tx.account_keys, pool_address) {
                continue;
            }

            let data = match bs58::decode(view.data).into_vec() {
                Ok(data) => data,
                Err(_) => continue,
            };

            let decoded = match decoder::decode_instruction(&data, view.account_count) {
                Ok(decoded) => decoded,
                Err(_) => continue,
            };

            let authority = match view.accounts.get(2).and_then(|&i| tx.account_keys.get(i)) {
                Some(authority) => authority,
                None => {
                    warn!(
                        "Skipping instruction {} in {}: no AMM authority account",
                        index, tx.signature
                    );
                    continue;
                }
            };

            let (vault0_index, vault1_index) = match balances::resolve_vault_indices(
                pre_balances,
                post_balances,
                &view.accounts,
                authority,
                &self.pool_config.token0,
                &self.pool_config.token1,
            ) {
                Ok(indices) => indices,
                Err(e) => {
                    warn!(
                        "Skipping instruction {} in {}: {:#}",
                        index, tx.signature, e
                    );
                    continue;
                }
            };

            results.push(DecodedInstructionContext {
                decoded,
                signature: *tx.signature,
                slot: tx.slot,
                block_time: tx.block_time,
                instruction_index: index,
                pool_address: pool_address.to_string(),
                user_authority: tx
                    .account_keys
                    .first()
                    .cloned()
                    .unwrap_or_else(|| pool_address.to_string()),
                vault0_index,
                vault1_index,
                pre_balances: pre_balances.to_vec(),
                post_balances: post_balances.to_vec(),
                meta: tx.meta.clone(),
            });
        }

        Ok(results)
    }
}

/// Transaction data needed for instruction decoding.
pub struct TransactionData<'a> {
    pub signature: &'a Signature,
    pub slot: u64,
    pub block_time: i64,
    pub account_keys: &'a [String],
    pub instructions: &'a [UiInstruction],
    pub meta: &'a UiTransactionStatusMeta,
}

/// A decoded instruction with all its context, ready for event building.
pub struct DecodedInstructionContext {
    pub decoded: DecodedInstruction,
    pub signature: Signature,
    pub slot: u64,
    pub block_time: i64,
    pub instruction_index: usize,
    pub pool_address: String,
    pub user_authority: String,
    pub vault0_index: usize,
    pub vault1_index: usize,
    pub pre_balances: Vec<UiTransactionTokenBalance>,
    pub post_balances: Vec<UiTransactionTokenBalance>,
    pub meta: UiTransactionStatusMeta,
}

/// An instruction resolved against the account table: `accounts` are transaction-wide
/// indices, equal to the declared account count for compiled instructions.
struct InstructionView<'a> {
    program_id: &'a str,
    data: &'a str,
    accounts: Vec<usize>,
    account_count: usize,
}

impl<'a> InstructionView<'a> {
    /// `Ok(None)` for instruction forms that carry no decodable data.
    fn parse(instruction: &'a UiInstruction, account_keys: &'a [String]) -> Result<Option<Self>> {
        Ok(match instruction {
            UiInstruction::Compiled(compiled) => {
                let program_id = account_keys
                    .get(compiled.program_id_index as usize)
                    .context("Program ID index out of bounds")?;
                Some(Self {
                    program_id: program_id.as_str(),
                    data: &compiled.data,
                    accounts: compiled.accounts.iter().map(|&i| i as usize).collect(),
                    account_count: compiled.accounts.len(),
                })
            }
            UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(partial)) => Some(Self {
                program_id: &partial.program_id,
                data: &partial.data,
                accounts: partial
                    .accounts
                    .iter()
                    .filter_map(|account| account_keys.iter().position(|key| key == account))
                    .collect(),
                account_count: partial.accounts.len(),
            }),
            UiInstruction::Parsed(_) => None,
        })
    }

    fn touches(&self, account_keys: &[String], address: &str) -> bool {
        self.accounts
            .iter()
            .any(|&i| account_keys.get(i).map(String::as_str) == Some(address))
    }
}

fn token_balances(
    meta: &UiTransactionStatusMeta,
) -> Option<(&[UiTransactionTokenBalance], &[UiTransactionTokenBalance])> {
    match (&meta.pre_token_balances, &meta.post_token_balances) {
        (OptionSerializer::Some(pre), OptionSerializer::Some(post)) => {
            Some((pre.as_slice(), post.as_slice()))
        }
        _ => None,
    }
}
