use core::convert::TryInto ;
use core::fmt::Write ;
use core::fmt::Error ;


// Some Rust Stuff:
// 1. volatile tells the compiler not to optimize (write_volatile , read_volatile , etc.)
//      Modern compilers are aggressive, they might reorder and do bakchodi that might be dangerous when talking to hardware. Just do what's there, skill issue of the programmer if it doesn't work.
//      For example - If we write to some MMOI (memory mapped I/O), we may not use the value but it should be there for the device. A compiler might skip doing this because our program isn't using it anyway.

// 2. unsafe - Wrap unsafe code with unsafe{} --> else rustc will cry

// 3. add() for pointers --> ptr.add(x) - advance the ptr by x element of type the ptr points to
//      here ptr points to u8 , so it advances by 8-bit for add(1)

// 4. unwrap() --> gives the value of the Option
pub struct Uart{
    base_addr: usize ,
}

// Write is a trait we imported, we are implementing it for Uart
// This looks similar to virtual functions in C++ (need to clarify)

impl Write for Uart{
    // This is a function needed to be implemented in Write trait
    // str - string slice
    // Returns a "Result" type , () if everything is fine else returns Error
    // This will override write! function (macro technically)?

    fn  write_str(&mut self, out: &str) -> Result<() , Error>{
        for c in out.bytes(){
            self.put(c) ;
        }
        Ok(())
    }
}

impl Uart{

    // Similar to constructor and return type is Uart
    pub fn new(base_addr: usize) -> Self {
        Uart{
            base_addr
        }
    }

    pub fn init(&mut self){
        let ptr = self.base_addr as *mut u8 ; // convert the integer into raw-pointer to an unsigned 8-bit integer
        unsafe{
            // Three things we need to take care - 
            // 1. Set word length to 8-bits (mentioned by LCR[1:0] --> set both to 1 for 8-bit word length)
            // 2. Enable FIFO (mentioned in FCR[0] --> set to 1 for enabling FIFO)
            // 3. Enable Reciever interrupts (mentioned in IER[0] - set to 1 for enabling recieved data available interrupt)
            // LCR - Line control Register (Register address 3)
            // FCR - FIFO Control Register (Register address 2)
            // IER - Interrupt Enable Register (Register address 1)

            let lcr = (1 << 0) | (1 << 1) ;
            ptr.add(3).write_volatile(lcr) ; 

            let fifo = 1 << 0;
            ptr.add(2).write_volatile(fifo) ;

            let ier = 1 << 0 ;
            ptr.add(1).write_volatile(ier) ;

            // Before writing Baud rate, we must set DLAB before writing it and clear it after.
            // When DLAB = 0 --> Ports 0 and 1 refer to Transmit Holding Register and IER
            // When DLAB = 1 --> They refer Divisor Latch Low Byte (DLL) and Divisor Latch High Byte (DLM)
            // Divisor = ceil(UART Clock frequency / (16 x Baud))
            // Divisor = 592

            let div: u16 = 592 ;
            let dll: u8 = (div & 0xff).try_into().unwrap() ;
            let dlm: u8 = (div >> 8).try_into().unwrap() ;
            let lcr = ptr.add(3).read_volatile() ;
            ptr.add(3).write_volatile(lcr | (1 << 7)) ; // dlab is now 1 so ports 0 and 1 are DLL and DLM
            ptr.add(0).write_volatile(dll) ;
            ptr.add(1).write_volatile(dlm) ;
            ptr.add(3).write_volatile(lcr) ; // Clear it back
        }
    }

    pub fn put(&mut self , c : u8){
        let ptr = self.base_addr as *mut u8 ;
        unsafe {
            ptr.add(0).write_volatile(c) ;
        }
    }

    pub fn get(&mut self) -> Option<u8> {
        // Option<u8> is similar to std::optional in C++
        let ptr = self.base_addr as *mut u8 ;
        unsafe{
            // Check if there is data to read --> Checked using LSR (Line Status Register at Register address 5)
            // If yes then read is else pls don't
            if ptr.add(5).read_volatile() & 1 == 0{
                None  // Return None
            }
            else{
                Some(ptr.add(0).read_volatile())  // Some() --> This is used for Option<>
            }
        }
    }
}
