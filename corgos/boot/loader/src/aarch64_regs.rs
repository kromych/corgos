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
pub struct CurrentEl {
    #[bits(2)]
    _mbz0: u64,
    #[bits(2)]
    pub el: El,
    #[bits(60)]
    _mbz1: u64,
}

#[bitfield(u64)]
pub struct SystemControlEl1 {
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

impl Default for SystemControlEl1 {
    fn default() -> Self {
        Self(0)
            .with_eos(1)
            .with_tscxt(1)
            .with_eis(1)
            .with_n_tlsmd(1)
            .with_lsmaoe(1)
            // Disable support for SETEND and IT of Aarch32
            // in EL0
            .with_sed(1)
            .with_itd(1)
    }
}

// Must be aligned to a 2KB boundary
#[bitfield(u64)]
pub struct VectorBaseEl1 {
    #[bits(11)]
    _mbz0: u64,
    #[bits(53)]
    pub vbar_shift_11: u64,
}

#[bitfield(u64)]
pub struct ExceptionLinkEl1 {
    #[bits(64)]
    pub bits: u64,
}

#[bitfield(u64)]
pub struct ExceptionSyndromeEl1 {
    #[bits(64)]
    pub bits: u64,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u64)]
pub enum SavedProgramStateMode {
    EL0t = 0b0000,
    EL1t = 0b0100,
    EL1h = 0b0101,
    EL2t = 0b1000,
    EL2h = 0b1001,
    EL3t = 0b1100,
    EL3h = 0b1101,
}

impl From<SavedProgramStateMode> for u64 {
    fn from(value: SavedProgramStateMode) -> Self {
        value as u64
    }
}

impl From<u64> for SavedProgramStateMode {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => SavedProgramStateMode::EL0t,
            0b0100 => SavedProgramStateMode::EL1t,
            0b0101 => SavedProgramStateMode::EL1h,
            0b1000 => SavedProgramStateMode::EL2t,
            0b1001 => SavedProgramStateMode::EL2h,
            0b1100 => SavedProgramStateMode::EL3t,
            0b1101 => SavedProgramStateMode::EL3h,
            _ => panic!("illegal saved program state mode"),
        }
    }
}

#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct SavedProgramStateEl1 {
    #[bits(4)]
    pub mode: SavedProgramStateMode,
    pub aarch32: bool,
    #[bits(1)]
    _mbz0: u64,
    pub f: bool,
    pub i: bool,
    pub a: bool,
    pub d: bool,
    #[bits(54)]
    _rest: u64,
}

#[bitfield(u64)]
pub struct MainIdEl1 {
    #[bits(4)]
    pub revision: u64,
    #[bits(12)]
    pub part_num: u64,
    #[bits(4)]
    pub architecture: u64,
    #[bits(4)]
    pub variant: u64,
    #[bits(8)]
    pub implementer: u64,
    #[bits(32)]
    _mbz0: u64,
}

#[bitfield(u64)]
pub struct ProcessorFeatures0El1 {
    #[bits(4)]
    pub el0: u64,
    #[bits(4)]
    pub el1: u64,
    #[bits(4)]
    pub el2: u64,
    #[bits(4)]
    pub el3: u64,
    #[bits(4)]
    pub fp: u64,
    #[bits(4)]
    pub adv_simd: u64,
    #[bits(4)]
    pub gic: u64,
    #[bits(4)]
    pub ras: u64,
    #[bits(4)]
    pub sve: u64,
    #[bits(4)]
    pub sel2: u64,
    #[bits(4)]
    pub mpam: u64,
    #[bits(4)]
    pub amu: u64,
    #[bits(4)]
    pub dit: u64,
    #[bits(4)]
    pub rme: u64,
    #[bits(4)]
    pub csv2: u64,
    #[bits(4)]
    pub csv3: u64,
}

#[bitfield(u64)]
pub struct ProcessorFeatures1El1 {
    #[bits(4)]
    pub bt: u64,
    #[bits(4)]
    pub ssbs: u64,
    #[bits(4)]
    pub mte: u64,
    #[bits(4)]
    pub ras_frac: u64,
    #[bits(4)]
    pub mpam_frac: u64,
    #[bits(4)]
    pub res0: u64,
    #[bits(4)]
    pub sme: u64,
    #[bits(4)]
    pub rndr_trap: u64,
    #[bits(4)]
    pub csv2_frac: u64,
    #[bits(4)]
    pub nmi: u64,
    #[bits(4)]
    pub mte_frac: u64,
    #[bits(4)]
    pub gcs: u64,
    #[bits(4)]
    pub the: u64,
    #[bits(4)]
    pub mtex: u64,
    #[bits(4)]
    pub df2: u64,
    #[bits(4)]
    pub pfar: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum MemoryAttributeEl1 {
    Device_nGnRnE = 0,
    Normal_NonCacheable = 0x44,
    Normal_WriteThrough = 0xbb,
    Normal_WriteBack = 0xff,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryAttributeIndirectionEl1([u8; 8]);

impl MemoryAttributeIndirectionEl1 {
    pub fn new() -> Self {
        Self([0; 8])
    }

    pub fn get_index(&self, a: MemoryAttributeEl1) -> Option<usize> {
        self.0.iter().position(|&x| x == a as u8)
    }
}

impl Default for MemoryAttributeIndirectionEl1 {
    fn default() -> Self {
        Self([
            MemoryAttributeEl1::Device_nGnRnE as u8,
            MemoryAttributeEl1::Normal_NonCacheable as u8,
            MemoryAttributeEl1::Normal_WriteBack as u8,
            MemoryAttributeEl1::Normal_WriteThrough as u8,
            MemoryAttributeEl1::Device_nGnRnE as u8,
            MemoryAttributeEl1::Device_nGnRnE as u8,
            MemoryAttributeEl1::Device_nGnRnE as u8,
            MemoryAttributeEl1::Device_nGnRnE as u8,
        ])
    }
}

impl From<u64> for MemoryAttributeIndirectionEl1 {
    fn from(value: u64) -> Self {
        MemoryAttributeIndirectionEl1(value.to_le_bytes())
    }
}

impl From<MemoryAttributeIndirectionEl1> for u64 {
    fn from(value: MemoryAttributeIndirectionEl1) -> Self {
        u64::from_le_bytes(value.0)
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
    _Invalid = 0b00,
    _16KB = 0b01,
    _4KB = 0b10,
    _64KB = 0b11,
}

impl From<u64> for TranslationGranule1 {
    fn from(value: u64) -> Self {
        match value {
            0b00 => TranslationGranule1::_Invalid,
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
pub struct TranslationControlEl1 {
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
pub struct TranslationBase0El1 {
    // #[bits(1)]
    // pub cnp: u64,
    #[bits(48)]
    pub baddr: u64,
    #[bits(16)]
    pub asid: u64,
}

#[bitfield(u64)]
pub struct TranslationBase1El1 {
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
pub enum MmfPaRange {
    _32_bits_4GB = 0b0000,
    _36_bits_64GB = 0b0001,
    _40_bits_1TB = 0b0010,
    _42_bits_4TB = 0b0011,
    _44_bits_16TB = 0b0100,
    _48_bits_256TB = 0b0101,
    _52_bits_4PB = 0b0110,
    _56_bits_64PB = 0b0111,
}

impl From<u64> for MmfPaRange {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmfPaRange::_32_bits_4GB,
            0b0001 => MmfPaRange::_36_bits_64GB,
            0b0010 => MmfPaRange::_40_bits_1TB,
            0b0011 => MmfPaRange::_42_bits_4TB,
            0b0100 => MmfPaRange::_44_bits_16TB,
            0b0101 => MmfPaRange::_48_bits_256TB,
            0b0110 => MmfPaRange::_52_bits_4PB,
            0b0111 => MmfPaRange::_56_bits_64PB,
            _ => panic!("Invalid physical address range representation"),
        }
    }
}

impl From<MmfPaRange> for u64 {
    fn from(value: MmfPaRange) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]
pub enum MmfAsidBits {
    _8_bits_ASID = 0b0000,
    _16_bits_ASID = 0b0010,
}

impl From<u64> for MmfAsidBits {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmfAsidBits::_8_bits_ASID,
            0b0010 => MmfAsidBits::_16_bits_ASID,
            _ => panic!("Invalid ASID representation"),
        }
    }
}

impl From<MmfAsidBits> for u64 {
    fn from(value: MmfAsidBits) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]
pub enum MmfTGran4KBStage2 {
    AsStage1 = 0b0000,
    No = 0b0001,
    Yes = 0b0010,
    Yes_52bit = 0b0011,
}

impl From<u64> for MmfTGran4KBStage2 {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmfTGran4KBStage2::AsStage1,
            0b0001 => MmfTGran4KBStage2::No,
            0b0010 => MmfTGran4KBStage2::Yes,
            0b0011 => MmfTGran4KBStage2::Yes_52bit,
            _ => panic!("Invalid 4KB granule stage 2 representation"),
        }
    }
}

impl From<MmfTGran4KBStage2> for u64 {
    fn from(value: MmfTGran4KBStage2) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmfTGran16KBStage2 {
    AsStage1 = 0b0000,
    No = 0b0001,
    Yes = 0b0010,
    Yes_52bit = 0b0011,
}

impl From<u64> for MmfTGran16KBStage2 {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmfTGran16KBStage2::AsStage1,
            0b0001 => MmfTGran16KBStage2::No,
            0b0010 => MmfTGran16KBStage2::Yes,
            0b0011 => MmfTGran16KBStage2::Yes_52bit,
            _ => panic!("Invalid 16KB granule stage 2 representation"),
        }
    }
}

impl From<MmfTGran16KBStage2> for u64 {
    fn from(value: MmfTGran16KBStage2) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmfTGran64KBStage2 {
    AsStage1 = 0b0000,
    No = 0b0001,
    Yes = 0b0010,
}

impl From<u64> for MmfTGran64KBStage2 {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmfTGran64KBStage2::AsStage1,
            0b0001 => MmfTGran64KBStage2::No,
            0b0010 => MmfTGran64KBStage2::Yes,
            _ => panic!("Invalid 16KB granule stage 2 representation"),
        }
    }
}

impl From<MmfTGran64KBStage2> for u64 {
    fn from(value: MmfTGran64KBStage2) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmfTGran4KB {
    Yes = 0b0000,
    Yes_52bit = 0b0001,
    No = 0b1111,
}

impl From<u64> for MmfTGran4KB {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmfTGran4KB::Yes,
            0b0001 => MmfTGran4KB::Yes_52bit,
            0b1111 => MmfTGran4KB::No,
            _ => panic!("Invalid 4KB granule representation"),
        }
    }
}

impl From<MmfTGran4KB> for u64 {
    fn from(value: MmfTGran4KB) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmfTGran16KB {
    No = 0b0000,
    Yes = 0b0001,
    Yes_52bit = 0b0010,
}

impl From<u64> for MmfTGran16KB {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmfTGran16KB::No,
            0b0001 => MmfTGran16KB::Yes,
            0b0010 => MmfTGran16KB::Yes_52bit,
            _ => panic!("Invalid 16KB granule representation"),
        }
    }
}

impl From<MmfTGran16KB> for u64 {
    fn from(value: MmfTGran16KB) -> Self {
        value as u64
    }
}

#[derive(Debug)]
#[repr(u64)]
#[allow(non_camel_case_types)]

pub enum MmfTGran64KB {
    Yes = 0b0000,
    No = 0b1111,
}

impl From<u64> for MmfTGran64KB {
    fn from(value: u64) -> Self {
        match value {
            0b0000 => MmfTGran64KB::Yes,
            0b1111 => MmfTGran64KB::No,
            _ => panic!("Invalid 64KB granule representation"),
        }
    }
}

impl From<MmfTGran64KB> for u64 {
    fn from(value: MmfTGran64KB) -> Self {
        value as u64
    }
}

#[bitfield(u64)]
pub struct MmFeatures0El1 {
    #[bits(4)]
    pub pa_range: MmfPaRange,
    #[bits(4)]
    pub asid_bits: MmfAsidBits,
    #[bits(4)]
    pub big_end: u64,
    #[bits(4)]
    pub sns_mem: u64,
    #[bits(4)]
    pub big_end_el0: u64,
    #[bits(4)]
    pub t_gran16: MmfTGran16KB,
    #[bits(4)]
    pub t_gran64: MmfTGran64KB,
    #[bits(4)]
    pub t_gran4: MmfTGran4KB,
    #[bits(4)]
    pub t_gran16_2: MmfTGran16KBStage2,
    #[bits(4)]
    pub t_gran64_2: MmfTGran64KBStage2,
    #[bits(4)]
    pub t_gran4_2: MmfTGran4KBStage2,
    #[bits(4)]
    pub ex_s: u64,
    #[bits(8)]
    _mbz0: u64,
    #[bits(4)]
    pub fgt: u64,
    #[bits(4)]
    pub ecv: u64,
}

#[bitfield(u64)]
pub struct MmFeatures1El1 {
    #[bits(4)]
    pub hafdbs: u64,
    #[bits(4)]
    pub vmid_bits: u64,
    #[bits(4)]
    pub vh: u64,
    #[bits(4)]
    pub hpds: u64,
    #[bits(4)]
    pub lo: u64,
    #[bits(4)]
    pub pan: u64,
    #[bits(4)]
    pub spec_sei: u64,
    #[bits(4)]
    pub twed: u64,
    #[bits(4)]
    pub xnx: u64,
    #[bits(4)]
    pub ets: u64,
    #[bits(4)]
    pub hcx: u64,
    #[bits(4)]
    pub afp: u64,
    #[bits(4)]
    pub n_tlbpa: u64,
    #[bits(4)]
    pub tidcp1: u64,
    #[bits(4)]
    pub cmow: u64,
    #[bits(4)]
    pub ecbhb: u64,
}

#[bitfield(u64)]
pub struct MmFeatures2El1 {
    #[bits(4)]
    pub cn_p: u64,
    #[bits(4)]
    pub uao: u64,
    #[bits(4)]
    pub lsm: u64,
    #[bits(4)]
    pub iesb: u64,
    #[bits(4)]
    pub va_range: u64,
    #[bits(4)]
    pub ccidx: u64,
    #[bits(4)]
    pub nv: u64,
    #[bits(4)]
    pub st: u64,
    #[bits(4)]
    pub at: u64,
    #[bits(4)]
    pub ids: u64,
    #[bits(4)]
    pub fwb: u64,
    #[bits(4)]
    pub res0: u64,
    #[bits(4)]
    pub ttl: u64,
    #[bits(4)]
    pub bbm: u64,
    #[bits(4)]
    pub evt: u64,
    #[bits(4)]
    pub e0pd: u64,
}

#[bitfield(u64)]
pub struct MmFeatures3El1 {
    #[bits(4)]
    pub tcrx: u64,
    #[bits(4)]
    pub sctlrx: u64,
    #[bits(4)]
    pub s1pie: u64,
    #[bits(4)]
    pub s2pie: u64,
    #[bits(4)]
    pub s1poe: u64,
    #[bits(4)]
    pub s2poe: u64,
    #[bits(4)]
    pub aie: u64,
    #[bits(4)]
    pub mec: u64,
    #[bits(4)]
    pub d128: u64,
    #[bits(4)]
    pub d128_2: u64,
    #[bits(4)]
    pub snerr: u64,
    #[bits(4)]
    pub anerr: u64,
    #[bits(4)]
    pub res0: u64,
    #[bits(4)]
    pub sderr: u64,
    #[bits(4)]
    pub aderr: u64,
    #[bits(4)]
    pub spec_fpacc: u64,
}

#[bitfield(u64)]
pub struct MmFeatures4El1 {
    #[bits(4)]
    _mbz0: u64,
    #[bits(4)]
    pub eiesb: u64,
    #[bits(56)]
    _mbz1: u64,
}

#[bitfield(u64)]
pub struct PageTableEntry {
    pub valid: bool,
    pub table: bool, // Use PageBlockEntry if `false`
    #[bits(10)]
    _mbz0: u64,
    #[bits(35)]
    pub next_table_pfn: u64,
    #[bits(12)]
    _mbz1: u64,
    pub priv_x_never: bool,
    pub user_x_never: bool,
    // NoEffect = 0b00,
    // PrivOnly = 0b01,
    // ReadOnly = 0b10,
    // PrivReadOnly = 0b11
    #[bits(2)]
    pub access_perm: u64,
    pub non_secure: bool,
}

#[bitfield(u64)]
pub struct PageBlockEntry {
    pub valid: bool,
    pub page: bool,
    #[bits(3)]
    pub mair_idx: usize,
    #[bits(1)]
    _mbz0: u64,
    // PrivOnly = 0b00,
    // ReadWrite = 0b01,
    // PrivReadOnly = 0b10,
    // ReadOnly = 0b11
    #[bits(2)]
    pub access_perm: u64,
    // NonShareable = 0b00,
    // OuterShareable = 0b10,
    // InnerShareable = 0b11
    #[bits(2)]
    pub share_perm: u64,
    pub accessed: bool,
    pub not_global: bool,
    #[bits(35)]
    pub address_pfn: u64,
    #[bits(4)]
    _mbz1: u64,
    pub dirty: bool,
    pub contig: bool,
    pub priv_x_never: bool,
    pub user_x_never: bool,
    #[bits(9)]
    _mbz2: u64,
}

#[cfg(target_arch = "aarch64")]
pub mod access {
    use super::*;
    use core::arch::asm;

    #[macro_export]
    macro_rules! load_sys_reg {
        ($reg:ident) => {{
            let reg_val: u64;
            unsafe {
                asm!(concat!("mrs {}, ", stringify!($reg)), out(reg) reg_val);
            }
            reg_val
        }};
    }

    #[macro_export]
    macro_rules! store_sys_reg {
        ($reg:ident, $val:expr) => {{
            let val: u64 = $val;
            unsafe {
                asm!(concat!("msr ", stringify!($reg), ", {}; ", "dsb ishst; dsb ish; isb"), in(reg) val);
            }
        }};
    }

    pub trait Aarch64Register: core::fmt::Debug {
        fn load(&mut self);
        fn name(&self) -> &'static str;
        fn bits(&self) -> u64;
    }

    macro_rules! impl_register_access {
        ($register_type:ident, $register:ident) => {
            impl Aarch64Register for $register_type {
                fn load(&mut self) {
                    let val: u64 = load_sys_reg!($register).into();
                    *self = Self::from(val);
                }

                fn name(&self) -> &'static str {
                    stringify!($register)
                }

                fn bits(&self) -> u64 {
                    (*self).into()
                }
            }

            impl $register_type {
                pub fn store(&mut self) {
                    let val: u64 = (*self).into();
                    store_sys_reg!($register, val)
                }
            }
        };
    }

    macro_rules! impl_register_access_ro {
        ($register_type:ident, $register:ident) => {
            impl Aarch64Register for $register_type {
                fn load(&mut self) {
                    let val: u64 = load_sys_reg!($register).into();
                    *self = Self::from(val);
                }

                fn name(&self) -> &'static str {
                    stringify!($register)
                }

                fn bits(&self) -> u64 {
                    (*self).into()
                }
            }
        };
    }

    impl_register_access_ro!(MainIdEl1, MIDR_EL1);
    impl_register_access_ro!(ProcessorFeatures0El1, ID_AA64PFR0_EL1);
    impl_register_access_ro!(ProcessorFeatures1El1, ID_AA64PFR1_EL1);
    impl_register_access_ro!(MmFeatures0El1, ID_AA64MMFR0_EL1);
    impl_register_access_ro!(MmFeatures1El1, ID_AA64MMFR1_EL1);
    impl_register_access_ro!(MmFeatures2El1, ID_AA64MMFR2_EL1);
    impl_register_access_ro!(MmFeatures3El1, ID_AA64MMFR3_EL1);
    impl_register_access_ro!(MmFeatures4El1, ID_AA64MMFR4_EL1);

    impl_register_access_ro!(CurrentEl, CurrentEL);

    impl_register_access!(SystemControlEl1, SCTLR_EL1);
    impl_register_access!(VectorBaseEl1, VBAR_EL1);
    impl_register_access!(ExceptionLinkEl1, ELR_EL1);
    impl_register_access!(ExceptionSyndromeEl1, ESR_EL1);
    impl_register_access!(SavedProgramStateEl1, SPSR_EL1);
    impl_register_access!(TranslationControlEl1, TCR_EL1);
    impl_register_access!(TranslationBase0El1, TTBR0_EL1);
    impl_register_access!(TranslationBase1El1, TTBR1_EL1);
    impl_register_access!(MemoryAttributeIndirectionEl1, MAIR_EL1);

    #[macro_export]
    macro_rules! register {
        ($reg:ident) => {
            &mut $reg::new() as &mut dyn Aarch64Register
        };
    }
}
