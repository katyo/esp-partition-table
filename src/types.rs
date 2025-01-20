use crate::PartitionError;

/// Partition type and subtype
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PartitionType {
    /// Application partition
    App(AppPartitionType),

    /// Data partition
    Data(DataPartitionType),

    /// Any type
    #[default]
    Any,

    /// User-defined type
    User(u8, u8),
}

impl From<AppPartitionType> for PartitionType {
    fn from(subtype: AppPartitionType) -> Self {
        Self::App(subtype)
    }
}

impl From<DataPartitionType> for PartitionType {
    fn from(subtype: DataPartitionType) -> Self {
        Self::Data(subtype)
    }
}

impl TryFrom<(u8, u8)> for PartitionType {
    type Error = PartitionError;

    fn try_from((raw_type, raw_subtype): (u8, u8)) -> Result<Self, Self::Error> {
        Ok(match raw_type {
            0x00 => Self::App(raw_subtype.try_into()?),
            0x01 => Self::Data(raw_subtype.try_into()?),
            0x40..=0xfe => Self::User(raw_type, raw_subtype),
            0xff => Self::Any,
            _ => return Err(PartitionError::InvalidType(raw_type)),
        })
    }
}

impl TryFrom<&[u8; 2]> for PartitionType {
    type Error = PartitionError;

    fn try_from([raw_type, raw_subtype]: &[u8; 2]) -> Result<Self, Self::Error> {
        (*raw_type, *raw_subtype).try_into()
    }
}

impl TryFrom<[u8; 2]> for PartitionType {
    type Error = PartitionError;

    fn try_from([raw_type, raw_subtype]: [u8; 2]) -> Result<Self, Self::Error> {
        (raw_type, raw_subtype).try_into()
    }
}

impl TryFrom<&[u8]> for PartitionType {
    type Error = PartitionError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        <[u8; 2]>::try_from(slice)
            .map_err(|_| PartitionError::NotEnoughData)?
            .try_into()
    }
}

impl TryFrom<PartitionType> for (u8, u8) {
    type Error = PartitionError;

    fn try_from(ty: PartitionType) -> Result<Self, Self::Error> {
        use PartitionType::*;
        Ok(match ty {
            App(subtype) => (0x00, subtype.try_into()?),
            Data(subtype) => (0x01, subtype.into()),
            User(usertype @ 0x40..=0xfe, subtype) => (usertype, subtype),
            User(usertype, _) => return Err(PartitionError::InvalidUserType(usertype)),
            Any => (0xff, 0x00),
        })
    }
}

impl TryFrom<PartitionType> for [u8; 2] {
    type Error = PartitionError;

    fn try_from(ty: PartitionType) -> Result<Self, Self::Error> {
        let (type_, subtype) = ty.try_into()?;
        Ok([type_, subtype])
    }
}

impl PartitionType {
    /// Application partition alignment
    const APP_ALIGN: u32 = 0x10000;

    /// Data partition alignment
    const DATA_ALIGN: u32 = 0x1000;

    /// Get partition alignment
    pub fn align(&self) -> u32 {
        match self {
            PartitionType::App(_) => Self::APP_ALIGN,
            _ => Self::DATA_ALIGN,
        }
    }

    /// Check offset for alignment
    pub fn check_offset(&self, offset: u32) -> Result<(), PartitionError> {
        if offset & (self.align() - 1) == 0 {
            Ok(())
        } else {
            Err(PartitionError::InvalidAlignment)
        }
    }

    /// Convert type and subtype from binary representation
    pub fn from_bytes(data: &[u8; 2]) -> Result<Self, PartitionError> {
        data.try_into()
    }

    /// Convert type and subtype to binary representation
    pub fn to_bytes(&self, data: &mut [u8; 2]) -> Result<(), PartitionError> {
        let (type_, subtype) = (*self).try_into()?;
        data[0] = type_;
        data[1] = subtype;
        Ok(())
    }
}

/// Application partition subtype
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AppPartitionType {
    /// Factory application
    #[default]
    Factory,

    /// OTA application
    Ota(u8),

    /// Test application
    Test,
}

impl TryFrom<u8> for AppPartitionType {
    type Error = PartitionError;

    fn try_from(raw: u8) -> Result<Self, Self::Error> {
        Ok(match raw {
            0x00 => Self::Factory,
            0x10..=0x1f => Self::Ota(raw - 0x10),
            0x20 => Self::Test,
            _ => return Err(PartitionError::InvalidSubType(raw)),
        })
    }
}

impl TryFrom<AppPartitionType> for u8 {
    type Error = PartitionError;

    fn try_from(ty: AppPartitionType) -> Result<Self, Self::Error> {
        use AppPartitionType::*;
        Ok(match ty {
            Factory => 0x00,
            Ota(number @ 0x00..=0x0f) => number + 0x10,
            Ota(number) => return Err(PartitionError::InvalidOtaNumber(number)),
            Test => 0x20,
        })
    }
}

/// Data partition subtype
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DataPartitionType {
    /// OTA data
    #[default]
    Ota = 0x00,

    /// Phy data
    Phy = 0x01,

    /// Non-volatile storage data
    Nvs = 0x02,

    /// Core dump
    CoreDump = 0x03,

    /// Encrypted non-volatile storage keys
    NvsKeys = 0x04,

    /// Efuse data
    EfuseEm = 0x05,

    /// Undefined data
    Undefined = 0x06,

    /// ESP HTTPd data
    EspHttpd = 0x80,

    /// FAT partition
    Fat = 0x81,

    /// SPIFFS partition
    SpiFfs = 0x82,

    /// LittleFS partition
    LittleFS = 0x83,
}

impl TryFrom<u8> for DataPartitionType {
    type Error = PartitionError;

    fn try_from(raw: u8) -> Result<Self, Self::Error> {
        Ok(match raw {
            0x00 => Self::Ota,
            0x01 => Self::Phy,
            0x02 => Self::Nvs,
            0x03 => Self::CoreDump,
            0x04 => Self::NvsKeys,
            0x05 => Self::EfuseEm,
            0x06 => Self::Undefined,
            0x80 => Self::EspHttpd,
            0x81 => Self::Fat,
            0x82 => Self::SpiFfs,
            0x83 => Self::LittleFS,
            _ => return Err(PartitionError::InvalidSubType(raw)),
        })
    }
}

impl From<DataPartitionType> for u8 {
    fn from(ty: DataPartitionType) -> Self {
        ty as _
    }
}
