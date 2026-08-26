use super::capture::{f32_to_i16, f64_to_i16, is_recoverable_stream_error};
use cpal::ErrorKind;

#[test]
fn converts_and_clamps_float_samples_for_pcm_wav() {
    assert_eq!(f32_to_i16(-2.0), i16::MIN);
    assert_eq!(f32_to_i16(-1.0), i16::MIN);
    assert_eq!(f32_to_i16(0.0), 0);
    assert_eq!(f32_to_i16(1.0), i16::MAX);
    assert_eq!(f32_to_i16(2.0), i16::MAX);
    assert_eq!(f32_to_i16(f32::NAN), 0);
    assert_eq!(f64_to_i16(f64::INFINITY), 0);
}

#[test]
fn treats_core_audio_xrun_as_recoverable() {
    assert!(is_recoverable_stream_error(ErrorKind::Xrun));
    assert!(!is_recoverable_stream_error(ErrorKind::DeviceNotAvailable));
}

/// Skipped where `pmset` is unavailable or restricted (sandboxed CI images);
/// run it with `cargo test -- --ignored` on a real desktop session.
#[test]
#[ignore = "reads the live system assertion table through pmset"]
fn sleep_blocker_holds_and_releases_a_system_sleep_assertion() -> Result<(), String> {
    // Given: a distinctive assertion name to find in the system assertion table
    let name = "galpi-test-recording-sleep-blocker";

    // When: the blocker is held
    let blocker = super::power::SleepBlocker::acquire(name)
        .ok_or("power assertion should be created on macOS")?;
    let held = pmset_assertions()?;

    // Then: the system reports the named assertion while held and drops it on release
    assert!(held.contains(name), "assertion missing while held:\n{held}");
    drop(blocker);
    let released = pmset_assertions()?;
    assert!(
        !released.contains(name),
        "assertion leaked after drop:\n{released}"
    );
    Ok(())
}

fn pmset_assertions() -> Result<String, String> {
    let output = std::process::Command::new("pmset")
        .args(["-g", "assertions"])
        .output()
        .map_err(|error| format!("pmset should run on macOS: {error}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
