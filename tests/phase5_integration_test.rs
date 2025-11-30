//! Integration tests for Phase 5: Text Insertion
//!
//! These tests verify the end-to-end integration of:
//! - Transcription engine with audio samples
//! - Text insertion via CGEvent
//! - Error handling and logging
//!
//! Most tests are marked with #[ignore] as they require:
//! - Accessibility permissions
//! - Active cursor position in a text input
//! - Whisper model file
//!
//! Run with: cargo test --test phase5_integration_test -- --ignored

use std::path::PathBuf;

fn get_test_model_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let path = PathBuf::from(home)
        .join(".whisper-hotkey")
        .join("models")
        .join("ggml-tiny.bin");

    if path.exists() {
        Some(path)
    } else {
        None
    }
}

#[test]
#[ignore] // Requires model file and Accessibility permissions with active text input
fn test_transcribe_silence_to_text_insertion() {
    use whisper_hotkey::input::cgevent;
    use whisper_hotkey::transcription::TranscriptionEngine;

    let model_path = match get_test_model_path() {
        Some(path) => path,
        None => {
            eprintln!("Skipping: no model at ~/.whisper-hotkey/models/ggml-tiny.bin");
            return;
        }
    };

    // Load transcription engine
    let engine = TranscriptionEngine::new(&model_path, 4, 5).expect("failed to load model");

    // 1 second of silence
    let silence: Vec<f32> = vec![0.0; 16000];

    // Transcribe
    let result = engine.transcribe(&silence).expect("transcription failed");

    // Should produce empty or minimal text
    assert!(
        result.is_empty() || result.len() < 50,
        "Expected minimal output for silence"
    );

    // If text is non-empty, test insertion (requires Accessibility permissions)
    if !result.is_empty() {
        let inserted = cgevent::insert_text_safe(&result);
        assert!(inserted, "Failed to insert transcribed text");
    }
}

#[test]
#[ignore] // Requires model file and Accessibility permissions
fn test_full_pipeline_with_tone() {
    use whisper_hotkey::input::cgevent;
    use whisper_hotkey::transcription::TranscriptionEngine;

    let model_path = match get_test_model_path() {
        Some(path) => path,
        None => {
            eprintln!("Skipping: no model");
            return;
        }
    };

    let engine = TranscriptionEngine::new(&model_path, 4, 5).expect("failed to load model");

    // 1 second of 440Hz tone
    let sample_rate = 16000.0;
    let frequency = 440.0;
    let samples: Vec<f32> = (0..16000)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5
        })
        .collect();

    let result = engine.transcribe(&samples).expect("transcription failed");

    println!("Transcribed tone: '{}'", result);

    // Insert any resulting text (might be empty or gibberish)
    if !result.is_empty() {
        let inserted = cgevent::insert_text_safe(&result);
        assert!(inserted, "Failed to insert text");
    }
}

#[test]
#[ignore] // Requires Accessibility permissions and active text input
fn test_text_insertion_various_apps() {
    use whisper_hotkey::input::cgevent;

    // This test should be run manually with different apps:
    // 1. TextEdit
    // 2. VS Code
    // 3. Chrome (Google Docs, Gmail)
    // 4. Slack
    //
    // Instructions:
    // 1. Open one of the apps above
    // 2. Focus a text input
    // 3. Run: cargo test test_text_insertion_various_apps -- --ignored --nocapture
    // 4. Verify text appears in the app
    //
    // Wait 3 seconds for user to focus app
    println!("Focus a text input in 3 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let test_text = "Hello from Whisper Hotkey! 👋";

    let result = cgevent::insert_text_safe(test_text);
    assert!(result, "Text insertion failed");

    println!("✓ Text inserted: '{}'", test_text);
    println!("Verify it appeared in your focused app");
}

#[test]
#[ignore] // Requires Accessibility permissions and active text input
fn test_unicode_insertion_polish() {
    use whisper_hotkey::input::cgevent;

    println!("Focus a text input in 3 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let test_text = "Zażółć gęślą jaźń 🇵🇱";

    let result = cgevent::insert_text_safe(test_text);
    assert!(result, "Polish text insertion failed");

    println!("✓ Polish text inserted: '{}'", test_text);
}

#[test]
#[ignore] // Requires Accessibility permissions and active text input
fn test_multiline_insertion() {
    use whisper_hotkey::input::cgevent;

    println!("Focus a text input in 3 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let test_text = "Line 1: Testing multiline\nLine 2: Text insertion\nLine 3: Via CGEvent";

    let result = cgevent::insert_text_safe(test_text);
    assert!(result, "Multiline text insertion failed");

    println!("✓ Multiline text inserted");
}

#[test]
#[ignore] // Requires Accessibility permissions and active text input
fn test_long_text_insertion() {
    use whisper_hotkey::input::cgevent;

    println!("Focus a text input in 3 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Simulate a long transcription (~500 words)
    let test_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(63);

    let result = cgevent::insert_text_safe(&test_text);
    assert!(result, "Long text insertion failed");

    println!("✓ Long text inserted ({} chars)", test_text.len());
}

#[test]
#[ignore] // Requires Accessibility permissions
fn test_text_insertion_error_handling() {
    use whisper_hotkey::input::cgevent;

    // Empty text should fail gracefully
    let result = cgevent::insert_text_safe("");
    assert!(!result, "Empty text should return false");

    // Valid text should succeed (if permissions granted)
    let result = cgevent::insert_text_safe("test");
    // Can't assert success without knowing if app is focused
    println!("Text insertion result: {}", result);
}

#[test]
#[ignore] // Requires model file
fn test_transcription_performance() {
    use std::time::Instant;
    use whisper_hotkey::transcription::TranscriptionEngine;

    let model_path = match get_test_model_path() {
        Some(path) => path,
        None => {
            eprintln!("Skipping: no model");
            return;
        }
    };

    let engine = TranscriptionEngine::new(&model_path, 4, 5).expect("failed to load model");

    // Test different audio lengths
    let test_cases = vec![
        ("5s", 5 * 16000),
        ("10s", 10 * 16000),
        ("15s", 15 * 16000),
        ("30s", 30 * 16000),
    ];

    for (name, length) in test_cases {
        let audio: Vec<f32> = vec![0.0; length];

        let start = Instant::now();
        let result = engine.transcribe(&audio);
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Failed to transcribe {}", name);
        println!(
            "{}: {} samples → {:.2}s (target <2s for 10s audio)",
            name,
            length,
            elapsed.as_secs_f64()
        );
    }
}

#[test]
#[ignore] // Requires model file
fn test_concurrent_transcriptions() {
    use std::sync::Arc;
    use std::thread;
    use whisper_hotkey::transcription::TranscriptionEngine;

    let model_path = match get_test_model_path() {
        Some(path) => path,
        None => {
            eprintln!("Skipping: no model");
            return;
        }
    };

    let engine =
        Arc::new(TranscriptionEngine::new(&model_path, 4, 5).expect("failed to load model"));

    // Verify TranscriptionEngine is thread-safe by running concurrent transcriptions
    let mut handles = vec![];

    for i in 0..3 {
        let engine = Arc::clone(&engine);
        let handle = thread::spawn(move || {
            let audio: Vec<f32> = vec![0.0; 16000];
            let result = engine.transcribe(&audio);
            assert!(result.is_ok(), "Thread {} transcription failed", i);
            println!("Thread {} completed", i);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    println!("✓ All concurrent transcriptions completed");
}

#[test]
fn test_phase5_integration_module_exports() {
    // Verify all Phase 5 modules are accessible
    use whisper_hotkey::input::cgevent;
    use whisper_hotkey::transcription::TranscriptionEngine;

    // Type checks (compile-time verification)
    let _: fn(&str) -> bool = cgevent::insert_text_safe;

    // Ensure TranscriptionEngine types are available
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<TranscriptionEngine>();
}
