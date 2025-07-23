use core::{mem::size_of , ptr::null_mut} ;

unsafe extern "C"{
    static HEAP_START: usize ;
    static HEAP_SIZE: usize ;
}

static mut ALLOC_START: usize = 0 ;
const PAGE_ORDER: usize = 12 ;
pub const PAGE_SIZE: usize = 1 << 12 ;

// Align it up to "order" bits
pub const fn align_val(val: usize , order: usize) -> usize{
    let o = (1usize << order) - 1 ;
    (val + o) & !o
}

#[repr(u8)]

pub enum PageBits{
    Empty = 0 ,
    Taken = 1 << 0 ,
    Last = 1 << 1 ,
}

impl PageBits{
    pub fn val(self) -> u8{
        self as u8
    }
}

pub struct Page{
    flags: u8 ,
}

impl Page{

    // Function that checks if this is the last allocated page
    pub fn is_last(&self) -> bool{
        (self.flags & PageBits::Last.val()) != 0
    }

    // Function that checks if the page is taken or not
    pub fn is_taken(&self) -> bool{
        (self.flags & PageBits::Taken.val()) != 0
    }

    pub fn clear(&mut self){
        self.flags = PageBits::Empty.val() ;
    }

    pub fn set_flag(&mut self , flag: PageBits){
        self.flags = flag.val() | self.flags ;
    }

    pub fn clear_flag(&mut self , flag: PageBits){
        self.flags = self.flags & !flag.val() ;
    }

}

pub fn init(){
    unsafe{
        let num_pages = HEAP_SIZE / PAGE_SIZE ;
        let ptr = HEAP_START as *mut Page ; // Pointer to the first page

        // Clear all pages
        let mut i = 0 ;
        while i < num_pages{
            (*ptr.add(i)).clear() ;
            i += 1 ;
        }

        // Check from where we can allocate pages. Align it to page boundary
        ALLOC_START = align_val(HEAP_START + num_pages * size_of::<Page, >() , PAGE_ORDER) ;
    }
}

// We want to do a contiguous allocation for the requested number of pages
pub fn alloc(pages: usize) -> *mut u8{
    assert!(pages > 0) ;
    unsafe{
        let num_pages = HEAP_SIZE / PAGE_SIZE ;
        let ptr = HEAP_START as *mut Page ;
        let mut i = 0 ;
        while i < num_pages{
            let mut flag = false ;
            if !(*ptr.add(i)).is_taken() {
                flag = true ;
                let mut j = i ;
                while j < i + pages && j < num_pages{
                    if (*ptr.add(j)).is_taken(){
                        flag = false ;
                        break ;
                    }
                    j += 1 ;
                }
            }

            if flag{
                let mut j = i ;
                while j < i + pages && j < num_pages{
                    (*ptr.add(j)).set_flag(PageBits::Taken) ;
                    j += 1 ;
                }

                // Set the last and taken flag
                (*ptr.add(i + pages - 1)).set_flag(PageBits::Last) ;
                (*ptr.add(i+pages-1)).set_flag(PageBits::Taken) ;

                return (ALLOC_START + i * PAGE_SIZE) as *mut u8 ;

            }
            i += 1 ;
        }
    }
    null_mut()  //  No page found
}

pub fn zero_alloc(pages:usize) -> *mut u8{
    let ret = alloc(pages) ;
    if !ret.is_null(){
        let size = (PAGE_SIZE * pages)/8  ;
        let big_ptr = ret as *mut u64 ; // This is to force sd instruction and lower the number of stores
        for i in 0..size{
            unsafe{
                *big_ptr.add(i) = 0 ;
            }
        }
    }
    ret
}
// pub fn dealloc(ptr: *mut u8){
//
// }

#[repr(i64)]  // Represent our entry bits as unsigned 64-bits integers
#[derive(Copy , Clone)] // Automatically derive Copy and Clone traits for our enum
pub enum EntryBits{
    // D|A|G|U|X|W|R|V
    None = 0 ,
    Valid = 1 << 0 ,
    Read = 1 << 1 ,
    Write = 1 << 2 ,
    Execute = 1 << 3 ,
    User = 1 << 4 ,
    Global = 1 << 5 ,
    Access = 1 << 6 ,
    Dirty = 1 << 7 ,
    ReadWrite = 1 << 1 | 1 << 2,
    ReadExecute = 1 << 1 | 1 << 3,
    ReadWriteExecute = 1 << 1 | 1 << 2 | 1 << 3,
    UserReadWrite = 1 << 1 | 1 << 2 | 1 << 4,
    UserReadExecute = 1 << 1 | 1 << 3 | 1 << 4,
    UserReadWriteExecute = 1 << 1 | 1 << 2 | 1 << 3 | 1 << 4,
}

impl EntryBits{
    pub fn val(self)-> i64{
        self as i64
    }
}

pub struct Entry{
    pub entry: i64 ,
}

impl Entry{
    pub fn is_valid(&self) -> bool{
        if self.get_entry() & EntryBits::Valid.val() != 0{
            true
        }
        else{
            false
        }
    }

    pub fn is_leaf(&self) -> bool{
        // Check if any one of RWX bits is set
        if self.get_entry() & 0xE != 0{
            true
        }
        else{
            false
        }
    }
    // Getters and Setters
    pub fn set_entry(&mut self , entry: i64){
        self.entry = entry ;
    }

    pub fn get_entry(&self) -> i64{
        self.entry
    }
}

pub struct Table{
    pub entries: [Entry; 512] ,  // 512 entries in the table (2^9)
}

impl Table{
    pub fn len() -> usize{
        512
    }
}

// We'll take the reference to root table , va , pa , bits -->
pub fn mapping(root: &mut Table , va: usize , pa:usize , bits:i64 , level:usize){

    // Check if we RWX have been provided
    assert!(bits & 0xE != 0) ;

    let vpn = [(va >> 12) & 0x1FF , (va >> 21) & 0x1FF , (va >> 30) & 0x1FF] ;
    // First 12 bits are offset
    // VPN[0] = bits from 20:12
    // VPN[1] = bits from 29:21
    // VPN[2] = bits from 38:30 (sv39)

    let ppn = [(pa >> 12) & 0x1FF , (pa >> 21) & 0x1FF , (pa >> 30) & 0x3FF_FFFF] ;
    // PPN[0] = bits from 20:12
    // PPN[1] = bits from 29:21
    // PPN[2] = bits from 55:30 (sv39)

    // Page table walk
    let mut v = &mut root.entries[vpn[2]] ;

    for i in (level..2).rev(){
        if !v.is_valid(){
            let page = zero_alloc(1) ;
            v.set_entry((page as i64 >> 2) | EntryBits::Valid.val() ,) ;
        }
        let entry = ((v.get_entry() & !0x3FF) << 2) as *mut Entry ;
        v = unsafe{
            entry.add(vpn[i]).as_mut().unwrap()
        };
    }
    // Make the leaf point to the physical page
    let entry = (ppn[2] << 28 ) as i64 | (ppn[1] << 19) as i64 | (ppn[0] << 10) as i64 | bits | EntryBits::Valid.val() ;
    v.set_entry(entry) ;
}

pub fn translate(root: &Table , va:usize) -> Option<usize>{
    let vpn = [(va >> 12) & 0x1FF , (va >> 21) & 0x1FF , (va >> 30) & 0x1FF] ;
    let mut v = &root.entries[vpn[2]] ;

    for i in (0..=2).rev(){
        if !v.is_valid(){
            // Not a valid entry --> Page Fault
            break ;
        }
        else if v.is_leaf(){
            let offset = (1 << (12 + i * 9)) - 1  as i64; // 12 + 0 = 12 , 12 + 9 = 21 , 12 + 18 = 30
            let pageoffset = va & (offset as usize) ;
            let addr = (v.get_entry() << 2 as usize) & !offset ;
            return Some(addr as usize | pageoffset ) ;
        }

        let entry = ((v.get_entry() & !0x3FF) << 2) as *const Entry ;

        // Set v properly
        v = unsafe{
            entry.add(vpn[i-1]).as_ref().unwrap()
        };
    }
    None
}
