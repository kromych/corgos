use bitfield_struct::bitfield;

#[derive(Debug)]
#[repr(u64)]
pub enum El {
    EL0 = 0b00,
    EL1 = 0b01,
    EL2 = 0b10,
    EL3 = 0b11,
}

impl From<El> for u64 {
    fn from(val: El) -> Self {
        val as u64
    }
}

impl From<u64> for El {
    fn from(val: u64) -> Self {
        match val {
            0b00 => El::EL0,
            0b01 => El::EL1,
            0b10 => El::EL2,
            0b11 => El::EL3,
            _ => panic!("invalid EL repr"),
        }
    }
}

#[bitfield(u64)]
pub struct CurrentElVal {
    #[bits(2)]
    _res0: u64,
    #[bits(2)]
    el: El,
    #[bits(60)]
    _res1: u64,
}
