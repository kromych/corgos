//! UEFI Table GUIDs

#![no_std]

use uefi::guid;

pub struct UefiTableGuidName {
    pub guid: uefi::Guid,
    pub name: &'static str,
}

/// Known UEFI table GUIDs.
/// NOTE: Keep sorted by the GUID!
#[cfg(not(feature = "all_uefi_table_guids"))]
const UEFI_TABLE_GUIDS: &[UefiTableGuidName] = &[
    UefiTableGuidName {
        guid: guid!("05ad34ba-6f02-4214-952e-4da0398e2bb9"),
        name: "EfiDxeServicesTableGuid",
    },
    UefiTableGuidName {
        guid: guid!("060cc026-4c0d-4dda-8f41-595fef00a502"),
        name: "MemoryStatusCodeRecordGuid",
    },
    UefiTableGuidName {
        guid: guid!("49152e77-1ada-4764-b7a2-7afefed95e8b"),
        name: "EfiDebugImageInfoTableGuid",
    },
    UefiTableGuidName {
        guid: guid!("4c19049f-4137-4dd3-9c10-8b97a83ffdfa"),
        name: "EfiMemoryTypeInformationGuid",
    },
    UefiTableGuidName {
        guid: guid!("7739f24c-93d7-11d4-9a3a-0090273fc14d"),
        name: "EfiHobListGuid",
    },
    UefiTableGuidName {
        guid: guid!("8868e871-e4f1-11d3-bc22-0080c73c8881"),
        name: "EfiAcpi20TableGuid",
    },
    UefiTableGuidName {
        guid: guid!("dcfa911d-26eb-469f-a220-38b7dc461220"),
        name: "EfiMemoryAttributesTableGuid",
    },
    UefiTableGuidName {
        guid: guid!("eb9d2d30-2d88-11d3-9a16-0090273fc14d"),
        name: "EfiAcpi10TableGuid",
    },
    UefiTableGuidName {
        guid: guid!("eb9d2d31-2d88-11d3-9a16-0090273fc14d"),
        name: "EfiSmbiosTableGuid",
    },
    UefiTableGuidName {
        guid: guid!("ee4e5898-3914-4259-9d6e-dc7bd79403cf"),
        name: "LzmaCustomDecompressGuid",
    },
    UefiTableGuidName {
        guid: guid!("f2fd1544-9794-4a2c-992e-e5bbcf20e394"),
        name: "EfiSmbios3TableGuid",
    },
    UefiTableGuidName {
        guid: guid!("fc1bcdb0-7d31-49aa-936a-a4600d9dd083"),
        name: "EfiCrc32GuidedSectionExtractionGuid",
    },
];

#[cfg(feature = "all_uefi_table_guids")]
const UEFI_TABLE_GUIDS: &[UefiTableGuidName] = include!("all_uefi_table_guids.irs");

pub fn get_uefi_table_name(guid: &uefi::Guid) -> &'static str {
    if let Ok(i) = UEFI_TABLE_GUIDS.binary_search_by_key(guid, |x: &UefiTableGuidName| x.guid) {
        UEFI_TABLE_GUIDS[i].name
    } else {
        "Unknown table GUID"
    }
}

pub fn get_uefi_known_guids_count() -> usize {
    UEFI_TABLE_GUIDS.len()
}
