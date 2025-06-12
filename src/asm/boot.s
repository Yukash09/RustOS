.option norvc  # No compressed instructions
.section .data

.section .text.init
.global  _start

_start:
	csrr	t0 , mhartid
	bnez	t0 , wait
	csrw	satp , zero	# No virtual address translation

.option	push
.option norelax	# No relaxation optimizations
	la	gp ,  _global_pointer
.option	pop

# Rust requires .bss section to be zeroed out.
	la	a0 , _bss_start
	la	a1 , _bss_end
	bgeu	a0 , a1 , bss2	# If there is no bss then continue, else zero it out.

bss1:
	sd	zero , (a0)	# zero it out
	addi	a0 , a0 , 8
	bltu	a0 , a1 , bss1

bss2:
	la	sp , _stack	# Setup stack
	li	t0 , (0b11 << 11) | (1 << 7) | (1 << 3) 	# Set 12-th , 11-th bits
	csrw	mstatus , t0 
	la	    t1 , kINIT
	csrw	mepc , t1
	la	    t2 , asm_trap_vector
	csrw	mtvec , t2
	li	    t3 , (1 << 3) | (1 << 7) | (1 << 11)
	csrw	mie , t3
	la	    ra , next1
	mret

next1:
    li      t0 , (1 << 8) | (1 << 5)  # SPP = 1 , SPIE = 1 , SIE = 1
    csrw    sstatus , t0
    la      t1 , kmain
    csrw    sepc , t1

    # We need to delegate the interrupt , set Software,timer and external interrupts delegate to supervisor mode

    li      t2 , (1 << 1) | (1 << 5) | (1 << 9)
    csrw    mideleg , t2
    csrw    sie , t2  # set SSIE , STIE , SEIE
    la      t3 , asm_trap_vector
    csrw    stvec , t3
    csrw    satp , a0
    sfence.vma
    sret

wait:
	wfi	# wait for interrupt
	j	wait
