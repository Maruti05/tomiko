use thiserror::Error;

/// Native error type. All JNI exports map this to an `i32` code; no Rust error
/// or panic text crosses the JNI boundary directly.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EngineError {
    #[error("engine is not initialized")]
    NotInitialized,

    #[error("engine is already initialized")]
    AlreadyInitialized,

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("audio initialization failed: {0}")]
    AudioInit(String),

    #[error("audio device error: {0}")]
    AudioDevice(String),

    #[error("invalid character state transition: {0}")]
    InvalidTransition(&'static str),

    #[error("command channel is closed")]
    ChannelClosed,

    #[error("I/O error: {0}")]
    Io(String),

    #[error("asset error: {0}")]
    Asset(String),

    #[error("JNI error: {0}")]
    Jni(String),

    #[error("native panic: {0}")]
    Panic(String),

    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
}

/// Stable numeric error codes shared with Kotlin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    Ok = 0,
    NotInitialized = 1,
    AlreadyInitialized = 2,
    InvalidConfig = 3,
    AudioInit = 4,
    AudioDevice = 5,
    InvalidTransition = 6,
    ChannelClosed = 7,
    Io = 8,
    Asset = 9,
    Jni = 10,
    Panic = 11,
    Unsupported = 12,
}

impl ErrorCode {
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NotInitialized => "engine is not initialized",
            Self::AlreadyInitialized => "engine is already initialized",
            Self::InvalidConfig => "invalid configuration",
            Self::AudioInit => "audio initialization failed",
            Self::AudioDevice => "audio device error",
            Self::InvalidTransition => "invalid character state transition",
            Self::ChannelClosed => "command channel is closed",
            Self::Io => "I/O error",
            Self::Asset => "asset error",
            Self::Jni => "JNI error",
            Self::Panic => "native panic",
            Self::Unsupported => "unsupported operation",
        }
    }
}

impl From<&EngineError> for ErrorCode {
    fn from(error: &EngineError) -> Self {
        match error {
            EngineError::NotInitialized => Self::NotInitialized,
            EngineError::AlreadyInitialized => Self::AlreadyInitialized,
            EngineError::InvalidConfig(_) => Self::InvalidConfig,
            EngineError::AudioInit(_) => Self::AudioInit,
            EngineError::AudioDevice(_) => Self::AudioDevice,
            EngineError::InvalidTransition(_) => Self::InvalidTransition,
            EngineError::ChannelClosed => Self::ChannelClosed,
            EngineError::Io(_) => Self::Io,
            EngineError::Asset(_) => Self::Asset,
            EngineError::Jni(_) => Self::Jni,
            EngineError::Panic(_) => Self::Panic,
            EngineError::Unsupported(_) => Self::Unsupported,
        }
    }
}

impl From<&EngineError> for i32 {
    fn from(error: &EngineError) -> Self {
        ErrorCode::from(error) as i32
    }
}

impl From<EngineError> for i32 {
    fn from(error: EngineError) -> Self {
        (&error).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(i32::from(EngineError::NotInitialized), 1);
        assert_eq!(i32::from(EngineError::AlreadyInitialized), 2);
        assert_eq!(i32::from(EngineError::InvalidConfig("x".into())), 3);
        assert_eq!(i32::from(EngineError::AudioInit("x".into())), 4);
        assert_eq!(i32::from(EngineError::AudioDevice("x".into())), 5);
        assert_eq!(i32::from(EngineError::InvalidTransition("x")), 6);
        assert_eq!(i32::from(EngineError::ChannelClosed), 7);
        assert_eq!(i32::from(EngineError::Io("x".into())), 8);
        assert_eq!(i32::from(EngineError::Asset("x".into())), 9);
        assert_eq!(i32::from(EngineError::Jni("x".into())), 10);
        assert_eq!(i32::from(EngineError::Panic("x".into())), 11);
        assert_eq!(i32::from(EngineError::Unsupported("x")), 12);
    }

    #[test]
    fn every_error_has_a_message() {
        let errors = [
            EngineError::NotInitialized,
            EngineError::AlreadyInitialized,
            EngineError::InvalidConfig("x".into()),
            EngineError::AudioInit("x".into()),
            EngineError::AudioDevice("x".into()),
            EngineError::InvalidTransition("x"),
            EngineError::ChannelClosed,
            EngineError::Io("x".into()),
            EngineError::Asset("x".into()),
            EngineError::Jni("x".into()),
            EngineError::Panic("x".into()),
            EngineError::Unsupported("x"),
        ];
        for error in errors {
            let code = ErrorCode::from(&error);
            assert_ne!(code, ErrorCode::Ok);
            assert!(!code.message().is_empty());
            assert!(!error.to_string().is_empty());
        }
    }
}
