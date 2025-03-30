use libc::{memset, mmap, munmap, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE};
use std::ptr;

pub const DEFAULT_K: usize = 30;
pub const MIN_K: usize = 20;
pub const MAX_K: usize = 48;
pub const SMALLEST_K: usize = 6;

pub const BLOCK_AVAIL: u16 = 1;
pub const BLOCK_RESERVED: u16 = 0;
pub const BLOCK_UNUSED: u16 = 3;

#[repr(C)]
#[derive(Debug)]
pub struct Avail {
    pub tag: u16,    // Block status: BLOCK_AVAIL, BLOCK_RESERVED
    pub kval: u16,   // kval of this block
    pub next: *mut Avail,
    pub prev: *mut Avail,
}

#[repr(C)]
#[derive(Debug)]
pub struct BuddyPool {
    pub kval_m: usize,         // Max kval of this pool
    pub numbytes: usize,       // Number of bytes in this pool
    pub base: *mut u8,         // Base address for memory calculations
    pub avail: [Avail; MAX_K], // Array of available memory blocks
}

#[no_mangle]
pub extern "C" fn btok(bytes: usize) -> usize {
    // Implementation here
    0
}

#[no_mangle]
pub extern "C" fn buddy_calc(pool: *mut BuddyPool, buddy: *mut Avail) -> *mut Avail {
    // Implementation here
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn buddy_malloc(pool: *mut BuddyPool, size: usize) -> *mut u8 {
    // Implementation here
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn buddy_free(pool: *mut BuddyPool, ptr: *mut u8) {
    // Implementation here
}

#[no_mangle]
pub extern "C" fn buddy_init(pool: *mut BuddyPool, size: usize) {
   unsafe {
        let kval = if size == 0 { DEFAULT_K } else { btok(size) };
        let kval = kval.clamp(MIN_K, MAX_K - 1);
        
        memset(pool as *mut _, 0, std::mem::size_of::<BuddyPool>());
        (*pool).kval_m = kval;
        (*pool).numbytes = 1 << kval;
        
        (*pool).base = mmap(
            ptr::null_mut(),
            (*pool).numbytes,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8;
        
        if (*pool).base == MAP_FAILED as *mut u8 {
            panic!("buddy_init avail array mmap failed");
        }
        
        for i in 0..=kval {
            (*pool).avail[i].next = &mut (*pool).avail[i];
            (*pool).avail[i].prev = &mut (*pool).avail[i];
            (*pool).avail[i].kval = i as u16;
            (*pool).avail[i].tag = BLOCK_UNUSED;
        }
        
        let m = (*pool).base as *mut Avail;
        (*pool).avail[kval].next = m;
        (*pool).avail[kval].prev = m;
        (*m).tag = BLOCK_AVAIL;
        (*m).kval = kval as u16;
        (*m).next = &mut (*pool).avail[kval];
        (*m).prev = &mut (*pool).avail[kval];
    } 
}

#[no_mangle]
pub extern "C" fn buddy_destroy(pool: *mut BuddyPool) {
    unsafe {
        if munmap((*pool).base as *mut _, (*pool).numbytes) == -1 {
            panic!("buddy_destroy avail array");
        }
        memset(pool as *mut _, 0, std::mem::size_of::<BuddyPool>());
    }
}

