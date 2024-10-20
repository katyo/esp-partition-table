use crate::{
    PartitionBuffer, PartitionEntry, PartitionError, PartitionReaderState, PartitionTable,
    PartitionWriterState,
};
use core::{mem::MaybeUninit, ops::Deref};
use embedded_storage::{ReadStorage, Storage};

/// Error type for embedded storage operations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageOpError<S: ReadStorage> {
    /// Partition specific error
    PartitionError(PartitionError),
    /// Storage specific error
    StorageError(S::Error),
}

impl<S: ReadStorage> From<PartitionError> for StorageOpError<S> {
    fn from(error: PartitionError) -> Self {
        Self::PartitionError(error)
    }
}

impl PartitionTable {
    /// Get iterator over partitions from table
    ///
    /// If `md5` feature isn't enabled `calc_md5` argument will be ignored.
    pub fn iter_storage<'s, S>(
        &self,
        storage: &'s mut S,
        calc_md5: bool,
    ) -> PartitionStorageIter<'s, S>
    where
        S: ReadStorage,
    {
        PartitionStorageIter {
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
    pub fn read_storage<S, T>(
        &self,
        storage: &mut S,
        check_md5: Option<bool>,
    ) -> Result<T, StorageOpError<S>>
    where
        S: ReadStorage,
        T: FromIterator<PartitionEntry>,
    {
        let mut iter = self.iter_storage(storage, check_md5.is_some());
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
    pub fn write_storage<S>(
        &self,
        storage: &mut S,
        partitions: impl IntoIterator<Item = impl AsRef<PartitionEntry>>,
        write_md5: bool,
    ) -> Result<usize, StorageOpError<S>>
    where
        S: Storage,
    {
        let mut data = MaybeUninit::<PartitionBuffer>::uninit();
        let mut state = PartitionWriterState::new(self.addr, self.size, write_md5);

        for partition in partitions {
            if state.is_done() {
                return Err(PartitionError::TooManyData.into());
            }

            state.write(unsafe { data.assume_init_mut() }, partition)?;

            storage
                .write(state.offset(), unsafe { data.assume_init_ref() })
                .map_err(StorageOpError::StorageError)?;
        }

        #[cfg(feature = "md5")]
        if write_md5 {
            if state.is_done() {
                return Err(PartitionError::TooManyData.into());
            }

            state.write_md5(unsafe { data.assume_init_mut() })?;

            storage
                .write(state.offset(), unsafe { data.assume_init_ref() })
                .map_err(StorageOpError::StorageError)?;
        }

        Ok((state.offset() - self.addr) as usize)
    }
}

/// Iterator over embedded partition table
pub struct PartitionStorageIter<'s, S> {
    storage: &'s mut S,
    state: PartitionReaderState,
    buffer: MaybeUninit<PartitionBuffer>,
}

impl<S> PartitionStorageIter<'_, S> {
    /// Read next partition entry
    pub fn next_partition(&mut self) -> Result<PartitionEntry, StorageOpError<S>>
    where
        S: ReadStorage,
    {
        if self.state.is_done() {
            return Err(StorageOpError::PartitionError(
                PartitionError::NotEnoughData,
            ));
        }

        if let Err(error) = self.storage.read(self.state.offset(), unsafe {
            self.buffer.assume_init_mut()
        }) {
            return Err(StorageOpError::StorageError(error));
        }

        self.state
            .read(unsafe { self.buffer.assume_init_ref() })
            .map_err(From::from)
    }
}

impl<S> Deref for PartitionStorageIter<'_, S> {
    type Target = PartitionReaderState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S> Iterator for PartitionStorageIter<'_, S>
where
    S: ReadStorage,
{
    type Item = Result<PartitionEntry, StorageOpError<S>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_partition()
            .map(Some)
            .or_else(|error| {
                if matches!(
                    error,
                    StorageOpError::PartitionError(PartitionError::NotEnoughData)
                ) {
                    Ok(None)
                } else {
                    Err(error)
                }
            })
            .transpose()
    }
}
