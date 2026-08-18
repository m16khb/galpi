use super::{MAX_LINE_BYTES, read_bounded_line};
use tokio::io::BufReader;

#[tokio::test]
async fn rejects_oversized_line_before_unbounded_growth() {
    let bytes = vec![b'x'; MAX_LINE_BYTES + 1];
    let mut reader = BufReader::new(bytes.as_slice());
    let mut buffer = Vec::new();
    let result = read_bounded_line(&mut reader, &mut buffer).await;

    assert!(result.is_err());
    assert!(buffer.len() <= MAX_LINE_BYTES);
}
