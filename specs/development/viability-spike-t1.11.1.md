# Viability Spike: Gemma 4 E2B on Hobbyist Hardware (T1.11.1)

| Field | Value |
|-------|-------|
| Task | T1.11.1 |
| Status | **Not yet executed** |
| Spec refs | OQ-021, OQ-024, ASM-014a |

## Objective

Determine whether **Gemma 4 E2B (5B params)** running locally via **Ollama**
is viable as a no-API-key fallback for hobbyist M1 agent flows on a **4 GB VPS**.

## Test Plan

### Environment
- **VM:** 4 GB RAM, 2 vCPU (e.g. DigitalOcean $24/yr droplet or equivalent)
- **OS:** Ubuntu 22.04 minimal
- **Runtime:** Ollama latest stable
- **Model:** `gemma4:e2b` (GGUF, Q4_K_M quantization)

### Steps

1. Provision VM (4 GB RAM, 2 vCPU)
2. Install Ollama: `curl -fsSL https://ollama.ai/install.sh | sh`
3. Pull model: `ollama pull gemma4:e2b`
4. Measure idle memory: `free -m` before and after model load
5. Run sample prompt:
   ```
   ollama run gemma4:e2b "You are a blockchain node operator assistant. The user just installed Karoowa for the first time. Walk them through generating a wallet key and starting a node."
   ```
6. Measure:
   - **Peak RSS** of the Ollama process (`/proc/<pid>/status` VmRSS)
   - **Time to first token** (seconds)
   - **Total response time** (seconds)
   - **Response quality** (does it make sense? Would it help a hobbyist?)
7. Repeat with a Karoowa node running alongside:
   - Start `karoowa node` in background
   - Measure combined RSS (node + Ollama)
   - Run the same prompt
8. Classify result:
   - **Viable:** Peak RSS < 3.5 GB, latency < 30s, quality acceptable
   - **Viable with tweaks:** Works but needs lower quantization or smaller model
   - **Not viable:** OOM, latency > 60s, or quality too poor

### Decision Matrix

| Result | Action |
|--------|--------|
| Viable | Gemma 4 E2B is the hobbyist no-key fallback |
| Viable with tweaks | Document the tweaks; offer both quantization levels |
| Not viable | Hobbyists must use a hosted provider (Anthropic API key). Local fallback is dropped or reduced to a smaller model (e.g. Gemma 4 E2B at Q2_K). |

## Notes

- The hobbyist **default** is always hosted LLM (Anthropic) with in-process agent
- Local fallback is a **nice-to-have** for privacy-conscious hobbyists
- This spike does NOT block the rest of Phase 1.11 — the `OllamaProvider` is implemented regardless; the spike determines whether we recommend it for hobbyists
