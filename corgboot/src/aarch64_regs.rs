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
    _mbz0: u64,
    #[bits(2)]
    pub el: El,
    #[bits(60)]
    _mbz1: u64,
}

#[bitfield(u64)]
pub struct SystemControlEl1Val {
    #[bits(1)]
    pub m: u64,
    #[bits(1)]
    pub a: u64,
    #[bits(1)]
    pub c: u64,
    #[bits(1)]
    pub sa: u64,
    #[bits(1)]
    pub sa0: u64,
    #[bits(1)]
    pub cp15ben: u64,
    #[bits(1)]
    pub n_aa: u64,
    #[bits(1)]
    pub itd: u64,
    #[bits(1)]
    pub sed: u64,
    #[bits(1)]
    pub uma: u64,
    #[bits(1)]
    pub en_rctx: u64,
    #[bits(1)]
    pub eos: u64,
    #[bits(1)]
    pub i: u64,
    #[bits(1)]
    pub en_db: u64,
    #[bits(1)]
    pub dze: u64,
    #[bits(1)]
    pub uct: u64,
    #[bits(1)]
    pub n_twi: u64,
    #[bits(1)]
    _mbz0: u64,
    #[bits(1)]
    pub n_twe: u64,
    #[bits(1)]
    pub wxn: u64,
    #[bits(1)]
    pub tscxt: u64,
    #[bits(1)]
    pub iesb: u64,
    #[bits(1)]
    pub eis: u64,
    #[bits(1)]
    pub span: u64,
    #[bits(1)]
    pub e0e: u64,
    #[bits(1)]
    pub ee: u64,
    #[bits(1)]
    pub uci: u64,
    #[bits(1)]
    pub en_da: u64,
    #[bits(1)]
    pub n_tlsmd: u64,
    #[bits(1)]
    pub lsmaoe: u64,
    #[bits(1)]
    pub en_ib: u64,
    #[bits(1)]
    pub en_ia: u64,
    #[bits(1)]
    pub cmow: u64,
    #[bits(1)]
    pub msc_en: u64,
    #[bits(1)]
    _mbz1: u64,
    #[bits(1)]
    pub bt0: u64,
    #[bits(1)]
    pub bt1: u64,
    #[bits(1)]
    pub itfsb: u64,
    #[bits(2)]
    pub tcf0: u64,
    #[bits(2)]
    pub tcf: u64,
    #[bits(1)]
    pub ata0: u64,
    #[bits(1)]
    pub ata: u64,
    #[bits(1)]
    pub dssbs: u64,
    #[bits(1)]
    pub twed_en: u64,
    #[bits(4)]
    pub twedel: u64,
    #[bits(1)]
    pub tmt0: u64,
    #[bits(1)]
    pub tmt: u64,
    #[bits(1)]
    pub tme0: u64,
    #[bits(1)]
    pub tme: u64,
    #[bits(1)]
    pub en_asr: u64,
    #[bits(1)]
    pub en_as0: u64,
    #[bits(1)]
    pub en_als: u64,
    #[bits(1)]
    pub epan: u64,
    #[bits(1)]
    pub tcso0: u64,
    #[bits(1)]
    pub tcso: u64,
    #[bits(1)]
    pub en_tp2: u64,
    #[bits(1)]
    pub nmi: u64,
    #[bits(1)]
    pub spintmask: u64,
    #[bits(1)]
    pub tidcp: u64,
}

#[bitfield(u64)]
pub struct VectorBaseEl1Val {
    #[bits(10)]
    _mbz0: u64,
    #[bits(54)]
    pub vbar: u64,
}

#[derive(Debug)]
pub struct MemoryAttributeIndirectionEl1Val([u8; 8]);

impl From<u64> for MemoryAttributeIndirectionEl1Val {
    fn from(value: u64) -> Self {
        Self(value.to_le_bytes())
    }
}

#[derive(Debug)]
#[repr(u64)]
pub enum TranslationGranule0 {
    _4KB = 0b00,
    _64KB = 0b01,
    _16KB = 0b10,
}

impl From<u64> for TranslationGranule0 {
    fn from(value: u64) -> Self {
        match value {
            0b00 => TranslationGranule0::_4KB,
            0b01 => TranslationGranule0::_64KB,
            0b10 => TranslationGranule0::_16KB,
            _ => panic!("Invalid translation granule 0 representation"),
        }
    }
}

impl From<TranslationGranule0> for u64 {
    fn from(value: TranslationGranule0) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
pub enum TranslationGranule1 {
    _16KB = 0b01,
    _4KB = 0b10,
    _64KB = 0b11,
}

impl From<u64> for TranslationGranule1 {
    fn from(value: u64) -> Self {
        match value {
            0b01 => TranslationGranule1::_16KB,
            0b10 => TranslationGranule1::_4KB,
            0b11 => TranslationGranule1::_64KB,
            _ => panic!("Invalid translation granule 0 representation"),
        }
    }
}

impl From<TranslationGranule1> for u64 {
    fn from(value: TranslationGranule1) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]
pub enum IntermPhysAddrSize {
    _32_bits_4GB = 0b000,
    _36_bits_64GB = 0b001,
    _40_bits_1TB = 0b010,
    _42_bits_4TB = 0b011,
    _44_bits_16TB = 0b100,
    _48_bits_256TB = 0b101,
    _52_bits_4PB = 0b110,
    _56_bits_64PB = 0b111,
}

impl From<u64> for IntermPhysAddrSize {
    fn from(value: u64) -> Self {
        match value {
            0b000 => IntermPhysAddrSize::_32_bits_4GB,
            0b001 => IntermPhysAddrSize::_36_bits_64GB,
            0b010 => IntermPhysAddrSize::_40_bits_1TB,
            0b011 => IntermPhysAddrSize::_42_bits_4TB,
            0b100 => IntermPhysAddrSize::_44_bits_16TB,
            0b101 => IntermPhysAddrSize::_48_bits_256TB,
            0b110 => IntermPhysAddrSize::_52_bits_4PB,
            0b111 => IntermPhysAddrSize::_56_bits_64PB,
            _ => panic!("Invalid intermediate physical address size representation"),
        }
    }
}

impl From<IntermPhysAddrSize> for u64 {
    fn from(value: IntermPhysAddrSize) -> Self {
        value as u64
    }
}

#[bitfield(u64)]
pub struct TranslationControlEl1Val {
    #[bits(6)]
    pub t0sz: u64,
    #[bits(1)]
    _mbz0: u64,
    #[bits(1)]
    pub epd0: u64,
    #[bits(2)]
    pub irgn0: u64,
    #[bits(2)]
    pub orgn0: u64,
    #[bits(2)]
    pub sh0: u64,
    #[bits(2)]
    pub tg0: TranslationGranule0,
    #[bits(6)]
    pub t1sz: u64,
    #[bits(1)]
    pub a1: u64,
    #[bits(1)]
    pub epd1: u64,
    #[bits(2)]
    pub irgn1: u64,
    #[bits(2)]
    pub orgn1: u64,
    #[bits(2)]
    pub sh1: u64,
    #[bits(2)]
    pub tg1: TranslationGranule1,
    #[bits(3)]
    pub ips: IntermPhysAddrSize,
    #[bits(1)]
    _mbz1: u64,
    #[bits(1)]
    pub a_s: u64,
    #[bits(1)]
    pub tbi0: u64,
    #[bits(1)]
    pub tbi1: u64,
    #[bits(1)]
    pub ha: u64,
    #[bits(1)]
    pub hd: u64,
    #[bits(1)]
    pub hpd0: u64,
    #[bits(1)]
    pub hpd1: u64,
    #[bits(1)]
    pub hwu059: u64,
    #[bits(1)]
    pub hwu060: u64,
    #[bits(1)]
    pub hwu061: u64,
    #[bits(1)]
    pub hwu062: u64,
    #[bits(1)]
    pub hwu159: u64,
    #[bits(1)]
    pub hwu160: u64,
    #[bits(1)]
    pub hwu161: u64,
    #[bits(1)]
    pub hwu162: u64,
    #[bits(1)]
    pub tbid0: u64,
    #[bits(1)]
    pub tbid1: u64,
    #[bits(1)]
    pub nfd0: u64,
    #[bits(1)]
    pub nfd1: u64,
    #[bits(1)]
    pub e0pd0: u64,
    #[bits(1)]
    pub e0pd1: u64,
    #[bits(1)]
    pub tcma0: u64,
    #[bits(1)]
    pub tcma1: u64,
    #[bits(1)]
    pub ds: u64,
    #[bits(1)]
    pub mtx0: u64,
    #[bits(1)]
    pub mtx1: u64,
    #[bits(2)]
    _mbz2: u64,
}

#[bitfield(u64)]
pub struct TranslationBaseEl1Val {
    // #[bits(1)]
    // pub cnp: u64,
    #[bits(48)]
    pub baddr: u64,
    #[bits(16)]
    pub asid: u64,
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]
pub enum MmufPaRange {
    _32_bits_4GB = 0b0000,
    _36_bits_64GB = 0b0001,
    _40_bits_1TB = 0b0010,
    _42_bits_4TB = 0b0011,
    _44_bits_16TB = 0b0100,
    _48_bits_256TB = 0b0101,
    _52_bits_4PB = 0b0110,
    _56_bits_64PB = 0b0111,
}

impl From<u64> for MmufPaRange {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmufPaRange::_32_bits_4GB,
            0b0001 => MmufPaRange::_36_bits_64GB,
            0b0010 => MmufPaRange::_40_bits_1TB,
            0b0011 => MmufPaRange::_42_bits_4TB,
            0b0100 => MmufPaRange::_44_bits_16TB,
            0b0101 => MmufPaRange::_48_bits_256TB,
            0b0110 => MmufPaRange::_52_bits_4PB,
            0b0111 => MmufPaRange::_56_bits_64PB,
            _ => panic!("Invalid physical address range representation"),
        }
    }
}

impl From<MmufPaRange> for u64 {
    fn from(value: MmufPaRange) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]
pub enum MmufAsidBits {
    _8_bits_ASID = 0b0000,
    _16_bits_ASID = 0b0010,
}

impl From<u64> for MmufAsidBits {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmufAsidBits::_8_bits_ASID,
            0b0010 => MmufAsidBits::_16_bits_ASID,
            _ => panic!("Invalid ASID representation"),
        }
    }
}

impl From<MmufAsidBits> for u64 {
    fn from(value: MmufAsidBits) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]
pub enum MmufTGran4KBStage2 {
    AsStage1 = 0b0000,
    No = 0b0001,
    Yes = 0b0010,
    Yes_52bit = 0b0011,
}

impl From<u64> for MmufTGran4KBStage2 {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmufTGran4KBStage2::AsStage1,
            0b0001 => MmufTGran4KBStage2::No,
            0b0010 => MmufTGran4KBStage2::Yes,
            0b0011 => MmufTGran4KBStage2::Yes_52bit,
            _ => panic!("Invalid 4KB granule stage 2 representation"),
        }
    }
}

impl From<MmufTGran4KBStage2> for u64 {
    fn from(value: MmufTGran4KBStage2) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmufTGran16KBStage2 {
    AsStage1 = 0b0000,
    No = 0b0001,
    Yes = 0b0010,
    Yes_52bit = 0b0011,
}

impl From<u64> for MmufTGran16KBStage2 {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmufTGran16KBStage2::AsStage1,
            0b0001 => MmufTGran16KBStage2::No,
            0b0010 => MmufTGran16KBStage2::Yes,
            0b0011 => MmufTGran16KBStage2::Yes_52bit,
            _ => panic!("Invalid 16KB granule stage 2 representation"),
        }
    }
}

impl From<MmufTGran16KBStage2> for u64 {
    fn from(value: MmufTGran16KBStage2) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmufTGran64KBStage2 {
    AsStage1 = 0b0000,
    No = 0b0001,
    Yes = 0b0010,
}

impl From<u64> for MmufTGran64KBStage2 {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmufTGran64KBStage2::AsStage1,
            0b0001 => MmufTGran64KBStage2::No,
            0b0010 => MmufTGran64KBStage2::Yes,
            _ => panic!("Invalid 16KB granule stage 2 representation"),
        }
    }
}

impl From<MmufTGran64KBStage2> for u64 {
    fn from(value: MmufTGran64KBStage2) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmufTGran4KB {
    Yes = 0b0000,
    Yes_52bit = 0b0001,
    No = 0b1111,
}

impl From<u64> for MmufTGran4KB {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmufTGran4KB::Yes,
            0b0001 => MmufTGran4KB::Yes_52bit,
            0b1111 => MmufTGran4KB::No,
            _ => panic!("Invalid 4KB granule representation"),
        }
    }
}

impl From<MmufTGran4KB> for u64 {
    fn from(value: MmufTGran4KB) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmufTGran16KB {
    No = 0b0000,
    Yes = 0b0001,
    Yes_52bit = 0b0010,
}

impl From<u64> for MmufTGran16KB {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmufTGran16KB::No,
            0b0001 => MmufTGran16KB::Yes,
            0b0010 => MmufTGran16KB::Yes_52bit,
            _ => panic!("Invalid 16KB granule representation"),
        }
    }
}

impl From<MmufTGran16KB> for u64 {
    fn from(value: MmufTGran16KB) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmufTGran64KB {
    Yes = 0b0000,
    No = 0b1111,
}

impl From<u64> for MmufTGran64KB {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmufTGran64KB::Yes,
            0b1111 => MmufTGran64KB::No,
            _ => panic!("Invalid 64KB granule representation"),
        }
    }
}

impl From<MmufTGran64KB> for u64 {
    fn from(value: MmufTGran64KB) -> Self {
        value as u64
    }
}

#[bitfield(u64)]
pub struct MmuFeatures0El1Val {
    #[bits(4)]
    pub pa_range: MmufPaRange,
    #[bits(4)]
    pub asid_bits: MmufAsidBits,
    #[bits(4)]
    pub big_end: u64,
    #[bits(4)]
    pub sns_mem: u64,
    #[bits(4)]
    pub big_end_el0: u64,
    #[bits(4)]
    pub t_gran16: MmufTGran16KB,
    #[bits(4)]
    pub t_gran64: MmufTGran64KB,
    #[bits(4)]
    pub t_gran4: MmufTGran4KB,
    #[bits(4)]
    pub t_gran16_2: MmufTGran16KBStage2,
    #[bits(4)]
    pub t_gran64_2: MmufTGran64KBStage2,
    #[bits(4)]
    pub t_gran4_2: MmufTGran4KBStage2,
    #[bits(4)]
    pub ex_s: u64,
    #[bits(8)]
    _mbz0: u64,
    #[bits(4)]
    pub fgt: u64,
    #[bits(4)]
    pub ecv: u64,
}
