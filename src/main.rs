#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(gale_sys::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use gale_sys::{println, vga_buffer};
#[cfg(test)]
use gale_sys::serial_println;

use bootloader::{BootInfo, entry_point};

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    gale_sys::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    gale_sys::test_panic_handler(info)
}

use gale_sys::memory;

entry_point!(kernel_main);

use gale_sys::combined_allocator::ALLOCATOR;

extern crate alloc;

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Inicializando");
    vga_buffer::print_in(30, 11, "Aguarde um Momento");

    gale_sys::init();

    use x86_64::{/*structures::paging::Translate, */VirtAddr};
    use memory::BootInfoFrameAllocator;

    //cria um desvio da memória física
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    //mapeador da memória
    unsafe { memory::init(phys_mem_offset) };

    unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    println!("antes de allocar");

    let allocator = &ALLOCATOR;

    println!("antes de allocar2");

    let ptr1 = unsafe { allocator.alloc(4096, 8) };
    if !ptr1.is_null() {
        println!("Alocação bem-sucedida: {:p}", ptr1);
        unsafe { allocator.dealloc(ptr1, 4096) };
        println!("Desalocação bem-sucedida");
    } else {
        println!("Falha na alocação");
    }

    let ptr2 = unsafe { allocator.alloc(1890, 8) };
    if !ptr2.is_null() {
        println!("Alocação bem-sucedida: {:p}", ptr2);
        unsafe { allocator.dealloc(ptr2, 1890) };
        println!("Desalocação bem-sucedida");
    } else {
        println!("Falha na alocação");
    }

    #[cfg(test)]
    use x86_64::registers::control::Cr3;

    #[cfg(test)]
    let (level_4_page_table, _) = Cr3::read();

    #[cfg(test)]
    serial_println!("Level 4 page table at: {:?}", level_4_page_table.start_address());

    #[cfg(test)]
    test_main();

    println!("Program dont has interruptions");

    gale_sys::hlt_loop();
}

//Windows
//cargo rustc -- -C link-args="/ENTRY:_start /SUBSYSTEM:console"