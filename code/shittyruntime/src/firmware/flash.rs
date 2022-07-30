use super::{executor_support::Mutex, Mutex as _};
use core::ops::{Deref, DerefMut};
use embedded_storage_async::nor_flash::AsyncNorFlash;

/// Wrapper around AsyncNorFlash providing interior mutability
pub struct FlashController<F: AsyncNorFlash>(Mutex<F>);

impl<F: AsyncNorFlash> FlashController<F> {
    // TODO Decide whether we want to partition the flash in any particular way or just leave that to the controlling host.
    //      It probably makes sense to just expose "UploadDictionary" functions instead of providing direct flash access.
    //      However, that would then be part of the messaging logic combined with some kind of flash offset registry.
    //      Providing a debug dump feature of the flash could be beneficial though.
    pub fn new(flash: F) -> Self {
        assert_eq!(4, F::READ_SIZE);
        assert_eq!(4, F::WRITE_SIZE);
        Self(Mutex::new(flash))
    }
}

impl<F: AsyncNorFlash> FlashController<F> {
    pub const ERASE_SIZE: usize = F::ERASE_SIZE;

    pub async fn capacity(&self) -> usize {
        let flash = self.0.lock().await;
        flash.capacity()
    }

    pub async fn read(&self, offset: u32, bytes: &mut [u8]) -> Result<(), F::Error> {
        // TODO Implement unaligned read by reading chunks of X bytes into an aligned buffer, then copying it over, then repeating until enough bytes have been read
        unimplemented!()
    }

    pub async fn write(&self, offset: u32, bytes: &[u8]) -> Result<(), F::Error> {
        // TODO Decide whether we want to allow writes to non-erased regions, maybe read before writing?
        //      That way we loose performance but could verify that the region only contains `1` / would contain the correct data after writing.
        unimplemented!()
    }

    pub async fn erase(&self, from: u32, to: u32) -> Result<(), F::Error> {
        // TODO Decide whether we even want to support unaligned flash erases
        unimplemented!()
    }

    /// Reads any range of data into a buffer aligned to READ_SIZE, internally performs up to three reads to satisfy boundary conditions
    pub async fn read_aligned(&self, offset: u32, bytes: &mut [u8]) -> Result<(), F::Error> {
        let offset = offset as usize;
        let read_size = F::READ_SIZE;
        let read_len = bytes.len();

        // Calculate the leading read & copy details
        let leading_len = (read_size - (offset % read_size)) % read_size;
        let leading_start = offset - offset % read_size;
        let leading_cpy = (read_size - leading_len..read_size, 0..leading_len);

        // Calculate the core read
        // Use the fact that integer division truncates the remainder
        let core_len = ((read_len - leading_len) / read_size) * read_size;
        let core_start = offset + leading_len;
        let core_shift = leading_len;
        let core_dst = 0..core_len;

        // Calculate the trailing read
        let trailing_len = read_len - core_len - leading_len;
        let trailing_start = core_start + core_len;
        let trailing_cpy = (
            0..trailing_len,
            leading_len + core_len..leading_len + core_len + trailing_len,
        );

        // --------

        // Execute the core read
        // We read this section first as the target buffer has to be aligned in memory thus we read to index 0 and then shift it into place.
        self.read_aligned_chunk(core_start as u32, &mut bytes[core_dst.clone()])
            .await?;
        bytes.copy_within(core_dst, core_shift);

        // Execute the leading read
        if leading_len > 0 {
            let mut leading_buf = AlignedArray([0; 4]);
            self.read_aligned_chunk(leading_start as u32, &mut (*leading_buf))
                .await?;
            bytes[leading_cpy.1].copy_from_slice(&leading_buf[leading_cpy.0]);
        }

        // Execute the trailing read
        if trailing_len > 0 {
            let mut trailing_buf = AlignedArray([0; 4]);
            self.read_aligned_chunk(trailing_start as u32, &mut (*trailing_buf))
                .await?;
            bytes[trailing_cpy.1].copy_from_slice(&trailing_buf[trailing_cpy.0]);
        }

        Ok(())
    }

    /// Reads a chunk of data with a size divisible by READ_SIZE and an address aligned to READ_SIZE into a buffer aligned to READ_SIZE
    async fn read_aligned_chunk(&self, offset: u32, bytes: &mut [u8]) -> Result<(), F::Error> {
        let mut flash = self.0.lock().await;
        flash.read(offset, bytes).await
    }

    // TODO make this private again :D
    pub async fn erase_aligned_chunk(&self, from: u32, to: u32) -> Result<(), F::Error> {
        let mut flash = self.0.lock().await;
        flash.erase(from, to).await
    }

    async fn write_aligned_chunk(&self, offset: u32, bytes: &[u8]) -> Result<(), F::Error> {
        let mut flash = self.0.lock().await;
        flash.write(offset, bytes).await
    }
}

/// Helper struct to create aligned arrays
#[derive(Debug, Clone, Copy)]
#[repr(C, align(4))]
pub struct AlignedArray<const SIZE: usize>([u8; SIZE]);

impl<const SIZE: usize> AlignedArray<SIZE> {
    pub fn new() -> Self {
        Self([1; SIZE])
    }
}

impl<const SIZE: usize> Deref for AlignedArray<SIZE> {
    type Target = [u8; SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const SIZE: usize> DerefMut for AlignedArray<SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const SIZE: usize> From<AlignedArray<SIZE>> for [u8; SIZE] {
    fn from(array: AlignedArray<SIZE>) -> Self {
        array.0
    }
}
