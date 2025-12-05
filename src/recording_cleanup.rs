use crate::config::RecordingConfig;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Clean up old recordings based on retention policy
///
/// Deletes recordings older than `retention_days` OR beyond `max_count` limit.
/// Returns the number of files deleted.
///
/// # Errors
/// Returns error if directory listing fails. Individual file deletion failures are logged but don't stop cleanup.
pub fn cleanup_old_recordings(config: &RecordingConfig) -> Result<usize> {
    let debug_dir = get_debug_dir()?;

    // If directory doesn't exist, nothing to clean
    if !debug_dir.exists() {
        tracing::debug!("debug directory does not exist, skipping cleanup");
        return Ok(0);
    }

    // Collect all recording files with their timestamps
    let mut recordings: Vec<(PathBuf, u64)> = fs::read_dir(&debug_dir)
        .context("failed to read debug directory")?
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }

            let filename = path.file_name()?.to_str()?;
            if !filename.starts_with("recording_")
                || !path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
            {
                return None;
            }

            // Extract timestamp from filename: recording_{timestamp}.wav
            let timestamp_str = filename.strip_prefix("recording_")?.strip_suffix(".wav")?;
            let timestamp: u64 = timestamp_str.parse().ok()?;

            Some((path, timestamp))
        })
        .collect();

    if recordings.is_empty() {
        tracing::debug!("no recordings found, skipping cleanup");
        return Ok(0);
    }

    // Sort by timestamp (newest first)
    recordings.sort_by(|a, b| b.1.cmp(&a.1));

    let mut to_delete = HashSet::new();

    // Apply age-based retention
    if config.retention_days > 0 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("failed to get current time")?
            .as_secs();
        let retention_secs = u64::from(config.retention_days) * 24 * 60 * 60;

        for (path, timestamp) in &recordings {
            if now.saturating_sub(*timestamp) > retention_secs {
                to_delete.insert(path.clone());
            }
        }
    }

    // Apply count-based retention
    if config.max_count > 0 && recordings.len() > config.max_count {
        for (path, _) in recordings.iter().skip(config.max_count) {
            to_delete.insert(path.clone());
        }
    }

    // Delete files
    let mut deleted_count = 0;
    for path in to_delete {
        match fs::remove_file(&path) {
            Ok(()) => {
                deleted_count += 1;
                tracing::debug!("deleted recording: {}", path.display());
            }
            Err(e) => {
                tracing::warn!("failed to delete {}: {}", path.display(), e);
            }
        }
    }

    if deleted_count > 0 {
        tracing::debug!(
            "cleanup complete: deleted {} recordings (total: {}, remaining: {})",
            deleted_count,
            recordings.len(),
            recordings.len() - deleted_count
        );
    }

    Ok(deleted_count)
}

fn get_debug_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".whisper-hotkey").join("debug"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Shared mutex for all tests that modify HOME
    static HOME_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn create_test_dir() -> PathBuf {
        let temp_base = std::env::temp_dir();
        let test_dir = temp_base.join(format!(
            "whisper_cleanup_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&test_dir).unwrap();
        test_dir
    }

    fn create_recording(dir: &Path, timestamp: u64) -> PathBuf {
        let path = dir.join(format!("recording_{timestamp}.wav"));
        fs::write(&path, b"fake wav data").unwrap();
        path
    }

    #[test]
    fn test_get_debug_dir() {
        let dir = get_debug_dir().unwrap();
        assert!(dir.to_string_lossy().contains(".whisper-hotkey/debug"));
    }

    #[test]
    fn test_cleanup_empty_directory() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();
        let test_dir = create_test_dir();

        // Save original HOME
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", test_dir.to_str().unwrap());

        // Create debug dir but keep it empty
        let debug_dir = test_dir.join(".whisper-hotkey/debug");
        fs::create_dir_all(&debug_dir).unwrap();

        let config = RecordingConfig {
            enabled: true,
            retention_days: 7,
            max_count: 100,
            cleanup_interval_hours: 1,
        };

        let deleted = cleanup_old_recordings(&config).unwrap();
        assert_eq!(deleted, 0);

        // Restore HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cleanup_missing_directory() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();
        let test_dir = create_test_dir();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", test_dir.to_str().unwrap());

        // Don't create debug dir

        let config = RecordingConfig {
            enabled: true,
            retention_days: 7,
            max_count: 100,
            cleanup_interval_hours: 1,
        };

        let deleted = cleanup_old_recordings(&config).unwrap();
        assert_eq!(deleted, 0);

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cleanup_age_based() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();
        let test_dir = create_test_dir();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", test_dir.to_str().unwrap());

        let debug_dir = test_dir.join(".whisper-hotkey/debug");
        fs::create_dir_all(&debug_dir).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create old recording (8 days ago)
        let old_ts = now - (8 * 24 * 60 * 60);
        create_recording(&debug_dir, old_ts);

        // Create recent recording (1 day ago)
        let recent_ts = now - (24 * 60 * 60);
        create_recording(&debug_dir, recent_ts);

        let config = RecordingConfig {
            enabled: true,
            retention_days: 7,
            max_count: 0,
            cleanup_interval_hours: 1,
        };

        let deleted = cleanup_old_recordings(&config).unwrap();
        assert_eq!(deleted, 1);

        // Verify old file deleted, recent remains
        assert!(!debug_dir.join(format!("recording_{old_ts}.wav")).exists());
        assert!(debug_dir
            .join(format!("recording_{recent_ts}.wav"))
            .exists());

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cleanup_count_based() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();
        let test_dir = create_test_dir();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", test_dir.to_str().unwrap());

        let debug_dir = test_dir.join(".whisper-hotkey/debug");
        fs::create_dir_all(&debug_dir).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create 5 recordings
        let timestamps: Vec<u64> = (0..5).map(|i| now - (i * 60)).collect();
        for ts in &timestamps {
            create_recording(&debug_dir, *ts);
        }

        let config = RecordingConfig {
            enabled: true,
            retention_days: 0,
            max_count: 3,
            cleanup_interval_hours: 1,
        };

        let deleted = cleanup_old_recordings(&config).unwrap();
        assert_eq!(deleted, 2);

        // Verify 3 most recent remain
        assert!(debug_dir
            .join(format!("recording_{}.wav", timestamps[0]))
            .exists());
        assert!(debug_dir
            .join(format!("recording_{}.wav", timestamps[1]))
            .exists());
        assert!(debug_dir
            .join(format!("recording_{}.wav", timestamps[2]))
            .exists());

        // Verify 2 oldest deleted
        assert!(!debug_dir
            .join(format!("recording_{}.wav", timestamps[3]))
            .exists());
        assert!(!debug_dir
            .join(format!("recording_{}.wav", timestamps[4]))
            .exists());

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cleanup_both_policies() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();
        let test_dir = create_test_dir();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", test_dir.to_str().unwrap());

        let debug_dir = test_dir.join(".whisper-hotkey/debug");
        fs::create_dir_all(&debug_dir).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create old file (will be deleted by age)
        let old_ts = now - (10 * 24 * 60 * 60);
        create_recording(&debug_dir, old_ts);

        // Create 4 recent files (1 will be deleted by count)
        for i in 0..4 {
            create_recording(&debug_dir, now - (i * 60));
        }

        let config = RecordingConfig {
            enabled: true,
            retention_days: 7,
            max_count: 3,
            cleanup_interval_hours: 1,
        };

        let deleted = cleanup_old_recordings(&config).unwrap();
        assert_eq!(deleted, 2); // 1 old + 1 exceeding count

        // Verify 3 most recent remain
        let remaining = fs::read_dir(&debug_dir).unwrap().count();
        assert_eq!(remaining, 3);

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cleanup_zero_values_no_deletion() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();
        let test_dir = create_test_dir();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", test_dir.to_str().unwrap());

        let debug_dir = test_dir.join(".whisper-hotkey/debug");
        fs::create_dir_all(&debug_dir).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create old file
        create_recording(&debug_dir, now - (30 * 24 * 60 * 60));

        // Create many files
        for i in 0..10 {
            create_recording(&debug_dir, now - (i * 60));
        }

        let config = RecordingConfig {
            enabled: true,
            retention_days: 0,
            max_count: 0,
            cleanup_interval_hours: 0,
        };

        let deleted = cleanup_old_recordings(&config).unwrap();
        assert_eq!(deleted, 0);

        // All files should remain
        let remaining = fs::read_dir(&debug_dir).unwrap().count();
        assert_eq!(remaining, 11);

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cleanup_ignores_non_recording_files() {
        let _guard = HOME_TEST_LOCK.lock().unwrap();
        let test_dir = create_test_dir();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", test_dir.to_str().unwrap());

        let debug_dir = test_dir.join(".whisper-hotkey/debug");
        fs::create_dir_all(&debug_dir).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create valid recording
        create_recording(&debug_dir, now - (10 * 24 * 60 * 60));

        // Create files that should be ignored
        fs::write(debug_dir.join("other_file.wav"), b"data").unwrap();
        fs::write(debug_dir.join("recording.txt"), b"data").unwrap();
        fs::write(debug_dir.join("recording_invalid.wav"), b"data").unwrap();

        let config = RecordingConfig {
            enabled: true,
            retention_days: 7,
            max_count: 0,
            cleanup_interval_hours: 1,
        };

        let deleted = cleanup_old_recordings(&config).unwrap();
        assert_eq!(deleted, 1); // Only the valid old recording

        // Other files should still exist
        assert!(debug_dir.join("other_file.wav").exists());
        assert!(debug_dir.join("recording.txt").exists());
        assert!(debug_dir.join("recording_invalid.wav").exists());

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        let _ = fs::remove_dir_all(&test_dir);
    }
}
