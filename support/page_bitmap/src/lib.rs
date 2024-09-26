//! Hierarchical bitmap of free physical pages.
//!
//! Each level is a bitmap and tracks a progressively smaller
//! range of memory, with the highest level tracking the largest
//! blocks and the lowest level (Level 0) tracking individual
//! 4 KiB pages.
//!
//! When a page is allocated or freed, all levels above it are
//! updated to reflect the change.
//!
//! +--------------------------------------------------+
//! | Level 7 (64 GiB blocks, 1 byte = 8 blocks)       | <- Highest level
//! +--------------------------------------------------+
//! | Level 6 (8 GiB blocks, 1 byte = 8 blocks)        |
//! +--------------------------------------------------+
//! | Level 5 (1 GiB blocks, 1 byte = 8 blocks)        |
//! +--------------------------------------------------+
//! | Level 4 (128 MiB blocks, 1 byte = 8 blocks)      |
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

fn cttz(_byte: u8) -> u8 {
    // TODO: the instrinsics are unstable, asm or bit twiddling for starters.
    0
}

// TODO: sizes and asserts for 64GiBs

/// A hierarchical bitmap system to track memory allocation using
/// 8 hierarchical levels to cover up to 64 GiB of memory with
/// 4 KiB pages.
pub struct PageBitmap<'a> {
    levels: [&'a mut [u8]; PAGE_BITMAP_LEVEL_NUMBER],
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

impl<'a> PageBitmap<'a> {
    /// Creates a new `PageBitmap` with provided slices for each level.
    /// The higher levels come first for cache-friendliness.
    /// TODO: should check each level sizes.
    /// TODO: if the RAM size is more than 64 GiB, return an error.
    pub fn new(levels: [&'a mut [u8]; PAGE_BITMAP_LEVEL_NUMBER], max_memory: usize) -> Self {
        let bitmap_size = page_bitmap_level_size(max_memory);
        for (size_idx, level) in levels.iter().enumerate() {
            if level.len() <= bitmap_size[size_idx] {
                panic!(
                    "Level {size_idx} bitmap storage must be at least {} bytes of size",
                    bitmap_size[size_idx]
                );
            }
        }
        Self { levels }
    }

    /// Initializes the bitmap using a closure that reports whether a page is free.
    /// The closure `is_page_free` takes a PFN and returns `true`` if the page is free.
    pub fn initialize<F>(&mut self, is_page_free: F)
    where
        F: Fn(usize) -> bool,
    {
        // Iterate over all pages, updating level 0 (the most granular level)
        for page_number in 0..(self.levels[PAGE_BITMAP_LEVEL_NUMBER - 1].len() * 8) {
            let byte_index = page_number / 8;
            let bit_index = page_number % 8;

            // Check if the page is free using the closure
            if is_page_free(page_number) {
                // Mark page as free (bit unset)
                self.levels[PAGE_BITMAP_LEVEL_NUMBER - 1][byte_index] &= !(1 << bit_index);
            } else {
                // Mark page as allocated (bit set)
                self.levels[PAGE_BITMAP_LEVEL_NUMBER - 1][byte_index] |= 1 << bit_index;
            }
        }

        self.update_all_levels();
    }

    fn update_all_levels(&mut self) {
        for page_number in 0..(self.levels[PAGE_BITMAP_LEVEL_NUMBER - 1].len() * 8) {
            self.update_higher_levels(page_number);
        }
    }

    /// Allocates a 4 KiB page, updating all levels accordingly.
    /// TODO: return an error if the page is already allocated.
    pub fn allocate_page(&mut self, page_number: usize) {
        let byte_index = page_number / 8;
        let bit_index = page_number % 8;

        // Set the bit for the specific page in Level 0
        self.levels[PAGE_BITMAP_LEVEL_NUMBER - 1][byte_index] |= 1 << bit_index;

        // Update all levels
        self.update_higher_levels(page_number);
    }

    /// Frees a 4 KiB page, updating all levels accordingly.
    /// TODO: return an error if the page is not allocated.
    pub fn free_page(&mut self, page_number: usize) {
        let byte_index = page_number / 8;
        let bit_index = page_number % 8;

        // Clear the bit for the specific page in Level 0
        self.levels[PAGE_BITMAP_LEVEL_NUMBER - 1][byte_index] &= !(1 << bit_index);

        // Update all levels
        self.update_higher_levels(page_number);
    }

    /// Checks if a specific page is allocated.
    pub fn is_page_allocated(&self, page_number: usize) -> bool {
        let byte_index = page_number / 8;
        let bit_index = page_number % 8;
        (self.levels[PAGE_BITMAP_LEVEL_NUMBER - 1][byte_index] & (1 << bit_index)) != 0
    }

    fn update_higher_levels(&mut self, page_number: usize) {
        let mut current_page = page_number;
        for level in (0..PAGE_BITMAP_LEVEL_NUMBER - 1).rev() {
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
        for level in 0..PAGE_BITMAP_LEVEL_NUMBER - 1 {
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
}
