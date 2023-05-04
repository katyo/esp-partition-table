#![doc = include_str!("../README.md")]
#![forbid(future_incompatible)]
#![deny(bad_style, missing_docs)]
#![no_std]

mod entry;
mod result;
mod table;
mod types;
mod utils;

#[cfg(feature = "embedded-storage")]
mod estor;

use utils::SliceExt;

pub use entry::{Md5Data, PartitionBuffer, PartitionEntry, PartitionMd5};
pub use result::PartitionError;
pub use table::{PartitionReaderState, PartitionTable, PartitionWriterState};
pub use types::{AppPartitionType, DataPartitionType, PartitionType};

#[cfg(feature = "embedded-storage")]
pub use estor::{PartitionStorageIter, StorageOpError};
