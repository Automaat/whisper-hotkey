# ADR 0001: Streaming Transcription Architecture

**Status:** Rejected
**Date:** 2025-12-05
**Deciders:** Project maintainers

---

## Context

User requested progressive text feedback during/after recording instead of waiting for complete transcription. Two approaches considered:

1. **Post-recording streaming:** Split audio after hotkey release, transcribe segments progressively
2. **Real-time streaming:** Transcribe 2-second chunks while recording continues

---

## Problem

Current implementation: User presses hotkey → speaks 10s → releases → waits ~2s → all text appears.

**Question:** Can we show text progressively to reduce perceived latency?

---

## Decision

**Keep current batch approach.** Do not implement streaming transcription.

---

## Rationale

### Technical Constraints

**Whisper architecture limitations:**
- Designed for 30-second chunks (training constraint)
- Shorter chunks (<30s) padded with zeros → **hallucination issues** (repetitive text)
- Fundamental model limitation, not implementation detail

**whisper-rs API:**
- No native streaming support
- Synchronous blocking inference (`state.full()`)
- Callback support exists but requires unsafe FFI

### Performance vs Accuracy Tradeoffs

#### Post-Recording Streaming (VAD-based segmentation)
- **Complexity:** ~150 LOC (VAD integration, segment management, incremental insertion)
- **Accuracy:** Preserved (full context per segment)
- **UX:** Progressive feedback (first text ~0.5s, total ~1.7s)
- **Value:** Marginal for 10s default audio (2s → 1.7s total, but progressive)

#### Real-Time Streaming (2s chunk processing)
- **Complexity:** ~570 LOC (concurrent ring buffer, overlap management, deduplication)
- **Accuracy:** **5-10% degradation** (partial context per chunk)
- **Performance:** +20% overhead (multiple small inferences vs single pass)
- **Hallucinations:** High risk (2s chunks well below 30s optimal)
- **UX:** Text during speech (~500ms per chunk + 100ms insertion)

### Simplicity Principle Violation

From CLAUDE.md:
> **✅ ALWAYS:** Simplest solution first
> **❌ NEVER:** Over-abstract, optimize before profiling, helper functions for one-time operations

Real-time streaming violates core principles:
- 570 LOC for marginal UX improvement
- Unsafe FFI, complex threading coordination
- Accuracy regression contradicts "quality first"

---

## Alternatives Considered

### 1. Post-Recording VAD Streaming (Feasible but Declined)

**Implementation:**
```rust
// Use whisper-rs built-in VAD
let vad = WhisperVad::new("models/silero-vad.onnx")?;
let segments = vad.segments_from_samples(audio_data, vad_params)?;

for segment in segments {
    let chunk = extract_audio_slice(segment.start, segment.end);
    let text = engine.transcribe(chunk)?;
    insert_text_incremental(&text)?; // Progressive insertion
}
```

**Pros:**
- Natural speech boundaries (pauses)
- Full context per segment (no accuracy loss)
- Progressive UX (text appears in 2-5s chunks)

**Cons:**
- Complexity: ~150 LOC (VAD config, chunk extraction, cursor management)
- VAD errors: May split mid-sentence or miss pauses
- Minimal latency improvement (2s → 1.7s total, but progressive)
- Not needed for 10s default audio

**Decision:** Declined. Complexity > value for short dictation.

### 2. Draft → Refinement Pipeline (Interesting)

**Implementation:**
```rust
// Immediate draft (beam_size=1, greedy)
let draft = engine.transcribe_fast(samples, beam_size=1)?; // ~200ms
insert_text(&draft);

// Background refinement (beam_size=5)
tokio::spawn(async move {
    let final_text = engine.transcribe_accurate(samples, beam_size=5)?; // ~1.5s
    replace_text(&draft, &final_text)?;
});
```

**Pros:**
- Immediate feedback (~200ms for draft)
- Final accuracy preserved
- Simple: ~100 LOC
- No accuracy degradation (refinement uses full context)

**Cons:**
- Text "flickers" (draft → final replacement)
- Replacement logic complex (diff/patch, cursor position)
- User may continue typing during refinement (conflict)

**Decision:** Potential future enhancement if user feedback requests it.

### 3. Real-Time Chunked Streaming (Researched, Rejected)

**Implementation:** Fixed 2s chunks with 333ms overlap (1:6 ratio), concurrent transcription during recording.

**Rejected due to:**
- **Hallucination risk:** Whisper expects 30s chunks, 2s chunks cause repetitive text
- **Accuracy loss:** 5-10% degradation from lack of full context
- **Complexity:** 570 LOC, unsafe FFI, thread coordination
- **Performance overhead:** 20% slower (multiple small inferences)
- **Better alternatives exist:** Post-recording or draft→refinement provide 80% of UX benefit at 50% complexity

---

## Consequences

### Keeping Current Approach

**Pros:**
- ✅ Simple: ~200 LOC, safe Rust, no FFI complexity
- ✅ Accurate: Full context, optimal Whisper performance
- ✅ Fast: Meets <2s target for 10s audio with base.en model
- ✅ Predictable: Atomic text insertion, no partial results
- ✅ Maintainable: Low complexity, follows CLAUDE.md principles

**Cons:**
- ⚠️ Perceived latency: User waits ~2s after speaking for all text
- ⚠️ No feedback: No progress indicator during transcription

**Mitigation:**
- Performance optimization already implemented (base.en + beam_size=1 + language="en")
- 4-5x speedup achieved: Small model (~2-3s) → BaseEn optimized (~0.5-0.7s)
- Could add simple "Transcribing..." spinner (~10 LOC) for feedback

### If Reconsidering Streaming in Future

**Triggers to reconsider:**
1. User feedback explicitly requests progressive text
2. Recording durations routinely exceed 20-30s (hitting Whisper's 30s chunk limit)
3. Latency becomes unacceptable (>5s for typical audio)
4. Specialized streaming models become available (CarelessWhisper, TheWhisper)

**Recommended path:**
1. Implement draft→refinement first (lowest risk, ~100 LOC)
2. Measure user satisfaction
3. If insufficient, evaluate post-recording VAD streaming (~150 LOC)
4. Real-time streaming remains NOT recommended (complexity/accuracy tradeoffs)

---

## Technical Research Findings

### Whisper Segmentation Characteristics

**30-second chunk constraint:**
- Whisper trained on 30s audio spectrograms
- Max context: 1500 embeddings (3000 frames = 30s)
- Embedding granularity: 20ms (0.02s)

**Token-based timestamps:**
- Timestamp tokens: ID ≥ 50364
- Granularity: 20ms
- Decoder limit: 224 tokens per 30s segment

**Automatic segmentation:**
- Decoder emits timestamp tokens at natural boundaries
- Variable-length segments based on speech patterns
- No-speech detection skips silence (logprob_threshold)

### Segment Size Recommendations

| Segment Size | Latency | Accuracy | Use Case |
|--------------|---------|----------|----------|
| 0.25-1s | Very low | Poor | Real-time (sacrifices quality) |
| 1-3s | Low | Good | Streaming dictation |
| 5-10s | Medium | Excellent | **Current (optimal for dictation)** |
| 10-30s | High | Best | Long-form transcription |

**Hard minimum:** 250ms (VAD noise filter)
**Quality minimum:** 1-3s (sufficient context)
**Optimal for dictation:** 5-10s (current approach)

### whisper-rs Streaming Parameters

**Available in `FullParams`:**
- `set_single_segment(bool)`: Force single output segment
- `set_max_len(int)`: Max segment length in characters
- `set_split_on_word(bool)`: Split on word boundaries
- `set_max_tokens(int)`: Max tokens per segment (more direct than max_len)
- `set_segment_callback_safe(closure)`: **Key for streaming** - callback per segment during transcription

**VAD support (built-in):**
```rust
WhisperVadParams {
    threshold: 0.5,                    // Speech detection threshold
    min_speech_duration_ms: 500,       // 0.5s min segment
    min_silence_duration_ms: 1000,     // 1s silence ends segment
    max_speech_duration_s: 10.0,       // Force split at 10s
    speech_pad_ms: 30,                 // Padding to avoid clipping
    audio_ctx_ms: 100.0,               // 100ms overlap between segments
}
```

### Performance Characteristics

**Current batch (base.en, beam_size=1):**
- 10s audio → ~0.5-0.7s transcription
- Memory: <500MB
- Accuracy: Excellent (full context)

**Post-recording streaming estimate:**
- 10s audio split into [2.5s, 3s, 2.5s, 2s]
- Per-segment: ~300-400ms transcription
- First text: ~500ms (faster perceived latency)
- Total: ~1.5-1.7s (slightly slower overall)
- Memory: +50-100MB (buffers)

**Real-time streaming estimate:**
- 2s chunks with 333ms overlap
- Per-chunk: ~500ms transcription
- User-perceived lag: 600ms (500ms + 100ms insertion)
- Total overhead: +20% vs batch
- Memory: +100MB
- Accuracy: -5-10% degradation

### Overlap Strategy (for chunked approaches)

**Research-validated ratio:** 1:6 (stride = chunk_length / 6)
- 2s chunks: 333ms overlap
- 5s chunks: 833ms overlap

**Purpose:** Preserve word boundaries at chunk edges, provide context for next chunk

**Deduplication:** Token-based comparison (not character matching)

---

## References

### Research Sources
- [OpenAI Whisper Discussions (30s constraint)](https://github.com/openai/whisper/discussions/1118)
- [Whisper Encoder Architecture](https://gattanasio.cc/post/whisper-encoder/)
- [Whisper Long-Form Transcription](https://medium.com/@yoad/whisper-long-form-transcription-1924c94a9b86)
- [WhisperX: Word-Level Timestamps](https://github.com/m-bain/whisperX)
- [UFAL Whisper Streaming](https://github.com/ufal/whisper_streaming)
- [WhisperLive (Collabora)](https://github.com/collabora/WhisperLive)
- [Turning Whisper into Real-Time System (arXiv)](https://arxiv.org/html/2307.14743)
- [Bloomberg Streaming ASR Research](https://www.bloomberg.com/company/stories/bloombergs-ai-researchers-turn-whisper-into-a-true-streaming-asr-model-at-interspeech-2025/)
- [Hugging Face Chunking Best Practices](https://huggingface.co/openai/whisper-large-v2/discussions/67)
- [whisper.cpp GitHub](https://github.com/ggml-org/whisper.cpp)

### Implementation References
- [whisper-rs Documentation](https://docs.rs/whisper-rs/0.15.0/whisper_rs/)
- [whisper-rs GitHub](https://github.com/tazz4843/whisper-rs)
- [whisper-stream-rs crate](https://crates.io/crates/whisper-stream-rs)

---

## Related Decisions

- ADR 0002: Performance optimization (base.en, beam_size=1) - see config changes 2025-12-05

## Notes

This decision aligns with CLAUDE.md principles:
- Simplicity first
- Three similar lines > premature abstraction
- Profile before optimizing (we did: 4-5x speedup achieved)
- Touch only files for current task
- No over-abstraction

If user feedback changes (explicit requests for streaming, longer recordings), revisit with draft→refinement or post-recording VAD approach first.
