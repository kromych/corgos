#![allow(dead_code)]

extern "C" {
    fn _page_tables_start();
    fn _page_tables_end();
}

pub fn page_tables_phys_start() -> usize {
    _page_tables_start as usize
}

pub fn page_tables_phys_end() -> usize {
    _page_tables_end as usize
}

pub fn page_tables_area() -> &'static mut [u8] {
    let s = page_tables_phys_start();
    let e = page_tables_phys_end();
    unsafe { core::slice::from_raw_parts_mut(s as *mut u8, e - s) }
}

extern "C" {
    fn _base();
    fn _end();
    fn _image_size();
    fn _payload_start();
}

pub fn base() -> usize {
    _base as usize
}

pub fn end() -> usize {
    _end as usize
}

pub fn size() -> usize {
    _image_size as usize
}

pub fn payload_start() -> usize {
    _payload_start as usize
}
