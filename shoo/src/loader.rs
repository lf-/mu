//! ELF loader

use core::convert::TryInto;
use core::mem;
use core::slice;

use goblin::elf64::*;
use header::{Header, ELFMAG};
use mem::MaybeUninit;
use program_header::*;
use riscv::addr;
use riscv::arch::{PhysAddr, PhysMem};
use riscv_paging::{Addr, MapError, PageSize, PageTable, PhysAccess, PteAttrs, VirtAddr, VirtSize};
use section_header::SectionHeader;
use spanner::Span;

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

pub struct ImageLoadInfo<'a> {
    pub virt_span: Span,
    pub phys_span: Span,
    pub headers: &'a [ProgramHeader],
    pub elf_header: Header,
}

pub unsafe fn map_executable(
    pt: PageTable<PhysMem>,
    phys_range: Span,
    virt_range: Span,
    headers: &[ProgramHeader],
    extra_flags: PteAttrs,
) -> Result<(), MapError> {
    for header in headers.iter().filter(|h| h.p_type == PT_LOAD) {
        // load the page into VM space
        let pa = header.p_vaddr as usize - virt_range.begin() + phys_range.begin();
        let pa = PhysAddr::new(pa);
        let va = VirtAddr(header.p_vaddr as usize);
        let len = VirtSize(header.p_memsz as usize)
            .round_up(PageSize::Page4k)
            .unwrap()
            .get();
        let flags = flags_to_riscv(header.p_flags);
        log::debug!(
            "map {:?} -> {:?} len {:x} flags {:?}",
            va,
            pa,
            len,
            flags | extra_flags
        );
        pt.virt_map(pa, va, len, flags | extra_flags)?;
    }
    Ok(())
}

pub unsafe fn load_image<'a>(image_slice: &'a [u8], start_at: *mut u8) -> ImageLoadInfo<'a> {
    let (hdr, headers) = get_headers(image_slice).expect("kern elf load err");

    let header_to_span = |h: &ProgramHeader| {
        if h.p_type != PT_LOAD {
            None
        } else {
            Some(Span::new(
                h.p_vaddr as usize,
                VirtSize(h.p_vaddr as usize + h.p_memsz as usize)
                    .round_up(PageSize::Page4k)
                    .unwrap()
                    .get(),
            ))
        }
    };
    let first_header = headers
        .iter()
        .filter_map(header_to_span)
        .position(|s| s.len() != 0)
        .expect("no nonempty regions in the kern elf");
    let kernel_range = headers[first_header + 1..]
        .iter()
        .filter_map(header_to_span)
        .filter(|s| s.len() != 0)
        .try_fold(header_to_span(&headers[first_header]).unwrap(), |s1, s2| {
            s1.merge(s2)
        })
        .expect("kernel regions not contiguous");

    // next, load the kernel into contiguous physical memory

    let kern_range_phys = kernel_range
        .offset(-(kernel_range.begin() as isize))
        .offset(start_at as isize);
    let kern_slice_w = kern_range_phys.as_slice_mut::<MaybeUninit<u8>>();
    let base = kernel_range.begin();

    // load the image into memory
    for header in headers.iter().filter(|h| h.p_type == PT_LOAD) {
        let start_idx = header.p_vaddr as usize - base;
        let end_idx = start_idx + header.p_filesz as usize;
        let extra = (header.p_memsz - header.p_filesz) as usize;
        let end_extra_align = VirtSize(end_idx + extra)
            .round_up(PageSize::Page4k)
            .unwrap()
            .get();
        // transmute is ok because it is transmuting slice of init to slice of
        // MaybeUninit, identical layout.
        kern_slice_w[start_idx..end_idx].copy_from_slice(mem::transmute::<_, &[MaybeUninit<u8>]>(
            &image_slice[header.p_offset as usize..(header.p_offset + header.p_filesz) as usize],
        ));
        // fill till the end of the section
        kern_slice_w[end_idx..end_extra_align].fill(MaybeUninit::new(0));
    }

    log::debug!(
        "kernel range is {:?}, phys: {:?}",
        &kernel_range,
        &kern_range_phys
    );
    ImageLoadInfo {
        virt_span: kernel_range,
        phys_span: kern_range_phys,
        headers,
        elf_header: hdr,
    }
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
