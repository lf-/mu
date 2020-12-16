//! A memory manager for RISC-V. Currently only Sv39 is supported (512GB address
//! space).
#![cfg(all(target_pointer_width = "64", target_arch = "riscv64"))]
#![feature(slice_fill)]
#![feature(asm)]
#![feature(try_trait)]
#![no_std]

use core::{marker::PhantomData, mem};
use core::{option::NoneError, ptr};

use bitvec::prelude::*;

/// Page table entry.
///
/// Format:
/// ```text
///  63  54  53  28   27  19   18  10   9 8   7      0
/// | ZERO | PPN[2] | PPN[1] | PPN[0] | RSW | DAGUXWRV |
/// +--9---+--26----+---9----+---9----+--2--+----8-----+
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pte(u64);

/// A newtype wrapper around a physical address.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysAddr<P: PhysAccess>(usize, PhantomData<P>);

/// A pointer to an object in physical memory
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phys<T, P: PhysAccess> {
    addr: PhysAddr<P>,
    typ: PhantomData<T>,
}

/// A newtype wrapper around a virtual address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtAddr(pub usize);

impl<T, P: PhysAccess> Phys<T, P> {
    /// Makes a new Phys from a raw usize interpreted as a physical pointer
    pub unsafe fn new_raw(addr: usize) -> Phys<T, P> {
        Self::new(PhysAddr(addr, PhantomData))
    }

    /// Gets the contained address
    pub unsafe fn addr(self) -> PhysAddr<P> {
        self.addr
    }

    /// Makes a new Phys pointer from the given PhysAddr
    pub unsafe fn new(addr: PhysAddr<P>) -> Phys<T, P> {
        Phys {
            addr,
            typ: PhantomData,
        }
    }

    /// Gets a pointer to this address in the current virtual memory scheme
    /// It is a Bad Idea to squirrel this pointer away somewhere. Do not.
    pub unsafe fn as_ptr(self) -> *mut T {
        P::address(self.addr)
    }
}

impl<P: PhysAccess> PhysAddr<P> {
    pub fn new(addr: usize) -> PhysAddr<P> {
        PhysAddr(addr, PhantomData)
    }

    pub fn get(self) -> usize {
        self.0
    }

    /// is the given address aligned to a page
    pub fn is_page_aligned(self) -> bool {
        self.0 & PAGE_MASK == 0
    }
}

bitflags::bitflags!(
    /// Attributes you can set on a page table entry.
    pub struct PteAttrs: u8 {
        /// Page has been modified. Set to 1 if this feature is unused.
        const Dirty = 1 << 7;
        /// Page has been accessed. Set to 1 if this feature is unused.
        const Accessed = 1 << 6;
        /// Page is mapped in all address spaces (i.e. for all ASIDs).
        const Global = 1 << 5;
        /// Page is accessible by user mode; page is *in*accessible by supervisor
        /// mode if `mstatus.SUM = 0`
        const User = 1 << 4;
        /// Page executable
        const X = 1 << 3;
        /// Page writable
        const W = 1 << 2;
        /// Page readable
        const R = 1 << 1;
        /// Valid PTE
        const V = 1 << 0;
    }
);

impl Pte {
    /// Makes a page table entry with the given attributes
    fn new<P: PhysAccess>(pa: PhysAddr<P>, attrs: PteAttrs) -> Pte {
        let mut inner = 0u64;
        let a = pa.0.view_bits::<Lsb0>();
        let h = inner.view_bits_mut::<Lsb0>();
        // PPN
        h[10..=53].store(a[12..=55].load::<u64>());
        h[0..=7].store(attrs.bits());
        Pte(inner)
    }

    /// Decomposes a page table entry into attributes and next PPN
    fn decompose(self) -> (u64, PteAttrs) {
        let h = self.0.view_bits::<Lsb0>();
        (
            h[10..=53].load(),
            PteAttrs::from_bits_truncate(h[0..=7].load()),
        )
    }
}

/// Stores our linked list free-list of physical memory pages, as well as other
/// metadata on unused pages (currently not sure on what that will be)
#[derive(Clone, Copy, Debug)]
pub struct PhysPageMetadata<P: PhysAccess> {
    /// Physical address of the next empty page
    pub next: Option<Phys<PhysPageMetadata<P>, P>>,
}

/// Object encapsulating a page table
#[derive(Clone, Copy, Debug)]
pub struct PageTable<P: PhysAccess> {
    base: Phys<Pte, P>,
}

/// Provides access to read/write to physical memory
pub trait PhysAccess: Copy {
    /// Gets a mut pointer to the given PhysAddr
    unsafe fn address<T>(ptr: PhysAddr<Self>) -> *mut T;

    /// Allocates a page by popping it off the page free list, returning the
    /// physical address of the start of that page
    unsafe fn alloc() -> Option<PhysAddr<Self>>;

    /// Frees the given page.
    unsafe fn free(addr: PhysAddr<Self>);
}

impl<P: PhysAccess> PageTable<P> {
    /// Creates a PageTable based at the given address
    pub unsafe fn from_raw(base: Phys<Pte, P>) -> PageTable<P> {
        assert!(
            base.addr.is_page_aligned(),
            "base addr must be aligned to a page"
        );
        PageTable { base }
    }

    /// Gets the base address
    pub fn get_base(&self) -> PhysAddr<P> {
        unsafe { self.base.addr() }
    }

    /// Allocates a page table and zeroes it
    pub unsafe fn alloc() -> Option<PageTable<P>> {
        let allocation = P::alloc()?;
        let base = Phys::new(allocation);
        let base_: *mut Pte = base.as_ptr();

        // zeroing the page sets it all to V = 0
        ptr::write_bytes(base_, 0, PT_ENTRIES);
        Some(PageTable { base })
    }

    /// Initializes the page table to all invalid entries
    pub unsafe fn clear(self) {
        self.base.as_ptr().write_bytes(0u8, PT_ENTRIES);
    }
}

pub const PAGE_SIZE: u64 = 4096;
pub const PT_ENTRIES: usize = PAGE_SIZE as usize / mem::size_of::<Pte>();
pub const PAGE_MASK: usize = PAGE_SIZE as usize - 1;

impl VirtAddr {
    /// Only considers the lower 39 bits, chopping off the top bits
    fn canonicalize(self) -> VirtAddr {
        VirtAddr(self.0.view_bits::<Lsb0>()[0..=38].load())
    }

    /// Aligns the address to a page
    fn page_aligned(self) -> VirtAddr {
        VirtAddr(self.0 & !PAGE_MASK)
    }

    /// Decomposes the address into an array VPN[0], VPN[1], VPN[2]
    fn parts(self) -> [u16; 3] {
        let h = self.0.view_bits::<Lsb0>();
        [h[12..=20].load(), h[21..=29].load(), h[30..=38].load()]
    }

    /// is the given address aligned to a page
    pub fn is_page_aligned(self) -> bool {
        self.0 & PAGE_MASK == 0
    }
}

/// Invalidates the page table cache for all the asids for the given address
// TODO: do this smarter
unsafe fn invalidate_cache(vaddr: VirtAddr) {
    asm!("sfence.vma x0, {vaddr}",
        vaddr = in (reg) vaddr.0);
}

/// Maps a page at virtual address `va` to physical address `pa`. `pa` and `va`
/// must be at least page (4k) aligned.
///
/// Assumes the root_pt is already present and initialized, that we have exclusive
/// access, and that interrupts are disabled.
///
/// Fails if it is trying to map something already mapped.
unsafe fn virt_map_one<P: PhysAccess>(
    root_pt: PageTable<P>,
    pa: PhysAddr<P>,
    va: VirtAddr,
    attrs: PteAttrs,
) -> Result<(), MapError> {
    // there is no reason you would want to map something invalid
    let attrs = attrs | PteAttrs::V;

    assert!(
        pa.is_page_aligned(),
        "mapped phys address must be page aligned"
    );
    assert!(
        va.is_page_aligned(),
        "mapped virt address must be page aligned"
    );
    let pa = PhysAddr::<P>::new(pa.get());
    let va = va.canonicalize().page_aligned();
    let va_parts = va.parts();

    let mut table = root_pt;
    let mut pte_addr;
    let mut level = 2;
    for i in (0..=2).rev() {
        pte_addr = table.base.as_ptr().offset(va_parts[i] as isize);
        let pte = pte_addr.read();
        level = i;

        let (next_ppn, attrs) = pte.decompose();
        if !attrs.contains(PteAttrs::V) {
            // if we hit an invalid entry, we're done as that's where we need
            // to start inserting entries.
            break;
        }

        if attrs.intersects(PteAttrs::R | PteAttrs::X) {
            // it's a leaf page, it's already mapped! oops! leave!
            return Err(MapError::AlreadyMapped);
        }

        assert!(
            i != 0,
            "we should never find non-leaf Ptes at the last level table"
        );
        table = PageTable {
            base: Phys::new_raw((next_ppn * PAGE_SIZE) as usize),
        };
    }

    // we need to allocate some page tables now if we are not at level 0 already
    for i in (1..=level).rev() {
        let entry = table.base.as_ptr().offset(va_parts[i] as isize);
        let next_pt = PageTable::<P>::alloc()?;
        *entry = Pte::new(next_pt.base.addr(), PteAttrs::V);
        table = next_pt;
    }

    // we have now reached level 0 and table points to the level 0 table
    let entry = table.base.as_ptr().offset(va_parts[0] as isize);
    *entry = Pte::new(pa, attrs);
    // TODO: We probably have to have some kind of TLB shootdown thing.
    // or cooperative thing. The reason for this is that the task might get
    // migrated to another core where the bad address is still cached in a TLB
    // The way I want to do this is by having a bitmap of ASIDs to be cleared
    // on next entry, I think.
    //
    // For the minute, we will clear the TLB for the task's ASID on task entry.
    // It's easy but not very good.
    invalidate_cache(va);
    Ok(())
}

#[derive(Debug)]
pub enum MapError {
    /// probably an arithmetic overflow
    NoneError,
    /// address already has been mapped on some level of the page table
    AlreadyMapped,
}

impl core::convert::From<NoneError> for MapError {
    fn from(_: NoneError) -> Self {
        MapError::NoneError
    }
}

/// Maps `len` worth of pages at `pa` to pages starting at the virtual address
/// `va`. The attributes `attrs` along with [PteAttr::V] are given to the created
/// page table entries.
///
/// This assumes the given `root_pt` is allocated and valid, and that this thread
/// has exclusive access to it. (As designed, we do not share page tables cross
/// threads so it should not be an issue)
///
/// If a failure occurs, the state will not be restored, i.e. the pages may be
/// partially mapped.
pub unsafe fn virt_map<P: PhysAccess>(
    root_pt: PageTable<P>,
    pa: PhysAddr<P>,
    va: VirtAddr,
    len: usize,
    attrs: PteAttrs,
) -> Result<(), MapError> {
    assert!(len > 0, "len must be >0");
    // round len up to the nearest page
    let len = (len.checked_add(PAGE_SIZE as usize - 1)?) & !(PAGE_SIZE as usize - 1);

    for offs in (0..len).step_by(PAGE_SIZE as _) {
        virt_map_one(
            root_pt,
            PhysAddr::new(pa.get().checked_add(offs)?),
            VirtAddr(va.0.checked_add(offs)?),
            attrs,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_canonicalize() {
        let addr = 0xff00_0010_1234_5789;
        assert_eq!(canonicalize(VirtAddr(addr)).0, 0x0000_0010_1234_5789);
    }
}
