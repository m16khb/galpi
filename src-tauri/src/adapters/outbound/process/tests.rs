use super::{MAX_LINE_BYTES, ProcessResult, handle_stdout, read_bounded_line};
use crate::application::error::AppError;
use crate::application::ports::JobEvents;
use crate::domain::worker::WorkerEvent;
use tokio::io::BufReader;
use uuid::Uuid;

struct IgnoredEvents;

impl JobEvents for IgnoredEvents {
    fn emit(&self, _job_id: Uuid, _event: WorkerEvent) -> Result<(), AppError> {
        Ok(())
    }
}

#[tokio::test]
async fn rejects_oversized_line_before_unbounded_growth() {
    let bytes = vec![b'x'; MAX_LINE_BYTES + 1];
    let mut reader = BufReader::new(bytes.as_slice());
    let mut buffer = Vec::new();
    let result = read_bounded_line(&mut reader, &mut buffer).await;

    assert!(result.is_err());
    assert!(buffer.len() <= MAX_LINE_BYTES);
}

#[test]
fn captures_refined_event_as_process_result() -> Result<(), AppError> {
    // Given
    let mut result = ProcessResult::default();
    let line = r#"{"v":1,"seq":3,"type":"refined","minutes":"/tmp/notes.md"}"#;

    // When
    handle_stdout(&IgnoredEvents, Uuid::nil(), line, true, &mut result)?;

    // Then
    assert_eq!(
        result.completed,
        Some(WorkerEvent::Refined {
            minutes: "/tmp/notes.md".to_owned(),
        })
    );
    Ok(())
}
