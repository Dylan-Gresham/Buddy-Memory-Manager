#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../buddy_memory_manager.h"

void check_buddy_pool_full(BuddyPool* pool) {
    for (int i = 0; i < pool->kval_m; i++) {
        Avail* avail = &pool->avail[i];
        assert((void*)avail->next == (void*)avail);
        assert((void*)avail->prev == (void*)avail);
        assert(avail->tag == BLOCK_UNUSED);
        assert((int)avail->kval == i);
    }

    Avail* top = &pool->avail[pool->kval_m];
    assert(top->next->tag == BLOCK_AVAIL);
    assert(top->next->next == top);
    assert(top->prev->prev == top);
    assert(top->next == (Avail*)pool->base);
}

void check_buddy_pool_empty(BuddyPool* pool) {
    for (int i = 0; i <= pool->kval_m; i++) {
        Avail* avail = &pool->avail[i];
        assert((void*)avail->next == (void*)avail);
        assert((void*)avail->prev == (void*)avail);
        assert(avail->tag == BLOCK_UNUSED);
        assert((int)avail->kval == i);
    }
}

void test_buddy_malloc_one_byte() {
    size_t kval = MIN_K;
    size_t size = 1 << kval;

    BuddyPool pool;
    buddy_init(&pool, size);

    void* mem = buddy_malloc(&pool, 1);
    assert(mem != NULL);

    assert(buddy_free(&pool, mem) == 0);
    check_buddy_pool_full(&pool);

    buddy_destroy(&pool);
}

void test_buddy_malloc_one_large() {
    size_t size = 1 << MIN_K;

    BuddyPool pool;
    buddy_init(&pool, size);

    size_t ask = size - sizeof(Avail);
    void* mem = buddy_malloc(&pool, ask);
    assert(mem != NULL);

    Avail* block = (Avail*)((char*)mem - sizeof(Avail));
    assert((int)block->kval == MIN_K);
    assert(block->tag == BLOCK_RESERVED);

    check_buddy_pool_empty(&pool);

    void* fail = buddy_malloc(&pool, 5);
    assert(fail == NULL);

    assert(buddy_free(&pool, mem) == 0);
    check_buddy_pool_full(&pool);

    buddy_destroy(&pool);
}

void test_buddy_init() {
    for (int i = MIN_K; i <= DEFAULT_K; i++) {
        size_t size = 1 << i;
        BuddyPool pool;
        buddy_init(&pool, size);
        check_buddy_pool_full(&pool);
        buddy_destroy(&pool);
    }
}

void test_buddy_calc_basic_pairs() {
    const int TEST_K = MIN_K + 2;
    size_t pool_size = 1 << TEST_K;

    BuddyPool pool;
    buddy_init(&pool, pool_size);

    Avail* top_block = pool.avail[TEST_K].next;
    assert(top_block->tag == BLOCK_AVAIL);
    remove_block(top_block);

    int kval = TEST_K - 1;
    Avail* block1 = top_block;
    Avail* block2 = (Avail*)((char*)block1 + (1 << kval));
    block1->kval = kval;
    block2->kval = kval;

    Avail* b1 = buddy_calc(&pool, block1);
    Avail* b2 = buddy_calc(&pool, block2);
    assert(b1 == block2);
    assert(b2 == block1);

    size_t offset1 = (char*)block1 - (char*)pool.base;
    size_t offset2 = (char*)block2 - (char*)pool.base;
    assert((offset1 ^ offset2) == (1 << kval));

    buddy_destroy(&pool);
}

void insert_block(BuddyPool* pool, Avail* block, int kval) {
    Avail* head = &pool->avail[kval];
    block->next = head->next;
    block->prev = head;
    head->next->prev = block;
    head->next = block;
    block->tag = BLOCK_AVAIL;
}

void test_buddy_calc_recursive_coalescing() {
    const int BASE_K = MIN_K + 3;
    size_t pool_size = 1 << BASE_K;

    BuddyPool pool;
    buddy_init(&pool, pool_size);

    Avail* top_block = pool.avail[BASE_K].next;
    remove_block(top_block);

    int k1 = BASE_K - 1;
    Avail* left1 = top_block;
    Avail* right1 = (Avail*)((char*)left1 + (1 << k1));
    left1->kval = k1;
    right1->kval = k1;

    int k2 = k1 - 1;
    Avail* left2 = left1;
    Avail* right2 = (Avail*)((char*)left2 + (1 << k2));
    left2->kval = k2;
    right2->kval = k2;

    int k3 = k2 - 1;
    Avail* left3 = left2;
    Avail* right3 = (Avail*)((char*)left3 + (1 << k3));
    left3->kval = k3;
    right3->kval = k3;

    insert_block(&pool, left3, k3);
    insert_block(&pool, right3, k3);

    Avail* buddy_of_left3 = buddy_calc(&pool, left3);
    assert(buddy_of_left3 == right3);

    remove_block(left3);
    remove_block(right3);

    int merged_kval = k3 + 1;
    Avail* merged_block = left3 < right3 ? left3 : right3;
    merged_block->kval = merged_kval;

    Avail* buddy = buddy_calc(&pool, merged_block);
    size_t expected_offset = 1 << merged_kval;
    size_t offset_diff = (char*)buddy - (char*)merged_block;
    assert(offset_diff == expected_offset);

    buddy_destroy(&pool);
}

void test_btok_one() {
    assert(btok(1) == 0);
}

void test_btok_range() {
    assert(btok(1) == 0);
    assert(btok(2) == 1);
    assert(btok(3) == 2);
    assert(btok(4) == 2);
    assert(btok(5) == 3);
    assert(btok(8) == 3);
    assert(btok(9) == 4);
    assert(btok(16) == 4);
    assert(btok(17) == 5);
    assert(btok(32) == 5);
    assert(btok(33) == 6);
    assert(btok(64) == 6);
    assert(btok(1024) == 10);
    assert(btok(1025) == 11);
    assert(btok(1099511627776ULL) == 40);
}

void test_double_free() {
    BuddyPool pool;
    buddy_init(&pool, 128);

    void* ptr = buddy_malloc(&pool, 64);
    assert(ptr != NULL);

    assert(buddy_free(&pool, ptr) == 0);
    assert(buddy_free(&pool, ptr) == 0);  // Should be okay even if undefined

    buddy_destroy(&pool);
}

int main() {
    test_buddy_malloc_one_byte();
    test_buddy_malloc_one_large();
    test_buddy_init();
    test_buddy_calc_basic_pairs();
    test_buddy_calc_recursive_coalescing();
    test_btok_one();
    test_btok_range();
    test_double_free();

    printf("All C tests passed successfully!\n");
    return 0;
}

