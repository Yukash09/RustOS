#![no_std]  // No standard library
#![feature(asm_experimental_arch , allocator_api , alloc_error_handler)]
use core::arch::asm;
//use core::option::Option;
pub mod uart;  // This is like #include in C++
//use core::panic;

extern crate alloc ;
use alloc::alloc::* ;

#[macro_export]
macro_rules! print
{
    ($($args:tt)+) => ({
        use core::fmt::Write ;
        let _ = write!(crate::uart::Uart::new(0x1000_0000) , $($args)+) ;
    });
}

#[macro_export]
macro_rules! println
{
    () => ({
        print!("\r\n") ;
    });

    ($fmt:expr) => ({
        print!(concat!($fmt , "\r\n")) 
    }) ;

    ($fmt:expr , $($args:tt)+) => ({
        print!(concat!($fmt , "\r\n") , $($args)+)
    }) ;
}

#[unsafe(no_mangle)] // no mangling
extern "C" fn eh_personality() {

}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> !{
    print!("Aborting: ") ;
    if let Some(_p) = info.location(){
        println!(
                "line {}, file {}: {}" ,
                _p.line() ,
                _p.file() , 
                info.message()
        );
    }
    else{
        println!("No info available") ;
    }
    abort() ;
}

#[unsafe(no_mangle)]   
extern "C"
fn abort() -> !{
    loop{
        unsafe{
            asm!("wfi" , options(nomem, nostack, preserves_flags)) ;
        }
    }
}


unsafe extern "C" {
        static TEXT_START: usize;
        static TEXT_END: usize;
        static DATA_START: usize;
        static DATA_END: usize;
        static RODATA_START: usize;
        static RODATA_END: usize;
        static BSS_START: usize;
        static BSS_END: usize;
        static KERNEL_STACK_START: usize;
        static KERNEL_STACK_END: usize;
        static HEAP_START: usize;
        static HEAP_SIZE: usize;
        static mut KERNEL_TABLE: usize;
}

pub fn id_map_range(root: &mut page::Table , start:usize , end:usize , bits:i64){
    let mut memaddr = start & !(page::PAGE_SIZE -1) ;
    let num_pages = (page::align_val(end , 12) - memaddr) / page::PAGE_SIZE ;
    
    for _ in 0..num_pages{
        page::mapping(root , memaddr , memaddr , bits , 0) ;
        memaddr += 1 << 12 ;
    }
}

#[unsafe(no_mangle)] 
extern "C" fn kinit() -> usize{
    // Interrupts should be disabled 
    uart::Uart::new(0x1000_0000).init() ;
    page::init() ;
    kmem::init() ;
    
    let root_ptr = kmem::get_page_table();
    let root_u = root_ptr as usize;
    let mut root = unsafe { root_ptr.as_mut().unwrap() };
    let kheap_head = kmem::get_head() as usize;
    let total_pages = kmem::get_num_allocations();

    id_map_range(&mut root, kheap_head, kheap_head + total_pages * 4096, page::EntryBits::ReadWrite.val(),);
    unsafe {
        // Map heap descriptors
        let num_pages = HEAP_SIZE / page::PAGE_SIZE;
        id_map_range(&mut root,
                     HEAP_START,
                     HEAP_START + num_pages,
                     page::EntryBits::ReadWrite.val()
        );
        // Map executable section
        id_map_range(
            &mut root,
            TEXT_START,
            TEXT_END,
            page::EntryBits::ReadExecute.val(),
        );
        // Map rodata section
        // We put the ROdata section into the text section, so they can
        // potentially overlap however, we only care that it's read
        // only.
        id_map_range(
            &mut root,
            RODATA_START,
            RODATA_END,
            page::EntryBits::ReadExecute.val(),
        );
        // Map data section
        id_map_range(
            &mut root,
            DATA_START,
            DATA_END,
            page::EntryBits::ReadWrite.val(),
        );
        // Map bss section
        id_map_range(
            &mut root,
            BSS_START,
            BSS_END,
            page::EntryBits::ReadWrite.val(),
        );
        // Map kernel stack
        id_map_range(
            &mut root,
            KERNEL_STACK_START,
            KERNEL_STACK_END,
            page::EntryBits::ReadWrite.val(),
        );
    }

    // UART
    page::mapping(
        &mut root,
        0x1000_0000,
        0x1000_0000,
        page::EntryBits::ReadWrite.val(),
        0
    );

    // CLINT
    //  -> MSIP
    page::mapping(
        &mut root,
        0x0200_0000,
        0x0200_0000,
        page::EntryBits::ReadWrite.val(),
        0
    );
    //  -> MTIMECMP
    page::mapping(
        &mut root,
        0x0200_b000,
        0x0200_b000,
        page::EntryBits::ReadWrite.val(),
        0
    );
    //  -> MTIME
    page::mapping(
        &mut root,
        0x0200_c000,
        0x0200_c000,
        page::EntryBits::ReadWrite.val(),
        0
    );
    // PLIC
    id_map_range(
        &mut root,
        0x0c00_0000,
        0x0c00_2000,
        page::EntryBits::ReadWrite.val(),
    );
    id_map_range(
        &mut root,
        0x0c20_0000,
        0x0c20_8000,
        page::EntryBits::ReadWrite.val(),
    );	
    
    let p = 0x8005_7000 as usize;
    let m = page::translate(&root, p).unwrap_or(0);

    unsafe {
        KERNEL_TABLE = root_u;
    }
    // table / 4096    Sv39 mode
    (root_u >> 12)  | (8 << 60)
}

#[unsafe(no_mangle)]
extern "C"
fn kmain(){
    let mut uart1 = uart::Uart::new(0x1000_0000) ;
    uart1.init() ;

    println!("Hehehehehaw") ;
    println!("Do something bruh") ;
    loop {
        if let Some(c) = uart1.get() {
            match c {
                8 => { // backspace \b
                    print!("{}{}{}", 8 as char, ' ', 8 as char);
                },

                10 | 13 => { // Newline or Carriage return (\r and \n)
                    println!();
                },

                0x1b => {
                    if let Some(nxt) = uart1.get(){
                        if nxt == 91 {
                            if let Some(b) = uart1.get(){
                                match b as char {
                                    'A' => {
                                        println!("Up");
                                    },
                                    'B' => {
                                        println!("Down");
                                    },
                                    'C' => {
                                        println!("Left");
                                    },
                                    'D' => {
                                        println!("Right");
                                    },
                                    _ => {
                                        println!("Idk");
                                    },
                                }
                            }
                        }
                    }
                }
                
                _ => {
                    print!("{}", c as char);
                }
            }
        }
    }	
}
pub mod kmem ;
pub mod page ;
