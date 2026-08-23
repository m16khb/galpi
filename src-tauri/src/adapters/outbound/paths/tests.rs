use super::{prepare_job_directory, sanitize_name};
use uuid::Uuid;

fn temp_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("galpi-{label}-{}", Uuid::now_v7()))
}

#[tokio::test]
async fn creates_a_meeting_folder_named_after_the_recording()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an imported audio file
    let root = temp_root("folder");
    let input = root.join("팀 미팅.m4a");
    let output = root.join("galpi");
    std::fs::create_dir_all(&root)?;
    std::fs::write(&input, b"audio")?;

    // When
    let (_input, job) = prepare_job_directory(&input, &output).await?;

    // Then: the meeting folder carries the recording name, no uuid suffix
    assert_eq!(job, output.join("팀 미팅").canonicalize()?);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[tokio::test]
async fn reuses_the_folder_a_recording_already_lives_in() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: a Galpi recording inside its own meeting folder
    let root = temp_root("reuse");
    let output = root.join("galpi");
    let folder = output.join("2026-08-24 143052 녹음");
    std::fs::create_dir_all(&folder)?;
    let input = folder.join("2026-08-24 143052 녹음.wav");
    std::fs::write(&input, b"audio")?;

    // When
    let (_input, job) = prepare_job_directory(&input, &output).await?;

    // Then: transcription targets the recording's own folder
    assert_eq!(job, folder.canonicalize()?);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[tokio::test]
async fn deduplicates_colliding_meeting_folders() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a meeting folder with the same name already exists
    let root = temp_root("dedup");
    let input = root.join("meeting.wav");
    let output = root.join("galpi");
    std::fs::create_dir_all(output.join("meeting"))?;
    std::fs::write(&input, b"audio")?;

    // When
    let (_input, job) = prepare_job_directory(&input, &output).await?;

    // Then
    assert_eq!(job, output.join("meeting 2").canonicalize()?);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[tokio::test]
async fn seeds_new_job_with_latest_matching_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
    // Given: an earlier meeting for the same audio name holds a checkpoint
    let root = temp_root("checkpoint");
    let input = root.join("meeting.wav");
    let output = root.join("galpi");
    let previous = output.join("meeting 2");
    std::fs::create_dir_all(&previous)?;
    std::fs::write(&input, b"audio")?;
    std::fs::write(
        previous.join("meeting.aligned.v2.json"),
        b"{\"segments\":[]}",
    )?;

    // When
    let (_input, job) = prepare_job_directory(&input, &output).await?;

    // Then: the checkpoint is reused even though the folder names differ
    assert_eq!(
        std::fs::read(job.join("meeting.aligned.v2.json"))?,
        b"{\"segments\":[]}"
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn sanitize_name_keeps_spaces_and_hangul() {
    assert_eq!(sanitize_name(" 팀 미팅: 8월 "), "팀 미팅- 8월");
}
