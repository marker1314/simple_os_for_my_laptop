//! Simple fixed-size slab allocator for small objects
use core::mem::size_of;
use spin::Mutex;

const SLAB_SIZES: [usize; 3] = [64, 128, 256];
const SLAB_CAPACITY: usize = 1024; // bytes per slab pool

struct SlabPool {
    buf: [u8; SLAB_CAPACITY],
    free_bitmap: [bool; SLAB_CAPACITY],
    chunk_size: usize,
}

impl SlabPool {
    const fn new(chunk_size: usize) -> Self {
        Self { buf: [0; SLAB_CAPACITY], free_bitmap: [true; SLAB_CAPACITY], chunk_size }
    }

    fn alloc(&mut self) -> Option<*mut u8> {
        let step = self.chunk_size;
        let mut i = 0;
        while i + step <= SLAB_CAPACITY {
            if self.free_bitmap[i] {
                self.free_bitmap[i] = false;
                return Some(unsafe { self.buf.as_mut_ptr().add(i) });
            }
            i += step;
        }
        None
    }

    fn dealloc(&mut self, ptr: *mut u8) {
        let base = self.buf.as_ptr() as usize;
        let off = (ptr as usize).saturating_sub(base);
        if off < SLAB_CAPACITY { self.free_bitmap[off] = true; }
    }
}

pub struct SlabAllocator {
    pools: [Mutex<SlabPool>; 3],
}

impl SlabAllocator {
    pub const fn new() -> Self {
        Self {
            pools: [
                Mutex::new(SlabPool::new(SLAB_SIZES[0])),
                Mutex::new(SlabPool::new(SLAB_SIZES[1])),
                Mutex::new(SlabPool::new(SLAB_SIZES[2])),
            ],
        }
    }

    pub fn alloc_small(&self, size: usize) -> Option<*mut u8> {
        for (i, &cs) in SLAB_SIZES.iter().enumerate() {
            if size <= cs {
                return self.pools[i].lock().alloc();
            }
        }
        None
    }

    pub fn dealloc_small(&self, ptr: *mut u8, size: usize) {
        for (i, &cs) in SLAB_SIZES.iter().enumerate() {
            if size <= cs {
                self.pools[i].lock().dealloc(ptr);
                return;
            }
        }
    }
}

pub static SLAB: SlabAllocator = SlabAllocator::new();
