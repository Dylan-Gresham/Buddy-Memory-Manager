#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

constexpr static const uintptr_t DEFAULT_K = 30;

constexpr static const uintptr_t MIN_K = 20;

constexpr static const uintptr_t MAX_K = 48;

constexpr static const uintptr_t SMALLEST_K = 6;

constexpr static const uint16_t BLOCK_AVAIL = 1;

constexpr static const uint16_t BLOCK_RESERVED = 0;

constexpr static const uint16_t BLOCK_UNUSED = 3;

/// Struct to represent the table of all available blocks do not reorder members
/// of this struct because internal calculations depend on the ordering.
struct Avail {
  uint16_t tag;
  uint16_t kval;
  Avail *next;
  Avail *prev;
};

/// The Buddy Memory Pool
struct BuddyPool {
  uintptr_t kval_m;
  uintptr_t numbytes;
  void *base;
  Avail avail[MAX_K];
};

extern "C" {

/// Converts bytes to its equivalent K value defined as bytes <= 2^K
///
/// ## Parameters
///
/// - bytes `usize` The number of bytes needed
///
/// ## Returns
///
/// - K The number of bytes expressed as 2^K
uintptr_t btok(uintptr_t bytes);

/// Find the buddy of a given pointer and kval relative to the base address we got from mmap
///
/// ## Parameters
/// - pool `*mut BuddyPool` The memory pool to work on (needed for the base addresses)
/// - buddy `*mut Avail` The memory block that we want to find the buddy for
///
///  ## Returns
///
///  - A pointer to the buddy. Type = `*mut Avail`
Avail *buddy_calc(BuddyPool *pool, Avail *buddy);

/// Helper function.
///
/// Removes a block from the free list.
void remove_block(Avail *block);

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
void *buddy_malloc(BuddyPool *pool, uintptr_t size);

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
uint8_t buddy_free(BuddyPool *pool, void *ptr);

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
void buddy_init(BuddyPool *pool, uintptr_t size);

/// Inverse of buddy_init.
///
/// Notice that this function does not change the value of pool itself,
/// hence it still points to the same (now invalid) location.
///
/// ## Parameters
///
/// - pool `*mut BuddyPool` The memory pool to destroy
void buddy_destroy(BuddyPool *pool);

}  // extern "C"
