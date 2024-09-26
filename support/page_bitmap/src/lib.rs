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
//! Nothing of this is really new and is remiscent of the buddy
//! allocators. This implementation uses hierarchical tracking,
//! bitmaps, and fast bit-searching (in progress). The coarsest
//! levels are stored first in hopes to be cache-friendly.

#![cfg_attr(not(test), no_std)]

const PAGE_BITMAP_LEVEL_NUMBER: usize = 8;
const MAX_MEMORY_SUPPORTED_BYTES: usize = 64 << 30;
const BLOCK_SIZE: usize = 4096;

mod tests;

#[derive(Debug, Copy, Clone)]
pub enum PageBitMapError {
    AlreadyAllocated,
    NotAllocated,
}

#[derive(Debug, Copy, Clone)]
pub enum PageBitMapBlock {
    Block4K,
    Block32K,
    Block256K,
    Block2M,
    Block16M,
    Block128M,
    Block1G,
    Block8G,
}

#[derive(Debug, Copy, Clone)]
pub struct MemoryMapEntry {
    start_pfn: usize,
    length: usize,
    allocated: bool,
}

fn cttz(_byte: u8) -> u8 {
    // TODO: the instrinsics are unstable, asm or bit twiddling for starters.
    0
}

pub const fn page_bitmap_level_size(max_memory: usize) -> [usize; PAGE_BITMAP_LEVEL_NUMBER] {
    const fn align_to_8(n: usize) -> usize {
        (n + 7) & !7
    }

    assert!(
        max_memory <= MAX_MEMORY_SUPPORTED_BYTES,
        "Can't support that much memory"
    );
    assert!(max_memory != 0, "Can't support a system without memory");
    assert!(
        (max_memory & (BLOCK_SIZE - 1)) == 0,
        "Memory size must have a whole number of 4096 byte blocks"
    );

    let mut bitmap_size = [0; PAGE_BITMAP_LEVEL_NUMBER];

    let mut i = 0;
    let mut size = max_memory / BLOCK_SIZE;
    while i < PAGE_BITMAP_LEVEL_NUMBER {
        if size <= 8 {
            bitmap_size[i] = 1;
            break;
        }
        bitmap_size[i] = align_to_8(size) / 8;
        size /= 8;
        i += 1;
    }

    bitmap_size
}

/// A hierarchical bitmap system to track memory allocation using
/// 8 hierarchical levels to cover up to 64 GiB of memory with
/// 4 KiB pages.
/// The higher levels come first for cache-friendliness.
pub struct PageBitmap<'a> {
    levels: [&'a mut [u8]; PAGE_BITMAP_LEVEL_NUMBER],
    max_memory: usize,
}

impl<'a> PageBitmap<'a> {
    /// Creates a new `PageBitmap` with provided slices for each level.
    /// Use `page_bitmap_level_size()` to provide the correct storage size
    /// to track `max_memory` bytes.
    /// The `memory_map_iter` "iterator" provides data on the available memory
    /// ranges.
    pub fn new<F>(
        mut levels: [&'a mut [u8]; PAGE_BITMAP_LEVEL_NUMBER],
        max_memory: usize,
        memory_map_iter: F,
    ) -> Self
    where
        F: Fn() -> Option<MemoryMapEntry>,
    {
        let bitmap_size = page_bitmap_level_size(max_memory);
        for (size_idx, level) in levels.iter_mut().enumerate() {
            let reqd_bitmap_size = bitmap_size[size_idx];
            if level.len() <= reqd_bitmap_size {
                panic!("Level {size_idx} bitmap storage must be at least {reqd_bitmap_size} bytes of size");
            }
        }

        while let Some(MemoryMapEntry {
            start_pfn: _,
            length: _,
            allocated: _,
        }) = memory_map_iter()
        {
            // Initialize the page bitmap
        }
        Self { max_memory, levels }
    }

    pub fn max_memory(&self) -> usize {
        self.max_memory
    }

    /// Allocates a 4 KiB page, updating all levels accordingly.
    /// TODO: return an error if the page is already allocated.
    pub fn allocate_page(&mut self, page_number: usize) -> Result<(), PageBitMapError> {
        if self.is_page_allocated(page_number) {
            return Err(PageBitMapError::AlreadyAllocated);
        }

        let byte_index = page_number / 8;
        let bit_index = page_number % 8;

        // Set the bit for the specific page in Level 0
        self.levels[0][byte_index] |= 1 << bit_index;

        // Update all levels
        self.update_higher_levels(page_number);

        Ok(())
    }

    /// Frees a 4 KiB page, updating all levels accordingly.
    /// TODO: return an error if the page is not allocated.
    pub fn free_page(&mut self, page_number: usize) -> Result<(), PageBitMapError> {
        if !self.is_page_allocated(page_number) {
            return Err(PageBitMapError::NotAllocated);
        }

        let byte_index = page_number / 8;
        let bit_index = page_number % 8;

        // Clear the bit for the specific page in Level 0
        self.levels[0][byte_index] &= !(1 << bit_index);

        // Update all levels
        self.update_higher_levels(page_number);

        Ok(())
    }

    /// Checks if a specific page is allocated.
    pub fn is_page_allocated(&self, page_number: usize) -> bool {
        let byte_index = page_number / 8;
        let bit_index = page_number % 8;
        (self.levels[0][byte_index] & (1 << bit_index)) != 0
    }

    fn update_higher_levels(&mut self, page_number: usize) {
        let mut current_page = page_number;
        for level in 1..PAGE_BITMAP_LEVEL_NUMBER - 1 {
            let group_index = current_page / 8;
            self.update_level(level, group_index);
            current_page /= 8;
        }
    }

    fn update_level(&mut self, level: usize, group_number: usize) {
        let start_unit = group_number * 8;
        let end_unit = start_unit + 8;

        let mut group_free = true;

        for unit in start_unit..end_unit {
            let byte_index = unit / 8;
            let bit_index = unit % 8;

            if (self.levels[level + 1][byte_index] & (1 << bit_index)) != 0 {
                group_free = false;
                break;
            }
        }

        let byte_index = group_number / 8;
        let bit_index = group_number % 8;

        if group_free {
            self.levels[level][byte_index] &= !(1 << bit_index); // Mark group as free
        } else {
            self.levels[level][byte_index] |= 1 << bit_index; // Mark group as allocated
        }
    }

    /// Finds the first free page.
    pub fn find_free_page(&self) -> Option<usize> {
        let mut current_group = 0;

        // Traverse levels from the highest (more coarse) down to the lowest
        for level in (0..PAGE_BITMAP_LEVEL_NUMBER - 1).rev() {
            let mut found = false;
            for (byte_index, &byte) in self.levels[level].iter().enumerate() {
                if byte != 0xFF {
                    // Use `cttz` to find the first zero bit
                    let first_zero_bit = cttz(!byte) as usize;
                    current_group = byte_index * 8 + first_zero_bit;
                    found = true;
                    break;
                }
            }

            if !found {
                // No free group found
                return None;
            }
            // Move down to the lower level
            current_group *= 8;
        }

        // At last, go check at the page level (Level 0) for the exatc free page
        (current_group..current_group + 8).find(|&page| !self.is_page_allocated(page))
    }

    /// Is the block allocated?
    pub fn is_block_allocated(_block_size: PageBitMapBlock) -> bool {
        todo!()
    }

    /// Find a free block of the specified size.
    pub fn find_free_block(_block_size: PageBitMapBlock) {
        todo!()
    }
}
