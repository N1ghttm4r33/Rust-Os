use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

const HEAP_START: usize = 0x_4444_4444_0000;
const HEAP_SIZE: usize = 200 * 1024; // 200 KiB
const PAGE_SIZE: usize = 4096; // 4 KiB pages
const SMALL_BLOCK_SIZE: usize = 256; // Threshold for small blocks
const MAX_CACHEABLE_SIZE: usize = 1024; // Defina o tamanho máximo para os blocos de memória alocados que podem ser reutilizados

extern crate alloc;
use alloc::vec::Vec;

use crate::println;

pub struct CombinedAllocator {
    _heap_start: usize,
    heap_end: usize,
    heap_current: AtomicUsize,
    pages: Mutex<Vec<Page>>,
    cache: Mutex<Vec<usize>>,
}

struct Page {
    start: usize,
    current_offset: AtomicUsize,
    allocations: Mutex<Vec<(usize, usize)>>,
    free_blocks: Mutex<Vec<(usize, usize)>>,
    small_block_bitmap: Mutex<u128>, // Using 128 bits for small blocks (256 / 2)
}

impl Page {
    fn new(start: usize) -> Self {
        Page {
            start,
            current_offset: AtomicUsize::new(0),
            allocations: Mutex::new(Vec::new()),
            free_blocks: Mutex::new(Vec::new()),
            small_block_bitmap: Mutex::new(0),
        }
    }

    fn compact_small_block_bitmap(bitmap: &mut u128) {
        let mut new_bitmap = 0;
        let mut bit_index = 0;
    
        for i in 0..128 {
            if (*bitmap & (1 << i)) != 0 {
                new_bitmap |= 1 << bit_index;
                bit_index += 1;
            }
        }
    
        *bitmap = new_bitmap;
    }    

    fn alloc_small(&self, _size: usize) -> Option<usize> {
        let mut bitmap = self.small_block_bitmap.lock();
        let mut block_index = 0;
        let mut bit = 1;
        while bit != 0 {
            if (*bitmap & bit) == 0 {
                *bitmap |= bit;
                return Some(self.start + block_index * SMALL_BLOCK_SIZE);
            }
            block_index += 1;
            bit <<= 1;
        }
        None
    }    

    fn dealloc_small(&self, ptr: usize) {
        let offset = ptr - self.start;
        let block_index = offset / SMALL_BLOCK_SIZE;
        let bit = 1 << block_index;
        let mut bitmap = self.small_block_bitmap.lock();
        *bitmap &= !bit;
        Self::compact_small_block_bitmap(&mut *bitmap);
    }

    fn alloc_large(&self, size: usize, align: usize) -> Option<usize> {
        println!("alloc_large");
        let mut free_blocks = self.free_blocks.lock();
        for (i, &(offset, block_size)) in free_blocks.iter().enumerate() {
            let aligned_offset = (offset + align - 1) & !(align - 1);
            if aligned_offset + size <= offset + block_size {
                free_blocks.remove(i);
                if aligned_offset > offset {
                    free_blocks.push((offset, aligned_offset - offset));
                }
                if aligned_offset + size < offset + block_size {
                    free_blocks.push((aligned_offset + size, offset + block_size - (aligned_offset + size)));
                }
                let mut allocations = self.allocations.lock();
                allocations.push((aligned_offset, size));
                println!("{}, {}", self.start, aligned_offset);
                return Some(self.start + aligned_offset);
            }
        }

        let current_offset = self.current_offset.load(Ordering::SeqCst);
        let aligned_offset = (current_offset + align - 1) & !(align - 1);
        if aligned_offset + size <= PAGE_SIZE {
            self.current_offset.store(aligned_offset + size, Ordering::SeqCst);
            let mut allocations = self.allocations.lock();
            allocations.push((aligned_offset, size));
            println!("{}, {}", self.start, aligned_offset);
            Some(self.start + aligned_offset)
        } else {
            println!("none");
            None
        }
    }

    fn dealloc_large(&self, ptr: usize, size: usize) {
        let mut allocations = self.allocations.lock();
        if let Some(pos) = allocations.iter().position(|&(offset, s)| offset == ptr - self.start && s == size) {
            allocations.remove(pos);
            let mut free_blocks = self.free_blocks.lock();
            free_blocks.push((ptr - self.start, size));
            free_blocks.sort_unstable_by_key(|&(offset, _)| offset);
            self.merge_free_blocks(&mut free_blocks);
        }
    }

    fn merge_free_blocks(&self, free_blocks: &mut Vec<(usize, usize)>) {
        let mut i = 0;
        while i < free_blocks.len() - 1 {
            if free_blocks[i].0 + free_blocks[i].1 == free_blocks[i + 1].0 {
                free_blocks[i].1 += free_blocks[i + 1].1;
                free_blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
}

impl CombinedAllocator {
    const fn new(_heap_start: usize, heap_size: usize) -> Self {
        CombinedAllocator {
            _heap_start,
            heap_end: _heap_start + heap_size,
            heap_current: AtomicUsize::new(_heap_start),
            pages: Mutex::new(Vec::new()),
            cache: Mutex::new(Vec::new()), // Inicializa o cache com Arc<Mutex<Vec<usize>>>
        }
    }

    fn allocate_page(&self) -> Option<*mut Page> {
        println!("-3");
        let heap_current = self.heap_current.load(Ordering::SeqCst);
        println!("-2");
        if heap_current + PAGE_SIZE <= self.heap_end {
            println!("-1");
            let mut pages = self.pages.lock();
            let page = Page::new(heap_current);
            println!("0, pages.len() => {}, heap_current => {}, heap_i + page => {}, heap_end => {}", pages.len(), heap_current, heap_current + PAGE_SIZE, self.heap_end);
            pages.push(page);
            println!("00");
            self.heap_current.store(heap_current + PAGE_SIZE, Ordering::SeqCst);
            println!("1");
            Some(pages.last_mut().unwrap() as *mut _)
        } else {
            println!("2");
            None
        }
    }

    pub unsafe fn alloc(&self, size: usize, align: usize) -> *mut u8 {
        let mut cache = self.cache.lock();
        if let Some(addr) = cache.pop() {
            println!("chegou no cache");
            return addr as *mut u8;
        }
        if size <= SMALL_BLOCK_SIZE {
            println!("ta no small");
            let pages = self.pages.lock();
            for page in pages.iter() {
                if let Some(addr) = page.alloc_small(size) {
                    println!("chegou no aloc small");
                    return addr as *mut u8;
                }
            }
            drop(pages);

            if let Some(new_page) = self.allocate_page() {
                if let Some(addr) = (*new_page).alloc_small(size) {
                    println!("chegou na nova pagina em alloc small");
                    return addr as *mut u8;
                }
            }
        } else { 
            println!("ta no large");
            let pages = self.pages.lock();
            println!("ta no large2, pages.len() => {}", pages.len());
            for page in pages.iter() {
                println!("ta no large3");
                if let Some(addr) = page.alloc_large(size, align) {
                    println!("chegou no alloc large");
                    return addr as *mut u8;
                }
            }
            println!("ta no large4");
            drop(pages);
            println!("dropou");

            if let Some(new_page) = self.allocate_page() {
                println!("allocate_page");
                if let Some(addr) = (*new_page).alloc_large(size, align) {
                    println!("chegou na nova página em alloc large");
                    return addr as *mut u8;
                }
            }
        }
        println!("null_mut");
        null_mut()
    }

    pub unsafe fn dealloc(&self, ptr: *mut u8, size: usize) {
        let addr = ptr as usize;
        let pages = self.pages.lock();
        let mut cache = self.cache.lock();

        if size <= MAX_CACHEABLE_SIZE {
            cache.push(addr);
            return;
        }

        for page in pages.iter() {
            if addr >= page.start && addr < page.start + PAGE_SIZE {
                if size <= SMALL_BLOCK_SIZE {
                    page.dealloc_small(addr);
                } else {
                    page.dealloc_large(addr, size);
                }
                cache.push(addr);
                return;
            }
        }
    }
}

unsafe impl GlobalAlloc for CombinedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc(layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc(ptr, layout.size())
    }
}

#[global_allocator]
pub static ALLOCATOR: CombinedAllocator = CombinedAllocator::new(HEAP_START, HEAP_SIZE);