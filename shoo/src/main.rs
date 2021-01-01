#![no_std]
#![no_main]
#![feature(asm, panic_info_message)]
#![feature(const_fn)]

mod interrupts;
mod isr;
mod loader;
mod task;

use core::mem::{self, MaybeUninit};
use core::slice;
use core::{ffi::c_void, sync::atomic::Ordering};

#[macro_use]
extern crate riscv;
use addr::{PHYSMEM, PHYSMEM_MAP};
use goblin::elf64::program_header::{ProgramHeader, PT_LOAD};
use loader::flags_to_riscv;
use microflop::FileName;
use riscv::addr;
use riscv::addr::PHYSMEM_LEN;
use riscv::arch::*;
use riscv::globals::*;
use riscv::print;
use riscv_paging::{Addr, PageSize, PageTable, PhysAccess, PteAttrs, VirtAddr, VirtSize};

use bitvec::prelude::*;
use fdt_rs::{base::DevTree, error::DevTreeError};
use fdt_rs::{base::DevTreeNode, prelude::*};
use log::info;
use spanner::Span;

const BANNER: &'static str = include_str!("logo.txt");

// TODO: how do I get these into assembly in my fault handler? I could put a pointer
// into the Task structure I suppose?? idk what the fuck im doing
pub static EXCEPTION_STACKS: PerHartMut<[u8; 8192]> = PerHartMut::new();

extern "C" {
    static SUPERVISOR_VECTORS: c_void;
    #[link_name = "stext"]
    static SEC_TEXT: c_void;
    #[link_name = "etext"]
    static SEC_ETEXT: c_void;
    #[link_name = "srodata"]
    static SEC_RODATA: c_void;
    #[link_name = "erodata"]
    static SEC_ERODATA: c_void;
    #[link_name = "srwdata"]
    static SEC_SRWDATA: c_void;
    #[link_name = "end"]
    static SEC_END: c_void;
}

#[no_mangle]
unsafe extern "C" fn startup(core_id: usize, dtb: *const u8) {
    // this function will be hit by as many harts as we have, at once
    // thus, we will spinloop the ones we don't have work for yet
    if core_id != 0 {
        loop {}
    }

    // § 3.1.6 RISC-V privileged ISA
    let mut new_mstatus = get_mstatus();
    // set MPP (previous mode) to supervisor, privilege level 1
    new_mstatus.set_m_prev_pl(ArchPrivilegeLevel::Supervisor);

    set_mstatus(new_mstatus);

    // turn off paging
    set_satp(Satp(0));

    // set the exception return address
    set_mepc(shoo_main as *const _);

    // set the delegated exceptions and interrupts to be all of the base arch ones
    // ... except env calls from S-mode
    set_medeleg(0xffff & !(1 << 9));
    set_mideleg(0xffff);

    // ensure SEIE, STIE, SSIE are on
    let mut sie = get_sie();
    let view = sie.view_bits_mut::<Lsb0>();
    // we don't take interrupts in kernel mode
    view.set(SIE_SEIE, false);
    view.set(SIE_STIE, false);
    view.set(SIE_SSIE, false);
    set_sie(sie);

    // the 1 enables vectored mode
    set_stvec(&SUPERVISOR_VECTORS as *const c_void as u64 | 1);

    interrupts::init_timers();

    // put our hart id into the thread pointer
    set_core_id(core_id);
    riscv::NUM_CPUS.fetch_add(1, Ordering::SeqCst);

    asm!("mret", in("a0") core_id, in("a1") dtb);
    core::hint::unreachable_unchecked();
}

/// Data we get from reading the device tree
struct DtbRead {
    initrd: &'static [u8],
}

fn dump_dt(lvl: u8, dt: &DevTree) -> Result<(), DevTreeError> {
    let mut iter = dt.items();
    loop {
        let itm = iter.next()?;
        if itm.is_none() {
            break;
        }

        let itm = itm.unwrap();
        match itm {
            fdt_rs::base::DevTreeItem::Node(n) => {
                println!("node {:?}", n.name());
                let mut pi = n.props();
                loop {
                    let prop = pi.next()?;
                    if prop.is_none() {
                        break;
                    }
                    let prop = prop.unwrap();
                    println!("- {:?} {:?}", prop.name(), prop.str());
                }
            }
            fdt_rs::base::DevTreeItem::Prop(p) => {
                println!("-- {:?} {:?}", p.name(), p.str());
            }
        }
    }
    // let mut items = node.props();
    // for _ in 0..lvl {
    //     print!("  ");
    // }

    // println!("node {:?}", node.name()?);
    // loop {
    //     let itm = items.next()?;
    //     if itm.is_none() {
    //         break;
    //     }
    //     let itm = itm.unwrap();
    //     for _ in 0..lvl + 1 {
    //         print!("  ");
    //     }
    //     println!("{}: {:?}", itm.name()?, itm.str());
    // }
    Ok(())
}

unsafe fn read_dtb(dtb: *const u8) -> Result<DtbRead, DevTreeError> {
    info!("loading device tree");
    // safety: we'd be hosed if it was not this size so,,
    let len = DevTree::read_totalsize(slice::from_raw_parts(dtb, DevTree::MIN_HEADER_SIZE))?;
    let buf = slice::from_raw_parts(dtb, len);

    let dtb = DevTree::new(buf)?;
    let chosen = dtb
        .nodes()
        .find(|n| Ok(n.name()? == "chosen"))?
        .expect("failed to get initrd");

    let mut props = chosen.props();
    let mut initrd_start = None;
    let mut initrd_end = None;
    while let Some(p) = props.next()? {
        match p.name() {
            Ok("linux,initrd-start") => initrd_start = Some(p.u32(0)?),
            Ok("linux,initrd-end") => initrd_end = Some(p.u32(0)?),
            _ => (),
        }
    }
    let initrd_start = initrd_start.expect("missing initrd start");
    let initrd_end = initrd_end.expect("missing initrd end");

    let initrd = slice::from_raw_parts(
        initrd_start as usize as *const u8,
        (initrd_end - initrd_start) as usize,
    );

    // dump_dt(0, &dtb)?;
    Ok(DtbRead { initrd })
}

unsafe extern "C" fn shoo_main(core_id: usize, dtb: *const u8) -> ! {
    let endaddr = &SEC_END as *const _ as usize;
    if core_id != 0 {
        loop {}
    }

    crate::print::init();
    let DtbRead {
        initrd: initrd_slice,
    } = read_dtb(dtb).expect("dtb");

    // CORE0
    let kern = FileName(*b"kern\0\0\0\0\0\0\0\0\0\0\0");

    let initrd = microflop::Microflop::new(initrd_slice).expect("failed to open initrd");
    let mut files = initrd.files();
    let mut kern_slice = None;
    while let Some((name, content)) = files.next().expect("initrd parse err") {
        if name == kern {
            kern_slice = Some(content);
        }
    }
    let kern_slice = kern_slice.expect("could not find kern in initrd");
    let (hdr, headers) = loader::get_headers(kern_slice).expect("kern elf load err");

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
    // at this stage we don't have anything in the physical memory after our end,
    // of significance, at least
    let kern_ptr = PhysAddr::new(&SEC_END as *const _ as usize)
        .round_up(PageSize::Page4k)
        .unwrap()
        .as_u8_ptr();

    let kern_range_phys = kernel_range
        .offset(-(kernel_range.begin() as isize))
        .offset(kern_ptr as isize);
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
            &kern_slice[header.p_offset as usize..(header.p_offset + header.p_filesz) as usize],
        ));
        // fill till the end of the section
        kern_slice_w[end_idx..end_extra_align].fill(MaybeUninit::new(0));
    }

    log::debug!(
        "kernel range is {:?}, phys: {:?}",
        &kernel_range,
        &kern_range_phys
    );
    info!("init physical memory allocator");
    for page in (endaddr..addr::PHYSMEM + addr::PHYSMEM_LEN).step_by(4096) {
        //println!("wtf {:x}", page);

        // If the page intersects initrd, we don't want to clobber it
        // We don't really care so much about clobbering dtb.
        let page_span = Span::new(page, page + 4096);
        let initrd_span = initrd_slice.into();
        if page_span.intersect(initrd_span).is_some()
            || page_span.intersect(kern_range_phys).is_some()
        {
            continue;
        }
        PhysMem::free(PhysAddr::new(page))
    }

    // println!("{}", BANNER);

    // we will hit this with one core!
    // println!("hello world from risc-v!!");
    get_sstatus();
    get_sip();

    let root_pt = PageTable::<PhysMem>::alloc().expect("root pagetable alloc failed");
    let satp = Satp::new(&root_pt, 0, TranslationMode::Sv39);

    // sets the running task so we can hit exceptions properly
    let task = task::FAULT_TASKS.get(core_id);
    task.hart_id = core_id;
    // crash stack
    // set the pointer to the END of the stack, lol
    task.kernel_sp = EXCEPTION_STACKS.get(core_id).as_mut_ptr().offset(8192) as *mut _;
    task.kernel_satp = satp;
    set_running_task(task as *mut _ as usize);

    // ALL CORES
    for header in headers.iter().filter(|h| h.p_type == PT_LOAD) {
        // load the page into VM space
        let pa = header.p_vaddr as usize - base + kern_range_phys.begin();
        let pa = PhysAddr::new(pa);
        let va = VirtAddr(header.p_vaddr as usize);
        let len = VirtSize(header.p_memsz as usize)
            .round_up(PageSize::Page4k)
            .unwrap()
            .get();
        let flags = flags_to_riscv(header.p_flags);
        log::debug!("map {:?} -> {:?} len {:x} flags {:?}", va, pa, len, flags);
        root_pt
            .virt_map(pa, va, len, flags)
            .expect("failed to map kernel mem");
    }

    info!("map shoo");
    let textaddr = &SEC_TEXT as *const _ as usize;
    let etextaddr = &SEC_ETEXT as *const _ as usize;
    root_pt
        .virt_map(
            PhysAddr::new(textaddr),
            VirtAddr(textaddr),
            etextaddr.checked_sub(textaddr).unwrap(),
            PteAttrs::R | PteAttrs::X,
        )
        .unwrap();

    let rodataaddr = &SEC_RODATA as *const _ as usize;
    let erodataaddr = &SEC_ERODATA as *const _ as usize;
    root_pt
        .virt_map(
            PhysAddr::new(rodataaddr),
            VirtAddr(rodataaddr),
            erodataaddr.checked_sub(rodataaddr).unwrap(),
            PteAttrs::R.into(),
        )
        .unwrap();

    let srwdataaddr = &SEC_SRWDATA as *const _ as usize;
    root_pt
        .virt_map(
            PhysAddr::new(srwdataaddr),
            VirtAddr(srwdataaddr),
            endaddr.checked_sub(srwdataaddr).unwrap(),
            PteAttrs::R | PteAttrs::W,
        )
        .unwrap();

    log::info!("map phys mem");
    for offs in (0..PHYSMEM_LEN + PHYSMEM).step_by(PageSize::Page1g.size()) {
        root_pt
            .virt_map_one(
                PhysAddr::new(offs),
                VirtAddr(addr::PHYSMEM_MAP + offs),
                PageSize::Page1g,
                PteAttrs::R | PteAttrs::W,
            )
            .unwrap();
    }

    root_pt
        .virt_map(
            PhysAddr::new(addr::UART0),
            VirtAddr(addr::UART0),
            addr::UART0LEN,
            PteAttrs::R | PteAttrs::W,
        )
        .unwrap();

    // TODO: this is probably actually not usable from S-mode so we can probably
    // not map it
    root_pt
        .virt_map(
            PhysAddr::new(addr::CLINT),
            VirtAddr(addr::CLINT),
            addr::CLINT_LEN,
            PteAttrs::R | PteAttrs::W,
        )
        .unwrap();

    info!("allocate kernel stack");
    // make a new kernel stack
    let kstack_begin = PHYSMEM_MAP - 0x8000;
    for page in (kstack_begin..PHYSMEM_MAP).step_by(0x1000) {
        root_pt
            .virt_alloc_one(VirtAddr::new(page), PteAttrs::R | PteAttrs::W)
            .expect("failed to alloc kernel stack");
    }

    let k_entry_va = hdr.e_entry;

    set_satp(satp);
    info!("paging enabled, jumping to the kernel");

    // jmp kernel!!!! hell yeah
    asm!(
        "mv sp, {}",
        "jr {}",
        "1: j 1b",
        in (reg) PHYSMEM_MAP, // end of the kernel stack
        in (reg) k_entry_va,
        in ("a0") core_id,
        options(noreturn)
    );
}
