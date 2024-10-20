use crate::{
    PartitionBuffer, PartitionEntry, PartitionError, PartitionReaderState, PartitionTable,
    PartitionWriterState,
};
use core::{mem::MaybeUninit, ops::Deref};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};

/// Error type for embedded storage operations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NorFlashOpError<S: ReadNorFlash> {
    /// Partition specific error
    PartitionError(PartitionError),
    /// Storage specific error
    StorageError(S::Error),
}

impl<S: ReadNorFlash> From<PartitionError> for NorFlashOpError<S> {
    fn from(error: PartitionError) -> Self {
        Self::PartitionError(error)
    }
}

impl PartitionTable {
    /// Get iterator over partitions from table
    ///
    /// If `md5` feature isn't enabled `calc_md5` argument will be ignored.
    pub fn iter_nor_flash<'s, S>(
        &self,
        storage: &'s mut S,
        calc_md5: bool,
    ) -> PartitionNorFlashIter<'s, S>
    where
        S: ReadNorFlash,
    {
        PartitionNorFlashIter {
            storage,
            state: PartitionReaderState::new(self.addr, self.size, calc_md5),
            buffer: MaybeUninit::uninit(),
        }
    }

    /// Read partitions from table
    ///
    /// The `check_md5` argument means following:
    /// - None - ignore MD5 checksum
    /// - Some(false) - check MD5 when found (optional MD5)
    /// - Some(true) - MD5 checksum is mandatory
    ///
    /// If `md5` feature isn't enabled `check_md5` argument will be ignored.
    #[cfg(feature = "embedded-storage")]
    pub fn read_nor_flash<S, T>(
        &self,
        storage: &mut S,
        check_md5: Option<bool>,
    ) -> Result<T, NorFlashOpError<S>>
    where
        S: ReadNorFlash,
        T: FromIterator<PartitionEntry>,
    {
        let mut iter = self.iter_nor_flash(storage, check_md5.is_some());
        let result = (&mut iter).collect::<Result<_, _>>()?;

        #[cfg(feature = "md5")]
        if let Some(mandatory_md5) = check_md5 {
            if !iter.check_md5().unwrap_or(!mandatory_md5) {
                return Err(PartitionError::InvalidMd5.into());
            }
        }

        Ok(result)
    }

    /// Write partitions into table
    ///
    /// If `md5` feature isn't enabled `write_md5` argument will be ignored.
    #[cfg(feature = "embedded-storage")]
    pub fn write_nor_flash<S>(
        &self,
        storage: &mut S,
        partitions: impl IntoIterator<Item = impl AsRef<PartitionEntry>>,
        write_md5: bool,
    ) -> Result<usize, NorFlashOpError<S>>
    where
        S: NorFlash,
    {
        // The following is not supported by the compiler
        // (can't use generic parameters from outer function)
        // const SECTOR_SIZE: usize = S::ERASE_SIZE;
        const SECTOR_SIZE: usize = PartitionTable::MAX_SIZE;

        let mut sector_data = MaybeUninit::<[u8; SECTOR_SIZE]>::uninit();
        let sector_data = unsafe { sector_data.assume_init_mut() };
        let mut data = &mut sector_data[..];
        let mut state = PartitionWriterState::new(self.addr, self.size, write_md5);

        for partition in partitions {
            if state.is_done() {
                return Err(PartitionError::TooManyData.into());
            }

            let (head, rest) = data
                .split_first_chunk_mut()
                .ok_or(PartitionError::NotEnoughData)?;

            state.write(head, partition)?;

            data = rest;
        }

        #[cfg(feature = "md5")]
        if write_md5 {
            if state.is_done() {
                return Err(PartitionError::TooManyData.into());
            }

            let (head, rest) = data
                .split_first_chunk_mut()
                .ok_or(PartitionError::NotEnoughData)?;

            state.write_md5(head)?;

            data = rest;
        }

        data.fill(0);

        storage
            .write(0, sector_data)
            .map_err(NorFlashOpError::StorageError)?;

        Ok((state.offset() - self.addr) as usize)
    }
}

/// Iterator over embedded partition table
pub struct PartitionNorFlashIter<'s, S> {
    storage: &'s mut S,
    state: PartitionReaderState,
    buffer: MaybeUninit<PartitionBuffer>,
}

impl<S> PartitionNorFlashIter<'_, S> {
    /// Read next partition entry
    pub fn next_partition(&mut self) -> Result<PartitionEntry, NorFlashOpError<S>>
    where
        S: ReadNorFlash,
    {
        if self.state.is_done() {
            return Err(NorFlashOpError::PartitionError(
                PartitionError::NotEnoughData,
            ));
        }

        // Assume that partition data buffer aligned and bigger than S::READ_SIZE
        if let Err(error) = self.storage.read(self.state.offset(), unsafe {
            self.buffer.assume_init_mut()
        }) {
            return Err(NorFlashOpError::StorageError(error));
        }

        self.state
            .read(unsafe { self.buffer.assume_init_ref() })
            .map_err(From::from)
    }
}

impl<S> Deref for PartitionNorFlashIter<'_, S> {
    type Target = PartitionReaderState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S> Iterator for PartitionNorFlashIter<'_, S>
where
    S: ReadNorFlash,
{
    type Item = Result<PartitionEntry, NorFlashOpError<S>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_partition()
            .map(Some)
            .or_else(|error| {
                if matches!(
                    error,
                    NorFlashOpError::PartitionError(PartitionError::NotEnoughData)
                ) {
                    Ok(None)
                } else {
                    Err(error)
                }
            })
            .transpose()
    }
}
