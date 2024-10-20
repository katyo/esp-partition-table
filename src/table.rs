use crate::{Md5Data, PartitionBuffer, PartitionEntry, PartitionError, PartitionMd5};

/// Partition table info
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionTable {
    /// Address of table
    pub addr: u32,

    /// Size of table
    pub size: usize,
}

impl PartitionTable {
    /// Address of partition table
    pub const DEFAULT_ADDR: u32 = 0x8000;

    /// Maximum size of partition table
    pub const MAX_SIZE: usize = 0x1000;

    /// Maxumum number of partition entries
    pub const MAX_ENTRIES: usize = Self::MAX_SIZE / PartitionEntry::SIZE;
}

impl Default for PartitionTable {
    fn default() -> Self {
        Self::new(Self::DEFAULT_ADDR, Self::MAX_SIZE)
    }
}

impl PartitionTable {
    /// Instantiate partition table with specified address and size
    pub fn new(addr: u32, size: usize) -> Self {
        Self { addr, size }
    }

    /// Get maximum number of entries
    pub fn max_entries(&self) -> usize {
        self.size / PartitionEntry::SIZE
    }
}

#[derive(Clone, Copy, Debug)]
enum InternalState {
    Init,
    Proc,
    Done,
}

/// Partition table reader state
#[derive(Clone)]
pub struct PartitionReaderState {
    offset: u32,
    end: u32,

    #[cfg(feature = "md5")]
    md5: Result<Md5Data, md5::Context>,

    #[cfg(feature = "md5")]
    calc_md5: bool,

    stored_md5: Option<Md5Data>,

    state: InternalState,
}

impl PartitionReaderState {
    /// Instantiate reader state
    ///
    /// If `md5` feature isn't enabled `calc_md5` argument will be ignored.
    pub fn new(offset: u32, length: usize, calc_md5: bool) -> Self {
        #[cfg(not(feature = "md5"))]
        let _ = calc_md5;

        Self {
            offset,
            end: offset + length as u32,

            #[cfg(feature = "md5")]
            md5: Err(md5::Context::new()),

            #[cfg(feature = "md5")]
            calc_md5,

            stored_md5: None,

            state: InternalState::Proc,
        }
    }

    /// Get current offset
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Reader reached end of data
    pub fn is_done(&self) -> bool {
        matches!(self.state, InternalState::Done)
    }

    /// Get stored MD5 checksum
    pub fn stored_md5(&self) -> Option<&Md5Data> {
        self.stored_md5.as_ref()
    }

    /// Get computed MD5 checksum
    pub fn actual_md5(&self) -> Option<&Md5Data> {
        #[cfg(feature = "md5")]
        {
            self.md5.as_ref().ok()
        }

        #[cfg(not(feature = "md5"))]
        {
            None
        }
    }

    /// Check partition table consistency
    pub fn check_md5(&self) -> Option<bool> {
        #[cfg(feature = "md5")]
        if let (Some(stored_md5), Some(actual_md5)) = (self.stored_md5(), self.actual_md5()) {
            Some(stored_md5 == actual_md5)
        } else {
            None
        }

        #[cfg(not(feature = "md5"))]
        {
            None
        }
    }

    fn check(&mut self) -> Result<(), PartitionError> {
        if self.offset >= self.end {
            self.state = InternalState::Done;
        }

        if matches!(self.state, InternalState::Done) {
            Err(PartitionError::NotEnoughData)
        } else {
            Ok(())
        }
    }

    /// Read partition data from buffer
    pub fn read(&mut self, buffer: &PartitionBuffer) -> Result<PartitionEntry, PartitionError> {
        self.check()?;

        let result = match *buffer
            .split_first_chunk()
            .ok_or(PartitionError::NotEnoughData)?
            .0
        {
            PartitionEntry::MAGIC => {
                #[cfg(feature = "md5")]
                if self.calc_md5 {
                    if let Err(ctx) = &mut self.md5 {
                        ctx.consume(buffer);
                    }
                }

                buffer.try_into()
            }
            PartitionMd5::MAGIC => match buffer.try_into() {
                Ok(PartitionMd5 { data }) => {
                    self.stored_md5 = Some(data);
                    self.offset += PartitionEntry::SIZE as u32;
                    Err(PartitionError::NotEnoughData)
                }
                Err(error) => Err(error),
            },
            [0xff, 0xff] => Err(PartitionError::NotEnoughData),
            _ => Err(PartitionError::InvalidMagic),
        };

        if let Err(error) = &result {
            if let PartitionError::NotEnoughData = error {
                #[cfg(feature = "md5")]
                if self.calc_md5 && self.md5.is_err() {
                    self.md5 = Ok(self.md5.as_mut().unwrap_err().clone().compute().into());
                }
            }

            self.state = InternalState::Done;
        } else {
            self.offset += PartitionEntry::SIZE as u32;
        }

        result
    }
}

/// Partition table writer state
pub struct PartitionWriterState {
    offset: u32,
    end: u32,

    #[cfg(feature = "md5")]
    md5: Result<Md5Data, md5::Context>,

    #[cfg(feature = "md5")]
    write_md5: bool,

    state: InternalState,
}

impl PartitionWriterState {
    /// Instantiate writer state
    ///
    /// If `md5` feature isn't enabled `write_md5` argument will be ignored.
    pub fn new(offset: u32, length: usize, write_md5: bool) -> Self {
        #[cfg(not(feature = "md5"))]
        let _ = write_md5;

        Self {
            offset,
            end: offset + length as u32,

            #[cfg(feature = "md5")]
            md5: Err(md5::Context::new()),

            #[cfg(feature = "md5")]
            write_md5,

            state: InternalState::Init,
        }
    }

    /// Get current offset
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Writer reached end of data
    pub fn is_done(&self) -> bool {
        matches!(self.state, InternalState::Done)
    }

    /// Get computed MD5 checksum
    pub fn actual_md5(&self) -> Option<&Md5Data> {
        #[cfg(feature = "md5")]
        {
            self.md5.as_ref().ok()
        }

        #[cfg(not(feature = "md5"))]
        {
            None
        }
    }

    fn check(&mut self) -> Result<(), PartitionError> {
        if self.offset >= self.end {
            self.state = InternalState::Done;
        }

        match self.state {
            InternalState::Init => {
                self.state = InternalState::Proc;
                Ok(())
            }
            InternalState::Proc => {
                self.offset += PartitionEntry::SIZE as u32;
                Ok(())
            }
            InternalState::Done => Err(PartitionError::TooManyData),
        }
    }

    /// Write partition data into buffer
    ///
    /// If `md5` feature is used and partition is None then MD5 checksum will be written.
    pub fn write(
        &mut self,
        buffer: &mut PartitionBuffer,
        partition: impl AsRef<PartitionEntry>,
    ) -> Result<(), PartitionError> {
        self.check()?;

        partition.as_ref().to_bytes(buffer)?;

        #[cfg(feature = "md5")]
        if self.write_md5 {
            if let Err(ctx) = &mut self.md5 {
                ctx.consume(buffer);
            }
        }

        Ok(())
    }

    /// Write partition MD5 into buffer
    ///
    /// If `md5` feature is used and partition is None then MD5 checksum will be written.
    pub fn write_md5(&mut self, buffer: &mut PartitionBuffer) -> Result<(), PartitionError> {
        self.check()?;

        self.state = InternalState::Done;

        #[cfg(not(feature = "md5"))]
        let _ = buffer;

        #[cfg(feature = "md5")]
        if self.write_md5 && self.md5.is_err() {
            let md5 = PartitionMd5::from(self.md5.as_mut().unwrap_err().clone().compute());
            md5.to_bytes(buffer)?;
            self.md5 = Ok(md5.into());
        }

        Ok(())
    }

    /// Finalize writer
    pub fn finish(&mut self) {
        self.state = InternalState::Done;
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn read_partitions() {
        let table = include_bytes!("../tests/partitions.bin");
        let data = &table[..];
        let mut reader = PartitionReaderState::new(0, data.len(), true);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::Nvs));
        assert_eq!(part.offset, 36 << 10);
        assert_eq!(part.size, 24 << 10);
        assert_eq!(part.name(), "nvs");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::Phy));
        assert_eq!(part.offset, 60 << 10);
        assert_eq!(part.size, 4 << 10);
        assert_eq!(part.name(), "phy_init");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::App(AppPartitionType::Factory));
        assert_eq!(part.offset, 64 << 10);
        assert_eq!(part.size, 3 << 20);
        assert_eq!(part.name(), "factory");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::CoreDump));
        assert_eq!(part.offset, (64 << 10) + (3 << 20));
        assert_eq!(part.size, 64 << 10);
        assert_eq!(part.name(), "coredump");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::Nvs));
        assert_eq!(part.offset, (128 << 10) + (3 << 20));
        assert_eq!(part.size, 64 << 10);
        assert_eq!(part.name(), "nvs_ext");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        assert!(matches!(
            reader.read(part).unwrap_err(),
            PartitionError::NotEnoughData
        ));
        if let Some(md5) = reader.check_md5() {
            assert!(md5);
        }

        let (part, _) = data.split_first_chunk().unwrap();
        assert!(matches!(
            reader.read(part).unwrap_err(),
            PartitionError::NotEnoughData
        ));
    }

    #[test]
    fn read_partitions_ota() {
        let table = include_bytes!("../tests/partitions-ota.bin");
        let data = &table[..];
        let mut reader = PartitionReaderState::new(0, data.len(), true);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::Nvs));
        assert_eq!(part.offset, 36 << 10);
        assert_eq!(part.size, 16 << 10);
        assert_eq!(part.name(), "nvs");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::Ota));
        assert_eq!(part.offset, 52 << 10);
        assert_eq!(part.size, 8 << 10);
        assert_eq!(part.name(), "otadata");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::Phy));
        assert_eq!(part.offset, 60 << 10);
        assert_eq!(part.size, 4 << 10);
        assert_eq!(part.name(), "phy_init");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::App(AppPartitionType::Factory));
        assert_eq!(part.offset, 64 << 10);
        assert_eq!(part.size, 1 << 20);
        assert_eq!(part.name(), "factory");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::App(AppPartitionType::Ota(0)));
        assert_eq!(part.offset, (64 << 10) + (1 << 20));
        assert_eq!(part.size, 1 << 20);
        assert_eq!(part.name(), "ota_0");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::App(AppPartitionType::Ota(1)));
        assert_eq!(part.offset, (64 << 10) + (2 << 20));
        assert_eq!(part.size, 1 << 20);
        assert_eq!(part.name(), "ota_1");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::CoreDump));
        assert_eq!(part.offset, (64 << 10) + (3 << 20));
        assert_eq!(part.size, 64 << 10);
        assert_eq!(part.name(), "coredump");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        let part = reader.read(part).unwrap();
        assert_eq!(part.type_, PartitionType::Data(DataPartitionType::Nvs));
        assert_eq!(part.offset, (128 << 10) + (3 << 20));
        assert_eq!(part.size, 64 << 10);
        assert_eq!(part.name(), "nvs_ext");
        assert!(!part.encrypted);

        let (part, data) = data.split_first_chunk().unwrap();
        assert!(matches!(
            reader.read(part).unwrap_err(),
            PartitionError::NotEnoughData
        ));
        if let Some(md5) = reader.check_md5() {
            assert!(md5);
        }

        let (part, _) = data.split_first_chunk().unwrap();
        assert!(matches!(
            reader.read(part).unwrap_err(),
            PartitionError::NotEnoughData
        ));
    }

    #[test]
    fn write_partitions() {
        let src_table = include_bytes!("../tests/partitions.bin");
        let mut dst_table = [0u8; PartitionTable::MAX_SIZE];

        let mut src_data = &src_table[..];
        let mut dst_data = &mut dst_table[..];
        let mut reader = PartitionReaderState::new(0, src_data.len(), true);
        let mut writer = PartitionWriterState::new(0, dst_data.len(), true);

        loop {
            let (src_part, next_src_data) = src_data.split_first_chunk().unwrap();
            src_data = next_src_data;
            let part = match reader.read(src_part) {
                Ok(part) => Some(part),
                Err(PartitionError::NotEnoughData) => None,
                Err(error) => panic!("{error:?}"),
            };
            let (dst_part, next_dst_data) = dst_data.split_first_chunk_mut().unwrap();
            dst_data = next_dst_data;
            if let Some(part) = part {
                writer.write(dst_part, part).unwrap();
            } else {
                writer.write_md5(dst_part).unwrap();
                break;
            }
        }

        let len = src_table.len()
            - src_data.len()
            - if !cfg!(feature = "md5") {
                PartitionEntry::SIZE
            } else {
                0
            };

        assert_eq!(&dst_table[..len], &src_table[..len]);
    }

    #[test]
    fn write_partitions_ota() {
        let src_table = include_bytes!("../tests/partitions-ota.bin");
        let mut dst_table = [0u8; PartitionTable::MAX_SIZE];

        let mut src_data = &src_table[..];
        let mut dst_data = &mut dst_table[..];
        let mut reader = PartitionReaderState::new(0, src_data.len(), true);
        let mut writer = PartitionWriterState::new(0, dst_data.len(), true);

        loop {
            let (src_part, next_src_data) = src_data.split_first_chunk().unwrap();
            src_data = next_src_data;
            let part = match reader.read(src_part) {
                Ok(part) => Some(part),
                Err(PartitionError::NotEnoughData) => None,
                Err(error) => panic!("{error:?}"),
            };
            let (dst_part, next_dst_data) = dst_data.split_first_chunk_mut().unwrap();
            dst_data = next_dst_data;
            if let Some(part) = part {
                writer.write(dst_part, part).unwrap();
            } else {
                writer.write_md5(dst_part).unwrap();
                break;
            }
        }

        let len = src_table.len()
            - src_data.len()
            - if !cfg!(feature = "md5") {
                PartitionEntry::SIZE
            } else {
                0
            };

        assert_eq!(&dst_table[..len], &src_table[..len]);
    }
}
