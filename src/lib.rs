use libc::{memset, mmap, munmap, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE, __errno_location, ENOMEM};
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
    // Return the smallest block size if no bytes are requested
    if bytes == 0 {
        return 0;
    }

    // Initialize k to the smallest block size
    let mut k = 0;

    // Iterate to find the smallest k where 2^k >= to the requested number of bytes
    while (1 << k) < bytes {
        k += 1
    }

    k
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
    unsafe {
        // Calculate the offset of the current block from the base of the pool
        let offset = (buddy as usize) - ((*pool).base as usize);

        // Get the size of the buddy block based on its kval
        let size = 1 << (*buddy).kval;

        // Calculate the offset of the buddy block by XORing the original block's offset with its
        // size
        let buddy_offset = offset ^ size;

        // Return a pointer to the buddy block by adding the buddy offset to the pool's base
        // address
        ((*pool).base as usize + buddy_offset) as *mut Avail
    }
}

/// Helper function.
///
/// Removes a block from the free list.
#[no_mangle]
pub unsafe extern "C" fn remove_block(block: *mut Avail) {
    // Update the previous pointer of the block's next block
    (*(*block).prev).next = (*block).next;
    //
    // Update the next pointer of the block's previous block
    (*(*block).next).prev = (*block).prev;
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
    // Return null pointer if pool is null or size is 0
    if pool.is_null() || size == 0 {
        return ptr::null_mut();
    }

    unsafe {
        // Calculate the required block size (including space for the header)
        let mut req_k = btok(size + std::mem::size_of::<Avail>());
        if req_k < SMALLEST_K {
            req_k = SMALLEST_K;
        }

        // Search for the first available block of sufficient size
        let mut k = req_k; 
        while k <= (*pool).kval_m && (*pool).avail[k].next == &mut (*pool).avail[k] {
            k += 1;
        }

        // If no block is found, set errno and return null (memory not available)
        if k > (*pool).kval_m {
            // Set errno to ENOMEM
            (*__errno_location()) = ENOMEM;

            return ptr::null_mut();
        }

        let block = (*pool).avail[k].next;
        remove_block(block);

        // Split blocks down to the required size (req_k)
        while k > req_k {
            k -= 1;
            let buddy = (block as usize + (1 << k)) as *mut Avail;

            (*buddy).kval = k as u16;
            (*buddy).tag = BLOCK_AVAIL;
            (*buddy).next = (*pool).avail[k].next;
            (*buddy).prev = &mut (*pool).avail[k];

            (*(*pool).avail[k].next).prev = buddy;
            (*pool).avail[k].next = buddy;
        }

        // Mark the block as reserved
        (*block).tag = BLOCK_RESERVED;
        (*block).kval = k as u16;

        // Return the memory location after the block header (pointer to the user data)
        (block as *mut u8).add(std::mem::size_of::<Avail>()) as *mut c_void
    }
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
pub extern "C" fn buddy_free(pool: *mut BuddyPool, ptr: *mut c_void) -> u8 {
    // Return early if the pointer is null or the pool is null
    if ptr.is_null() || pool.is_null() {
        return 1;
    }

    unsafe {
        // Get the block header by subtracting the size of Avail from the pointer
        let mut block = (ptr as *mut u8).sub(std::mem::size_of::<Avail>()) as *mut Avail;

        (*block).tag = BLOCK_AVAIL;

        // Try to coalesce the block with its buddy if they are both available
        while ((*block).kval as usize) < (*pool).kval_m {
            let buddy = buddy_calc(pool, block);

            // If the buddy is available or has a different size, break out of the loop
            if (*buddy).tag != BLOCK_AVAIL || (*buddy).kval != (*block).kval {
                break;
            }

            // Remove the buddy from the available list
            remove_block(buddy);

            // If the buddy is smaller in address, update block to point to it
            if buddy < block {
                block = buddy;
            }

            // Increase the kval (combine blocks into a larger one)
            (*block).kval += 1;
        }

        (*block).next = (*pool).avail[(*block).kval as usize].next;
        (*block).prev = &mut (*pool).avail[(*block).kval as usize];

        (*(*pool).avail[(*block).kval as usize].next).prev = block;
        (*pool).avail[(*block).kval as usize].next = block;
    }


    0
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::MaybeUninit;

    fn check_buddy_pool_full(pool: &mut BuddyPool) {
        for i in 0..pool.kval_m {
            let avail = &pool.avail[i];
            assert_eq!(avail.next as *const _, avail as *const _);
            assert_eq!(avail.prev as *const _, avail as *const _);
            assert_eq!(avail.tag, BLOCK_UNUSED);
            assert_eq!(avail.kval as usize, i);
        }

        let top = &pool.avail[pool.kval_m];
        unsafe {
            assert_eq!((*top.next).tag, BLOCK_AVAIL);
            assert_eq!((*top.next).next, top as *const _ as *mut _);
            assert_eq!((*top.prev).prev, top as *const _ as *mut _);
            assert_eq!(top.next, pool.base as *mut Avail);
        }
    }

    fn check_buddy_pool_empty(pool: &mut BuddyPool) {
        // All avail lists should be empty
        for i in 0..=pool.kval_m {
            let avail = &pool.avail[i];
            assert_eq!(avail.next as *const _, avail as *const _);
            assert_eq!(avail.prev as *const _, avail as *const _);
            assert_eq!(avail.tag, BLOCK_UNUSED);
            assert_eq!(avail.kval as usize, i);
        }
    }

    #[test]
    fn test_buddy_malloc_one_byte() {
        let kval = MIN_K as usize;
        let size = 1 << kval;

        let mut pool = MaybeUninit::<BuddyPool>::uninit();
        let pool_ptr = pool.as_mut_ptr();

        unsafe {
            buddy_init(pool_ptr, size);
            let pool_ref = &mut *pool_ptr;

            let mem = buddy_malloc(pool_ref, 1);
            assert!(!mem.is_null());

            assert_eq!(buddy_free(pool_ref, mem), 0);
            check_buddy_pool_full(pool_ref);

            buddy_destroy(pool_ref);
        }
    }

    #[test]
    fn test_buddy_malloc_one_large() {
        let size = 1 << MIN_K;

        let mut pool = MaybeUninit::<BuddyPool>::uninit();
        let pool_ptr = pool.as_mut_ptr();
        
        unsafe {
            buddy_init(pool_ptr, size);
            let pool_ref = &mut *pool_ptr;

            let ask = size - std::mem::size_of::<Avail>();
            let mem = buddy_malloc(pool_ref, ask);
            assert!(!mem.is_null());

            let block = (mem as *mut u8).offset(-(std::mem::size_of::<Avail>() as isize)) as *mut Avail;
            assert_eq!((*block).kval as usize, MIN_K);
            assert_eq!((*block).tag, BLOCK_RESERVED);

            check_buddy_pool_empty(pool_ref);

            let fail = buddy_malloc(pool_ref, 5);
            assert!(fail.is_null());

            assert_eq!(buddy_free(pool_ref, mem), 0);
            check_buddy_pool_full(pool_ref);

            buddy_destroy(pool_ref);
        }
    }

    #[test]
    fn test_buddy_init() {
        for i in MIN_K as usize..=DEFAULT_K as usize {
            let size = 1 << i;
            
            let mut pool = MaybeUninit::<BuddyPool>::uninit();
            let pool_ptr = pool.as_mut_ptr();

            unsafe {
                buddy_init(pool_ptr, size);
                let pool_ref = &mut *pool_ptr;

                check_buddy_pool_full(pool_ref);
                buddy_destroy(pool_ref);
            }
        }
    }

    #[test]
    fn test_buddy_calc_basic_pairs() {
        const TEST_K: usize = MIN_K + 2;
        let pool_size = 1 << TEST_K;

        let mut pool = MaybeUninit::<BuddyPool>::uninit();
        let pool_ptr = pool.as_mut_ptr();

        unsafe {
            buddy_init(pool_ptr, pool_size);
            let pool_ref = &mut *pool_ptr;

            // Allocate 2 small blocks manually by splitting top-level block
            let top_block = pool_ref.avail[TEST_K].next;
            assert_eq!((*top_block).tag, BLOCK_AVAIL);

            // Remove top block from the free list
            remove_block(top_block);

            // Split it into two buddies
            let kval = TEST_K - 1;
            let block1 = top_block;
            let block2 = (block1 as usize + (1 << kval)) as *mut Avail;

            (*block1).kval = kval as u16;
            (*block2).kval = kval as u16;

            // Calculate each other as buddy
            let b1 = buddy_calc(pool_ref, block1);
            let b2 = buddy_calc(pool_ref, block2);

            assert_eq!(b1, block2, "Buddy of block1 should be block2");
            assert_eq!(b2, block1, "Buddy of block2 should be block1");

            // Check that buddy address is offset by correct power of two
            let offset1 = (block1 as usize) - (pool_ref.base as usize);
            let offset2 = (block2 as usize) - (pool_ref.base as usize);
            assert_eq!(offset1 ^ offset2, 1 << kval);

            buddy_destroy(pool_ref);
        }
    }

    /// Helper function.
    ///
    /// Inserts a block into the free list at kval
    unsafe fn insert_block(pool: *mut BuddyPool, block: *mut Avail, kval: usize) {
        // Get the head of the linked list for blocks of size 2^k where k = kval
        let head = &mut (*pool).avail[kval];
    
        // Insert the block at the head of the list
        (*block).next = head.next;
        (*block).prev = head;
    
        // Update the next pointer of the block's previous node
        (*head.next).prev = block;
    
        // Update the head's next pointer to the new block
        (*head).next = block;
    
        // Set the block's tag to indicate its available
        (*block).tag = BLOCK_AVAIL;
    }

    #[test]
    fn test_buddy_calc_recursive_coalescing() {
        const BASE_K: usize = MIN_K + 3; // 2^7 = 128 bytes
        let pool_size = 1 << BASE_K;
    
        let mut pool = MaybeUninit::<BuddyPool>::uninit();
        let pool_ptr = pool.as_mut_ptr();
    
        unsafe {
            buddy_init(pool_ptr, pool_size);
            let pool_ref = &mut *pool_ptr;
    
            // Manually take the top block
            let top_block = pool_ref.avail[BASE_K].next;
            remove_block(top_block);
    
            // Split into two level BASE_K - 1 blocks
            let k1 = BASE_K - 1;
            let left1 = top_block;
            let right1 = (left1 as usize + (1 << k1)) as *mut Avail;
            (*left1).kval = k1 as u16;
            (*right1).kval = k1 as u16;
    
            // Split left1 into two BASE_K - 2 blocks
            let k2 = k1 - 1;
            let left2 = left1;
            let right2 = (left2 as usize + (1 << k2)) as *mut Avail;
            (*left2).kval = k2 as u16;
            (*right2).kval = k2 as u16;
    
            // Split left2 again
            let k3 = k2 - 1;
            let left3 = left2;
            let right3 = (left3 as usize + (1 << k3)) as *mut Avail;
            (*left3).kval = k3 as u16;
            (*right3).kval = k3 as u16;
    
            // Now free right3 and left3 and ensure they coalesce into left2
            insert_block(pool_ref, left3, k3);
            insert_block(pool_ref, right3, k3);
    
            let buddy_of_left3 = buddy_calc(pool_ref, left3);
            assert_eq!(buddy_of_left3, right3, "Buddy of left3 should be right3");
    
            // Remove both from free list to simulate coalescing
            remove_block(left3);
            remove_block(right3);
    
            // Merge into left2
            let merged_kval = k3 + 1;
            let merged_block = if left3 < right3 { left3 } else { right3 };
            (*merged_block).kval = merged_kval as u16;
    
            // Check buddy of merged_block is still correct
            let buddy = buddy_calc(pool_ref, merged_block);
            let expected_offset = 1 << merged_kval;
            let offset_diff = (buddy as usize).wrapping_sub(merged_block as usize);
            assert_eq!(offset_diff, expected_offset, "Merged block buddy is offset correctly");
    
            buddy_destroy(pool_ref);
        }
    }

    #[test]
    fn test_btok_one() {
        assert_eq!(0, btok(1));
    }

    #[test]
    fn test_btok_range() {
        assert_eq!(0, btok(1));
        assert_eq!(1, btok(2));
        assert_eq!(2, btok(3));
        assert_eq!(2, btok(4));
        assert_eq!(3, btok(5));
        assert_eq!(3, btok(8));
        assert_eq!(4, btok(9));
        assert_eq!(4, btok(16));
        assert_eq!(5, btok(17));
        assert_eq!(5, btok(32));
        assert_eq!(6, btok(33));
        assert_eq!(6, btok(64));
        assert_eq!(10, btok(1024));
        assert_eq!(11, btok(1025));
        assert_eq!(40, btok(1099511627776));
    }

    #[test]
    fn test_double_free() {
        let mut pool = MaybeUninit::<BuddyPool>::uninit();
        let pool_ptr = pool.as_mut_ptr();

        unsafe {
            buddy_init(pool_ptr, 128);
            let pool_ref = &mut *pool_ptr;

            let ptr = buddy_malloc(pool_ref, 64);
            assert!(!ptr.is_null());

            assert_eq!(buddy_free(pool_ref, ptr), 0);

            // This free is undefined behavior and shouldn't fail
            assert_eq!(buddy_free(pool_ref, ptr), 0);
        }
    }
}
