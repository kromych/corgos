#![cfg(test)]

use crate::*;

#[test]
fn test_page_bitmap_size() {
    let max_memory = 4096;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [8, 8, 8, 8, 8, 8, 8, 8]);

    let max_memory = 4096 * 7;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [8, 8, 8, 8, 8, 8, 8, 8]);

    let max_memory = 4096 * 9;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [8, 8, 8, 8, 8, 8, 8, 8]);

    let max_memory = (1 << 20) + 4096;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [40, 8, 8, 8, 8, 8, 8, 8]);

    let max_memory = 1 << 21;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [64, 8, 8, 8, 8, 8, 8, 8]);

    let max_memory = (1 << 21) + 4096;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [72, 8, 8, 8, 8, 8, 8, 8]);

    let max_memory = 1 << 30;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [32768, 4096, 512, 64, 8, 8, 8, 8]);

    let max_memory = (1 << 30) + 4096;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [32776, 4096, 512, 64, 8, 8, 8, 8]);

    let max_memory = 64 << 30;
    let size = page_bitmap_level_size(max_memory);
    assert!(size == [2097152, 262144, 32768, 4096, 512, 64, 8, 8]);
}

#[test]
fn test_alloc_page() {
    let max_memory = 4096;
    let bitmap_size = DefaultPageBitmap::bitmap_storage_size(max_memory);

    {
        let mut alloc_storage = vec![0xaaaaaaaaaaaaaaaau64; bitmap_size / 8];
        let mut reserved_storage = vec![0xaaaaaaaaaaaaaaaau64; bitmap_size / 8];

        let mut bitmap = DefaultPageBitmap::new(
            max_memory,
            [alloc_storage.as_mut_ptr(), reserved_storage.as_mut_ptr()],
            || {
                // No free pages
                None
            },
        );
        bitmap.dump_to_stdout();
        assert!(bitmap.available_pages() == 0);
        assert!(bitmap.allocate_page() == Err(PageBitmapError::OutOfMemory));
    }

    {
        let available_pages = [PageRange::new(PageFrameNumber(0), NonZero::new(1).unwrap())];
        let mut available_pages_iter = available_pages.into_iter();
        let mut alloc_storage = vec![0u64; bitmap_size / 8];
        let mut reserved_storage = vec![0u64; bitmap_size / 8];
        let mut bitmap = DefaultPageBitmap::new(
            max_memory,
            [alloc_storage.as_mut_ptr(), reserved_storage.as_mut_ptr()],
            || available_pages_iter.next(),
        );
        bitmap.dump_to_stdout();
        assert!(bitmap.available_pages() == 1);
        assert!(bitmap.allocate_page() == Ok(PageFrameNumber(0)));
    }
}

#[test]
fn test_alloc_free_pages() {
    let pages = 777;
    let max_memory = pages * 4096;
    let bitmap_size = DefaultPageBitmap::bitmap_storage_size(max_memory);

    {
        let available_pages = [
            PageRange::new(PageFrameNumber(0), NonZero::new(1).unwrap()),
            PageRange::new(PageFrameNumber(63), NonZero::new(2).unwrap()),
            PageRange::new(PageFrameNumber(91), NonZero::new(4).unwrap()),
            PageRange::new(PageFrameNumber(97), NonZero::new(7).unwrap()),
            PageRange::new(PageFrameNumber(125), NonZero::new(17).unwrap()),
            PageRange::new(PageFrameNumber(193), NonZero::new(177).unwrap()),
        ];
        let reserved_pages = [
            PageRange::new(PageFrameNumber(1), NonZero::new(62).unwrap()),
            PageRange::new(PageFrameNumber(65), NonZero::new(26).unwrap()),
            PageRange::new(PageFrameNumber(95), NonZero::new(2).unwrap()),
            PageRange::new(PageFrameNumber(104), NonZero::new(21).unwrap()),
            PageRange::new(PageFrameNumber(142), NonZero::new(51).unwrap()),
            PageRange::new(PageFrameNumber(370), NonZero::new(407).unwrap()),
        ];
        let available_pages_count: usize = available_pages.iter().map(|r| r.page_count.get()).sum();
        let non_available_pages_count: usize =
            reserved_pages.iter().map(|r| r.page_count.get()).sum();
        assert!(available_pages_count + non_available_pages_count == pages);

        let mut available_pages_iter = available_pages.clone().into_iter();
        let mut alloc_storage = vec![0u64; bitmap_size / 8];
        let mut reserved_storage = vec![0u64; bitmap_size / 8];
        let mut bitmap = DefaultPageBitmap::new(
            max_memory,
            [alloc_storage.as_mut_ptr(), reserved_storage.as_mut_ptr()],
            || available_pages_iter.next(),
        );
        bitmap.dump_to_stdout();

        for range in available_pages.iter() {
            for pfn in range.start_pfn.pfn()..range.start_pfn.pfn() + range.page_count.get() {
                assert!(bitmap.is_page_free(PageFrameNumber(pfn)));
            }
        }
        for range in reserved_pages.iter() {
            for pfn in range.start_pfn.pfn()..range.start_pfn.pfn() + range.page_count.get() {
                assert!(!bitmap.is_page_free(PageFrameNumber(pfn)));
            }
        }

        // Build vector of available pages
        let mut available_pages_vec = Vec::new();
        for range in available_pages.iter() {
            for pfn in range.start_pfn.pfn()..range.start_pfn.pfn() + range.page_count.get() {
                available_pages_vec.push(PageFrameNumber(pfn));
            }
        }

        // Check that we can allocate all available pages
        for i in 0..available_pages_count {
            assert!(bitmap.available_pages() == available_pages_count - i);
            assert!(bitmap.allocate_page() == Ok(available_pages_vec[i]));
            bitmap.dump_to_stdout();
        }
        assert!(bitmap.available_pages() == 0);
        assert!(bitmap.allocate_page() == Err(PageBitmapError::OutOfMemory));
        assert!(bitmap.available_pages() == 0);

        // Build array of non available pages
        let mut non_available_pages_vec = Vec::new();
        for range in reserved_pages.iter() {
            for pfn in range.start_pfn.pfn()..range.start_pfn.pfn() + range.page_count.get() {
                non_available_pages_vec.push(PageFrameNumber(pfn));
            }
        }

        // Check that we can't free non available pages
        for pfn in non_available_pages_vec.iter() {
            assert!(bitmap.is_page_reserved(*pfn));
            assert!(bitmap.free_page(*pfn) == Err(PageBitmapError::PageIsReserved));
        }

        // Check that we can free all available pages
        for i in 0..available_pages_count {
            assert!(bitmap.free_page(available_pages_vec[i]) == Ok(()));
            assert!(bitmap.is_page_free(available_pages_vec[i]));
        }

        // Check that we can't free already freed pages
        for i in 0..available_pages_count {
            assert!(
                bitmap.free_page(available_pages_vec[i])
                    == Err(PageBitmapError::PageIsNotAllocated)
            );
            assert!(bitmap.is_page_free(available_pages_vec[i]));
        }

        // Check that we can't free non available pages
        for pfn in non_available_pages_vec.iter() {
            assert!(bitmap.is_page_reserved(*pfn));
            assert!(bitmap.free_page(*pfn) == Err(PageBitmapError::PageIsReserved));
        }

        // Check the amount of available pages
        assert!(bitmap.available_pages() == available_pages_count);
    }
}
