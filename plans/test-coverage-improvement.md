# Test Coverage Improvement Plan: 47.80% → 80%+

## Current State

**Coverage:** 47.80% (1163/2228 lines missed)
**Tests:** 36 total (13 runnable unit, 17 ignored integration, 6 hardware-dependent)
**Tool:** `cargo-llvm-cov`, target 80% (codecov.yml)

**Well-tested modules:**

- [config.rs](src/config.rs) - 84.34% (14 tests) ✅
- [audio/capture.rs](src/audio/capture.rs) - 63.22% (25 tests) ✅

**Critical gaps (631 lines needed for 80%):**

- [transcription/engine.rs](src/transcription/engine.rs) - 27.63% → 80% (+186 lines)
- [tray.rs](src/tray.rs) - 47.03% → 80% (+223 lines)
- [input/hotkey.rs](src/input/hotkey.rs) - 49.84% → 80% (+156 lines)
- [permissions.rs](src/permissions.rs) - 15.25% → 80% (+50 lines)
- [telemetry.rs](src/telemetry.rs) - 31.15% → 80% (+42 lines)
- [input/cgevent.rs](src/input/cgevent.rs) - 44.44% → 80% (+55 lines)
- [transcription/download.rs](src/transcription/download.rs) - 61.06% → 80% (+44 lines)

---

## Implementation Strategy

### Phase 1: Low-Hanging Fruit (2-3 days, 47% → 60%)

**Goal:** Quick wins with minimal mocking, +200 lines coverage

#### 1.1 [permissions.rs](src/permissions.rs) (+40 lines)

**Tests (6 unit):**

- `test_check_microphone_permission_always_ok` - Verify always Ok
- `test_request_all_permissions_chains_checks` - All 3 checks called
- `test_accessibility_permission_error_message` - Mock error, verify message
- `test_input_monitoring_permission_error_message` - Mock error, verify message
- `test_permissions_on_non_macos` - Verify #[cfg(not(target_os = "macos"))]
- `test_check_input_monitoring_event_creation_fails` - Mock CGEvent error

**Approach:** Extract error message formatting, test without macOS APIs

#### 1.2 [telemetry.rs](src/telemetry.rs) (+30 lines)

**Tests (5 unit):**

- `test_init_creates_log_directory` - Verify parent dirs created
- `test_init_with_unwritable_path` - Mock filesystem error
- `test_init_with_invalid_utf8_path` - Non-UTF8 handling
- `test_expand_log_path_with_tilde` - ~/path expansion
- `test_init_disabled_telemetry` - No files when enabled=false

**Approach:** Test file I/O error paths, mock tracing subscriber

#### 1.3 [transcription/download.rs](src/transcription/download.rs) (+30 lines)

**Tests (5 unit):**

- `test_download_model_http_404` - Mock 404, verify error
- `test_download_model_http_500` - Mock 500, verify error
- `test_download_model_network_timeout` - Mock timeout
- `test_download_model_temp_file_cleanup` - Verify .tmp removed on success
- `test_download_model_atomic_rename_failure` - Mock rename error, .tmp remains

**Approach:** Mock HTTP with mockito (26.2M downloads)

#### 1.4 [input/cgevent.rs](src/input/cgevent.rs) (+35 lines)

**Tests (6 unit):**

- `test_insert_text_utf16_encoding` - Emojis, Polish chars
- `test_insert_text_preview_long_text` - "..." truncation at 50 chars
- `test_insert_text_preview_short_text` - No truncation <50 chars
- `test_insert_text_safe_returns_false_on_error` - Error → false
- `test_insert_text_newlines_and_tabs` - Whitespace preserved
- `test_insert_text_special_characters` - <, >, &, " pass through

**Approach:** Test UTF-16 encoding logic separately from CGEvent FFI

#### 1.5 [audio/capture.rs](src/audio/capture.rs) (+60 lines)

**Tests (8 unit):**

- `test_ring_buffer_overflow_drops_samples` - Full ring buffer, verify warning
- `test_start_recording_with_stream_error` - Mock stream.play() error
- `test_stop_recording_with_stream_error` - Mock stream.pause() error
- `test_recording_flag_race_condition` - Flag set before stream.play()
- `test_convert_to_16khz_mono_with_invalid_samples` - NaN, infinity
- `test_save_wav_debug_with_invalid_path` - Unwritable path error
- `test_resampling_ratio_precision` - 44100→16000 exact ratio
- `test_multiple_start_stop_cycles` - Ring buffer clears between cycles

**Approach:** Expand existing MockStreamControl tests

---

### Phase 2: Mock Infrastructure (3-4 days, 60% → 75%)

**Goal:** Build mocking for FFI boundaries, +280 lines coverage

#### 2.1 Create Mock Traits (1 day)

**WhisperContextTrait ([transcription/engine.rs](src/transcription/engine.rs:30)):**

```rust
#[cfg(test)]
pub trait WhisperContextTrait: Send {
    fn full(&mut self, params: FullParams, samples: &[f32]) -> Result<(), String>;
    fn full_n_segments(&self) -> i32;
    fn full_get_segment_text(&self, idx: i32) -> Result<String, String>;
}

#[cfg(test)]
pub struct MockWhisperContext {
    pub segments: Vec<String>,
    pub should_fail: bool,
    pub captured_params: Option<FullParams>,
}
```

**AudioCaptureTrait ([input/hotkey.rs](src/input/hotkey.rs:26)):**

```rust
#[cfg(test)]
pub trait AudioCaptureTrait {
    fn start_recording(&mut self) -> Result<()>;
    fn stop_recording(&mut self) -> Result<Vec<f32>>;
}

#[cfg(test)]
pub struct MockAudioCapture {
    pub start_called: Arc<AtomicBool>,
    pub stop_samples: Vec<f32>,
    pub start_error: Option<anyhow::Error>,
    pub stop_error: Option<anyhow::Error>,
}
```

**TranscriptionEngineMock ([input/hotkey.rs](src/input/hotkey.rs:31)):**

```rust
#[cfg(test)]
pub struct MockTranscriptionEngine {
    pub result: Result<String>,
}
```

#### 2.2 [transcription/engine.rs](src/transcription/engine.rs) (+150 lines, 8 unit tests)

**Tests:**

- `test_transcribe_with_mock_context` - Mock WhisperContext, verify param application
- `test_transcribe_error_handling` - Mock context error
- `test_language_parameter_validation` - Valid/invalid language codes
- `test_thread_count_edge_cases` - Max i32, usize overflow
- `test_beam_size_edge_cases` - Max i32, usize overflow
- `test_transcribe_with_language_en` - Verify language param passed
- `test_transcribe_with_auto_detect_language` - None language handling
- `test_transcribe_result_text_extraction` - Mock segment iteration

**Approach:**

1. Add `TranscriptionEngine::with_mock()` constructor (test-only)
2. Use mockall to generate mock for WhisperContext trait
3. Keep existing 3 integration tests as #[ignore]

#### 2.3 [input/hotkey.rs](src/input/hotkey.rs) (+120 lines, 14 unit tests)

**Tests:**

- `test_on_press_from_idle_starts_recording` - Mock audio.start_recording() called
- `test_on_press_from_recording_ignored` - No action when already recording
- `test_on_press_from_processing_ignored` - No action when processing
- `test_on_release_from_recording_stops_and_transcribes` - Mock audio.stop(), transcription spawned
- `test_on_release_from_idle_ignored` - No action when not recording
- `test_on_release_with_empty_samples` - Handle silent recording (0 samples)
- `test_on_release_with_audio_error` - Revert to Idle on stop_recording error
- `test_process_transcription_success` - Mock transcription, verify insert_text_safe called
- `test_process_transcription_failure` - Mock error, verify state → Idle
- `test_process_transcription_empty_text` - No insertion when text empty
- `test_process_transcription_no_engine` - Handle None transcription engine
- `test_save_debug_wav_creates_directory` - Verify WAV saved to ~/.whisper-hotkey/debug/
- `test_handle_event_correct_hotkey_id` - Route to on_press/on_release
- `test_handle_event_wrong_hotkey_id` - Ignore events for other hotkeys

**Approach:**

1. Add `HotkeyManager::with_mocks()` constructor (test-only)
2. Inject MockAudioCapture + MockTranscriptionEngine using mockall
3. Test state machine transitions (Idle → Recording → Processing → Idle)

---

### Phase 3: Tray Menu Logic (2-3 days, 75% → 82%)

**Goal:** Test menu structure without macOS tray system, +180 lines coverage

#### 3.1 [tray.rs](src/tray.rs) (+180 lines, 12 unit tests)

**Tests:**

- `test_build_menu_idle_state` - Verify menu items for Idle
- `test_build_menu_recording_state` - Status text "🎤 Recording..."
- `test_build_menu_processing_state` - Status text "⏳ Transcribing..."
- `test_build_menu_hotkey_selection` - Checkmark on current hotkey
- `test_build_menu_model_selection` - Checkmark on current model
- `test_build_menu_threads_selection` - Checkmark on current thread count
- `test_build_menu_beam_size_selection` - Checkmark on current beam size
- `test_build_menu_language_selection` - Checkmark on current language
- `test_build_menu_buffer_size_selection` - Checkmark on current buffer size
- `test_build_menu_preload_toggle_checked` - CheckMenuItem state
- `test_build_menu_telemetry_toggle_unchecked` - CheckMenuItem state
- `test_parse_menu_event_with_checkmark` - Strip "✓ " prefix

**Approach:**

1. Extract menu structure logic into testable function
2. Return `Vec<(String, bool)>` (label, enabled) instead of Menu
3. Test logic without macOS tray dependency
4. Keep 1 integration test (#[ignore]) for real TrayManager

---

### Phase 4: Final Gaps & Validation (1-2 days, 82% → 85%+)

**Goal:** Edge cases, integration test documentation, coverage validation

1. **Run coverage report:** `cargo llvm-cov --lcov --output-path lcov.info`
2. **Review gaps:** Identify remaining untested lines (target: <20% gaps)
3. **Enable integration tests:** Document requirements in README/TESTING.md
4. **CI validation:** Ensure unit tests run in CI, integration tests manual

---

## Test Organization

**Inline `#[cfg(test)]` (Unit Tests):**

- All modules: Co-locate tests with implementation

**tests/ Directory (Integration Tests):**

- [tests/phase5_integration_test.rs](tests/phase5_integration_test.rs) - Existing end-to-end (10 tests)
- `tests/hotkey_integration_test.rs` - NEW: State machine with real audio/transcription
- `tests/audio_integration_test.rs` - NEW: Real CoreAudio capture (hardware)

---

## Mock/Fixture Infrastructure

**Test Fixtures ([audio/capture.rs](src/audio/capture.rs)):**

```rust
#[cfg(test)]
pub mod fixtures {
    pub fn silence_samples(duration_secs: usize) -> Vec<f32> {
        vec![0.0; duration_secs * 16000]
    }

    pub fn sine_wave_samples(freq_hz: f32, duration_secs: usize) -> Vec<f32> {
        (0..(duration_secs * 16000))
            .map(|i| (2.0 * std::f32::consts::PI * freq_hz * i as f32 / 16000.0).sin())
            .collect()
    }

    pub fn noise_samples(duration_secs: usize) -> Vec<f32> {
        // LCG for deterministic noise
    }
}
```

---

## Validation Criteria

**Per-module targets:**

- transcription/engine.rs: 27.63% → 80% ✅
- tray.rs: 47.03% → 80% ✅
- input/hotkey.rs: 49.84% → 80% ✅
- permissions.rs: 15.25% → 80% ✅
- telemetry.rs: 31.15% → 80% ✅
- input/cgevent.rs: 44.44% → 80% ✅
- transcription/download.rs: 61.06% → 80% ✅
- audio/capture.rs: 63.22% → 80% ✅

**Overall target:** 47.80% → 80%+

**Test count:** 36 existing → 118 total (+82 new tests)

**Effort estimate:** 8-12 days (4 phases)

---

## Critical Files

- [src/transcription/engine.rs](src/transcription/engine.rs:30) - Core transcription, highest impact (+186 lines)
- [src/tray.rs](src/tray.rs:135) - Menu building, second highest (+223 lines)
- [src/input/hotkey.rs](src/input/hotkey.rs:71) - State machine coordination (+156 lines)
- [src/audio/capture.rs](src/audio/capture.rs) - Already 63%, need edge cases (+60 lines)
- [src/permissions.rs](src/permissions.rs) - Quick win, error messages (+50 lines)
- [src/telemetry.rs](src/telemetry.rs) - File I/O errors (+42 lines)
- [src/input/cgevent.rs](src/input/cgevent.rs) - UTF-16 encoding (+55 lines)
- [src/transcription/download.rs](src/transcription/download.rs) - HTTP errors (+44 lines)

---

## Decisions

1. **Mock library:** Use **mockall** (84.9M downloads, best feature set, 100% safe stable Rust) for general mocking + **mockito** (26.2M downloads) for HTTP-specific mocking in [transcription/download.rs](src/transcription/download.rs)
2. **CI strategy:** All tests automatic - mock FFI boundaries (CoreAudio, CGEvent, Whisper) to run in CI without hardware/permissions
3. **Coverage tool:** Keep cargo-llvm-cov (current tool)
4. **Performance tests:** Don't include in coverage (separate benchmarking suite)
5. **Fuzzing:** Add cargo-fuzz for audio edge cases (NaN, infinity, overflow, extreme sample rates)

## Additional Work

### 5.1 Fuzzing Setup (1 day)

**Create `fuzz/` directory:**

- `fuzz_audio_samples` - Test audio/capture.rs with random f32 samples
- `fuzz_utf16_encoding` - Test input/cgevent.rs with random Unicode strings
- `fuzz_config_parsing` - Test config.rs with malformed TOML

### 5.2 CI Configuration Update

**Update [.github/workflows/ci.yml](.github/workflows/ci.yml):**

- Add mockall/mockito as dev-dependencies
- Run all tests (no #[ignore] for mocked tests)
- Keep integration tests as #[ignore] for manual verification only
