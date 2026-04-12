# Karoowa — Tokenomics

**Status:** Draft for mainnet launch review.
**Version:** v1.0.
**Last updated:** 2026-04-12.

This document describes the native token (KAR) supply schedule, the EIP-1559 fee model, staking rewards, treasury funding, and validator economics for the Karoowa v1.0 mainnet. Any mainnet change requires a governance proposal through the validator chamber (see `core/karoowa-governance`).

---

## 1. Native Token: KAR

- **Symbol:** KAR
- **Decimals:** 18 (matches EVM convention so tooling interop is cheap)
- **Genesis supply:** 1,000,000,000 KAR (1B)
- **Max supply:** uncapped; annual inflation governed by §4

KAR is used for:

1. **Gas payment** — every transaction pays a base fee + priority tip in KAR.
2. **Staking** — validators bond KAR to produce blocks; delegators bond KAR to validators for a share of rewards.
3. **Governance weight** — token-weighted chamber votes use staked KAR.
4. **Treasury** — a fraction of every block's issuance + fee burn flows into the on-chain treasury.

---

## 2. Genesis Allocation

The 1B genesis supply is split as follows:

| Allocation | Amount | % | Vesting |
|---|---|---|---|
| Community & ecosystem | 350M | 35% | linear over 4 years |
| Core contributors | 200M | 20% | 1-year cliff, linear 3 years after |
| Treasury (on-chain) | 200M | 20% | none — governance-controlled day 1 |
| Validators (initial set) | 150M | 15% | linear over 2 years, forfeited on slashing |
| Public sale | 80M | 8% | none |
| Foundation | 20M | 2% | linear over 5 years |

**Vesting** is enforced on-chain via time-locked accounts. The full schedule lands in `genesis.json` before mainnet genesis ceremony (Phase 6.7) and is audited by the systems engagement.

---

## 3. Fee Model (EIP-1559)

Karoowa implements EIP-1559 / EIP-2718 / EIP-2930 from day one. Every block has:

- **`base_fee`** — the minimum gas price to be included. Adjusts per block based on the previous block's gas usage relative to `base_fee_target_gas` (a governable parameter, 15M by default).
- **`priority_fee`** — the tip sent to the block proposer above the base fee. Paid in full on successful inclusion.

### 3.1 Base fee curve

```
if gas_used > target:
    base_fee += base_fee * (gas_used - target) / target / 8
else:
    base_fee -= base_fee * (target - gas_used) / target / 8
```

This is the canonical EIP-1559 adjustment. The `/8` divisor means the base fee can move at most ~12.5% per block, smoothing out demand spikes over 7–8 blocks.

### 3.2 Fee burn + treasury split

Of the base fee portion of each transaction:

- **70% burned** — permanent supply reduction
- **30% to treasury** — credited to the treasury account atomically with the block

The 30% treasury split is a deliberate divergence from Ethereum's pure-burn model. It funds the on-chain treasury without relying on new inflation, which gives long-term projects a predictable grant stream.

Priority fees go entirely to the block proposer (unchanged from EIP-1559).

### 3.3 Governability

The base-fee-target-gas and block-gas-limit parameters live in the governable parameter registry (`core/karoowa-governance/src/params.rs`) under the `ValidatorOnly` tier. Changing them requires a 2/3+ validator supermajority vote.

---

## 4. Inflation & Block Rewards

Each block mints new KAR as a staking reward. The issuance curve is:

```
annual_inflation_rate = max(2%, 10% - (years_since_genesis × 1%))
```

- **Year 0:** 10% APY (attracts early validators)
- **Year 1:** 9% APY
- **Year 2:** 8% APY
- ...
- **Year 8+:** floor at 2% APY permanently

### 4.1 Block reward split

Each newly-minted block reward is split:

| Recipient | Share |
|---|---|
| Block proposer | 5% |
| All validators (stake-weighted) | 75% |
| Treasury | 15% |
| Burn (demand-gated) | 5% |

The 5% burn is only applied when network utilization exceeds the target — under low utilization it flows to the treasury instead. This creates mild deflationary pressure on busy chains and keeps the treasury funded on quiet ones.

### 4.2 Delegation split

A validator that has accepted delegations splits their 75% share with delegators pro-rata by bonded amount, minus a validator commission (default 10%, max 25%, enforced on-chain).

---

## 5. Staking

### 5.1 Validator bonds

- **Minimum self-bond:** 100,000 KAR
- **Minimum total bond (self + delegations):** 1,000,000 KAR to be elected
- **Maximum active set:** 100 validators (governable, starts at 30 at mainnet)
- **Bond unbonding period:** 21 days

### 5.2 Delegator bonds

- **Minimum delegation:** 10 KAR
- **Unbonding period:** 21 days
- **Slashing exposure:** delegators share the validator's slashing penalty pro-rata

### 5.3 Slashing

| Offense | Penalty | Jail |
|---|---|---|
| Downtime (missed > 5% of a 10,000-block window) | 0.01% | 1 hour |
| Double-signing | 5% | permanent |
| Equivocation (conflicting BFT votes) | 5% | permanent |

Slashing is deterministic on-chain and permanent — there is no appeals process at the protocol layer. Off-chain evidence of coercion or hardware failure can motivate a governance refund proposal through the token chamber.

---

## 6. Treasury

The on-chain treasury is an account controlled by governance. Its inflows are:

- **Genesis allocation:** 200M KAR
- **15% of every block's issuance reward**
- **30% of every block's base fee**
- **100% of slashing penalties** (forfeited validator bonds)
- **Confiscated delegations** from slashed validators (same pool)

Treasury disbursements require a `ProposalKind::TreasuryDisbursement` proposal through the **token chamber** (40% quorum, 50%+1 majority, token-weighted). These proposals go through the same `Voting → Timelock → Executed` lifecycle as any other; the timelock is a deliberate safety window for validators to veto a clearly-malicious disbursement.

### 6.1 Expected treasury inflow (year 1)

At mainnet launch with 1B supply and 10% inflation:

- Issuance: 100M KAR/year × 15% = **15M KAR/year to treasury from rewards**
- Fees: estimated ~5M KAR/year base fee × 30% = **1.5M KAR/year to treasury from fees**
- Total: **~16.5M KAR/year** in year 1

At a nominal $1 = 1 KAR this is enough for a few dozen substantial grants per year without touching the genesis allocation.

### 6.2 Grant policy

Grants are authored as `TreasuryDisbursement` proposals. The treasury agent (`core/karoowa-agents/src/agents/treasury.rs`) flags unusual disbursement patterns and summarises each proposal for voters.

---

## 7. Validator Economics

### 7.1 Expected APY (year 1, ignoring fees)

At 10% annual inflation with 75% of issuance going to validators stake-weighted:

- Staking APY ≈ 10% × 75% / staking_ratio
- At a 50% staking ratio (500M KAR staked), APY ≈ **15%**
- At a 70% staking ratio (700M KAR staked), APY ≈ **~10.7%**
- At a 30% staking ratio (300M KAR staked), APY ≈ **25%** (rapid equilibration expected)

Delegators earn this minus commission; validators add their commission on top of their own-stake yield.

### 7.2 Operating cost breakdown

Rough annualised cost for a single validator meeting the hardware requirements in `docs/operator-guide.md`:

| Line item | $/year |
|---|---|
| NVMe cloud instance (8 core, 32 GB, 1 TB SSD) | 3,600 |
| Bandwidth (1 Gbps egress, modest traffic) | 1,200 |
| HSM (YubiHSM 2 amortized, or AWS CloudHSM subscription) | 2,400 |
| Monitoring (Grafana Cloud tier) | 600 |
| On-call engineer (10% of one FTE) | 15,000 |
| **Total** | **≈ $22,800** |

With the minimum 1M KAR total bond at nominal $1, validator revenue at 10% APY is ~$100k/year — comfortably covering cost even after delegator commission.

---

## 8. Parameter Summary

All of the following parameters live in the governable registry (`core/karoowa-governance/src/params.rs`) and can be adjusted by governance within the declared ranges:

| Parameter | Default | Range | Tier |
|---|---|---|---|
| `block_time_ms` | 2000 | 500 – 60000 | ValidatorOnly |
| `block_gas_limit` | 30M | 1M – 1B | ValidatorOnly |
| `min_gas_price` | 1 | 1 – 1M | ValidatorOnly |
| `base_fee_target_gas` | 15M | 500k – 500M | ValidatorOnly |
| `voting_period_blocks` | 100k | 100 – 10M | General |
| `timelock_blocks` | 20k | 0 – 1M | General |
| `min_proposal_deposit` | 1M | 1 – u64::MAX | General |

The two chambers:

- **ValidatorOnly:** 2/3+ supermajority of validator weight required.
- **General:** 40% token quorum, 50%+1 majority, no-with-veto support.

---

## 9. Open Decisions Before Mainnet

- **Final inflation floor** — 2% vs 1.5% vs 1% at year 8+. Currently 2%.
- **Validator commission cap** — 25% is the current draft; some chains run 20% or 15%.
- **Slashing on downtime** — 0.01% is lenient; may tighten after incentivised testnet data.
- **Treasury share of base fee** — 30% is aggressive by Ethereum standards; reduce if the burn-vs-treasury tradeoff feels wrong after 3 months of mainnet.
- **Public sale sizing** — 8% is the current draft; final number depends on regulatory guidance.

These numbers are open for community review until the genesis ceremony (Phase 6.7). Proposed changes go through the normal spec-review process.
