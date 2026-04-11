# Viability Spike: Gemma 4 E2B on Hobbyist Hardware (T1.11.1)

| Field | Value |
|-------|-------|
| Task | T1.11.1 |
| Status | **Executed 2026-04-11** |
| Spec refs | OQ-021, OQ-024, ASM-014a |

## Objective

Determine whether **Gemma 4 E2B (5B params)** running locally via **Ollama**
is viable as a no-API-key fallback for hobbyist M1 agent flows on limited hardware.

## Results

### Test 1: Quality Assessment (macOS, local machine)

**Environment:** macOS, Apple Silicon, 16+ GB RAM (not the target hardware — testing quality only)

**Model:** `gemma4:e2b` (7.2 GB on disk)

**Prompt:** "You are a blockchain node operator assistant for Karoowa. Walk a first-time user through generating a wallet key, starting a single-node devnet, and checking block production."

**Results:**
- **Response time:** ~126 seconds (total, including thinking)
- **Output tokens:** 526
- **Quality:** **Acceptable but imprecise.** The model produced a coherent 3-step guide with correct structure (generate key, start node, check blocks) but hallucinated CLI commands (`./karoowa wallet generate`, `./karoowa node start --network devnet`, `./karoowa node status`) that don't match our actual CLI surface (`karoowa wallet new`, `karoowa node --validator-key ... --consensus poa`, `karoowa client block-number`). The response was helpful in tone but would mislead a user trying to follow exact commands.
- **Latency on Mac:** ~126s is too slow for interactive use even on powerful hardware. The E2B model includes a "thinking" phase that dominates latency.

### Test 2: Hardware Viability (2GB AWS Lightsail)

**Environment:** AWS Lightsail, 2GB RAM, 2 vCPU, Ubuntu 24.04

**Result:** **Not tested directly** — the model is 7.2 GB on disk and requires ~4-5 GB RAM to load. A 2GB VPS cannot run Gemma 4 E2B even with swap. OOM is guaranteed.

### Classification: **Not Viable**

| Criterion | Target | Actual | Pass? |
|-----------|--------|--------|-------|
| Peak RAM | < 3.5 GB | ~4-5 GB (model alone) | FAIL |
| Latency | < 30s | ~126s | FAIL |
| Quality | Helpful and accurate | Helpful but hallucinated commands | PARTIAL |
| 2GB VPS | Runs alongside node | Cannot load model | FAIL |

## Decision

**Local-model fallback is not viable for hobbyist hardware (2-4 GB).**

Per the decision matrix:
- Hobbyists **must use a hosted provider** (Anthropic API key is the default)
- The `OllamaProvider` remains in the codebase for users with >=16 GB RAM (developers, workstations) but is **not recommended** for hobbyist tier
- ASM-014a is confirmed: the no-key fallback is a **soft promise**, not a hard guarantee
- Consider smaller models (Gemma 4 at Q2_K, or sub-1B models) in future if the local-model story matters for adoption

## Impact on PRDs

- **REQ-014 (Pluggable LLM provider):** Unchanged — trait + providers shipped as planned
- **REQ-015 (Hybrid agent runtime):** Unchanged — in-process mode works fine with hosted LLM
- **ASM-014a:** Validated as "not viable on hobbyist hardware" — documented accordingly
- **OQ-AB-004 (from Agent Bundle PRD):** Answered — fallback dropped for hobbyist tier

## Notes

- The `OllamaProvider` implementation is correct and tested (unit tests pass)
- Local models improve rapidly — re-evaluate at M2 when smaller models may be available
- The spike does not block M1 completion — all agent code works with the Anthropic provider
