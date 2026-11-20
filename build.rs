//! Embeds `assets/tray-info.ico` as the executable's icon.
//!
//! The MSVC linker accepts compiled resource (`.res`) files as inputs, so the `.ico` is turned
//! into one here instead of depending on `rc.exe` or a resource-compiler crate.

use std::path::Path;

const ICON: &str = "assets/tray-info.ico";
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
/// Explorer shows the icon group with the lowest id as the executable's icon.
const GROUP_ID: u16 = 1;
const MOVEABLE_DISCARDABLE: u16 = 0x1010;
const MOVEABLE_PURE_DISCARDABLE: u16 = 0x1030;
const ICO_DIRECTORY: usize = 6;
const ICO_ENTRY: usize = 16;

fn main() {
    println!("cargo:rerun-if-changed={ICON}");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        println!("cargo:warning=executable icon is embedded only with the MSVC linker");
        return;
    }
    let ico = std::fs::read(ICON).unwrap_or_else(|error| panic!("cannot read {ICON}: {error}"));
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    let res = Path::new(&out_dir).join("tray-info.res");
    std::fs::write(&res, resources(&ico)).expect("cannot write the .res file");
    println!("cargo:rustc-link-arg-bins={}", res.display());
}

/// A `.res` file with one `RT_ICON` per image and the `RT_GROUP_ICON` directory over them.
fn resources(ico: &[u8]) -> Vec<u8> {
    let count = usize::from(u16_at(ico, 4));
    let mut res = Vec::new();
    // A .res file starts with an empty entry that marks it as 32-bit.
    push_entry(&mut res, 0, 0, 0, &[]);
    let mut group = Vec::new();
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&(count as u16).to_le_bytes());
    for index in 0..count {
        let entry = &ico[ICO_DIRECTORY + index * ICO_ENTRY..][..ICO_ENTRY];
        let size = u32_at(entry, 8) as usize;
        let offset = u32_at(entry, 12) as usize;
        let id = index as u16 + 1;
        push_entry(
            &mut res,
            RT_ICON,
            id,
            MOVEABLE_DISCARDABLE,
            &ico[offset..offset + size],
        );
        // GRPICONDIRENTRY: the ICONDIRENTRY's first 12 bytes, then the resource id.
        group.extend_from_slice(&entry[..12]);
        group.extend_from_slice(&id.to_le_bytes());
    }
    push_entry(
        &mut res,
        RT_GROUP_ICON,
        GROUP_ID,
        MOVEABLE_PURE_DISCARDABLE,
        &group,
    );
    res
}

/// RESOURCEHEADER with numeric type and name, neutral language, then DWORD-aligned data.
fn push_entry(res: &mut Vec<u8>, kind: u16, name: u16, flags: u16, data: &[u8]) {
    const HEADER_SIZE: u32 = 32;
    res.extend_from_slice(&(data.len() as u32).to_le_bytes());
    res.extend_from_slice(&HEADER_SIZE.to_le_bytes());
    for id in [kind, name] {
        res.extend_from_slice(&0xFFFFu16.to_le_bytes());
        res.extend_from_slice(&id.to_le_bytes());
    }
    res.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    res.extend_from_slice(&flags.to_le_bytes());
    res.extend_from_slice(&0u16.to_le_bytes()); // LanguageId: neutral
    res.extend_from_slice(&0u32.to_le_bytes()); // Version
    res.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    res.extend_from_slice(data);
    res.resize(res.len().next_multiple_of(4), 0);
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("4 bytes"))
}
