use core::fmt;

/// Partition manipulation error
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartitionError {
    /// Magic bytes is not a valid
    InvalidMagic,

    /// Partition type is not a valid
    InvalidType(u8),

    /// Partition subtype is not a valid
    InvalidSubType(u8),

    /// User-defined type is not a valid
    InvalidUserType(u8),

    /// OTA partition number is not a valid
    InvalidOtaNumber(u8),

    /// String data is not a valid
    InvalidString,

    /// Partition alignment is not a valid
    InvalidAlignment,

    /// MD5 checksum is not a valid
    InvalidMd5,

    /// Not enough data
    NotEnoughData,

    /// Too many data
    TooManyData,
}

impl fmt::Display for PartitionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use PartitionError::*;
        match self {
            InvalidMagic => "Invalid magic".fmt(f),
            InvalidType(ty) => {
                "Invalid type: ".fmt(f)?;
                ty.fmt(f)
            }
            InvalidSubType(ty) => {
                "Invalid sub type: ".fmt(f)?;
                ty.fmt(f)
            }
            InvalidUserType(ty) => {
                "Invalid user type: ".fmt(f)?;
                ty.fmt(f)
            }
            InvalidOtaNumber(no) => {
                "Invalid OTA: #".fmt(f)?;
                no.fmt(f)
            }
            InvalidString => "Invalid string".fmt(f),
            InvalidAlignment => "Invalid alignment".fmt(f),
            InvalidMd5 => "Invalid MD5".fmt(f),
            NotEnoughData => "Not enough data".fmt(f),
            TooManyData => "Too many data".fmt(f),
        }
    }
}
