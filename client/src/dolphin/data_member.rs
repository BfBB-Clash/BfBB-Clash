use process_memory::{Architecture, CopyAddress, Memory, ProcessHandle, PutAddress};

const GCN_BASE_ADDRESS: usize = 0x80000000;

/// A specialized version of `DataMember` from the `process_memory` crate,
/// meant for reading/writing to emulated GameCube memory within Dolphin.
///
/// Offsets are constructed as if they were in the GameCube's memory space.
pub struct DataMember<T> {
    offsets: Vec<usize>,
    process: ProcessHandle,
    emulated_region_address: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Sized + Copy> DataMember<T> {
    #[must_use]
    pub fn new_offset(
        handle: ProcessHandle,
        emulated_region_address: usize,
        offsets: Vec<usize>,
    ) -> Self {
        Self {
            offsets,
            process: handle,
            emulated_region_address,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Sized + Copy> Memory<T> for DataMember<T> {
    fn set_offset(&mut self, new_offsets: Vec<usize>) {
        self.offsets = new_offsets;
    }

    fn get_offset(&self) -> std::io::Result<usize> {
        // We cannot call `self.process.get_offset` as it assumes that the
        // endianness of the pointers it traverses are the same as the host
        // system, which is not the case with Dolphin running on a little endian system.

        let mut offset = 0;
        let noffsets = self.offsets.len();
        let mut copy = [0u8; Architecture::Arch32Bit as usize];
        for next_offset in self.offsets.iter().take(noffsets - 1) {
            offset += next_offset;
            offset = offset.checked_sub(GCN_BASE_ADDRESS).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Attempt to dereference an invalid pointer.",
                )
            })?;
            offset += self.emulated_region_address;
            self.process.copy_address(offset, &mut copy)?;
            offset = u32::from_be_bytes(copy) as usize;
        }

        offset += self.offsets[noffsets - 1];
        offset = offset.checked_sub(GCN_BASE_ADDRESS).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Attempt to dereference an invalid pointer.",
            )
        })?;
        offset += self.emulated_region_address;
        Ok(offset)
    }

    /// Returned value will be big endian!
    fn read(&self) -> std::io::Result<T> {
        let offset = self.get_offset()?;

        let mut buffer = vec![0u8; std::mem::size_of::<T>()];
        self.process.copy_address(offset, &mut buffer)?;

        Ok(unsafe { (buffer.as_ptr() as *const T).read_unaligned() })
    }

    /// `value` is expected to be big endian.
    fn write(&self, value: &T) -> std::io::Result<()> {
        use std::slice;
        let offset = self.get_offset()?;
        let buffer =
            unsafe { slice::from_raw_parts(value as *const _ as _, std::mem::size_of::<T>()) };
        self.process.put_address(offset, buffer)
    }
}
