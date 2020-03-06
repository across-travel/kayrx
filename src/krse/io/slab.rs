//! A lock-free concurrent slab.	

use std::sync::atomic::AtomicUsize;	
use std::sync::atomic::Ordering;	
use std::usize;	
use std::fmt;	
use std::sync::Mutex;	
use crate::krse::cell::CausalCell;	
use crate::krse::io::{self, Pack};	


#[cfg(target_pointer_width = "64")]	
const MAX_THREADS: usize = 4096;	

#[cfg(target_pointer_width = "32")]	
const MAX_THREADS: usize = 2048;	

/// Max number of pages per slab	
const MAX_PAGES: usize = io::pointer_width() as usize / 4;	

/// Size of first page	
const INITIAL_PAGE_SIZE: usize = 32;	

/// A sharded slab.	
pub(crate) struct Slab<T> {	
    // Signal shard for now. Eventually there will be more.	
    shard: Shard<T>,	
    local: Mutex<()>,	
}	

unsafe impl<T: Send> Send for Slab<T> {}	
unsafe impl<T: Sync> Sync for Slab<T> {}	

impl<T: Entry> Slab<T> {	
    /// Returns a new slab with the default configuration parameters.	
    pub(crate) fn new() -> Slab<T> {	
        Slab {	
            shard: Shard::new(),	
            local: Mutex::new(()),	
        }	
    }	

    /// allocs a value into the slab, returning a key that can be used to	
    /// access it.	
    ///	
    /// If this function returns `None`, then the shard for the current thread	
    /// is full and no items can be added until some are removed, or the maximum	
    /// number of shards has been reached.	
    pub(crate) fn alloc(&self) -> Option<Address> {	
        // we must lock the slab to alloc an item.	
        let _local = self.local.lock().unwrap();	
        self.shard.alloc()	
    }	

    /// Removes the value associated with the given key from the slab.	
    pub(crate) fn remove(&self, idx: Address) {	
        // try to lock the slab so that we can use `remove_local`.	
        let lock = self.local.try_lock();	

        // if we were able to lock the slab, we are "local" and can use the fast	
        // path; otherwise, we will use `remove_remote`.	
        if lock.is_ok() {	
            self.shard.remove_local(idx)	
        } else {	
            self.shard.remove_remote(idx)	
        }	
    }	

    /// Return a reference to the value associated with the given key.	
    ///	
    /// If the slab does not contain a value for the given key, `None` is	
    /// returned instead.	
    pub(crate) fn get(&self, token: Address) -> Option<&T> {	
        self.shard.get(token)	
    }	
}	

impl<T> fmt::Debug for Slab<T> {	
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {	
        f.debug_struct("Slab")	
            .field("shard", &self.shard)	
            .finish()	
    }	
}	

// ====================Address=========================	

/// Tracks the location of an entry in a slab.	
///	
/// # Index packing	
///	
/// A slab index consists of multiple indices packed into a single `usize` value	
/// that correspond to different parts of the slab.	
///	
/// The least significant `MAX_PAGES + INITIAL_PAGE_SIZE.trailing_zeros() + 1`	
/// bits store the address within a shard, starting at 0 for the first slot on	
/// the first page. To index a slot within a shard, we first find the index of	
/// the page that the address falls on, and then the offset of the slot within	
/// that page.	
///	
/// Since every page is twice as large as the previous page, and all page sizes	
/// are powers of two, we can determine the page index that contains a given	
/// address by shifting the address down by the smallest page size and looking	
/// at how many twos places necessary to represent that number, telling us what	
/// power of two page size it fits inside of. We can determine the number of	
/// twos places by counting the number of leading zeros (unused twos places) in	
/// the number's binary representation, and subtracting that count from the	
/// total number of bits in a word.	
///	
/// Once we know what page contains an address, we can subtract the size of all	
/// previous pages from the address to determine the offset within the page.	
///	
/// After the page address, the next `MAX_THREADS.trailing_zeros() + 1` least	
/// significant bits are the thread ID. These are used to index the array of	
/// shards to find which shard a slot belongs to. If an entry is being removed	
/// and the thread ID of its index matches that of the current thread, we can	
/// use the `remove_local` fast path; otherwise, we have to use the synchronized	
/// `remove_remote` path.	
///	
/// Finally, a generation value is packed into the index. The `RESERVED_BITS`	
/// most significant bits are left unused, and the remaining bits between the	
/// last bit of the thread ID and the first reserved bit are used to store the	
/// generation. The generation is used as part of an atomic read-modify-write	
/// loop every time a `ScheduledIo`'s readiness is modified, or when the	
/// resource is removed, to guard against the ABA problem.	
///	
/// Visualized:	
///	
/// ```text	
///     ┌──────────┬───────────────┬──────────────────┬──────────────────────────┐	
///     │ reserved │  generation   │    thread ID     │         address          │	
///     └▲─────────┴▲──────────────┴▲─────────────────┴▲────────────────────────▲┘	
///      │          │               │                  │                        │	
/// bits(usize)     │       bits(MAX_THREADS)          │                        0	
///                 │                                  │	
///      bits(usize) - RESERVED       MAX_PAGES + bits(INITIAL_PAGE_SIZE)	
/// ```	

/// References the location at which an entry is stored in a slab.	
#[derive(Debug, Copy, Clone, Eq, PartialEq)]	
pub(crate) struct Address(usize);	

const PAGE_INDEX_SHIFT: u32 = INITIAL_PAGE_SIZE.trailing_zeros() + 1;	

/// Address in the shard	
const SLOT: Pack = Pack::least_significant(	
    MAX_PAGES as u32 + PAGE_INDEX_SHIFT);	

/// Masks the thread identifier	
const THREAD: Pack = SLOT.then(MAX_THREADS.trailing_zeros() + 1);	

/// Masks the generation	
const GENERATION: Pack = THREAD.then(	
    io::pointer_width().wrapping_sub(RESERVED.width() + THREAD.width() + SLOT.width()));	

// Chosen arbitrarily	
const RESERVED: Pack = Pack::most_significant(5);	

impl Address {	
    /// Represents no entry, picked to avoid collision with Mio's internals.	
    /// This value should not be passed to linux.	
    pub(crate) const NULL: usize = usize::MAX >> 1;	

    /// Re-exported by `Generation`.	
    pub(super) const GENERATION_WIDTH: u32 = GENERATION.width();	

    pub(super) fn new(shard_index: usize, generation: Generation) -> Address {	
        let mut repr = 0;	

        repr = SLOT.pack(shard_index, repr);	
        repr = GENERATION.pack(generation.to_usize(), repr);	

        Address(repr)	
    }	

    /// Convert from a `usize` representation.	
    pub(crate) fn from_usize(src: usize) -> Address {	
        assert_ne!(src, Self::NULL);	

        Address(src)	
    }	

    /// Convert to a `usize` representation	
    pub(crate) fn to_usize(self) -> usize {	
        self.0	
    }	

    pub(crate) fn generation(self) -> Generation {	
        Generation::new(GENERATION.unpack(self.0))	
    }	

    /// Returns the page index	
    pub(super) fn page(self) -> usize {	
        // Since every page is twice as large as the previous page, and all page	
        // sizes are powers of two, we can determine the page index that	
        // contains a given address by shifting the address down by the smallest	
        // page size and looking at how many twos places necessary to represent	
        // that number, telling us what power of two page size it fits inside	
        // of. We can determine the number of twos places by counting the number	
        // of leading zeros (unused twos places) in the number's binary	
        // representation, and subtracting that count from the total number of	
        // bits in a word.	
        let slot_shifted = (self.slot() + INITIAL_PAGE_SIZE) >> PAGE_INDEX_SHIFT;	
        (io::pointer_width() - slot_shifted.leading_zeros()) as usize	
    }	

    /// Returns the slot index	
    pub(super) fn slot(self) -> usize {	
        SLOT.unpack(self.0)	
    }	
}	

// =======================Entry=======================	

pub(crate) trait Entry: Default {	
    fn generation(&self) -> Generation;	

    fn reset(&self, generation: Generation) -> bool;	
}	


// =======================Generation=======================	

/// An mutation identifier for a slot in the slab. The generation helps prevent	
/// accessing an entry with an outdated token.	
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]	
pub(crate) struct Generation(usize);	

impl Generation {	
    pub(crate) const WIDTH: u32 = Address::GENERATION_WIDTH;	

    pub(super) const MAX: usize = io::mask_for(Address::GENERATION_WIDTH);	

    /// Create a new generation	
    ///	
    /// # Panics	
    ///	
    /// Panics if `value` is greater than max generation.	
    pub(crate) fn new(value: usize) -> Generation {	
        assert!(value <= Self::MAX);	
        Generation(value)	
    }	

    /// Returns the next generation value	
    pub(crate) fn next(self) -> Generation {	
        Generation((self.0 + 1) & Self::MAX)	
    }	

    pub(crate) fn to_usize(self) -> usize {	
        self.0	
    }	
}	


// =======================Stack=======================	


pub(super) struct TransferStack {	
    head: AtomicUsize,	
}	

impl TransferStack {	
    pub(super) fn new() -> Self {	
        Self {	
            head: AtomicUsize::new(Address::NULL),	
        }	
    }	

    pub(super) fn pop_all(&self) -> Option<usize> {	
        let val = self.head.swap(Address::NULL, Ordering::Acquire);	

        if val == Address::NULL {	
            None	
        } else {	
            Some(val)	
        }	
    }	

    pub(super) fn push(&self, value: usize, before: impl Fn(usize)) {	
        let mut next = self.head.load(Ordering::Relaxed);	

        loop {	
            before(next);	

            match self	
                .head	
                .compare_exchange(next, value, Ordering::AcqRel, Ordering::Acquire)	
            {	
                // lost the race!	
                Err(actual) => next = actual,	
                Ok(_) => return,	
            }	
        }	
    }	
}	

impl fmt::Debug for TransferStack {	
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {	
        // Loom likes to dump all its internal state in `fmt::Debug` impls, so	
        // we override this to just print the current value in tests.	
        f.debug_struct("TransferStack")	
            .field(	
                "head",	
                &format_args!("{:#x}", self.head.load(Ordering::Relaxed)),	
            )	
            .finish()	
    }	
}	


//      ========================Shard=======================	

// ┌─────────────┐      ┌────────┐	
// │ page 1      │      │        │	
// ├─────────────┤ ┌───▶│  next──┼─┐	
// │ page 2      │ │    ├────────┤ │	
// │             │ │    │XXXXXXXX│ │	
// │ local_free──┼─┘    ├────────┤ │	
// │ global_free─┼─┐    │        │◀┘	
// ├─────────────┤ └───▶│  next──┼─┐	
// │   page 3    │      ├────────┤ │	
// └─────────────┘      │XXXXXXXX│ │	
//       ...            ├────────┤ │	
// ┌─────────────┐      │XXXXXXXX│ │	
// │ page n      │      ├────────┤ │	
// └─────────────┘      │        │◀┘	
//                      │  next──┼───▶	
//                      ├────────┤	
//                      │XXXXXXXX│	
//                      └────────┘	
//                         ...	
pub(super) struct Shard<T> {	
    /// The local free list for each page.	
    ///	
    /// These are only ever accessed from this shard's thread, so they are	
    /// stored separately from the shared state for the page that can be	
    /// accessed concurrently, to minimize false sharing.	
    local: Box<[Local]>,	
    /// The shared state for each page in this shard.	
    ///	
    /// This consists of the page's metadata (size, previous size), remote free	
    /// list, and a pointer to the actual array backing that page.	
    shared: Box<[Shared<T>]>,	
}	

impl<T: Entry> Shard<T> {	
    pub(super) fn new() -> Shard<T> {	
        let mut total_sz = 0;	
        let shared = (0..MAX_PAGES)	
            .map(|page_num| {	
                let sz = page_size(page_num);	
                let prev_sz = total_sz;	
                total_sz += sz;	
                Shared::new(sz, prev_sz)	
            })	
            .collect();	

        let local = (0..MAX_PAGES).map(|_| Local::new()).collect();	

        Shard {	
            local,	
            shared,	
        }	
    }	

    pub(super) fn alloc(&self) -> Option<Address> {	
        // Can we fit the value into an existing page?	
        for (page_idx, page) in self.shared.iter().enumerate() {	
            let local = self.local(page_idx);	

            if let Some(page_offset) = page.alloc(local) {	
                return Some(page_offset);	
            }	
        }	

        None	
    }	

    pub(super) fn get(&self, addr: Address) -> Option<&T> {	
        let page_idx = addr.page();	

        if page_idx > self.shared.len() {	
            return None;	
        }	

        self.shared[page_idx].get(addr)	
    }	

    /// Remove an item on the shard's local thread.	
    pub(super) fn remove_local(&self, addr: Address) {	
        let page_idx = addr.page();	

        if let Some(page) = self.shared.get(page_idx) {	
            page.remove_local(self.local(page_idx), addr);	
        }	
    }	

    /// Remove an item, while on a different thread from the shard's local thread.	
    pub(super) fn remove_remote(&self, addr: Address) {	
        if let Some(page) = self.shared.get(addr.page()) {	
            page.remove_remote(addr);	
        }	
    }	

    fn local(&self, i: usize) -> &Local {	
        &self.local[i]	
    }	
}	

impl<T> fmt::Debug for Shard<T> {	
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {	
        f.debug_struct("Shard")	
            .field("shared", &self.shared)	
            .finish()	
    }	
}	


// ===================================Slot===========================	

/// Stores an entry in the slab.	
pub(super) struct Slot<T> {	
    next: CausalCell<usize>,	
    entry: T,	
}	

impl<T: Entry> Slot<T> {	
    /// Initialize a new `Slot` linked to `next`.	
    ///	
    /// The entry is initialized to a default value.	
    pub(super) fn new(next: usize) -> Slot<T> {	
        Slot {	
            next: CausalCell::new(next),	
            entry: T::default(),	
        }	
    }	

    pub(super) fn get(&self) -> &T {	
        &self.entry	
    }	

    pub(super) fn generation(&self) -> Generation {	
        self.entry.generation()	
    }	

    pub(super) fn reset(&self, generation: Generation) -> bool {	
        self.entry.reset(generation)	
    }	

    pub(super) fn next(&self) -> usize {	
        self.next.with(|next| unsafe { *next })	
    }	

    pub(super) fn set_next(&self, next: usize) {	
        self.next.with_mut(|n| unsafe {	
            (*n) = next;	
        })	
    }	
}	

// =======================Page ========================	


/// Data accessed only by the thread that owns the shard.	
pub(crate) struct Local {	
    head: CausalCell<usize>,	
}	

/// Data accessed by any thread.	
pub(crate) struct Shared<T> {	
    remote: TransferStack,	
    size: usize,	
    prev_sz: usize,	
    slab: CausalCell<Option<Box<[Slot<T>]>>>,	
}	

/// Returns the size of the page at index `n`	
pub(super) fn page_size(n: usize) -> usize {	
    INITIAL_PAGE_SIZE << n	
}	

impl Local {	
    pub(crate) fn new() -> Self {	
        Self {	
            head: CausalCell::new(0),	
        }	
    }	

    fn head(&self) -> usize {	
        self.head.with(|head| unsafe { *head })	
    }	

    fn set_head(&self, new_head: usize) {	
        self.head.with_mut(|head| unsafe {	
            *head = new_head;	
        })	
    }	
}	

impl<T: Entry> Shared<T> {	
    pub(crate) fn new(size: usize, prev_sz: usize) -> Shared<T> {	
        Self {	
            prev_sz,	
            size,	
            remote: TransferStack::new(),	
            slab: CausalCell::new(None),	
        }	
    }	

    /// Allocates storage for this page if it does not allready exist.	
    ///	
    /// This requires unique access to the page (e.g. it is called from the	
    /// thread that owns the page, or, in the case of `SingleShard`, while the	
    /// lock is held). In order to indicate this, a reference to the page's	
    /// `Local` data is taken by this function; the `Local` argument is not	
    /// actually used, but requiring it ensures that this is only called when	
    /// local access is held.	
    #[cold]	
    fn alloc_page(&self, _: &Local) {	
        debug_assert!(self.slab.with(|s| unsafe { (*s).is_none() }));	

        let mut slab = Vec::with_capacity(self.size);	
        slab.extend((1..self.size).map(Slot::new));	
        slab.push(Slot::new(Address::NULL));	

        self.slab.with_mut(|s| {	
            // this mut access is safe — it only occurs to initially	
            // allocate the page, which only happens on this thread; if the	
            // page has not yet been allocated, other threads will not try	
            // to access it yet.	
            unsafe {	
                *s = Some(slab.into_boxed_slice());	
            }	
        });	
    }	

    pub(crate) fn alloc(&self, local: &Local) -> Option<Address> {	
        let head = local.head();	

        // are there any items on the local free list? (fast path)	
        let head = if head < self.size {	
            head	
        } else {	
            // if the local free list is empty, pop all the items on the remote	
            // free list onto the local free list.	
            self.remote.pop_all()?	
        };	

        // if the head is still null, both the local and remote free lists are	
        // empty --- we can't fit any more items on this page.	
        if head == Address::NULL {	
            return None;	
        }	

        // do we need to allocate storage for this page?	
        let page_needs_alloc = self.slab.with(|s| unsafe { (*s).is_none() });	
        if page_needs_alloc {	
            self.alloc_page(local);	
        }	

        let gen = self.slab.with(|slab| {	
            let slab = unsafe { &*(slab) }	
                .as_ref()	
                .expect("page must have been allocated to alloc!");	

            let slot = &slab[head];	

            local.set_head(slot.next());	
            slot.generation()	
        });	

        let index = head + self.prev_sz;	

        Some(Address::new(index, gen))	
    }	

    pub(crate) fn get(&self, addr: Address) -> Option<&T> {	
        let page_offset = addr.slot() - self.prev_sz;	

        self.slab	
            .with(|slab| unsafe { &*slab }.as_ref()?.get(page_offset))	
            .map(|slot| slot.get())	
    }	

    pub(crate) fn remove_local(&self, local: &Local, addr: Address) {	
        let offset = addr.slot() - self.prev_sz;	

        self.slab.with(|slab| {	
            let slab = unsafe { &*slab }.as_ref();	

            let slot = if let Some(slot) = slab.and_then(|slab| slab.get(offset)) {	
                slot	
            } else {	
                return;	
            };	

            if slot.reset(addr.generation()) {	
                slot.set_next(local.head());	
                local.set_head(offset);	
            }	
        })	
    }	

    pub(crate) fn remove_remote(&self, addr: Address) {	
        let offset = addr.slot() - self.prev_sz;	

        self.slab.with(|slab| {	
            let slab = unsafe { &*slab }.as_ref();	

            let slot = if let Some(slot) = slab.and_then(|slab| slab.get(offset)) {	
                slot	
            } else {	
                return;	
            };	

            if !slot.reset(addr.generation()) {	
                return;	
            }	

            self.remote.push(offset, |next| slot.set_next(next));	
        })	
    }	
}	

impl fmt::Debug for Local {	
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {	
        self.head.with(|head| {	
            let head = unsafe { *head };	
            f.debug_struct("Local")	
                .field("head", &format_args!("{:#0x}", head))	
                .finish()	
        })	
    }	
}	

impl<T> fmt::Debug for Shared<T> {	
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {	
        f.debug_struct("Shared")	
            .field("remote", &self.remote)	
            .field("prev_sz", &self.prev_sz)	
            .field("size", &self.size)	
            // .field("slab", &self.slab)	
            .finish()	
    }	
}