//! Header bitmask helpers and response/error decoding, mirroring the bit
//! layout documented in the Kortex `Frame.proto`.

use prost::Message;

use crate::api::{Error as KortexError, Frame, Header};

pub const MSG_FRAME_REQUEST: u32 = 1;
pub const MSG_FRAME_RESPONSE: u32 = 3;
#[allow(dead_code)] // notifications are not subscribed to in this crate
pub const MSG_FRAME_NOTIFICATION: u32 = 5;

const MASK_HEADER_VERSION: u32 = 0xF000_0000;
const MASK_FRAME_TYPE: u32 = 0x0F00_0000;
const MASK_DEVICE_ID: u32 = 0x00FF_0000;
const MASK_ERROR_CODE: u32 = 0x0000_F000;
const MASK_ERROR_SUB_CODE: u32 = 0x0000_0FFF;

const MASK_SERVICE_VERSION: u32 = 0xF000_0000;

const MASK_SESSION_ID: u32 = 0xFFFF_0000;
pub const MASK_MESSAGE_ID: u32 = 0x0000_FFFF;

pub const ERROR_NONE: u32 = 0;

pub fn frame_type(header: &Header) -> u32 {
    (header.frame_info & MASK_FRAME_TYPE) >> 24
}

pub fn error_code(header: &Header) -> u32 {
    (header.frame_info & MASK_ERROR_CODE) >> 12
}

pub fn error_sub_code(header: &Header) -> u32 {
    header.frame_info & MASK_ERROR_SUB_CODE
}

pub fn session_id(header: &Header) -> u32 {
    (header.message_info & MASK_SESSION_ID) >> 16
}

pub fn message_id(header: &Header) -> u16 {
    (header.message_info & MASK_MESSAGE_ID) as u16
}

/// If the response carries a non-zero error code, decodes the embedded
/// `Kinova.Api.Error` payload into a [`RobotError`].
pub fn decode_response_error(frame: &Frame) -> Option<RobotError> {
    let header = frame.header.as_ref()?;
    if error_code(header) == ERROR_NONE {
        return None;
    }

    let mut err = KortexError {
        error_code: error_code(header) as i32,
        error_sub_code: error_sub_code(header),
        error_sub_string: String::new(),
    };
    if let Ok(decoded) = KortexError::decode(frame.payload.as_slice())
        && decoded.error_code != 0
    {
        err = decoded;
    }

    Some(RobotError::Robot {
        code: err.error_code,
        sub_code: err.error_sub_code,
        message: err.error_sub_string,
    })
}

/// Error returned by the Kortex protocol layer.
#[derive(Debug, thiserror::Error)]
pub enum RobotError {
    #[error("not connected to robot")]
    NotConnected,
    #[error("robot rejected the request: code={code} sub={sub_code} {message}")]
    Robot {
        code: i32,
        sub_code: u32,
        message: String,
    },
    #[error("frame error: {0}")]
    Frame(#[from] crate::robot::transport::FrameError),
    #[error("protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("request timed out")]
    Timeout,
    #[error("session not established: {0}")]
    Session(&'static str),
    #[error("reader task is no longer running")]
    ReaderGone,
    #[error("action not found: {0}")]
    ActionNotFound(String),
    #[error("{0}")]
    Other(String),
}

/// The service version all Kortex 2.3 services run at.
pub const SERVICE_VERSION: u32 = 1;

// Unused mask keeps the bit layout self-documenting.
#[allow(dead_code)]
const _: () = {
    let _ = MASK_HEADER_VERSION;
    let _ = MASK_DEVICE_ID;
    let _ = MASK_SERVICE_VERSION;
};
