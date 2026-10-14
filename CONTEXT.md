# Context

Glossary for the DEX Stream Analytics domain. Terms only — no implementation detail,
no decisions. Decisions live in `docs/adr/`.

## Chain

The blockchain whose logs are being ingested. Carried as `chain`, a **CAIP-2**
namespaced identifier (`eip155:137` for Polygon, `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp...`
for Solana mainnet-beta) with `chainName` as a display-only label. EIP-155's bare
numeric chain ID is EVM-only and cannot name a Solana cluster, so CAIP-2 is the
canonical form even while only one EVM chain is monitored. A chain is a source of
events, not an identity of a pool: the same logical market (WETH/USDC) exists as
*different pools* on different chains.

## Pool

A single AMM pool contract on one chain — a Uniswap V2 pair on EVM chains, a Raydium
AMM v4 pool on Solana. **A pool's identity is the pair of `(chain, pairAddress)`** —
a pair address alone does not identify a pool, because the same address string can
collide across chains with different address formats (EVM `0x...` vs. Solana
base58), and the same token pair legitimately has different addresses per chain.

_Monitored pools today: WMATIC/USDC on Polygon. Next chain family: Solana (see
[ARCHITECTURE.md](ARCHITECTURE.md#future-work))._

## Event

A single decoded contract log from one pool: a `Swap`, `Mint`, `Burn`, or `Transfer`.
An event's identity is `(blockNumber, txHash, logIndex)` — globally unique within a
chain. The same tuple on two chains is two different events.

## Window

A bounded set of events grouped by event time for aggregation: tumbling (fixed,
non-overlapping), sliding (fixed size, moving), or session (bounded by an inactivity
gap). Windows are the unit at which raw events become **analytics**.

## Analytics

An aggregated, persisted observation over one window of one pool: trading summaries,
liquidity summaries, market trends, or MEV alerts. Analytics are computed once by the
pipeline and queried thereafter; they are not recomputed per query.

## MEV Alert

An analytics record asserting that a window's events contain a detectable MEV pattern
(sandwich attack or JIT liquidity). An alert is a *heuristic accusation*, not proof:
it names a suspected attacker and an estimated profit.

## Pool Health

A composite score over a pool's recent analytics (trading, liquidity, safety). Health
is **derived at query time**, never persisted — it is a view over analytics, and its
formula is expected to change faster than the analytics it reads.

## Finality Gate

The deliberate delay before an event is published downstream, so that an event from a
block or slot that later gets reorged out is never published at all. What "final enough"
means is chain-specific: on EVM chains it's N confirmations past the current head; on
Solana it's a **commitment level** (`processed` < `confirmed` < `finalized`), not a
block count. A finality gate trades latency for the guarantee that downstream consumers
never see a reorged event.

## Natural-Language Query

A user question expressed in prose, compiled into a **deterministic query against
persisted analytics**. The model's only output is *which* query to run; it never
computes, estimates, or emits a number. Numbers in an answer come from the engine.

## Query Plan

The typed object a Natural-Language Query compiles into: which metric, which pool
(identified by `(chain, pairAddress)`), which window, which time range. A query
plan's shape is fixed and enumerable **before** the model is called, so a malformed
or out-of-scope plan is impossible by construction — the model selects among valid
plans rather than inventing one. A question that maps to no valid plan is **refused**,
not approximated.

## Grounded Answer

A response whose every numeral originates from engine output, not from the model.
The model may arrange and explain, but any number it presents must appear verbatim
in the query result — a numeral with no engine provenance is a defect.

