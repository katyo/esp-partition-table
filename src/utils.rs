use crate::{PartitionEntry, PartitionError};
use core::str;

pub fn name_trim(data: &[u8; PartitionEntry::MAX_NAME_LEN]) -> &[u8] {
    data.split(|c| *c == b'\0').next().unwrap_or(data.as_ref())
}

pub fn name_from(data: &[u8; PartitionEntry::MAX_NAME_LEN]) -> Result<&str, PartitionError> {
    str::from_utf8(name_trim(data)).map_err(|_| PartitionError::InvalidString)
}

pub fn name_into(
    data: &mut [u8; PartitionEntry::MAX_NAME_LEN],
    name: &str,
) -> Result<(), PartitionError> {
    let bytes = name.as_bytes();
    if bytes.len() > PartitionEntry::MAX_NAME_LEN {
        return Err(PartitionError::InvalidString);
    }
    let (head, tail) = data.split_at_mut(bytes.len());
    head.copy_from_slice(bytes);
    tail.fill(0);
    Ok(())
}
