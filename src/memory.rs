use x86_64::{
    structures::paging::PageTable,
    VirtAddr,
    PhysAddr,
    structures::paging::PhysFrame
};

use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::paging::Size4KiB;

use bootloader::bootinfo::MemoryMap;
use bootloader::bootinfo::MemoryRegionType;

//inicia uma nova tabela de nível 4
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

//ativa a pagina de level4
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    //faz a leitura da tabela de level 4
    let (level_4_table_frame, _) = Cr3::read();

    //mapeia a memória física
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

//alocador de frame que retorna o memory map

extern crate alloc;
use alloc::vec::Vec;
pub struct BootInfoFrameAllocator {
    pub memory_map: &'static MemoryMap,
    pub free_frames: Vec<PhysFrame<Size4KiB>>,
    _next: usize,
}

impl BootInfoFrameAllocator {
    //cria um frame alocattor para o memory map
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            free_frames: Vec::new(),
            _next: 0,
        }
    }

    // retorna um iterador de frames usados pelo memory map
    fn _usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        //consegue as regiões
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        // mapeia cada região
        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        // transforma o iterador
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // cria a memória física
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    // Verifica se um frame está livre
    fn is_frame_free(&self, frame: PhysFrame<Size4KiB>) -> bool {
        // Percorre a lista de frames livres e verifica se o frame fornecido está nela
        for free_frame in &self.free_frames {
            if *free_frame == frame {
                return true;
            }
        }
        false
    }

    // Marca um frame como alocado
    fn mark_frame_allocated(&mut self, frame: PhysFrame<Size4KiB>) {
        // Remove o frame da lista de frames livres
        self.free_frames.retain(|&free_frame| free_frame != frame);
    }
}

use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::PageSize;

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        // Itera sobre as regiões de memória do boot info
        for region in self.memory_map.iter() {
            // Obtém o início e o fim da região de memória
            let region_start = region.range.start_addr();
            let region_end = region.range.end_addr();
    
            // Itera sobre os endereços dentro da região de memória
            let size = Size4KiB::SIZE as u64;
            for address in (region_start..region_end).step_by(size as usize) {
                // Verifica se o endereço está dentro de uma página de 4 KiB
                let addr = PhysAddr::new(address);
                let frame = PhysFrame::containing_address(addr);
                // Verifica se o frame está livre
                if self.is_frame_free(frame) {
                    // Marca o frame como ocupado e retorna
                    self.mark_frame_allocated(frame);
                    return Some(frame);
                }
            }
        }
        None // Retorna None se nenhum frame livre for encontrado
    }
}

use x86_64::structures::paging::FrameDeallocator;

impl FrameDeallocator<Size4KiB> for BootInfoFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        // Encontrar a página física correspondente ao frame
        let phys_addr = frame.start_address();
        
        // Procurar na lista de regiões de memória do boot info pela região que contém este endereço físico
        for region in self.memory_map.iter() {
            let region_start = PhysAddr::new(region.range.start_addr());
            let region_end = PhysAddr::new(region.range.end_addr());

            if phys_addr >= region_start && phys_addr < region_end {
                // Se encontrarmos a região, devolvemos o frame à lista de frames livres
                self.free_frames.push(frame);
                return;
            }
        }

        // Se o frame não estiver contido em nenhuma região, imprima um aviso
        crate::println!("Aviso: Tentativa de desalocação de um frame que não está em nenhuma região de memória conhecida.");
    }
}