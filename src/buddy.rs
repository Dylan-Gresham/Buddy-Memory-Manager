use libc::{memset, mmap, munmap, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE};
use std::ptr;
use std::ffi::c_void;

pub const DEFAULT_K: usize = 30;
pub const MIN_K: usize = 20;
pub const MAX_K: usize = 48;
pub const SMALLEST_K: usize = 6;

pub const BLOCK_AVAIL: u16 = 1;
pub const BLOCK_RESERVED: u16 = 0;
pub const BLOCK_UNUSED: u16 = 3;

/// Struct to represent the table of all available blocks do not reorder members 
/// of this struct because internal calculations depend on the ordering.
#[repr(C)]
#[derive(Debug)]
pub struct Avail {
    pub tag: u16,    // Block status: BLOCK_AVAIL, BLOCK_RESERVED
    pub kval: u16,   // kval of this block
    pub next: *mut Avail,
    pub prev: *mut Avail,
}

/// The Buddy Memory Pool
#[repr(C)]
#[derive(Debug)]
pub struct BuddyPool {
    pub kval_m: usize,         // Max kval of this pool
    pub numbytes: usize,       // Number of bytes in this pool
    pub base: *mut c_void,     // Base address for memory calculations
    pub avail: [Avail; MAX_K], // Array of available memory blocks
}

/// Converts bytes to its equivalent K value defined as bytes <= 2^K
///
/// ## Parameters
///
/// - bytes `usize` The number of bytes needed
///
/// ## Returns
///
/// - K The number of bytes expressed as 2^K
#[no_mangle]
pub extern "C" fn btok(bytes: usize) -> usize {
    // Implementation here
    0
}


/// Find the buddy of a given pointer and kval relative to the base address we got from mmap
///
/// ## Parameters
/// - pool `*mut BuddyPool` The memory pool to work on (needed for the base addresses)
/// - buddy `*mut Avail` The memory block that we want to find the buddy for
///
///  ## Returns
///
///  - A pointer to the buddy. Type = `*mut Avail`
#[no_mangle]
pub extern "C" fn buddy_calc(pool: *mut BuddyPool, buddy: *mut Avail) -> *mut Avail {
    // Implementation here
    std::ptr::null_mut()
}

/// Allocates a block of size bytes of memory, returning a pointer to
/// the beginning of the block. The content of the newly allocated block
/// of memory is not initialized, remaining with indeterminate values.
///
/// If size is zero, the return value will be NULL
/// If pool is NULL, the return value will be NULL
///
/// ## Parameters
///
/// - pool `*mut BuddyPool` The memory pool to alloc from
/// - size  `usize` The size of the user requested memory block in bytes
///
/// ## Returns
///
/// - A pointer to the memory block. Type = `*mut c_void`
#[no_mangle]
pub extern "C" fn buddy_malloc(pool: *mut BuddyPool, size: usize) -> *mut c_void {
    // Implementation here
    std::ptr::null_mut()
}

/// A block of memory previously allocated by a call to malloc,
/// calloc or realloc is deallocated, making it available again
/// for further allocations.
///
/// If ptr does not point to a block of memory allocated with
/// the above functions, it causes undefined behavior.
///
/// If ptr is a null pointer, the function does nothing.
/// Notice that this function does not change the value of ptr itself,
/// hence it still points to the same (now invalid) location.
///
/// ## Parameters
///
/// - pool `*mut BuddyPool` The memory pool
/// - ptr `*mut c_void` Pointer to the memory block to free
#[no_mangle]
pub extern "C" fn buddy_free(pool: *mut BuddyPool, ptr: *mut c_void) {
    // Implementation here
}

/// Initialize a new memory pool using the buddy algorithm. Internally,
/// this function uses mmap to get a block of memory to manage so should be
/// portable to any system that implements mmap. This function will round
/// up to the nearest power of two. So if the user requests 503MiB
/// it will be rounded up to 512MiB.
///
/// Note that if a 0 is passed as an argument then it initializes
/// the memory pool to be of the default size of DEFAULT_K. If the caller
/// specifies an unreasonably small size, then the buddy system may
/// not be able to satisfy any requests.
///
/// NOTE: Memory pools returned by this function can not be intermingled.
/// Calling buddy_malloc with pool A and then calling buddy_free with
/// pool B will result in undefined behavior.
///
/// ## Parameters
///
/// - pool `*mut BuddyPool` A pointer to the pool to initialize
/// - size `usize` The size of the pool in bytes.
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
        );
        
        if (*pool).base == MAP_FAILED {
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

/// Inverse of buddy_init.
///
/// Notice that this function does not change the value of pool itself,
/// hence it still points to the same (now invalid) location.
///
/// ## Parameters
///
/// - pool `*mut BuddyPool` The memory pool to destroy
#[no_mangle]
pub extern "C" fn buddy_destroy(pool: *mut BuddyPool) {
    unsafe {
        if munmap((*pool).base as *mut _, (*pool).numbytes) == -1 {
            panic!("buddy_destroy avail array");
        }

        memset(pool as *mut _, 0, std::mem::size_of::<BuddyPool>());
    }
}

