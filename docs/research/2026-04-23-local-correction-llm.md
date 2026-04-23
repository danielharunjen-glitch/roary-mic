# Local correction-LLM research

**Status:** investigation-only. No code changes in this doc. Implementation is a follow-up plan.

## Problem

Whisper transcripts drift on:

- **Punctuation** — Whisper-Small/Turbo drop commas, leave run-on sentences.
- **Homophones** — "their/there/they're", "to/too/two", "principal/principle".
- **Proper nouns** — low-frequency names get hallucinated to near-phonetic wrong spellings (e.g., "Roary" → "Rory", "Tauri" → "Torre").
- **Numbers and units** — "fifty dollars" vs "$50", "two k" vs "2K".
- **Filler words** — "um", "uh", and false starts persist even with VAD trimming.

A small local LLM as a post-processor, running on the user's machine after Whisper returns text, could fix these without sending audio or text to the cloud.

The constraint: **≤1GB RAM** footprint in quantized form, so it runs alongside Whisper on modest hardware (8GB total RAM with OS + Roary Mic + user's other apps).

## Candidate models

Sized for the q4_k_m quantization typical on Apple Silicon with `llama.cpp`. RAM figures are model weights only — add ~200–400MB for KV cache depending on context length.

| Model | Params | Quant | Size on disk | Inference RAM | License | Notes |
|---|---|---|---|---|---|---|
| **Qwen2.5 0.5B Instruct** | 0.5B | q4_k_m | ~350MB | ~450MB | Apache 2.0 | Strong instruction-following for size; Alibaba/Qwen team puts serious effort into sub-1B tier. Good candidate. |
| **SmolLM2 360M Instruct** | 360M | q4_k_m | ~220MB | ~320MB | Apache 2.0 | HuggingFace's small-model project; English-first, tuned for short tasks. Smallest viable option. |
| **Llama 3.2 1B Instruct** | 1B | q4_k_m | ~800MB | ~950MB | Llama 3.2 community license | At the upper bound of the budget. Best quality/size ratio in the <1B class. |
| **Gemma 3 270M** | 270M | q4_k_m | ~180MB | ~280MB | Gemma terms of use | Released mid-2025; smallest modern candidate. Quality on edit tasks unverified. |
| **TinyLlama 1.1B Chat** | 1.1B | q4_k_m | ~650MB | ~800MB | Apache 2.0 | Older (2024), but well-supported. Quality noticeably below Qwen/Llama per reported benchmarks. |
| **Phi-3.5 Mini Instruct** | 3.8B | q4_k_m | ~2.3GB | ~2.6GB | MIT | **Over budget.** Included as a quality ceiling reference only. |

All but Phi-3.5 Mini fit the <1GB runtime-RAM constraint. Phi-3.5 is listed so the reader can compare against the realistic best-case quality ceiling if the budget were relaxed.

### Quality expectation (qualitative, not measured here)

Rough ordering of instruction-following quality on a short edit task in the <1B tier, based on public benchmark reporting:

1. **Llama 3.2 1B** — highest, closest to 3B-tier quality for simple instructions.
2. **Qwen2.5 0.5B** — strong for 0.5B; punctuation and homophone fixes should work well.
3. **SmolLM2 360M** — good at tight, English-only tasks; may struggle with nuanced rewrites.
4. **Gemma 3 270M** — too small to be confident without empirical validation; include in any bake-off.
5. **TinyLlama 1.1B** — older architecture, generally weaker than Qwen2.5 0.5B despite being larger.

**Recommended bake-off:** Llama 3.2 1B, Qwen2.5 0.5B, SmolLM2 360M. If the 800MB-ish footprint of Llama 3.2 1B is too large in practice, fall back to Qwen2.5 0.5B as the default.

## Inference runtimes

| Runtime | Language | Model formats | Metal support | Rust crate | Notes |
|---|---|---|---|---|---|
| **llama.cpp** via `llama-cpp-2` | C++ core, Rust bindings | GGUF | Yes (Metal/CUDA/Vulkan) | `llama-cpp-2` | Most mature; best model coverage; adds a C++ dep at build time. Matches what the project already does with `whisper-rs` (also llama.cpp-family). |
| **candle** | Pure Rust | SafeTensors, GGUF (partial) | Yes (Metal) | `candle-core` + `candle-transformers` | Pure Rust; clean build story; model coverage narrower than llama.cpp but improving. Would be the cleanest fit architecturally. |
| **mistral.rs** | Rust | SafeTensors, GGUF, GGML | Yes | `mistralrs` | Production-focused pure-Rust runtime; fast; smaller ecosystem than candle; quantization support is strong. |
| **ONNX Runtime** via `ort` | C++ core, Rust bindings | ONNX | Yes (CoreML EP) | `ort` | Best if we wanted to share runtime with the existing Parakeet pipeline. Model-format conversion step is friction for GGUF-first model releases. |

### Runtime recommendation

**`llama-cpp-2`** is the pragmatic choice:

- Matches the existing `whisper-rs` build story (both are llama.cpp-family), so no new C++ toolchain assumptions.
- Every candidate model above publishes official GGUF quantizations.
- Metal kernels are well-tested.

**`candle`** is the architecturally purer choice if you want pure-Rust and don't mind slightly narrower model coverage. Worth revisiting if the project wants to reduce C++ surface area.

## Prompt template

Keep it short. Every extra token is latency. The prompt should not let the model "explain" — just output the corrected text.

```
You are a silent editor. Fix transcription errors in the USER message.
Only output the corrected text — no quotes, no preamble, no explanation.
If the input is already correct, echo it verbatim.

USER: {transcript}
```

With `llama.cpp` chat templates, this maps to the model's system/user roles. Use `temperature=0`, `top_p=1`, `max_tokens=len(transcript) * 1.3`.

## Integration shape (proposal — not implementation)

A correction mode lives parallel to the existing `llm_client.rs` AI-mode plumbing, not inside it:

```
Audio → VAD → Whisper → [correction mode?] → clipboard/paste
                           │
                           └─→ local llama-cpp-2 call
```

### Minimal touch points

- **`settings.rs`** — add `correction_mode_enabled: bool` and `correction_model_path: Option<PathBuf>`. Both off/None by default.
- **`managers/model.rs`** — extend to download the chosen GGUF to app data dir, same way Whisper models are fetched today.
- **New `src-tauri/src/correction.rs`** — holds the `llama_cpp_2::LlamaModel` handle, exposes `async fn correct(text: &str) -> Result<String>`.
- **`actions.rs`** — between Whisper output and paste/clipboard, call `correction::correct` when the setting is on.
- **`settings/` UI** — new toggle + model selector (mirrors the existing model-selector pattern).

### What this is NOT

- Not a replacement for AI mode — AI mode is for user-directed tasks (summarize, extend, answer). Correction is silent, always-on, non-interactive.
- Not a reason to re-architect the existing Whisper path.
- Not wired to the network — all local, all the time.

## Sample transcripts (placeholder)

TODO for the follow-up implementation plan: curate 10–15 short noisy transcripts from the user's actual history (`managers/history.rs` stores these) and evaluate each candidate model's correction quality on them. Score by:

- Number of real errors caught.
- Number of new errors introduced (hallucinations, over-editing).
- Latency from Whisper-done to corrected text.

## Recommendation

- **Model:** prototype with **Qwen2.5 0.5B Instruct** as the default (~350MB, Apache 2.0, strong instruction-following for size). Offer **Llama 3.2 1B Instruct** as an optional "higher quality" selection for machines with headroom.
- **Runtime:** `llama-cpp-2`, matching the existing `whisper-rs` story.
- **Scope:** silent correction only, no user-facing rewrites. Always-on when enabled.
- **Out of scope for the follow-up plan:** multiple-language correction (start English-only), streaming correction mid-transcription, on-device fine-tuning.

A follow-up plan should set up the `correction.rs` module, the settings toggle, the model downloader, and the integration into `actions.rs`. That plan is **not** part of the current TCC/install/correction-research plan.
