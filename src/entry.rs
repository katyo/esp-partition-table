use crate::{utils, PartitionError, PartitionType};

#[cfg(feature = "heapless")]
use heapless::String;

/// Data buffer for partition entry
pub type PartitionBuffer = [u8; PartitionEntry::SIZE];

/// ESP Partition info
///
/// Binary representation:
///
/// Off | Len | Desc
/// --- | --- | ----
///   0 |   2 | Magic
///   2 |   1 | Type
///   3 |   1 | SubType
///   4 |   4 | Offset
///   8 |   4 | Size
///  12 |  16 | Name
///  28 |   4 | Flags
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartitionEntry {
    /// Partition type and subtype
    pub type_: PartitionType,

    /// Partition offset
    pub offset: u32,

    /// Partition size
    pub size: usize,

    /// Partition name
    #[cfg(feature = "heapless")]
    pub name: String<{ Self::MAX_NAME_LEN }>,

    #[cfg(not(feature = "heapless"))]
    name: [u8; Self::MAX_NAME_LEN],

    /// Partition encrypted flag
    pub encrypted: bool,
}

impl PartitionEntry {
    /// Magic bytes at beginning binary partition representation
    pub const MAGIC: [u8; 2] = [0xAA, 0x50];

    /// The size of binary represented partition data
    pub const SIZE: usize = 32;

    /// Max partition name length
    pub const MAX_NAME_LEN: usize = 16;

    /// Create partition info
    pub fn new(
        type_: impl Into<PartitionType>,
        offset: u32,
        size: usize,
        name: impl AsRef<str>,
        encrypted: bool,
    ) -> Result<Self, PartitionError> {
        let name = name.as_ref();

        #[cfg(feature = "heapless")]
        let name = name.try_into().map_err(|_| PartitionError::InvalidString)?;

        #[cfg(not(feature = "heapless"))]
        let name = {
            let mut name_data = [0u8; Self::MAX_NAME_LEN];
            utils::name_into(&mut name_data, name)?;
            name_data
        };

        Ok(Self {
            type_: type_.into(),
            offset,
            size,
            name,
            encrypted,
        })
    }

    /// Set partition offset with alignment check
    pub fn set_offset(&mut self, offset: u32) -> Result<(), PartitionError> {
        self.type_.check_offset(offset)?;
        self.offset = offset;
        Ok(())
    }

    /// Get partition name
    pub fn name(&self) -> &str {
        #[cfg(not(feature = "heapless"))]
        {
            let name = utils::name_trim(&self.name);
            // utf8 data already validated
            unsafe { core::str::from_utf8_unchecked(name) }
        }

        #[cfg(feature = "heapless")]
        {
            &self.name
        }
    }

    /// Set partition name
    pub fn set_name(&mut self, name: impl AsRef<str>) -> Result<(), PartitionError> {
        let name = name.as_ref();

        #[cfg(feature = "heapless")]
        {
            self.name = name.try_into().map_err(|_| PartitionError::InvalidString)?;
            Ok(())
        }

        #[cfg(not(feature = "heapless"))]
        {
            utils::name_into(&mut self.name, name)
        }
    }

    /// Convert partition data from binary representation
    pub fn from_bytes(data: &PartitionBuffer) -> Result<Self, PartitionError> {
        let (magic, data) = data
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?;
        if magic != &Self::MAGIC {
            return Err(PartitionError::InvalidMagic);
        }

        let (type_data, data) = data
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?;
        let type_ = type_data.try_into()?;

        let (offset_data, data) = data
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?;
        let offset = u32::from_le_bytes(*offset_data);

        let (size_data, data) = data
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?;
        let size = u32::from_le_bytes(*size_data) as usize;

        let (name_data, data) = data
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?;
        let _name_str = utils::name_from(name_data)?;

        #[cfg(feature = "heapless")]
        let name = _name_str
            .try_into()
            .map_err(|_| PartitionError::TooManyData)?;

        #[cfg(not(feature = "heapless"))]
        let name = *name_data;

        let (flags_data, _) = data
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?;
        let flags = u32::from_le_bytes(*flags_data);

        let encrypted = flags & 0x01 != 0;

        Ok(Self {
            type_,
            offset,
            size,
            name,
            encrypted,
        })
    }

    /// Convert partition data to binary representation
    pub fn to_bytes(&self, data: &mut PartitionBuffer) -> Result<(), PartitionError> {
        self.type_.check_offset(self.offset)?;

        let (magic_data, data) = data
            .split_first_chunk_mut()
            .ok_or(PartitionError::NotEnoughData)?;
        *magic_data = Self::MAGIC;

        let (type_data, data) = data
            .split_first_chunk_mut()
            .ok_or(PartitionError::NotEnoughData)?;
        self.type_.to_bytes(type_data)?;

        let (offset_data, data) = data
            .split_first_chunk_mut()
            .ok_or(PartitionError::NotEnoughData)?;
        *offset_data = self.offset.to_le_bytes();

        let (size_data, data) = data
            .split_first_chunk_mut()
            .ok_or(PartitionError::NotEnoughData)?;
        *size_data = (self.size as u32).to_le_bytes();

        let (name_data, data) = data
            .split_first_chunk_mut()
            .ok_or(PartitionError::NotEnoughData)?;

        #[cfg(feature = "heapless")]
        utils::name_into(name_data, self.name.as_str())?;

        #[cfg(not(feature = "heapless"))]
        {
            *name_data = self.name;
        }

        let (flags_data, _) = data
            .split_first_chunk_mut()
            .ok_or(PartitionError::NotEnoughData)?;
        *flags_data = (self.encrypted as u32).to_le_bytes();

        Ok(())
    }
}

impl AsRef<PartitionEntry> for PartitionEntry {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl TryFrom<&[u8]> for PartitionEntry {
    type Error = PartitionError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        <&PartitionBuffer>::try_from(data)
            .map_err(|_| PartitionError::NotEnoughData)
            .and_then(Self::try_from)
    }
}

impl TryFrom<&PartitionBuffer> for PartitionEntry {
    type Error = PartitionError;

    fn try_from(data: &PartitionBuffer) -> Result<Self, Self::Error> {
        Self::from_bytes(data)
    }
}

/// MD5 checksum data
pub type Md5Data = [u8; 16];

/// ESP Partition MD5
///
/// Binary representation:
///
/// Off | Len | Desc
/// --- | --- | ----
///   0 |   2 | Magic
///   2 |  14 | Reserved
///  16 |  16 | MD5 data
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartitionMd5 {
    /// MD5 checksum data
    pub data: Md5Data,
}

impl From<PartitionMd5> for Md5Data {
    fn from(md5: PartitionMd5) -> Self {
        md5.data
    }
}

impl From<Md5Data> for PartitionMd5 {
    fn from(data: Md5Data) -> Self {
        Self { data }
    }
}

#[cfg(feature = "md5")]
impl From<md5::Digest> for PartitionMd5 {
    fn from(digest: md5::Digest) -> Self {
        Self {
            data: digest.into(),
        }
    }
}

impl PartitionMd5 {
    /// Magic bytes
    pub const MAGIC: [u8; 2] = [0xeb, 0xeb];

    /// The size of reserved space between magic bytes and MD5 data
    pub const RESERVED_SIZE: usize = 14;

    /// The content of reserved space between magic bytes and MD5 data
    pub const RESERVED_DATA: u8 = 0xff;

    /// Convert md5 data from binary representation
    pub fn from_bytes(data: &PartitionBuffer) -> Result<Self, PartitionError> {
        let (magic, data) = data
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?;
        if magic != &Self::MAGIC {
            return Err(PartitionError::InvalidMagic);
        }

        let (reserved_data, data) = data
            .split_first_chunk::<{ Self::RESERVED_SIZE }>()
            .ok_or(PartitionError::NotEnoughData)?;
        for reserved in reserved_data {
            if *reserved != Self::RESERVED_DATA {
                return Err(PartitionError::InvalidMagic);
            }
        }

        let (md5_data, _) = data
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?;

        Ok(Self { data: *md5_data })
    }

    /// Convert md5 data to binary representation
    pub fn to_bytes(&self, data: &mut PartitionBuffer) -> Result<(), PartitionError> {
        let (magic_data, data) = data
            .split_first_chunk_mut()
            .ok_or(PartitionError::NotEnoughData)?;
        *magic_data = Self::MAGIC;

        let (reserved_data, data) = data
            .split_first_chunk_mut::<{ Self::RESERVED_SIZE }>()
            .ok_or(PartitionError::NotEnoughData)?;
        reserved_data.fill(Self::RESERVED_DATA);

        let (md5_data, _) = data
            .split_first_chunk_mut()
            .ok_or(PartitionError::NotEnoughData)?;
        *md5_data = self.data;

        Ok(())
    }
}

impl TryFrom<&[u8]> for PartitionMd5 {
    type Error = PartitionError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        <&PartitionBuffer>::try_from(data)
            .map_err(|_| PartitionError::NotEnoughData)
            .and_then(Self::try_from)
    }
}

impl TryFrom<&PartitionBuffer> for PartitionMd5 {
    type Error = PartitionError;

    fn try_from(data: &PartitionBuffer) -> Result<Self, Self::Error> {
        Self::from_bytes(data)
    }
}
