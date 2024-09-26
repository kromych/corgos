#![cfg(test)]

use crate::page_bitmap_level_size;

#[test]
fn test_page_bitmap_size() {
    let max_memory = 4096;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [1, 0, 0, 0, 0, 0, 0, 0]);

    let max_memory = 4096 * 7;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [1, 0, 0, 0, 0, 0, 0, 0]);

    let max_memory = 4096 * 9;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [2, 1, 0, 0, 0, 0, 0, 0]);

    let max_memory = (1 << 20) + 4096;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [33, 4, 1, 0, 0, 0, 0, 0]);

    let max_memory = 1 << 21;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [64, 8, 1, 0, 0, 0, 0, 0]);

    let max_memory = (1 << 21) + 4096;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [65, 8, 1, 0, 0, 0, 0, 0]);

    let max_memory = 1 << 30;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [32768, 4096, 512, 64, 8, 1, 0, 0]);

    let max_memory = (1 << 30) + 4096;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [32769, 4096, 512, 64, 8, 1, 0, 0]);

    let max_memory = 64 << 30;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [2097152, 262144, 32768, 4096, 512, 64, 8, 1]);
}
