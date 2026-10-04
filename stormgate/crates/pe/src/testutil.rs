//! Builder for tiny synthetic PE images, used by tests across the workspace.

/// Builds a minimal, valid PE image with the given import and delay-import
/// DLL names. Not intended for execution; only for parser tests.
pub fn build_test_pe(pe32_plus: bool, imports: &[&str], delay_imports: &[&str]) -> Vec<u8> {
    const SECTION_RVA: u32 = 0x1000;
    const SECTION_RAW: u32 = 0x200;

    // Build .idata section contents.
    let mut data = Vec::new();
    let imp_table = 0usize;
    let imp_size = (imports.len() + 1) * 20;
    let delay_table = imp_table + imp_size;
    let delay_size = (delay_imports.len() + 1) * 32;
    let names_start = delay_table + delay_size;
    data.resize(names_start, 0);

    let mut name_rvas = Vec::new();
    for name in imports.iter().chain(delay_imports.iter()) {
        name_rvas.push(SECTION_RVA + data.len() as u32);
        data.extend_from_slice(name.as_bytes());
        data.push(0);
    }
    for (i, rva) in name_rvas.iter().take(imports.len()).enumerate() {
        let o = imp_table + i * 20 + 12;
        data[o..o + 4].copy_from_slice(&rva.to_le_bytes());
    }
    for (i, rva) in name_rvas.iter().skip(imports.len()).enumerate() {
        let o = delay_table + i * 32;
        data[o..o + 4].copy_from_slice(&1u32.to_le_bytes()); // RVA based
        data[o + 4..o + 8].copy_from_slice(&rva.to_le_bytes());
    }
    while data.len() % 0x200 != 0 {
        data.push(0);
    }

    let mut out = vec![0u8; SECTION_RAW as usize];
    out[0..2].copy_from_slice(b"MZ");
    let pe_off = 0x80u32;
    out[0x3c..0x40].copy_from_slice(&pe_off.to_le_bytes());

    let opt_size: u16 = if pe32_plus { 240 } else { 224 };
    let mut p = pe_off as usize;
    out[p..p + 4].copy_from_slice(b"PE\0\0");
    p += 4;
    let machine: u16 = if pe32_plus { 0x8664 } else { 0x014c };
    out[p..p + 2].copy_from_slice(&machine.to_le_bytes());
    out[p + 2..p + 4].copy_from_slice(&1u16.to_le_bytes());
    out[p + 16..p + 18].copy_from_slice(&opt_size.to_le_bytes());
    out[p + 18..p + 20].copy_from_slice(&0x0022u16.to_le_bytes());
    p += 20;

    let opt = p;
    let magic: u16 = if pe32_plus { 0x20b } else { 0x10b };
    out[opt..opt + 2].copy_from_slice(&magic.to_le_bytes());
    out[opt + 68..opt + 70].copy_from_slice(&2u16.to_le_bytes()); // GUI
    let (ndirs_off, dirs_off) = if pe32_plus { (108, 112) } else { (92, 96) };
    out[opt + ndirs_off..opt + ndirs_off + 4].copy_from_slice(&16u32.to_le_bytes());
    let set_dir = |out: &mut Vec<u8>, idx: usize, rva: u32, size: u32| {
        let o = opt + dirs_off + idx * 8;
        out[o..o + 4].copy_from_slice(&rva.to_le_bytes());
        out[o + 4..o + 8].copy_from_slice(&size.to_le_bytes());
    };
    if !imports.is_empty() {
        set_dir(&mut out, 1, SECTION_RVA + imp_table as u32, imp_size as u32);
    }
    if !delay_imports.is_empty() {
        set_dir(
            &mut out,
            13,
            SECTION_RVA + delay_table as u32,
            delay_size as u32,
        );
    }

    let sec = opt + opt_size as usize;
    out[sec..sec + 6].copy_from_slice(b".idata");
    out[sec + 8..sec + 12].copy_from_slice(&(data.len() as u32).to_le_bytes());
    out[sec + 12..sec + 16].copy_from_slice(&SECTION_RVA.to_le_bytes());
    out[sec + 16..sec + 20].copy_from_slice(&(data.len() as u32).to_le_bytes());
    out[sec + 20..sec + 24].copy_from_slice(&SECTION_RAW.to_le_bytes());

    out.extend_from_slice(&data);
    out
}
