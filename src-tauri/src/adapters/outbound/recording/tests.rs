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
