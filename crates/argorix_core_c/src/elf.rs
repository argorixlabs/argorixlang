//! Minimal ELF64 reader for dependency inspection.
//!
//! Only what the ESP-008.R policy needs: the shared libraries an executable
//! requires, and its symbol names with whether each one is defined here or
//! imported. Reading the file directly keeps `readelf` out of the required
//! toolchain, so the execution host needs a C compiler and nothing else.

use anyhow::{bail, Result};

pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

const SHT_DYNAMIC: u32 = 6;
const SHT_SYMTAB: u32 = 2;
const SHT_DYNSYM: u32 = 11;
const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const SHN_UNDEF: u16 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// True when the symbol is undefined here, that is, imported at run time.
    pub imported: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inspection {
    pub needed: Vec<String>,
    pub symbols: Vec<Symbol>,
}

impl Inspection {
    pub fn imported(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.imported)
            .map(|symbol| symbol.name.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

struct Section {
    kind: u32,
    offset: usize,
    size: usize,
    link: u32,
    entry_size: usize,
    address: u64,
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow::anyhow!("truncated ELF at {offset}"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow::anyhow!("truncated ELF at {offset}"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn u64_at(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| anyhow::anyhow!("truncated ELF at {offset}"))?;
    let mut value = [0u8; 8];
    value.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(value))
}

fn string_at(data: &[u8], table: &Section, index: usize) -> String {
    let start = table.offset + index;
    let end = data[start.min(data.len())..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|length| start + length)
        .unwrap_or(data.len());
    String::from_utf8_lossy(&data[start.min(data.len())..end]).into_owned()
}

pub fn is_elf(data: &[u8]) -> bool {
    data.starts_with(&ELF_MAGIC)
}

/// Read the dynamic dependencies and symbols of a 64-bit little-endian ELF.
pub fn inspect(data: &[u8]) -> Result<Inspection> {
    if !is_elf(data) {
        bail!("not an ELF file");
    }
    if data.get(4) != Some(&2) || data.get(5) != Some(&1) {
        bail!("only 64-bit little-endian ELF is supported");
    }
    let section_offset = u64_at(data, 0x28)? as usize;
    let section_size = u16_at(data, 0x3a)? as usize;
    let section_count = u16_at(data, 0x3c)? as usize;
    let mut sections = Vec::with_capacity(section_count);
    for index in 0..section_count {
        let base = section_offset + index * section_size;
        sections.push(Section {
            kind: u32_at(data, base + 0x04)?,
            address: u64_at(data, base + 0x10)?,
            offset: u64_at(data, base + 0x18)? as usize,
            size: u64_at(data, base + 0x20)? as usize,
            link: u32_at(data, base + 0x28)?,
            entry_size: u64_at(data, base + 0x38)? as usize,
        });
    }

    let mut inspection = Inspection::default();
    for section in sections
        .iter()
        .filter(|section| section.kind == SHT_DYNAMIC)
    {
        inspection
            .needed
            .extend(needed_libraries(data, section, &sections)?);
    }
    for section in sections
        .iter()
        .filter(|section| section.kind == SHT_SYMTAB || section.kind == SHT_DYNSYM)
    {
        let strings = sections
            .get(section.link as usize)
            .ok_or_else(|| anyhow::anyhow!("symbol table without string table"))?;
        inspection.symbols.extend(symbols(data, section, strings)?);
    }
    inspection
        .symbols
        .sort_by(|left, right| left.name.cmp(&right.name));
    inspection.symbols.dedup();
    Ok(inspection)
}

fn needed_libraries(data: &[u8], dynamic: &Section, sections: &[Section]) -> Result<Vec<String>> {
    // DT_STRTAB holds a virtual address; map it back to a section to read it.
    let mut strtab_address = None;
    let mut offsets = Vec::new();
    let mut cursor = dynamic.offset;
    let end = dynamic.offset + dynamic.size;
    while cursor + 16 <= end {
        let tag = u64_at(data, cursor)?;
        let value = u64_at(data, cursor + 8)?;
        match tag {
            DT_NULL => break,
            DT_NEEDED => offsets.push(value as usize),
            DT_STRTAB => strtab_address = Some(value),
            _ => {}
        }
        cursor += 16;
    }
    let Some(address) = strtab_address else {
        return Ok(Vec::new());
    };
    let table = sections
        .iter()
        .find(|section| section.address == address)
        .ok_or_else(|| anyhow::anyhow!("dynamic string table not found"))?;
    Ok(offsets
        .into_iter()
        .map(|offset| string_at(data, table, offset))
        .collect())
}

fn symbols(data: &[u8], table: &Section, strings: &Section) -> Result<Vec<Symbol>> {
    let entry_size = if table.entry_size == 0 {
        24
    } else {
        table.entry_size
    };
    let mut found = Vec::new();
    let mut cursor = table.offset;
    let end = table.offset + table.size;
    while cursor + entry_size <= end {
        let name_index = u32_at(data, cursor)? as usize;
        let section_index = u16_at(data, cursor + 6)?;
        let name = string_at(data, strings, name_index);
        if !name.is_empty() {
            // Versioned names such as `exit@GLIBC_2.2.5` compare by base name.
            let base = name.split('@').next().unwrap_or(&name).to_owned();
            found.push(Symbol {
                name: base,
                imported: section_index == SHN_UNDEF,
            });
        }
        cursor += entry_size;
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_files_that_are_not_elf() {
        assert!(!is_elf(b"MZ\x90\x00"));
        assert!(inspect(b"MZ\x90\x00").is_err());
    }

    #[test]
    fn rejects_32_bit_and_big_endian_objects() {
        let mut header = vec![0u8; 64];
        header[..4].copy_from_slice(&ELF_MAGIC);
        header[4] = 1; // ELFCLASS32
        header[5] = 1;
        assert!(inspect(&header).is_err());
    }

    #[test]
    fn imported_lists_undefined_symbols_once_and_sorted() {
        let inspection = Inspection {
            needed: vec!["libc.so.6".into()],
            symbols: vec![
                Symbol {
                    name: "printf".into(),
                    imported: true,
                },
                Symbol {
                    name: "argorix_trap".into(),
                    imported: false,
                },
                Symbol {
                    name: "exit".into(),
                    imported: true,
                },
                Symbol {
                    name: "printf".into(),
                    imported: true,
                },
            ],
        };
        assert_eq!(inspection.imported(), vec!["exit", "printf"]);
    }
}
