//! Simple fixed-size slab allocator for small objects
use core::mem::size_of;
use spin::Mutex;

const SLAB_SIZES: [usize; 3] = [64, 128, 256];
const SLAB_CAPACITY: usize = 1024; // bytes per slab pool

// Redzone 크기 (디버그 모드에서만 활성화)
#[cfg(debug_assertions)]
const REDZONE_SIZE: usize = 16; // 16바이트 redzone

#[cfg(not(debug_assertions))]
const REDZONE_SIZE: usize = 0;

struct SlabPool {
    buf: [u8; SLAB_CAPACITY],
    free_bitmap: [bool; SLAB_CAPACITY],
    chunk_size: usize,
    #[cfg(debug_assertions)]
    allocated: alloc::collections::BTreeSet<*mut u8>, // 할당된 포인터 추적
}

impl SlabPool {
    fn new(chunk_size: usize) -> Self {
        Self {
            buf: [0; SLAB_CAPACITY],
            free_bitmap: [true; SLAB_CAPACITY],
            chunk_size,
            #[cfg(debug_assertions)]
            allocated: alloc::collections::BTreeSet::new(),
        }
    }

    fn alloc(&mut self) -> Option<*mut u8> {
        let step = self.chunk_size + REDZONE_SIZE;
        let mut i = 0;
        while i + step <= SLAB_CAPACITY {
            if self.free_bitmap[i] {
                self.free_bitmap[i] = false;
                let ptr = unsafe { self.buf.as_mut_ptr().add(i) };
                
                #[cfg(debug_assertions)]
                {
                    // Redzone 설정 (0xAA로 채움)
                    if REDZONE_SIZE > 0 {
                        unsafe {
                            core::ptr::write_bytes(ptr.add(self.chunk_size), 0xAA, REDZONE_SIZE);
                        }
                    }
                    
                    // Double-free 검사
                    if self.allocated.contains(&ptr) {
                        crate::log_error!("Double-free detected: {:p}", ptr);
                        return None;
                    }
                    self.allocated.insert(ptr);
                }
                
                return Some(ptr);
            }
            i += step;
        }
        None
    }

    fn dealloc(&mut self, ptr: *mut u8) {
        let base = self.buf.as_ptr() as usize;
        let off = (ptr as usize).saturating_sub(base);
        
        #[cfg(debug_assertions)]
        {
            // Use-after-free 검사
            if !self.allocated.contains(&ptr) {
                crate::log_error!("Use-after-free detected: {:p}", ptr);
                return;
            }
            
            // Redzone 검사
            if REDZONE_SIZE > 0 {
                let redzone_ptr = unsafe { ptr.add(self.chunk_size) };
                for i in 0..REDZONE_SIZE {
                    let byte = unsafe { *redzone_ptr.add(i) };
                    if byte != 0xAA {
                        crate::log_error!("Redzone corruption detected at {:p}+{}", ptr, i);
                    }
                }
            }
            
            self.allocated.remove(&ptr);
        }
        
        if off < SLAB_CAPACITY {
            self.free_bitmap[off] = true;
        }
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
