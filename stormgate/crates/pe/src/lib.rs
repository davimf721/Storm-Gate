//! Minimal PE/COFF reader.
//!
//! Only what the launcher needs is parsed: machine type, subsystem and the
//! names of imported (and delay-imported) DLLs. The parser reads small chunks
//! at known offsets, so inspecting multi-gigabyte executables is cheap, and it
//! never trusts sizes from the file without bounds checks.

mod api;
mod testutil;

pub use api::{GraphicsApi, GraphicsApis};
#[doc(hidden)]
pub use testutil::build_test_pe;

use serde::Serialize;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const MAX_SECTIONS: u16 = 96;
const MAX_IMPORTS: usize = 4096;
const MAX_NAME: usize = 256;

#[derive(Debug, thiserror::Error)]
pub enum PeError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("not a PE executable: {0}")]
    Malformed(&'static str),
}

/// CPU architecture of a PE image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Machine {
    I386,
    Amd64,
    Arm64,
    Arm,
    Unknown(u16),
}

impl Machine {
    fn from_raw(raw: u16) -> Self {
        match raw {
            0x014c => Machine::I386,
            0x8664 => Machine::Amd64,
            0xaa64 => Machine::Arm64,
            0x01c4 => Machine::Arm,
            other => Machine::Unknown(other),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Machine::I386 => "x86 (32-bit)",
            Machine::Amd64 => "x86_64",
            Machine::Arm64 => "arm64",
            Machine::Arm => "arm (32-bit)",
            Machine::Unknown(_) => "unknown",
        }
    }

    /// Whether Storm Gate's Tier 1 path (Wine x86_64 under Rosetta 2) can run it.
    pub fn is_tier1(self) -> bool {
        matches!(self, Machine::I386 | Machine::Amd64)
    }
}

/// Windows subsystem of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Subsystem {
    Gui,
    Console,
    Native,
    Other(u16),
}

impl Subsystem {
    fn from_raw(raw: u16) -> Self {
        match raw {
            1 => Subsystem::Native,
            2 => Subsystem::Gui,
            3 => Subsystem::Console,
            other => Subsystem::Other(other),
        }
    }
}

/// Summary of a PE image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PeInfo {
    pub machine: Machine,
    pub pe32_plus: bool,
    pub is_dll: bool,
    pub subsystem: Subsystem,
    /// Lower-cased imported DLL names, in file order, deduplicated.
    pub imports: Vec<String>,
    /// Lower-cased delay-loaded DLL names.
    pub delay_imports: Vec<String>,
}

impl PeInfo {
    /// Every DLL the image may load statically or lazily.
    pub fn all_imports(&self) -> impl Iterator<Item = &str> {
        self.imports
            .iter()
            .chain(self.delay_imports.iter())
            .map(String::as_str)
    }

    pub fn graphics_apis(&self) -> GraphicsApis {
        GraphicsApis::from_imports(self.all_imports())
    }
}

/// Random access byte source.
trait Source {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()>;
}

impl Source for File {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(buf)
    }
}

struct Slice<'a>(&'a [u8]);

impl Source for Slice<'_> {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let start = usize::try_from(offset).map_err(|_| eof())?;
        let end = start.checked_add(buf.len()).ok_or_else(eof)?;
        let src = self.0.get(start..end).ok_or_else(eof)?;
        buf.copy_from_slice(src);
        Ok(())
    }
}

fn eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "read past end of file")
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn u64_at(b: &[u8], o: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[o..o + 8]);
    u64::from_le_bytes(a)
}

#[derive(Clone, Copy)]
struct Section {
    va: u32,
    vsize: u32,
    raw_ptr: u32,
    raw_size: u32,
}

struct Image {
    sections: Vec<Section>,
}

impl Image {
    fn rva_to_offset(&self, rva: u32) -> Option<u64> {
        self.sections.iter().find_map(|s| {
            let size = s.vsize.max(s.raw_size);
            if rva >= s.va && rva < s.va.checked_add(size)? {
                let delta = rva - s.va;
                (delta < s.raw_size).then(|| u64::from(s.raw_ptr) + u64::from(delta))
            } else {
                None
            }
        })
    }

    fn read_cstr(&self, src: &mut dyn Source, rva: u32) -> Option<String> {
        let off = self.rva_to_offset(rva)?;
        let mut out = Vec::new();
        let mut chunk = [0u8; 32];
        while out.len() < MAX_NAME {
            // Near the end of the file a full chunk may not exist; fall back to bytes.
            if src.read_at(off + out.len() as u64, &mut chunk).is_err() {
                let mut one = [0u8; 1];
                src.read_at(off + out.len() as u64, &mut one).ok()?;
                if one[0] == 0 {
                    break;
                }
                out.push(one[0]);
                continue;
            }
            if let Some(nul) = chunk.iter().position(|&c| c == 0) {
                out.extend_from_slice(&chunk[..nul]);
                break;
            }
            out.extend_from_slice(&chunk);
        }
        let name = String::from_utf8(out).ok()?;
        (!name.is_empty() && name.is_ascii()).then(|| name.to_ascii_lowercase())
    }
}

/// Inspects a PE file on disk.
pub fn inspect_file(path: &Path) -> Result<PeInfo, PeError> {
    let mut file = File::open(path)?;
    parse(&mut file)
}

/// Inspects a PE image held in memory.
pub fn inspect_bytes(bytes: &[u8]) -> Result<PeInfo, PeError> {
    parse(&mut Slice(bytes))
}

fn parse(src: &mut dyn Source) -> Result<PeInfo, PeError> {
    let mut dos = [0u8; 64];
    src.read_at(0, &mut dos)
        .map_err(|_| PeError::Malformed("file too small"))?;
    if &dos[0..2] != b"MZ" {
        return Err(PeError::Malformed("missing MZ signature"));
    }
    let pe_off = u64::from(u32_at(&dos, 0x3c));

    let mut hdr = [0u8; 24];
    src.read_at(pe_off, &mut hdr)
        .map_err(|_| PeError::Malformed("truncated PE header"))?;
    if &hdr[0..4] != b"PE\0\0" {
        return Err(PeError::Malformed("missing PE signature"));
    }
    let machine = Machine::from_raw(u16_at(&hdr, 4));
    let num_sections = u16_at(&hdr, 6);
    let opt_size = usize::from(u16_at(&hdr, 20));
    let characteristics = u16_at(&hdr, 22);
    if num_sections > MAX_SECTIONS {
        return Err(PeError::Malformed("too many sections"));
    }

    let opt_off = pe_off + 24;
    let mut opt = vec![0u8; opt_size];
    src.read_at(opt_off, &mut opt)
        .map_err(|_| PeError::Malformed("truncated optional header"))?;
    if opt.len() < 2 {
        return Err(PeError::Malformed("missing optional header"));
    }
    let pe32_plus = match u16_at(&opt, 0) {
        0x10b => false,
        0x20b => true,
        _ => return Err(PeError::Malformed("unknown optional header magic")),
    };
    // Offsets of Subsystem, ImageBase, NumberOfRvaAndSizes and the data directories.
    let (subsys_off, base_off, ndirs_off, dirs_off) = if pe32_plus {
        (68, 24, 108, 112)
    } else {
        (68, 28, 92, 96)
    };
    if opt.len() < dirs_off {
        return Err(PeError::Malformed("optional header too small"));
    }
    let subsystem = Subsystem::from_raw(u16_at(&opt, subsys_off));
    let image_base = if pe32_plus {
        u64_at(&opt, base_off)
    } else {
        u64::from(u32_at(&opt, base_off))
    };
    let ndirs = (u32_at(&opt, ndirs_off) as usize).min((opt.len() - dirs_off) / 8);
    let dir = |idx: usize| -> Option<(u32, u32)> {
        (idx < ndirs).then(|| {
            let o = dirs_off + idx * 8;
            (u32_at(&opt, o), u32_at(&opt, o + 4))
        })
    };

    let sec_off = opt_off + opt_size as u64;
    let mut raw = vec![0u8; usize::from(num_sections) * 40];
    src.read_at(sec_off, &mut raw)
        .map_err(|_| PeError::Malformed("truncated section table"))?;
    let sections = raw
        .chunks_exact(40)
        .map(|s| Section {
            vsize: u32_at(s, 8),
            va: u32_at(s, 12),
            raw_size: u32_at(s, 16),
            raw_ptr: u32_at(s, 20),
        })
        .collect();
    let image = Image { sections };

    let mut imports = Vec::new();
    if let Some((rva, _)) = dir(1).filter(|(rva, _)| *rva != 0) {
        read_descriptors(src, &image, rva, 20, 12, |_| 0, &mut imports);
    }
    let mut delay_imports = Vec::new();
    if let Some((rva, _)) = dir(13).filter(|(rva, _)| *rva != 0) {
        // Attribute bit 0 set => RVAs; otherwise (legacy VC6) virtual addresses.
        read_descriptors(
            src,
            &image,
            rva,
            32,
            4,
            |desc: &[u8]| {
                if u32_at(desc, 0) & 1 == 1 {
                    0
                } else {
                    image_base
                }
            },
            &mut delay_imports,
        );
    }

    Ok(PeInfo {
        machine,
        pe32_plus,
        is_dll: characteristics & 0x2000 != 0,
        subsystem,
        imports,
        delay_imports,
    })
}

fn read_descriptors(
    src: &mut dyn Source,
    image: &Image,
    table_rva: u32,
    desc_size: usize,
    name_field: usize,
    base_for: impl Fn(&[u8]) -> u64,
    out: &mut Vec<String>,
) {
    let mut desc = vec![0u8; desc_size];
    for i in 0..MAX_IMPORTS {
        let Some(rva) = table_rva.checked_add((i * desc_size) as u32) else {
            break;
        };
        let Some(off) = image.rva_to_offset(rva) else {
            break;
        };
        if src.read_at(off, &mut desc).is_err() || desc.iter().all(|&b| b == 0) {
            break;
        }
        let raw_name = u64::from(u32_at(&desc, name_field));
        let Some(name_rva) = raw_name
            .checked_sub(base_for(&desc))
            .and_then(|r| u32::try_from(r).ok())
        else {
            continue;
        };
        if let Some(name) = image.read_cstr(src, name_rva) {
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_synthetic_pe64() {
        let bytes = build_test_pe(
            true,
            &["KERNEL32.dll", "d3d11.dll", "dxgi.dll"],
            &["XINPUT1_4.dll"],
        );
        let info = inspect_bytes(&bytes).unwrap();
        assert_eq!(info.machine, Machine::Amd64);
        assert!(info.pe32_plus);
        assert_eq!(info.subsystem, Subsystem::Gui);
        assert_eq!(info.imports, vec!["kernel32.dll", "d3d11.dll", "dxgi.dll"]);
        assert_eq!(info.delay_imports, vec!["xinput1_4.dll"]);
        assert!(info.graphics_apis().contains(GraphicsApi::D3D11));
    }

    #[test]
    fn parses_synthetic_pe32() {
        let bytes = build_test_pe(false, &["d3d9.dll"], &[]);
        let info = inspect_bytes(&bytes).unwrap();
        assert_eq!(info.machine, Machine::I386);
        assert!(!info.pe32_plus);
        assert_eq!(info.imports, vec!["d3d9.dll"]);
    }

    #[test]
    fn parses_from_file() {
        let dir = std::env::temp_dir().join(format!("stormgate-pe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("game.exe");
        std::fs::write(&path, build_test_pe(true, &["d3d12.dll"], &[])).unwrap();
        let info = inspect_file(&path).unwrap();
        assert_eq!(info.imports, vec!["d3d12.dll"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_garbage() {
        assert!(inspect_bytes(b"").is_err());
        assert!(inspect_bytes(&[0u8; 128]).is_err());
        let mut bytes = build_test_pe(true, &["a.dll"], &[]);
        bytes.truncate(300);
        assert!(inspect_bytes(&bytes).is_err());
    }

    #[test]
    fn survives_corrupted_offsets() {
        let base = build_test_pe(true, &["d3d11.dll"], &[]);
        // Flip every byte one at a time in the headers; parsing must never panic.
        for i in 0..base.len().min(1024) {
            let mut b = base.clone();
            b[i] ^= 0xff;
            let _ = inspect_bytes(&b);
        }
    }
}
