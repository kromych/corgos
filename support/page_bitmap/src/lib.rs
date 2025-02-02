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

use core::num::NonZero;
use zerocopy::IntoBytes;

mod tests;

const PAGE_BITMAP_LEVEL_NUMBER: usize = 8;
const MAX_MEMORY_SUPPORTED_BYTES: usize = 64 << 30;
const BLOCK_SIZE: usize = 4096;

const fn first_clear_bit(n: u64) -> usize {
    (!n & (n.wrapping_add(1))).trailing_zeros() as usize
}

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

/// For each of the 8 bytes if that byte is 0b1111_1111 then the corresponding bit
/// in the result is set to 1, otherwise it is set to 0 (i.e. bitwise AND for the bits
/// of each byte). The result is a byte where each bit corresponds to a byte in the input.
pub fn collapse_8bit_and(x: u64) -> u8 {
    // This function is branchless in hopes to be more performant
    let nx = !x;

    // Use the "has-zero-byte" bit twiddling in parallel for each byte
    let tmp = nx.wrapping_sub(0x0101010101010101) & !nx & 0x8080808080808080;

    // Shift each byte right by 7 so that each byte now contains either 0 or 1
    let bits = tmp >> 7;

    // Extract the bits from each byte and combine them into a single byte.
    // Probably can figure out a constant for doing that with multiplication?
    let b0 = (bits >> 0) & 1;
    let b1 = ((bits >> 8) & 1) << 1;
    let b2 = ((bits >> 16) & 1) << 2;
    let b3 = ((bits >> 24) & 1) << 3;
    let b4 = ((bits >> 32) & 1) << 4;
    let b5 = ((bits >> 40) & 1) << 5;
    let b6 = ((bits >> 48) & 1) << 6;
    let b7 = ((bits >> 56) & 1) << 7;

    (b0 | b1 | b2 | b3 | b4 | b5 | b6 | b7) as u8
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PageBitmapError {
    PageIsNotAllocated,
    OutOfMemory,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    page_count: NonZero<usize>,
}

impl PageRange {
    pub fn new(start_pfn: PageFrameNumber, page_count: NonZero<usize>) -> Self {
        Self {
            start_pfn,
            page_count,
        }
    }

    pub fn start_phys_address(&self) -> usize {
        self.start_pfn.0 * BLOCK_SIZE
    }

    pub fn end_phys_address(&self) -> usize {
        self.start_phys_address() + self.page_count.get() * BLOCK_SIZE
    }

    pub fn page_count(&self) -> usize {
        self.page_count.get()
    }

    pub fn size(&self) -> usize {
        self.page_count.get() * BLOCK_SIZE
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
// "PgeBtM_6"
const PAGE_BITMAP_SIGNATURE6: u64 = 0x365f4d7442656750;

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

    signature6: u64,
    level_size: [usize; N],
}

impl<const N: usize> PageBitmap<N> {
    /// Creates a new `PageBitmap` with provided slices for each level.
    /// Use `page_bitmap_size()` to provide the correct storage size
    /// to track `max_memory` bytes.
    /// The `available_ram_map_iter` "iterator" provides data on the
    /// available memory ranges, must be RAM, not ROM, MMIO, etc.
    fn build<F>(
        bitmap_size: usize,
        bitmap_storage: *mut u64,
        max_memory: usize,
        available_ram_map_iter: F,
    ) -> Self
    where
        F: FnMut() -> Option<PageRange>,
    {
        assert!(
            page_bitmap_size::<N>(max_memory) == bitmap_size,
            "Invalid bitmap size"
        );

        // Calculate the start of each level

        let mut level_start = [0; N];
        let mut current_level_start = 0;
        let mut level_size = page_bitmap_level_size::<N>(max_memory);
        for level in 0..N {
            let size = level_size[level];
            assert!(size != 0, "Level {level} size is 0");
            assert!(
                level_size[level] % 8 == 0,
                "Level {level} size of {size} is not a multiple of 8",
            );

            level_start[level] = current_level_start / 8;
            level_size[level] = size / 8;
            current_level_start += size;
        }
        assert!(level_start[0] == 0, "Level 0 start is not 0");

        let mut page_bitmap = Self {
            signature0: PAGE_BITMAP_SIGNATURE0,
            signature1: PAGE_BITMAP_SIGNATURE1,
            signature2: PAGE_BITMAP_SIGNATURE2,
            signature3: PAGE_BITMAP_SIGNATURE3,
            signature4: PAGE_BITMAP_SIGNATURE4,
            signature5: PAGE_BITMAP_SIGNATURE5,
            signature6: PAGE_BITMAP_SIGNATURE6,
            levels_number: N,
            max_memory,
            bitmap: bitmap_storage,
            bitmap_size,
            level_start,
            level_size,
            available_pages: 0,
        };

        page_bitmap.init(available_ram_map_iter);

        page_bitmap
    }

    fn init<F>(&mut self, mut available_ram_map_iter: F)
    where
        F: FnMut() -> Option<PageRange>,
    {
        // Initialize the bitmap to all 1's, meaning all memory is non-available/busy
        for level in 0..N {
            let level_map = self.level_map_mut(level);
            level_map.fill(!0);
        }

        while let Some(range) = available_ram_map_iter() {
            assert!(
                range.end_phys_address() <= self.max_memory,
                "memory range out of bounds"
            );

            self.available_pages += range.page_count.get();

            let mut start_bit = range.start_phys_address() / BLOCK_SIZE;
            let mut end_bit = range.end_phys_address() / BLOCK_SIZE;
            for level in 0..N {
                let level_map = self.level_map_mut(level);

                let start = start_bit / 64;
                let end = end_bit / 64;

                if start == end {
                    // Need to update bits within one u64

                    let mask = ((1 << (end_bit - start_bit)) - 1) << (start_bit % 64);
                    level_map[start] &= !mask;
                } else {
                    // Bulk clear the bits by setting the whole u64's to 0

                    let aligned_start_bit = align_to(start_bit, 64);
                    let aligned_end_bit = end_bit & !(64 - 1);

                    let aligned_start = aligned_start_bit / 64;
                    let aligned_end = aligned_end_bit / 64;

                    level_map[aligned_start..aligned_end].fill(0);

                    // Set the remaining bits using masks

                    if aligned_start_bit != start_bit {
                        let start_mask = (1 << (start_bit % 64)) - 1;
                        level_map[start] &= start_mask;
                    }

                    if aligned_end_bit != end_bit {
                        let end_mask = !((1 << (end_bit % 64)) - 1);
                        level_map[end] &= end_mask;
                    }
                }

                start_bit /= 8;
                end_bit /= 8;
                if end_bit == start_bit {
                    end_bit = start_bit + 1;
                }
            }
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
            && maybe_page_bitmap.signature6 == PAGE_BITMAP_SIGNATURE6
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

    fn level_map(&self, level: usize) -> &[u64] {
        let level_start = self.level_start[level];
        let level_size = self.level_size[level];

        unsafe { core::slice::from_raw_parts(self.bitmap.add(level_start), level_size) }
    }

    fn level_map_mut(&mut self, level: usize) -> &mut [u64] {
        let level_start = self.level_start[level];
        let level_size = self.level_size[level];

        unsafe { core::slice::from_raw_parts_mut(self.bitmap.add(level_start), level_size) }
    }

    fn is_block_free(&self, level: usize, block_index: usize) -> bool {
        let bitmap_index = block_index / 64;
        let bit_offset = block_index % 64;
        let block = self.level_map(level)[bitmap_index];

        block & (1 << bit_offset) == 0
    }

    /// Check if a page is allocated
    pub fn is_page_free(&self, pfn: PageFrameNumber) -> bool {
        self.is_block_free(0, pfn.pfn())
    }

    /// Find a free page
    fn find_free_page(&self) -> Option<PageFrameNumber> {
        // Find the most coarse-grained block that leads a free page
        let mut free_block_bit_index = {
            let level_map = self.level_map(N - 1);
            let mut free_block_bit_index = None;
            for (block_index, block) in level_map.iter().enumerate() {
                if *block == !0 {
                    // The whole block is allocated, get to the next one.
                    continue;
                }

                // Find the first free bit
                let free_bit = first_clear_bit(*block);
                free_block_bit_index = Some(block_index * 64 + free_bit);
                break;
            }

            if free_block_bit_index.is_none() {
                // No free blocks, out of memory
                return None;
            }

            free_block_bit_index.unwrap()
        };

        for level in (0..N - 1).rev() {
            free_block_bit_index *= 8;

            let block_index = free_block_bit_index / 64;
            let block = self.level_map(level)[block_index];

            debug_assert!(block != !0, "Block must point to free sub-blocks");

            free_block_bit_index = block_index * 64 + first_clear_bit(block);
        }

        Some(PageFrameNumber(free_block_bit_index))
    }

    /// Mark a block as allocated or free at all levels for a given page
    fn mark_all_levels(&mut self, pfn: PageFrameNumber, allocated: bool) {
        // At the leaf level, the block is a page, mark it as requested

        let mut block_index = pfn.pfn();

        let level_map = self.level_map_mut(0);
        let bitmap_index = block_index / 64;
        let bit_offset = block_index % 64;

        let block = &mut level_map[bitmap_index];
        if allocated {
            *block |= 1 << bit_offset;
        } else {
            *block &= !(1 << bit_offset);
        }
        let mut compressed = collapse_8bit_and(*block);

        // Propagate the change to the upper levels
        for level in 1..N {
            block_index /= 8;

            let level_map = self.level_map_mut(level);
            let bitmap_index = block_index / 64;
            let bit_offset = block_index % 64;
            let bytes = &mut level_map[bitmap_index].as_mut_bytes();
            let byte_index = bit_offset / 8;
            bytes[byte_index] = compressed;

            compressed = collapse_8bit_and(level_map[bitmap_index]);
        }
    }

    fn mark_page_as_allocated(&mut self, pfn: PageFrameNumber) {
        self.mark_all_levels(pfn, true);
    }

    fn mark_page_as_free(&mut self, pfn: PageFrameNumber) {
        self.mark_all_levels(pfn, false);
    }

    /// Allocate a page
    pub fn allocate_page(&mut self) -> Result<PageFrameNumber, PageBitmapError> {
        if let Some(p) = self.find_free_page() {
            self.mark_page_as_allocated(p);
            self.available_pages -= 1;

            debug_assert!(!self.is_page_free(p));

            Ok(p)
        } else {
            Err(PageBitmapError::OutOfMemory)
        }
    }

    /// Free a page
    pub fn free_page(&mut self, page: PageFrameNumber) -> Result<(), PageBitmapError> {
        if self.is_page_free(page) {
            return Err(PageBitmapError::PageIsNotAllocated);
        }

        self.mark_page_as_free(page);
        self.available_pages += 1;

        debug_assert!(self.is_page_free(page));

        Ok(())
    }

    /// Dump the bitmap to a writer
    pub fn dump(&self, writer: &mut impl core::fmt::Write) {
        writer
            .write_fmt(format_args!(
                "*** PAGE BITMAP, available pages: {}\n",
                self.available_pages
            ))
            .ok();
        for level in 0..N {
            let level_start = self.level_start[level];
            let level_size = self.level_size[level];
            writer
                .write_fmt(format_args!(
                    ">>> Level {level}, starts @ {level_start}, size {level_size}\n"
                ))
                .ok();

            let level_map = self.level_map(level);
            for (idx, block) in level_map.iter().enumerate() {
                writer
                    .write_fmt(format_args!(
                        "\t|{:064b}| # {}..{}\n",
                        block.reverse_bits(),
                        idx * 64,
                        (idx + 1) * 64
                    ))
                    .ok();
            }
        }
    }

    #[cfg(test)]
    pub fn dump_to_stdout(&self) {
        let mut out = String::new();
        self.dump(&mut out);
        println!("{}", out);
    }
}

pub type DefaultPageBitmap = PageBitmap<PAGE_BITMAP_LEVEL_NUMBER>;

impl DefaultPageBitmap {
    pub fn new(
        max_memory: usize,
        bitmap_storage: *mut u64,
        available_ram_map_iter: impl FnMut() -> Option<PageRange>,
    ) -> Self {
        PageBitmap::build(
            page_bitmap_size::<PAGE_BITMAP_LEVEL_NUMBER>(max_memory),
            bitmap_storage,
            max_memory,
            available_ram_map_iter,
        )
    }

    pub fn bitmap_storage_size(max_memory: usize) -> usize {
        page_bitmap_size::<PAGE_BITMAP_LEVEL_NUMBER>(max_memory)
    }
}
