//! Wire-format framing for the Kortex TCP transport.
//!
//! Every message on the wire is a protobuf [`Frame`] preceded by a fixed
//! header: the magic string `\x07xEtRoK\x07` followed by the big-endian
//! payload length (4 bytes).

use std::io;

use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::api::{Frame, Header};

/// `\x07xEtRoK\x07`
const MAGIC: [u8; 8] = [0x07, 0x78, 0x45, 0x74, 0x52, 0x6f, 0x4b, 0x07];
const LENGTH_FIELD_SIZE: usize = 4;
const HEADER_SIZE: usize = MAGIC.len() + LENGTH_FIELD_SIZE;

/// Largest payload we are willing to accept from the robot (64 KiB).
const MAX_PAYLOAD_SIZE: usize = 64 * 1024;

/// Serializes a `Frame` with the Kortex framing applied.
pub fn encode_frame(frame: &Frame) -> Result<Vec<u8>, prost::EncodeError> {
    let mut payload = Vec::new();
    frame.encode(&mut payload)?;

    let mut out = Vec::with_capacity(HEADER_SIZE + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Reads one length-prefixed `Frame` from the stream.
pub async fn read_frame<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Frame, FrameError> {
    let mut head = [0u8; HEADER_SIZE];
    reader.read_exact(&mut head).await.map_err(FrameError::Io)?;

    if head[..MAGIC.len()] != MAGIC {
        return Err(FrameError::BadMagic);
    }

    let len = u32::from_be_bytes(head[MAGIC.len()..HEADER_SIZE].try_into().unwrap()) as usize;
    if len > MAX_PAYLOAD_SIZE {
        return Err(FrameError::Oversized(len));
    }

    let mut payload = vec![0u8; len];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(FrameError::Io)?;
    Frame::decode(payload.as_slice()).map_err(FrameError::Decode)
}

/// Writes a frame with Kortex framing applied.
pub async fn write_frame<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    frame: &Frame,
) -> Result<(), FrameError> {
    let encoded = encode_frame(frame).map_err(FrameError::Encode)?;
    writer.write_all(&encoded).await.map_err(FrameError::Io)
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("transport error: {0}")]
    Io(#[from] io::Error),
    #[error("frame is missing the Kortex magic header")]
    BadMagic,
    #[error("frame payload is too large: {0} bytes")]
    Oversized(usize),
    #[error("failed to encode frame: {0}")]
    Encode(prost::EncodeError),
    #[error("failed to decode frame payload: {0}")]
    Decode(prost::DecodeError),
}

/// Builds a request `Frame` for a service RPC.
///
/// `service_uid` encodes the service id and function id in its low 28 bits,
/// matching the Kortex `service_info` layout (`version << 28 | uid`).
pub fn request_frame(
    service_version: u32,
    service_uid: u32,
    session_id: u32,
    msg_id: u16,
    payload: Option<Vec<u8>>,
) -> Frame {
    let mut header = Header {
        frame_info: 0,
        message_info: 0,
        service_info: 0,
        payload_info: 0,
    };

    header.frame_info |= (1 << 28)              // header version
        | (super::router::MSG_FRAME_REQUEST << 24); // frame type (device id 0, no error)
    header.service_info = (service_version << 28) | (service_uid & 0x0fff_ffff);
    header.message_info = (session_id << 16) | u32::from(msg_id);

    let payload = payload.unwrap_or_default();
    header.payload_info = (payload.len() as u32) & 0x00ff_ffff;

    Frame {
        header: Some(header),
        payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_is_correct() {
        assert_eq!(&MAGIC, b"\x07xEtRoK\x07");
    }

    #[test]
    fn header_size_is_twelve() {
        assert_eq!(HEADER_SIZE, 12);
    }

    #[tokio::test]
    async fn frame_roundtrip_preserves_correlation_and_payload() {
        let frame = request_frame(1, 0x200ea, 7, 42, Some(vec![1, 2, 3]));
        let bytes = encode_frame(&frame).unwrap();
        let decoded = read_frame(&mut bytes.as_slice()).await.unwrap();
        assert_eq!(decoded, frame);
        assert_eq!(
            super::super::router::message_id(decoded.header.as_ref().unwrap()),
            42
        );
    }

    #[tokio::test]
    async fn rejects_bad_magic_oversized_and_truncated_frames() {
        assert!(matches!(
            read_frame(&mut [0u8; 12].as_slice()).await,
            Err(FrameError::BadMagic)
        ));
        let bytes = [
            MAGIC.as_slice(),
            &((MAX_PAYLOAD_SIZE + 1) as u32).to_be_bytes(),
        ]
        .concat();
        assert!(matches!(
            read_frame(&mut bytes.as_slice()).await,
            Err(FrameError::Oversized(_))
        ));
        let mut bytes = encode_frame(&request_frame(1, 1, 1, 1, Some(vec![1]))).unwrap();
        bytes.pop();
        assert!(matches!(
            read_frame(&mut bytes.as_slice()).await,
            Err(FrameError::Io(_))
        ));
    }
}
