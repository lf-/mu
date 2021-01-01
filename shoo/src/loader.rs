//! ELF loader

use core::convert::TryInto;
use core::mem;
use core::slice;

use goblin::elf64::*;
use header::{Header, ELFMAG};
use program_header::*;
use riscv_paging::{Addr, PageSize, PteAttrs, VirtSize};
use section_header::SectionHeader;

/// Converts the ELF Phdr.p_flags to PteAttrs
pub fn flags_to_riscv(p_flags: u32) -> PteAttrs {
    let mut out = PteAttrs::empty();
    if p_flags & PF_R != 0 {
        out |= PteAttrs::R;
    }
    if p_flags & PF_W != 0 {
        out |= PteAttrs::W;
    }
    if p_flags & PF_X != 0 {
        out |= PteAttrs::X;
    }
    out
}

#[derive(Clone, Copy, Debug)]
pub enum ElfLoadErr {
    Oop,
}

pub fn get_total_size(headers: &[ProgramHeader]) -> usize {
    headers
        .iter()
        .map(|h| {
            VirtSize(h.p_memsz as usize)
                .round_up(PageSize::Page4k)
                .unwrap()
                .get()
        })
        .sum()
}

pub fn get_headers(elf: &[u8]) -> Result<(Header, &[ProgramHeader]), ElfLoadErr> {
    let bits = &elf[..64].try_into().unwrap();
    let hdr = Header::from_bytes(bits);

    assert_eq!(&hdr.e_ident[..4], ELFMAG);
    let phentsize = mem::size_of::<ProgramHeader>();
    assert_eq!(hdr.e_phentsize as usize, phentsize);

    let prog_headers =
        &elf[hdr.e_phoff as usize..hdr.e_phoff as usize + phentsize * hdr.e_phnum as usize];
    let (empty1, prog_headers, empty2) = unsafe { prog_headers.align_to::<ProgramHeader>() };
    assert!(
        empty1.is_empty() && empty2.is_empty(),
        "program headers misaligned?!"
    );

    for phdr in prog_headers {
        let ProgramHeader {
            p_vaddr,
            p_filesz,
            p_memsz,
            p_flags,
            ..
        } = phdr;
        log::debug!(
            "prog header: @{:x} f:{:x} m:{:x} flags:{:x}",
            p_vaddr,
            p_filesz,
            p_memsz,
            p_flags
        );
    }
    Ok((hdr.clone(), prog_headers))
}
