use super::prepare_job_directory;
use uuid::Uuid;

#[tokio::test]
async fn seeds_new_job_with_latest_matching_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("galpi-checkpoint-{}", Uuid::now_v7()));
    let input = root.join("meeting.wav");
    let output = root.join("output");
    let previous = output.join(format!("meeting-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&previous)?;
    std::fs::write(&input, b"audio")?;
    std::fs::write(
        previous.join("meeting.aligned.v2.json"),
        b"{\"segments\":[]}",
    )?;

    let (_input, job) = prepare_job_directory(&input, &output).await?;

    assert_eq!(
        std::fs::read(job.join("meeting.aligned.v2.json"))?,
        b"{\"segments\":[]}"
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}
