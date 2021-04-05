//! A memory manager for RISC-V. Currently only Sv39 is supported (512GB address
//! space).
// TODO: This is doing a whole load of unsound shit with forgetting to use
// volatile ops with pointers
#![cfg(any(all(target_pointer_width = "64", test), target_arch = "riscv64"))]
#![feature(asm)]
#![feature(try_trait)]
#![allow(non_upper_case_globals)]
#![no_std]

use core::{marker::PhantomData, mem};
use core::{option::NoneError, ptr};

use bitvec::prelude::*;

pub const PAGE_SIZE: u64 = 4096;
pub const PT_ENTRIES: usize = PAGE_SIZE as usize / mem::size_of::<Pte>();
pub const PAGE_MASK: usize = PAGE_SIZE as usize - 1;

/// A newtype wrapper around a physical address.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PhysAddr<P: PhysAccess>(usize, PhantomData<P>);

/// A newtype wrapper around a virtual address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(pub usize);

/// A newtype wrapper around a virtual space size
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtSize(pub usize);

/// Operations that can be performed on a memory address
pub trait Addr: Copy {
    /// Returns the numeric value of the address
    fn get(self) -> usize;

    /// Makes an [`Addr`] instance for a numeric address
    fn new(addr: usize) -> Self;

    /// is the given address aligned to a page
    fn is_page_aligned(self, size: PageSize) -> bool {
        self.get() & size.offs_mask() == 0
    }

    /// Given a fallible mapping function `f`, map the numeric value of the
    /// address to another value
    fn map_r<F>(self, f: F) -> Result<Self, MapError>
    where
        F: FnOnce(usize) -> Result<usize, MapError>,
    {
        Ok(Self::new(f(self.get())?))
    }

    /// Given a mapping function `f`, map the numeric value of the address to
    /// another value
    fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(usize) -> usize,
    {
        Self::new(f(self.get()))
    }

    /// Rounds the address up to the [`PageSize`] given
    fn round_up(self, size: PageSize) -> Option<Self> {
        Some(Self::new(
            self.get().checked_add(size.offs_mask())? & !(size.offs_mask()),
        ))
    }

    fn round_down(self, size: PageSize) -> Option<Self> {
        Some(Self::new(self.get() & !size.offs_mask()))
    }
}

impl<P: PhysAccess> PhysAddr<P> {
    pub fn new(addr: usize) -> PhysAddr<P> {
        PhysAddr(addr, PhantomData)
    }

    /// Gets the PhysAddr as a pointer to u8 in whatever virtual space is
    /// currently active.
    pub unsafe fn as_u8_ptr(self) -> *mut u8 {
        P::address(self)
    }
}

// apparently I have to impl these myself because it tries to generate bounds
// on my value of P, which does not matter to equality/ordering

impl<P: PhysAccess, T> core::fmt::Debug for Phys<T, P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Phys({:016x})", self.addr.0)
    }
}

impl<P: PhysAccess> core::fmt::Debug for PhysAddr<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("PhysAddr")
            .field(&format_args!("0x{:016x}", self.0))
            .finish()
    }
}

impl core::fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("VirtAddr")
            .field(&format_args!("0x{:016x}", self.0))
            .finish()
    }
}

impl<P: PhysAccess> PartialEq for PhysAddr<P> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<P: PhysAccess> Eq for PhysAddr<P> {}

impl<P: PhysAccess> PartialOrd for PhysAddr<P> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<P: PhysAccess> Ord for PhysAddr<P> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<P: PhysAccess> Addr for PhysAddr<P> {
    fn new(addr: usize) -> Self {
        Self(addr, PhantomData)
    }

    fn get(self) -> usize {
        self.0
    }
}

impl VirtAddr {
    /// Gets the virtual address as an arbitrary pointer type
    ///
    /// This function is probably a mistake.
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut _
    }

    /// Only considers the lower 39 bits, chopping off the top bits
    fn canonicalize(self) -> VirtAddr {
        VirtAddr(self.0.view_bits::<Lsb0>()[0..=38].load())
    }

    /// Decomposes the address into an array `VPN[0]`, `VPN[1]`, `VPN[2]`
    fn parts(self) -> [u16; 3] {
        let h = self.0.view_bits::<Lsb0>();
        [h[12..=20].load(), h[21..=29].load(), h[30..=38].load()]
    }
}

impl Addr for VirtAddr {
    fn get(self) -> usize {
        self.0
    }

    fn new(addr: usize) -> Self {
        VirtAddr(addr)
    }
}

impl Addr for VirtSize {
    fn get(self) -> usize {
        self.0
    }

    fn new(addr: usize) -> Self {
        VirtSize(addr)
    }
}

/// A pointer to an object in physical memory
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Phys<T, P: PhysAccess> {
    addr: PhysAddr<P>,
    typ: PhantomData<T>,
}

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

//==============================================================================

/// Page table entry.
///
/// Format:
/// ```text
///  63  54  53  28   27  19   18  10   9 8   7      0
/// | ZERO | PPN[2] | PPN[1] | PPN[0] | RSW | DAGUXWRV |
/// +--9---+--26----+---9----+---9----+--2--+----8-----+
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pte(u64);

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

impl PteAttrs {
    fn is_leaf(self) -> bool {
        self.intersects(PteAttrs::R | PteAttrs::W | PteAttrs::X)
    }
}

impl Pte {
    const UNMAPPED: Pte = Pte(0);

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

impl core::fmt::Debug for Pte {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (ppn, flags) = self.decompose();
        write!(f, "Pte(ppn={:016x}, flags={:?})", ppn, flags)
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
            base.addr.is_page_aligned(PageSize::Page4k),
            "base addr must be aligned to a page"
        );
        PageTable { base }
    }

    /// Gets the entry at the index `num` in the page table. Panics if it is out
    /// of range (this is always a bug).
    pub unsafe fn entry(&self, num: u16) -> Pte {
        self.entry_ptr(num).read_volatile()
    }

    /// Gets a pointer to the given page table entry. Panics if it is out of
    /// range.
    pub fn entry_ptr(&self, num: u16) -> *mut Pte {
        assert!(num < PT_ENTRIES as u16, "page table entry out of range");
        unsafe { self.base.as_ptr().add(num as usize) }
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

/// The size of a page
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageSize {
    Page4k = 0,
    Page2m = 1,
    Page1g = 2,
}

impl PageSize {
    /// An iterator of page sizes in descending size order
    const SIZES_DESC: [PageSize; 3] = [PageSize::Page1g, PageSize::Page2m, PageSize::Page4k];

    /// Returns the numeric size of the page.
    #[inline]
    pub fn size(&self) -> usize {
        match self {
            PageSize::Page4k => 4096,
            PageSize::Page2m => 2 * 1024 * 1024,
            PageSize::Page1g => 1 * 1024 * 1024 * 1024,
        }
    }

    /// Gets the mask to get the offset of an address with respect to the
    /// [`PageSize`]
    #[inline]
    pub fn offs_mask(&self) -> usize {
        self.size() - 1
    }
}

/// The result of walking the page table for some virtual address.
#[derive(Debug, Clone, Copy, Default)]
pub struct PageWalkResult {
    /// The up to three Ptes that are encountered on the walk
    pub parts: [Option<Pte>; 3],

    /// If the resolution succeeded, this will be Some with the last level PTE
    pub last_level: Option<Pte>,
}

/// Invalidates the page table cache for all the asids for the given address
// TODO: do this smarter
unsafe fn invalidate_cache(vaddr: VirtAddr) {
    asm!("sfence.vma x0, {vaddr}",
        vaddr = in (reg) vaddr.0);
}

impl<P: PhysAccess> PageTable<P> {
    /// Resolves a virtual address using a page table, returning all the relevant PTEs.
    pub unsafe fn resolve(self, va: VirtAddr) -> Result<PageWalkResult, MapError> {
        log::debug!("resolve {:?}", va);
        let parts = va.parts();
        let mut res: PageWalkResult = Default::default();
        let mut pt = self;
        for level in (0..=2).rev() {
            let part = parts[level];
            log::debug!(
                "level {:?} part {:?} table at {:?}",
                level,
                part,
                pt.get_base()
            );
            let pte = pt.entry(part);
            let (pnum, attrs) = pte.decompose();
            res.parts[level] = Some(pte);

            if !attrs.contains(PteAttrs::V) {
                // invalid entry
                break;
            }
            if attrs.intersects(PteAttrs::R | PteAttrs::X) {
                // last level entry
                res.last_level = Some(pte);
                return Ok(res);
            }
            pt = PageTable::from_raw(Phys::new_raw((pnum * PAGE_SIZE) as usize));
        }
        Ok(res)
    }

    /// Maps a page at virtual address `va` to physical address `pa`. `pa` and `va`
    /// must be at least page (4k) aligned.
    ///
    /// Assumes the root_pt is already present and initialized, that we have exclusive
    /// access, and that interrupts are disabled.
    ///
    /// Fails if it is trying to map something already mapped.
    pub unsafe fn virt_map_one(
        self,
        pa: PhysAddr<P>,
        va: VirtAddr,
        size: PageSize,
        attrs: PteAttrs,
    ) -> Result<(), MapError> {
        // there is no reason you would want to map something invalid
        let attrs = attrs | PteAttrs::V;
        log::debug!(
            "virt_map_one pa={:?}, va={:?}, size={:?}, attrs={:?}",
            pa,
            va,
            size,
            attrs
        );

        assert!(
            pa.is_page_aligned(size),
            "mapped phys address must be page aligned"
        );
        assert!(
            va.is_page_aligned(size),
            "mapped virt address must be page aligned"
        );
        let pa = PhysAddr::<P>::new(pa.get());
        let va = va.canonicalize().round_up(PageSize::Page4k)?;
        let va_parts = va.parts();

        let mut table = self;
        let mut pte_addr;
        let mut level = 2;
        for i in (size as usize..=2).rev() {
            log::debug!(
                "look level {} index {:3} at {:?}",
                i,
                va_parts[i],
                table.base
            );
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
        for i in (size as usize + 1..=level).rev() {
            let entry = table.base.as_ptr().offset(va_parts[i] as isize);
            let next_pt = PageTable::<P>::alloc()?;
            let pte = Pte::new(next_pt.base.addr(), PteAttrs::V);
            log::debug!(
                "write level {} index {:3} at {:?} pte {:?}",
                i,
                va_parts[i],
                table.base,
                pte
            );
            entry.write(pte);
            table = next_pt;
        }

        // we have now reached level `size` and table points to the last level table
        let entry = table.base.as_ptr().offset(va_parts[size as usize] as isize);
        let pte = Pte::new(pa, attrs);
        log::debug!(
            "leaf pte level {} pos {} is {:?}",
            size as usize,
            va_parts[size as usize],
            pte
        );
        entry.write(pte);
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
    pub unsafe fn virt_map(
        self,
        pa: PhysAddr<P>,
        va: VirtAddr,
        len: usize,
        attrs: PteAttrs,
    ) -> Result<(), MapError> {
        assert!(len > 0, "len must be >0");
        // round len up to the nearest page
        let len = len;

        for offs in (0..len).step_by(PAGE_SIZE as _) {
            self.virt_map_one(
                PhysAddr::new(pa.get().checked_add(offs)?),
                VirtAddr(va.0.checked_add(offs)?),
                PageSize::Page4k,
                attrs,
            )?;
        }

        Ok(())
    }

    pub unsafe fn virt_unmap_one(self, va: VirtAddr) -> Result<(), UnmapError> {
        let parts = va.parts();
        let mut pt = self;
        for i in 0..=2 {
            let pte = pt.entry(parts[i]);
            let pte_p = pt.entry_ptr(parts[i]);
            let (next_ppn, attrs) = pte.decompose();
            if !attrs.contains(PteAttrs::V) {
                return Err(UnmapError::NotMapped);
            }

            if attrs.is_leaf() {
                pte_p.write_volatile(Pte::UNMAPPED);
                return Ok(());
            }

            pt = PageTable::from_raw(Phys::new_raw((next_ppn * PAGE_SIZE) as usize));
        }
        Ok(())
    }

    /// Allocates a new page from the pool at `va`.
    pub unsafe fn virt_alloc_one(self, va: VirtAddr, attrs: PteAttrs) -> Result<(), MapError> {
        let page = P::alloc().ok_or(MapError::OOM)?;
        self.virt_map_one(page, va, PageSize::Page4k, attrs)
    }
}

#[derive(Debug)]
pub enum MapError {
    /// probably an arithmetic overflow
    NoneError,
    /// given addresses are unaligned
    Unaligned,
    /// address already has been mapped on some level of the page table
    AlreadyMapped,
    /// ran out of memory
    OOM,
}

#[derive(Debug)]
pub enum UnmapError {
    /// Address was not mapped.
    NotMapped,
}

impl core::convert::From<NoneError> for MapError {
    fn from(_: NoneError) -> Self {
        MapError::NoneError
    }
}

/*
/// Performs the same function as [`virt_map`] but uses large pages automatically
///
/// If you use this, you may have a hard time if you want to unmap part of a
/// large page backed region.
pub unsafe fn virt_map_large<P: PhysAccess>(
    root_pt: PageTable<P>,
    pa: PhysAddr<P>,
    va: VirtAddr,
    len: usize,
    attrs: PteAttrs,
) -> Result<(), MapError> {
    /*
    Start | MaxAlignRegion | End
    4k* 2m*     1g*          2m* 4k*
    */
    if !pa.is_page_aligned(PageSize::Page4k) || !pa.is_page_aligned(PageSize::Page4k) {
        return Err(MapError::Unaligned);
    }
    assert!(len > PAGE_SIZE as usize);

    let end_pa = pa.map_r(|a| Ok(a.checked_add(len)?))?;
    let end_va = va.map_r(|a| Ok(a.checked_add(len)?))?;

    let mut max_pgsz = PageSize::Page4k;
    for pgsz in PageSize::SIZES_DESC.iter() {
        max_pgsz = *pgsz;
        if va.round_up(*pgsz)? < end_va && pa.round_up(*pgsz)? < end_pa {
            // region can fit that size
            break;
        }
    }
    let max_start = va.round_up(max_pgsz)?.0 - va.0;
    let max_end = end_va.round_down(max_pgsz)?.0 - va.0;

    // start_va |----------------|---------------------|--------------| end_va
    //                           ^ max_start | max_end ^
    //           <-remain_start->                       <-remain_end->

    // first try to grab a 1G region, then a 2M region, then a 4k region for the body

    // for pgsz in PageSize::SIZES_DESC.iter() {
    //     max_align = *pgsz;
    //     if (pgsz.offs_mask() & pa.get()) == pa.get() || (pgsz.offs_mask() & va.0) == va.0 {
    //         break;
    //     }
    // }

    // now we have the max alignment of the start of the region
    todo!()
}*/

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_canonicalize() {
        let addr = 0xff00_0010_1234_5789;
        assert_eq!(VirtAddr(addr).canonicalize().0, 0x0000_0010_1234_5789);
    }

    #[test]
    fn test_rounding() {
        for a in 1..=4096 {
            let va = VirtAddr(a);
            assert_eq!(VirtAddr(4096), va.round_up(PageSize::Page4k).unwrap());
        }

        for a in 1..=2 * 1024 * 1024 {
            let va = VirtAddr(a);
            assert_eq!(
                VirtAddr(2 * 1024 * 1024),
                va.round_up(PageSize::Page2m).unwrap()
            );
        }
    }
}
