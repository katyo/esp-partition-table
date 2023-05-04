use crate::{PartitionEntry, PartitionError};
use core::str;

/// TODO: Replace with `core::slice::split_array_ref` when stabilized
/// (see https://github.com/rust-lang/rust/issues/90091)
pub trait SliceExt<T> {
    fn split_array_ref_<const N: usize>(&self) -> (&[T; N], &Self);
    fn split_array_mut_<const N: usize>(&mut self) -> (&mut [T; N], &mut Self);
}

impl<T> SliceExt<T> for [T] {
    fn split_array_ref_<const N: usize>(&self) -> (&[T; N], &Self) {
        let (a, b) = self.split_at(N);
        // SAFETY: a points to [T; N]? Yes it's [T] of length N (checked by split_at)
        unsafe { (&*(a.as_ptr() as *const [T; N]), b) }
    }

    fn split_array_mut_<const N: usize>(&mut self) -> (&mut [T; N], &mut Self) {
        let (a, b) = self.split_at_mut(N);
        // SAFETY: a points to [T; N]? Yes it's [T] of length N (checked by split_at)
        unsafe { (&mut *(a.as_mut_ptr() as *mut [T; N]), b) }
    }
}

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
