//! Hierarchical bitmap of free physical pages.
//!
//! Each level is a bitmap and tracks a progressively smaller
//! range of memory, with the highest level tracking the largest
//! blocks and the lowest level (Level 0) tracking individual
//! 4 KiB pages.
//!
//! When a page is allocated or freed, all levels above it are
//! updated to reflect the change. `1` means that the block is
//! not available, while `0` indicates that the block (or one
//! of its sub-blocks) is.
//! In other words, bit 1 at a given level means that all sub-blocks
//! in that large block are allocated whereas bit 0 means at least
//! one sub-block is free.
//!
//! +--------------------------------------------------+
//! | Level 7 (8 GiB blocks, 1 byte = 8 blocks)        | <- Highest level
//! +--------------------------------------------------+
//! | Level 6 (1 GiB blocks, 1 byte = 8 blocks)        |
//! +--------------------------------------------------+
//! | Level 5 (128 GiB blocks, 1 byte = 8 blocks)      |
//! +--------------------------------------------------+
//! | Level 4 (16 MiB blocks, 1 byte = 8 blocks)       |
//! +--------------------------------------------------+
//! | Level 3 (2 MiB blocks, 1 byte = 8 blocks)        |
//! +--------------------------------------------------+
//! | Level 2 (256 KiB blocks, 1 byte = 8 blocks)      |
//! +--------------------------------------------------+
//! | Level 1 (32 KiB blocks, 1 byte = 8 blocks)       |
//! +--------------------------------------------------+
//! | Level 0 (4 KiB pages, 1 byte = 8 pages)          | <- Tracks individual pages
//! +--------------------------------------------------+
//!
//! That is, the bitmap is a tree with the root at the top
//! and the leaves at the bottom. The root tracks the largest
//! blocks and the leaves track the smallest blocks, in fact,
//! individual pages. Every non-leaf node fans out to 8 children.
//!
//! Nothing of this is really new and is remiscent of the buddy
//! allocators. This implementation uses hierarchical tracking,
//! bitmaps, and fast bit-searching.
//!
//! TODO: store the coarsest levels first in hopes to be
//! more cache-friendly.

#![cfg_attr(not(test), no_std)]

mod tests;

const PAGE_BITMAP_LEVEL_NUMBER: usize = 8;
const MAX_MEMORY_SUPPORTED_BYTES: usize = 64 << 30;
const BLOCK_SIZE: usize = 4096;

const fn is_power_of_2(n: usize) -> bool {
    n & (n - 1) == 0
}

const fn align_to(n: usize, align_to: usize) -> usize {
    assert!(is_power_of_2(align_to));
    (n + align_to - 1) & !(align_to - 1)
}

const fn block_size_for_level(level: usize) -> usize {
    BLOCK_SIZE * (1 << (3 * level))
}

#[derive(Debug, Copy, Clone)]
pub enum PageBitMapError {
    PageIsNotAllocated,
    OutOfMemory,
}

#[derive(Debug, Copy, Clone)]
pub struct PageFrameNumber(usize);

impl PageFrameNumber {
    pub fn new(pfn: usize) -> Self {
        Self(pfn)
    }

    pub fn pfn(&self) -> usize {
        self.0
    }

    pub fn phys_address(&self) -> usize {
        self.0 * BLOCK_SIZE
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PageRange {
    start_pfn: PageFrameNumber,
    page_count: usize,
}

impl PageRange {
    pub fn new(start_pfn: PageFrameNumber, page_count: usize) -> Self {
        Self {
            start_pfn,
            page_count,
        }
    }

    pub fn start_phys_address(&self) -> usize {
        self.start_pfn.0 * BLOCK_SIZE
    }

    pub fn end_phys_address(&self) -> usize {
        self.start_phys_address() + self.page_count * BLOCK_SIZE
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }

    pub fn size(&self) -> usize {
        self.page_count * BLOCK_SIZE
    }
}

pub enum PageBitmapRelocation {
    None,
    Relocate(usize),
}

impl PageBitmapRelocation {
    pub fn amount(&self) -> usize {
        match self {
            PageBitmapRelocation::None => 0,
            PageBitmapRelocation::Relocate(amount) => *amount,
        }
    }
}

/// Calculate the size of the page bitmap levels in bytes for a given
/// `max_memory` size in bytes. The result is can store a whole
/// number of u64's.
pub const fn page_bitmap_level_size<const N: usize>(max_memory: usize) -> [usize; N] {
    assert!(
        max_memory <= MAX_MEMORY_SUPPORTED_BYTES,
        "Can't support that much memory"
    );
    assert!(max_memory != 0, "Can't support a system without memory");
    assert!(
        (max_memory & (BLOCK_SIZE - 1)) == 0,
        "Memory size must have a whole number of 4096 byte blocks"
    );

    let mut bitmap_size_bytes = [0; N];
    let mut level = 0;
    while level < N {
        let level_bits = max_memory / block_size_for_level(level);
        if level_bits == 0 {
            bitmap_size_bytes[level] = 8;
        } else {
            let mut level_bytes;
            level_bytes = align_to(level_bits, 8) / 8;
            level_bytes = align_to(level_bytes, 8);

            bitmap_size_bytes[level] = level_bytes;
        }
        level += 1;
    }

    bitmap_size_bytes
}

/// Calculate the size of the page bitmap in bytes for a given
/// `max_memory` size in bytes. The result is can store a whole
/// number of u64's.
pub const fn page_bitmap_size<const N: usize>(max_memory: usize) -> usize {
    let level_sizes = page_bitmap_level_size::<N>(max_memory);
    let mut bitmap_size_bytes = 0;
    let mut level = 0;
    while level < N {
        bitmap_size_bytes += level_sizes[level];
        level += 1;
    }

    bitmap_size_bytes
}

// ASCII signatures for the page bitmap fields. Makes it easier to
// identify the structure in the physical memory when debugging.
// Helps to ensure that the structure is not corrupted.

// "PgeBtM_0"
const PAGE_BITMAP_SIGNATURE0: u64 = 0x305f4d7442656750;
// "PgeBtM_1"
const PAGE_BITMAP_SIGNATURE1: u64 = 0x315f4d7442656750;
// "PgeBtM_2"
const PAGE_BITMAP_SIGNATURE2: u64 = 0x325f4d7442656750;
// "PgeBtM_3"
const PAGE_BITMAP_SIGNATURE3: u64 = 0x335f4d7442656750;
// "PgeBtM_4"
const PAGE_BITMAP_SIGNATURE4: u64 = 0x345f4d7442656750;
// "PgeBtM_5"
const PAGE_BITMAP_SIGNATURE5: u64 = 0x355f4d7442656750;

/// A hierarchical bitmap system to track memory allocation
#[repr(C, align(8))]
pub struct PageBitmap<const N: usize = PAGE_BITMAP_LEVEL_NUMBER> {
    signature0: u64,
    max_memory: usize,

    signature1: u64,
    available_pages: usize,

    signature2: u64,
    levels_number: usize,

    signature3: u64,
    bitmap_size: usize,

    signature4: u64,
    // Using u64 to ensure 8-byte alignment, the higher levels come first for cache-friendliness.
    bitmap: *mut u64,

    signature5: u64,
    level_start: [usize; N],
}

impl<const N: usize> PageBitmap<N> {
    /// Creates a new `PageBitmap` with provided slices for each level.
    /// Use `page_bitmap_size()` to provide the correct storage size
    /// to track `max_memory` bytes.
    /// The `available_ram_map_iter` "iterator" provides data on the
    /// available memory ranges, must be RAM, not ROM, MMIO, etc.
    pub fn new<F>(
        bitmap_size: usize,
        bitmap_storage: *mut u64,
        max_memory: usize,
        available_ram_map_iter: F,
    ) -> Self
    where
        F: Fn() -> Option<PageRange>,
    {
        assert!(
            page_bitmap_size::<N>(max_memory) == bitmap_size,
            "Invalid bitmap size"
        );

        // First mark all memory as non-available/busy
        {
            let bitmap_storage =
                unsafe { core::slice::from_raw_parts_mut(bitmap_storage, bitmap_size) };
            bitmap_storage.fill(!0);
        }

        // Calculate the start of each level
        let mut level_start = [0; N];
        let mut current_level_start = 0;
        for level in 0..N {
            level_start[level] = current_level_start;

            let level_size = max_memory / block_size_for_level(level);
            assert!(level_size != 0, "Level {} size is 0", level);
            assert!(
                level_size % 8 == 0,
                "Level {} size is not a multiple of 8",
                level
            );

            current_level_start += level_size / core::mem::size_of::<u64>();
        }
        assert!(level_start[0] == 0, "Level 0 start is not 0");

        // Initialize the page bitmap using the caller-provided iterator,
        let mut available_pages = 0;
        {
            while let Some(range) = available_ram_map_iter() {
                available_pages += range.page_count;

                for level in 0..N {
                    let level_map = unsafe {
                        core::slice::from_raw_parts_mut(
                            bitmap_storage.add(level_start[level]),
                            block_size_for_level(level) / core::mem::size_of::<u64>(),
                        )
                    };

                    let start_bit = range.start_phys_address() / block_size_for_level(level);
                    let end_bit = range.end_phys_address() / block_size_for_level(level);

                    // Bulk clear the bits by setting the whole u64's to 0

                    let aligned_start = align_to(start_bit, 64) / 64;
                    let aligned_end = end_bit / 64;

                    level_map[aligned_start..aligned_end].fill(0);

                    // Set the remaining bits using masks

                    let start_mask = (1 << (start_bit % 64)) - 1;
                    let end_mask = !((1 << (end_bit % 64)) - 1);
                    level_map[aligned_start] &= start_mask;
                    level_map[aligned_end] &= end_mask;
                }
            }
        }

        Self {
            signature0: PAGE_BITMAP_SIGNATURE0,
            signature1: PAGE_BITMAP_SIGNATURE1,
            signature2: PAGE_BITMAP_SIGNATURE2,
            signature3: PAGE_BITMAP_SIGNATURE3,
            signature4: PAGE_BITMAP_SIGNATURE4,
            signature5: PAGE_BITMAP_SIGNATURE5,
            levels_number: N,
            max_memory,
            bitmap: bitmap_storage,
            bitmap_size,
            level_start,
            available_pages,
        }
    }

    pub unsafe fn from_ptr<'a>(
        ptr: *mut u64,
        relocation: PageBitmapRelocation,
    ) -> Option<&'a mut Self> {
        let maybe_page_bitmap = &mut *(ptr as *mut Self);
        if maybe_page_bitmap.signature0 == PAGE_BITMAP_SIGNATURE0
            && maybe_page_bitmap.signature1 == PAGE_BITMAP_SIGNATURE1
            && maybe_page_bitmap.signature2 == PAGE_BITMAP_SIGNATURE2
            && maybe_page_bitmap.signature3 == PAGE_BITMAP_SIGNATURE3
            && maybe_page_bitmap.signature4 == PAGE_BITMAP_SIGNATURE4
            && maybe_page_bitmap.signature5 == PAGE_BITMAP_SIGNATURE5
            && maybe_page_bitmap.levels_number == N
            && page_bitmap_size::<N>(maybe_page_bitmap.max_memory) == maybe_page_bitmap.bitmap_size
            && maybe_page_bitmap.level_start[0] == 0
        {
            maybe_page_bitmap.bitmap =
                (maybe_page_bitmap.bitmap as usize + relocation.amount()) as *mut u64;
            Some(maybe_page_bitmap)
        } else {
            None
        }
    }

    /// Convert the bitmap to a pointer, useful for passing to from the bootloader
    /// to the kernel. The poiners inside the bitmap might need to be updated.
    pub unsafe fn to_ptr(self) -> *const u64 {
        let ptr = &self as *const Self as *const u64;
        ptr
    }

    /// Maximum memory supported by the bitmap
    pub fn max_memory(&self) -> usize {
        self.max_memory
    }

    /// Size of the bitmap in bytes
    pub fn size(&self) -> usize {
        self.bitmap_size
    }

    /// Levels in the bitmap
    pub fn levels_number(&self) -> usize {
        self.levels_number
    }

    /// Number of available pages
    pub fn available_pages(&self) -> usize {
        self.available_pages
    }

    /// TODO: remove pub after testing
    pub fn is_block_free(&self, level: usize, block_index: usize) -> bool {
        let bitmap_index = block_index / 64;
        let bit_offset = block_index % 64;

        let chunk = unsafe {
            self.bitmap
                .add(self.level_start[level] + bitmap_index)
                .read()
        };
        chunk & (1 << bit_offset) == 0
    }

    /// Check if a page is allocated
    pub fn is_page_free(&self, pfn: PageFrameNumber) -> bool {
        self.is_block_free(0, pfn.pfn())
    }

    /// TODO: remove pub after testing
    pub fn find_free_page(&self) -> Option<PageFrameNumber> {
        let mut free_block = 0;

        for level in (0..N).rev() {
            let level_size = block_size_for_level(level);
            let level_blocks = self.max_memory / level_size;

            let level_map = unsafe {
                core::slice::from_raw_parts(
                    self.bitmap.add(self.level_start[level]),
                    level_blocks / 64,
                )
            };

            let mut first_free_block = None;
            for (block_index, chunk) in level_map[free_block..free_block + 8].iter().enumerate() {
                if *chunk == !0 {
                    // The whole block is allocated, get to the next one
                    continue;
                }

                // Find the first free bit
                let free_bit = chunk.trailing_zeros() as usize;
                first_free_block = Some(block_index * 64 + free_bit);
            }
            if first_free_block.is_none() {
                // No free blocks, out of memory
                return None;
            }

            free_block = first_free_block.unwrap() * 8;
        }

        Some(PageFrameNumber(free_block))
    }

    /// TODO: remove pub after testing
    pub fn mark_all_levels(&mut self, block_index_0: usize, value: bool) {
        for level in 0..N {
            let bitmap_index = block_index_0 / 64;
            let bit_offset = block_index_0 % 64;

            let chunk = unsafe {
                self.bitmap
                    .add(self.level_start[level] + bitmap_index)
                    .read()
            };

            let new_chunk = if value {
                chunk | (1 << bit_offset)
            } else {
                chunk & !(1 << bit_offset)
            };

            unsafe {
                self.bitmap
                    .add(self.level_start[level] + bitmap_index)
                    .write(new_chunk);
            }
        }
    }

    /// TODO: remove pub after testing
    pub fn mark_page_as_allocated(&mut self, pfn: PageFrameNumber) {
        self.mark_all_levels(pfn.pfn(), true);
    }

    /// TODO: remove pub after testing
    pub fn mark_page_as_free(&mut self, pfn: PageFrameNumber) {
        self.mark_all_levels(pfn.pfn(), false);
    }

    /// Allocate a page
    pub fn allocate_page(&mut self) -> Result<PageFrameNumber, PageBitMapError> {
        if let Some(p) = self.find_free_page() {
            self.mark_page_as_allocated(p);
            self.available_pages -= 1;

            debug_assert!(!self.is_page_free(p));

            Ok(p)
        } else {
            Err(PageBitMapError::OutOfMemory)
        }
    }

    /// Free a page
    pub fn free_page(&mut self, page: PageFrameNumber) -> Result<(), PageBitMapError> {
        if self.is_page_free(page) {
            return Err(PageBitMapError::PageIsNotAllocated);
        }

        self.mark_page_as_free(page);
        self.available_pages += 1;

        debug_assert!(self.is_page_free(page));

        Ok(())
    }
}
