use crate::bus::Bus;
use crate::opcodes;
use std::collections::HashMap;

bitflags! {
    #[derive(Clone, Copy)]
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const UNUSED            = 0b00100000;
        const OVERFLOW          = 0b01000000;
        const NEGATIVE          = 0b10000000;
    }
}

const STACK_BASE: u16 = 0x0100;
const STACK_RESET: u8 = 0xFD;

const RESET_VECTOR: u16 = 0xFFFC;

pub struct CPU<'a> {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: CpuFlags,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub bus: Bus<'a>,

    pub nmi_pending: bool,
    pub irq_pending: bool,

    pub cycles: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
    Relative,
    Indirect,
    Accumulator,
    Implied,
}

pub trait Mem {
    fn mem_read(&mut self, addr: u16) -> u8;
    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos.wrapping_add(1), hi);
    }
}

impl Mem for CPU<'_> {
    fn mem_read(&mut self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.bus.mem_write(addr, data)
    }

    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        self.bus.mem_read_u16(addr)
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        self.bus.mem_write_u16(addr, data)
    }
}

fn page_cross(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xFF00 != addr2 & 0xFF00
}

mod interrupt {
    #[derive(PartialEq, Eq, Clone, Copy)]
    pub enum InterruptType {
        NMI,
        IRQ,
        BRK,
        RESET,
    }

    #[derive(PartialEq, Eq, Clone, Copy)]
    pub(super) struct Interrupt {
        pub(super) itype: InterruptType,
        pub(super) vector_addr: u16,
        pub(super) b_flag_mask: u8,
        pub(super) cpu_cycles: u8,
    }

    pub(super) const NMI: Interrupt = Interrupt {
        itype: InterruptType::NMI,
        vector_addr: 0xFFFA,
        b_flag_mask: 0b00100000, // Bit 5 set to 1, BREAK set to 0
        cpu_cycles: 7,
    };

    pub(super) const IRQ: Interrupt = Interrupt {
        itype: InterruptType::IRQ,
        vector_addr: 0xFFFE,
        b_flag_mask: 0b00100000, // Bit 5 set to 1, BREAK set to 0
        cpu_cycles: 7,
    };

    pub(super) const BRK: Interrupt = Interrupt {
        itype: InterruptType::BRK,
        vector_addr: 0xFFFE,
        b_flag_mask: 0b00110000, // Bit 5 and BREAK set to 1
        cpu_cycles: 7,
    };

    pub(super) const _RESET: Interrupt = Interrupt {
        itype: InterruptType::RESET,
        vector_addr: 0xFFFC,
        b_flag_mask: 0b00100000,
        cpu_cycles: 7,
    };
}

impl<'a> CPU<'a> {
    pub fn new<'b>(bus: Bus<'b>) -> CPU<'b> {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: STACK_RESET,
            program_counter: 0,
            status: CpuFlags::from_bits_truncate(0b00100100),
            bus,
            nmi_pending: false,
            irq_pending: false,
            cycles: 0,
        }
    }

    /// CPU reset according to NES specifications
    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = STACK_RESET;
        self.status = CpuFlags::from_bits_truncate(0b00100100);
        self.program_counter = self.mem_read_u16(RESET_VECTOR);
        self.nmi_pending = false;
        self.irq_pending = false;
        self.cycles = 0;
    }

    /// Trigger an IRQ interrupt
    pub fn trigger_irq(&mut self) {
        self.irq_pending = true;
    }

    /// Calculate effective address according to addressing mode (public method for trace.rs)
    pub fn get_absolute_address(&mut self, mode: &AddressingMode, addr: u16) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate => (addr, false),

            AddressingMode::ZeroPage => (self.mem_read(addr) as u16, false),

            AddressingMode::ZeroPageX => {
                let addr = self.mem_read(addr).wrapping_add(self.register_x) as u16;
                (addr, false)
            }

            AddressingMode::ZeroPageY => {
                let addr = self.mem_read(addr).wrapping_add(self.register_y) as u16;
                (addr, false)
            }

            AddressingMode::Absolute => (self.mem_read_u16(addr), false),

            AddressingMode::AbsoluteX => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register_x as u16);
                (addr, page_cross(base, addr))
            }

            AddressingMode::AbsoluteY => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register_y as u16);
                (addr, page_cross(base, addr))
            }

            AddressingMode::IndirectX => {
                let base = self.mem_read(addr);
                let ptr = base.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8 | (lo as u16), false)
            }

            AddressingMode::IndirectY => {
                let base = self.mem_read(addr);
                let lo = self.mem_read(base as u16);
                let hi = self.mem_read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                (deref, page_cross(deref_base, deref))
            }

            AddressingMode::Relative => {
                let offset = self.mem_read(addr) as i8;
                let addr = addr.wrapping_add(1).wrapping_add(offset as u16);
                (addr, false)
            }

            AddressingMode::Indirect => {
                let ptr = self.mem_read_u16(addr);
                // 6502 bug: JMP ($xxFF) reads high byte from $xx00 instead of $xx+1,00
                let addr = if ptr & 0x00FF == 0x00FF {
                    let lo = self.mem_read(ptr);
                    let hi = self.mem_read(ptr & 0xFF00);
                    (hi as u16) << 8 | (lo as u16)
                } else {
                    self.mem_read_u16(ptr)
                };
                (addr, false)
            }

            _ => panic!("Addressing mode {:?} not supported", mode),
        }
    }

    /// Calculate effective address according to addressing mode
    fn get_operand_address(&mut self, mode: &AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate => (self.program_counter, false),

            AddressingMode::ZeroPage => (self.mem_read(self.program_counter) as u16, false),

            AddressingMode::ZeroPageX => {
                let addr = self
                    .mem_read(self.program_counter)
                    .wrapping_add(self.register_x) as u16;
                (addr, false)
            }

            AddressingMode::ZeroPageY => {
                let addr = self
                    .mem_read(self.program_counter)
                    .wrapping_add(self.register_y) as u16;
                (addr, false)
            }

            AddressingMode::Absolute => (self.mem_read_u16(self.program_counter), false),

            AddressingMode::AbsoluteX => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                (addr, page_cross(base, addr))
            }

            AddressingMode::AbsoluteY => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                (addr, page_cross(base, addr))
            }

            AddressingMode::IndirectX => {
                let base = self.mem_read(self.program_counter);
                let ptr = base.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8 | (lo as u16), false)
            }

            AddressingMode::IndirectY => {
                let base = self.mem_read(self.program_counter);
                let lo = self.mem_read(base as u16);
                let hi = self.mem_read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                (deref, page_cross(deref_base, deref))
            }

            AddressingMode::Relative => {
                let offset = self.mem_read(self.program_counter) as i8;
                let addr = self
                    .program_counter
                    .wrapping_add(1)
                    .wrapping_add(offset as u16);
                (addr, false)
            }

            AddressingMode::Indirect => {
                let ptr = self.mem_read_u16(self.program_counter);
                // 6502 bug: JMP ($xxFF) reads high byte from $xx00 instead of $xx+1,00
                let addr = if ptr & 0x00FF == 0x00FF {
                    let lo = self.mem_read(ptr);
                    let hi = self.mem_read(ptr & 0xFF00);
                    (hi as u16) << 8 | (lo as u16)
                } else {
                    self.mem_read_u16(ptr)
                };
                (addr, false)
            }

            _ => panic!("Addressing mode {:?} not supported", mode),
        }
    }

    /// Update Zero and Negative flags
    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.status.set(CpuFlags::ZERO, result == 0);
        self.status.set(CpuFlags::NEGATIVE, result & 0x80 != 0);
    }

    /// Stack management - Push
    fn stack_push(&mut self, data: u8) {
        self.mem_write(STACK_BASE + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    /// Stack management - Pop
    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read(STACK_BASE + self.stack_pointer as u16)
    }

    /// Push 16-bit to stack (high byte first)
    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    /// Pop 16-bit from stack
    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    /// Addition with carry - correct overflow flag implementation
    fn add_to_register_a(&mut self, data: u8) {
        let carry_in = if self.status.contains(CpuFlags::CARRY) {
            1
        } else {
            0
        };
        let sum = self.register_a as u16 + data as u16 + carry_in;

        // Carry flag
        self.status.set(CpuFlags::CARRY, sum > 0xFF);

        let result = sum as u8;

        // Overflow flag: detects signed overflow
        // V = (A^result) & (data^result) & 0x80
        let overflow = (self.register_a ^ result) & (data ^ result) & 0x80 != 0;
        self.status.set(CpuFlags::OVERFLOW, overflow);

        self.register_a = result;
        self.update_zero_and_negative_flags(result);
    }

    /// Subtraction with borrow
    fn sub_from_register_a(&mut self, data: u8) {
        // SBC = ADC with two's complement
        self.add_to_register_a(!data);
    }

    /// Comparison - corrected logic
    fn compare(&mut self, mode: &AddressingMode, compare_with: u8) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let data = self.mem_read(addr);

        // Carry flag set if compare_with >= data
        self.status.set(CpuFlags::CARRY, compare_with >= data);

        let result = compare_with.wrapping_sub(data);
        self.update_zero_and_negative_flags(result);

        page_cross
    }

    /// Conditional branch with correct cycle management
    fn branch(&mut self, condition: bool) {
        if condition {
            let old_pc = self.program_counter;
            let offset = self.mem_read(self.program_counter) as i8;
            let new_pc = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(offset as u16);

            self.program_counter = new_pc;

            // +1 cycle if branch taken
            self.bus.tick(1);

            // +1 additional cycle if page boundary crossed
            if page_cross(old_pc.wrapping_add(1), new_pc) {
                self.bus.tick(1);
            }
        }
    }

    /// Interrupt handling
    fn interrupt(&mut self, interrupt: interrupt::Interrupt) {
        if interrupt.itype != interrupt::InterruptType::RESET {
            self.stack_push_u16(self.program_counter);

            let mut status = self.status;
            status.set(CpuFlags::BREAK, interrupt.b_flag_mask & 0x10 != 0);
            status.insert(CpuFlags::UNUSED); // Bit 5 always set to 1

            self.stack_push(status.bits());
        }

        self.status.insert(CpuFlags::INTERRUPT_DISABLE);
        self.program_counter = self.mem_read_u16(interrupt.vector_addr);

        self.bus.tick(interrupt.cpu_cycles);
    }

    // Processor instructions

    /// LDA - Load Accumulator
    fn lda(&mut self, mode: &AddressingMode) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a = value;
        self.update_zero_and_negative_flags(value);
        page_cross
    }

    /// LDX - Load X Register
    fn ldx(&mut self, mode: &AddressingMode) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_x = value;
        self.update_zero_and_negative_flags(value);
        page_cross
    }

    /// LDY - Load Y Register
    fn ldy(&mut self, mode: &AddressingMode) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_y = value;
        self.update_zero_and_negative_flags(value);
        page_cross
    }

    /// STA - Store Accumulator
    fn sta(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    /// STX - Store X Register
    fn stx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.mem_write(addr, self.register_x);
    }

    /// STY - Store Y Register
    fn sty(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.mem_write(addr, self.register_y);
    }

    /// ADC - Add with Carry
    fn adc(&mut self, mode: &AddressingMode) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.add_to_register_a(value);
        page_cross
    }

    /// SBC - Subtract with Carry
    fn sbc(&mut self, mode: &AddressingMode) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.sub_from_register_a(value);
        page_cross
    }

    /// AND - Logical AND
    fn and(&mut self, mode: &AddressingMode) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
        page_cross
    }

    /// EOR - Exclusive OR
    fn eor(&mut self, mode: &AddressingMode) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a ^= value;
        self.update_zero_and_negative_flags(self.register_a);
        page_cross
    }

    /// ORA - Logical OR
    fn ora(&mut self, mode: &AddressingMode) -> bool {
        let (addr, page_cross) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a |= value;
        self.update_zero_and_negative_flags(self.register_a);
        page_cross
    }

    /// ASL - Arithmetic Shift Left
    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        match mode {
            AddressingMode::Accumulator => {
                self.status
                    .set(CpuFlags::CARRY, self.register_a & 0x80 != 0);
                self.register_a <<= 1;
                self.update_zero_and_negative_flags(self.register_a);
                self.register_a
            }
            _ => {
                let (addr, _) = self.get_operand_address(mode);
                let mut value = self.mem_read(addr);
                self.status.set(CpuFlags::CARRY, value & 0x80 != 0);
                value <<= 1;
                self.mem_write(addr, value);
                self.update_zero_and_negative_flags(value);
                value
            }
        }
    }

    /// LSR - Logical Shift Right
    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        match mode {
            AddressingMode::Accumulator => {
                self.status
                    .set(CpuFlags::CARRY, self.register_a & 0x01 != 0);
                self.register_a >>= 1;
                self.update_zero_and_negative_flags(self.register_a);
                self.register_a
            }
            _ => {
                let (addr, _) = self.get_operand_address(mode);
                let mut value = self.mem_read(addr);
                self.status.set(CpuFlags::CARRY, value & 0x01 != 0);
                value >>= 1;
                self.mem_write(addr, value);
                self.update_zero_and_negative_flags(value);
                value
            }
        }
    }

    /// ROL - Rotate Left
    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let old_carry = self.status.contains(CpuFlags::CARRY);

        match mode {
            AddressingMode::Accumulator => {
                self.status
                    .set(CpuFlags::CARRY, self.register_a & 0x80 != 0);
                self.register_a = (self.register_a << 1) | (old_carry as u8);
                self.update_zero_and_negative_flags(self.register_a);
                self.register_a
            }
            _ => {
                let (addr, _) = self.get_operand_address(mode);
                let mut value = self.mem_read(addr);
                self.status.set(CpuFlags::CARRY, value & 0x80 != 0);
                value = (value << 1) | (old_carry as u8);
                self.mem_write(addr, value);
                self.update_zero_and_negative_flags(value);
                value
            }
        }
    }

    /// ROR - Rotate Right
    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let old_carry = self.status.contains(CpuFlags::CARRY);

        match mode {
            AddressingMode::Accumulator => {
                self.status
                    .set(CpuFlags::CARRY, self.register_a & 0x01 != 0);
                self.register_a = (self.register_a >> 1) | ((old_carry as u8) << 7);
                self.update_zero_and_negative_flags(self.register_a);
                self.register_a
            }
            _ => {
                let (addr, _) = self.get_operand_address(mode);
                let mut value = self.mem_read(addr);
                self.status.set(CpuFlags::CARRY, value & 0x01 != 0);
                value = (value >> 1) | ((old_carry as u8) << 7);
                self.mem_write(addr, value);
                self.update_zero_and_negative_flags(value);
                value
            }
        }
    }

    /// INC - Increment Memory
    fn inc(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr).wrapping_add(1);
        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    /// DEC - Decrement Memory
    fn dec(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr).wrapping_sub(1);
        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
        value
    }

    /// INX - Increment X Register
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    /// INY - Increment Y Register
    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    /// DEX - Decrement X Register
    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    /// DEY - Decrement Y Register
    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    /// TAX - Transfer A to X
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    /// TAY - Transfer A to Y
    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_flags(self.register_y);
    }

    /// TXA - Transfer X to A
    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// TYA - Transfer Y to A
    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// TSX - Transfer Stack Pointer to X
    fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
        self.update_zero_and_negative_flags(self.register_x);
    }

    /// TXS - Transfer X to Stack Pointer
    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    /// PHA - Push Accumulator
    fn pha(&mut self) {
        self.stack_push(self.register_a);
    }

    /// PLA - Pull Accumulator
    fn pla(&mut self) {
        self.register_a = self.stack_pop();
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// PHP - Push Processor Status
    fn php(&mut self) {
        let mut status = self.status;
        status.insert(CpuFlags::BREAK);
        status.insert(CpuFlags::UNUSED);
        self.stack_push(status.bits());
    }

    /// PLP - Pull Processor Status
    fn plp(&mut self) {
        let status_bits = self.stack_pop();
        self.status = CpuFlags::from_bits_truncate(status_bits);
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::UNUSED);
    }

    /// BIT - Bit Test
    fn bit(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        let result = self.register_a & value;

        self.status.set(CpuFlags::ZERO, result == 0);
        self.status.set(CpuFlags::NEGATIVE, value & 0x80 != 0);
        self.status.set(CpuFlags::OVERFLOW, value & 0x40 != 0);
    }

    /// JMP - Jump
    fn jmp(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        self.program_counter = addr;
    }

    /// JSR - Jump to Subroutine
    fn jsr(&mut self) {
        self.stack_push_u16(self.program_counter.wrapping_add(1));
        self.program_counter = self.mem_read_u16(self.program_counter);
    }

    /// RTS - Return from Subroutine
    fn rts(&mut self) {
        self.program_counter = self.stack_pop_u16().wrapping_add(1);
    }

    /// RTI - Return from Interrupt
    fn rti(&mut self) {
        let status_bits = self.stack_pop();
        self.status = CpuFlags::from_bits_truncate(status_bits);
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::UNUSED);
        self.program_counter = self.stack_pop_u16();
    }

    /// BRK - Force Interrupt
    fn brk(&mut self) {
        self.program_counter = self.program_counter.wrapping_add(1);
        self.interrupt(interrupt::BRK);
    }

    // Undocumented instructions (Illegal Opcodes)

    /// LAX - Load A and X
    fn lax(&mut self, mode: &AddressingMode) -> bool {
        let page_cross = self.lda(mode);
        self.register_x = self.register_a;
        page_cross
    }

    /// SAX - Store A AND X
    fn sax(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.register_a & self.register_x;
        self.mem_write(addr, value);
    }

    /// DCP - Decrement and Compare
    fn dcp(&mut self, mode: &AddressingMode) {
        let value = self.dec(mode);
        self.status.set(CpuFlags::CARRY, self.register_a >= value);
        let result = self.register_a.wrapping_sub(value);
        self.update_zero_and_negative_flags(result);
    }

    /// ISC - Increment and Subtract with Carry
    fn isc(&mut self, mode: &AddressingMode) {
        let value = self.inc(mode);
        self.sub_from_register_a(value);
    }

    /// SLO - Shift Left and OR
    fn slo(&mut self, mode: &AddressingMode) {
        let value = self.asl(mode);
        self.register_a |= value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// RLA - Rotate Left and AND
    fn rla(&mut self, mode: &AddressingMode) {
        let value = self.rol(mode);
        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// SRE - Shift Right and EOR
    fn sre(&mut self, mode: &AddressingMode) {
        let value = self.lsr(mode);
        self.register_a ^= value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// RRA - Rotate Right and Add with Carry
    fn rra(&mut self, mode: &AddressingMode) {
        let value = self.ror(mode);
        self.add_to_register_a(value);
    }

    /// ANC - AND with Carry
    fn anc(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
        self.status
            .set(CpuFlags::CARRY, self.status.contains(CpuFlags::NEGATIVE));
    }

    /// ALR - AND and Logical Shift Right
    fn alr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a &= value;
        self.status
            .set(CpuFlags::CARRY, self.register_a & 0x01 != 0);
        self.register_a >>= 1;
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// ARR - AND and Rotate Right
    fn arr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a &= value;

        let old_carry = self.status.contains(CpuFlags::CARRY);
        self.register_a = (self.register_a >> 1) | ((old_carry as u8) << 7);

        let bit_5 = (self.register_a >> 5) & 1;
        let bit_6 = (self.register_a >> 6) & 1;

        self.status.set(CpuFlags::CARRY, bit_6 != 0);
        self.status.set(CpuFlags::OVERFLOW, bit_5 ^ bit_6 != 0);
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// AXS - AND X with A and Subtract
    fn axs(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        let x_and_a = self.register_x & self.register_a;
        let result = x_and_a.wrapping_sub(value);

        self.status.set(CpuFlags::CARRY, x_and_a >= value);
        self.register_x = result;
        self.update_zero_and_negative_flags(result);
    }

    /// LXA - Load X and A (unstable behavior)
    fn lxa(&mut self, mode: &AddressingMode) {
        let page_cross = self.lda(mode);
        self.register_x = self.register_a;
        if page_cross {
            self.bus.tick(1);
        }
    }

    /// XAA - Transfer X to A and AND (unstable behavior)
    fn xaa(&mut self, mode: &AddressingMode) {
        self.register_a = self.register_x;
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    /// LAS - Load A, X and S
    fn las(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let value = self.mem_read(addr) & self.stack_pointer;
        self.register_a = value;
        self.register_x = value;
        self.stack_pointer = value;
        self.update_zero_and_negative_flags(value);
    }

    /// TAS - Transfer A AND X to S
    fn tas(&mut self, mode: &AddressingMode) {
        let value = self.register_a & self.register_x;
        self.stack_pointer = value;
        let (addr, _) = self.get_operand_address(mode);
        let data = value & ((addr >> 8) as u8).wrapping_add(1);
        self.mem_write(addr, data);
    }

    /// AHX - AND A, X and High byte
    fn ahx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let data = self.register_a & self.register_x & ((addr >> 8) as u8).wrapping_add(1);
        self.mem_write(addr, data);
    }

    /// SHX - Store X AND High byte
    fn shx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let data = self.register_x & ((addr >> 8) as u8).wrapping_add(1);
        self.mem_write(addr, data);
    }

    /// SHY - Store Y AND High byte
    fn shy(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.get_operand_address(mode);
        let data = self.register_y & ((addr >> 8) as u8).wrapping_add(1);
        self.mem_write(addr, data);
    }

    /// NOP - No Operation (with read for certain variants)
    fn nop(&mut self, mode: &AddressingMode) -> bool {
        match mode {
            AddressingMode::Implied => false,
            _ => {
                let (_, page_cross) = self.get_operand_address(mode);
                page_cross
            }
        }
    }

    /// KIL - Halt processor (Jam)
    fn kil(&mut self) {
        // In a real NES, this would halt the processor
        // Here we can either panic or loop indefinitely
        panic!("CPU halted by KIL instruction");
    }

    pub fn collect_audio_sample(&mut self) -> Option<f32> {
        self.bus.collect_audio_sample()
    }

    // Utility methods for testing and debugging

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.program_counter = 0x0600;
        self.run()
    }

    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(0x0600 + i, program[i as usize]);
        }
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

        loop {
            let code = self.mem_read(self.program_counter);
            callback(self);

            if code == 0x00 {
                //BRK
                return;
            }
            if opcodes.get(&code).is_none() {
                // Not a valid opcode, might be data, stop execution
                // This is a simple way to end test programs
                return;
            }

            self.step();
        }
    }

    pub fn step(&mut self) -> u8 {
        // Interrupt handling
        if let Some(_nmi) = self.bus.poll_nmi_status() {
            self.interrupt(interrupt::NMI);
        }

        // IRQ interrupt handling
        if self.irq_pending && !self.status.contains(CpuFlags::INTERRUPT_DISABLE) {
            self.irq_pending = false;
            self.interrupt(interrupt::IRQ);
        }

        let code = self.mem_read(self.program_counter);
        self.program_counter = self.program_counter.wrapping_add(1);
        let program_counter_state = self.program_counter;

        let opcodes: &HashMap<u8, &'static opcodes::OpCode> = &opcodes::OPCODES_MAP;
        let opcode = opcodes
            .get(&code)
            .unwrap_or_else(|| panic!("OpCode 0x{:02X} not recognized", code));

        let mut page_cross = false;

        match code {
            // LDA
            0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                page_cross = self.lda(&opcode.mode);
            }

            // LDX
            0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => {
                page_cross = self.ldx(&opcode.mode);
            }

            // LDY
            0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => {
                page_cross = self.ldy(&opcode.mode);
            }

            // STA
            0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                self.sta(&opcode.mode);
            }

            // STX
            0x86 | 0x96 | 0x8E => {
                self.stx(&opcode.mode);
            }

            // STY
            0x84 | 0x94 | 0x8C => {
                self.sty(&opcode.mode);
            }

            // ADC
            0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => {
                page_cross = self.adc(&opcode.mode);
            }

            // SBC
            0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 | 0xEB => {
                page_cross = self.sbc(&opcode.mode);
            }

            // AND
            0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => {
                page_cross = self.and(&opcode.mode);
            }

            // EOR
            0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => {
                page_cross = self.eor(&opcode.mode);
            }

            // ORA
            0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => {
                page_cross = self.ora(&opcode.mode);
            }

            // ASL
            0x0A => {
                self.asl(&AddressingMode::Accumulator);
            }
            0x06 | 0x16 | 0x0E | 0x1E => {
                self.asl(&opcode.mode);
            }

            // LSR
            0x4A => {
                self.lsr(&AddressingMode::Accumulator);
            }
            0x46 | 0x56 | 0x4E | 0x5E => {
                self.lsr(&opcode.mode);
            }

            // ROL
            0x2A => {
                self.rol(&AddressingMode::Accumulator);
            }
            0x26 | 0x36 | 0x2E | 0x3E => {
                self.rol(&opcode.mode);
            }

            // ROR
            0x6A => {
                self.ror(&AddressingMode::Accumulator);
            }
            0x66 | 0x76 | 0x6E | 0x7E => {
                self.ror(&opcode.mode);
            }

            // INC
            0xE6 | 0xF6 | 0xEE | 0xFE => {
                self.inc(&opcode.mode);
            }

            // DEC
            0xC6 | 0xD6 | 0xCE | 0xDE => {
                self.dec(&opcode.mode);
            }

            // INX
            0xE8 => self.inx(),

            // INY
            0xC8 => self.iny(),

            // DEX
            0xCA => self.dex(),

            // DEY
            0x88 => self.dey(),

            // TAX
            0xAA => self.tax(),

            // TAY
            0xA8 => self.tay(),

            // TXA
            0x8A => self.txa(),

            // TYA
            0x98 => self.tya(),

            // TSX
            0xBA => self.tsx(),

            // TXS
            0x9A => self.txs(),

            // PHA
            0x48 => self.pha(),

            // PLA
            0x68 => self.pla(),

            // PHP
            0x08 => self.php(),

            // PLP
            0x28 => self.plp(),

            // BIT
            0x24 | 0x2C => {
                self.bit(&opcode.mode);
            }

            // JMP
            0x4C | 0x6C => {
                self.jmp(&opcode.mode);
            }

            // JSR
            0x20 => self.jsr(),

            // RTS
            0x60 => self.rts(),

            // RTI
            0x40 => self.rti(),

            // BRK - Force Interrupt then terminate for tests
            0x00 => {
                self.brk();
                // return; We can't return from step so we just let it run
            }

            // CMP
            0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => {
                page_cross = self.compare(&opcode.mode, self.register_a);
            }

            // CPX
            0xE0 | 0xE4 | 0xEC => {
                page_cross = self.compare(&opcode.mode, self.register_x);
            }

            // CPY
            0xC0 | 0xC4 | 0xCC => {
                page_cross = self.compare(&opcode.mode, self.register_y);
            }

            // Branch instructions
            0x10 => self.branch(!self.status.contains(CpuFlags::NEGATIVE)), // BPL
            0x30 => self.branch(self.status.contains(CpuFlags::NEGATIVE)),  // BMI
            0x50 => self.branch(!self.status.contains(CpuFlags::OVERFLOW)), // BVC
            0x70 => self.branch(self.status.contains(CpuFlags::OVERFLOW)),  // BVS
            0x90 => self.branch(!self.status.contains(CpuFlags::CARRY)),    // BCC
            0xB0 => self.branch(self.status.contains(CpuFlags::CARRY)),     // BCS
            0xD0 => self.branch(!self.status.contains(CpuFlags::ZERO)),     // BNE
            0xF0 => self.branch(self.status.contains(CpuFlags::ZERO)),      // BEQ

            // Flag operations
            0x18 => self.status.remove(CpuFlags::CARRY), // CLC
            0x38 => self.status.insert(CpuFlags::CARRY), // SEC
            0x58 => self.status.remove(CpuFlags::INTERRUPT_DISABLE), // CLI
            0x78 => self.status.insert(CpuFlags::INTERRUPT_DISABLE), // SEI
            0xB8 => self.status.remove(CpuFlags::OVERFLOW), // CLV
            0xD8 => self.status.remove(CpuFlags::DECIMAL_MODE), // CLD
            0xF8 => self.status.insert(CpuFlags::DECIMAL_MODE), // SED

            // NOP
            0xEA => {}

            // Undocumented instructions

            // LAX
            0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => {
                page_cross = self.lax(&opcode.mode);
            }

            // SAX
            0x87 | 0x97 | 0x8F | 0x83 => {
                self.sax(&opcode.mode);
            }

            // DCP
            0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xD3 | 0xC3 => {
                self.dcp(&opcode.mode);
            }

            // ISC
            0xE7 | 0xF7 | 0xEF | 0xFF | 0xFB | 0xE3 | 0xF3 => {
                self.isc(&opcode.mode);
            }

            // SLO
            0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => {
                self.slo(&opcode.mode);
            }

            // RLA
            0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x33 | 0x23 => {
                self.rla(&opcode.mode);
            }

            // SRE
            0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => {
                self.sre(&opcode.mode);
            }

            // RRA
            0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => {
                self.rra(&opcode.mode);
            }

            // ANC
            0x0B | 0x2B => {
                self.anc(&opcode.mode);
            }

            // ALR
            0x4B => {
                self.alr(&opcode.mode);
            }

            // ARR
            0x6B => {
                self.arr(&opcode.mode);
            }

            // AXS
            0xCB => {
                self.axs(&opcode.mode);
            }

            // LXA
            0xAB => {
                self.lxa(&opcode.mode);
            }

            // XAA
            0x8B => {
                self.xaa(&opcode.mode);
            }

            // LAS
            0xBB => {
                self.las(&opcode.mode);
            }

            // TAS
            0x9B => {
                self.tas(&opcode.mode);
            }

            // AHX
            0x9F | 0x93 => {
                self.ahx(&opcode.mode);
            }

            // SHX
            0x9E => {
                self.shx(&opcode.mode);
            }

            // SHY
            0x9C => {
                self.shy(&opcode.mode);
            }

            // NOP variants
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {} // Implied NOPs
            0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => {}        // Immediate NOPs
            0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
                page_cross = self.nop(&opcode.mode);
            }
            0x0C | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                page_cross = self.nop(&opcode.mode);
            }

            // KIL (JAM)
            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                self.kil();
            }
        }

        let mut cycles = opcode.cycles;
        if page_cross {
            cycles += 1;
        }

        // Cycle management
        self.bus.tick(cycles);

        // Update program counter if not modified by instruction
        if program_counter_state == self.program_counter {
            self.program_counter = self.program_counter.wrapping_add((opcode.len - 1) as u16);
        }

        cycles
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let bus = Bus::new(
            test::test_rom_containing(vec![]),
            44100.0,
            |_ppu, _joypad| {},
        );
        let mut cpu = CPU::new(bus);

        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);

        assert_eq!(cpu.register_a, 5);
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let bus = Bus::new(
            test::test_rom_containing(vec![]),
            44100.0,
            |_ppu, _joypad| {},
        );
        let mut cpu = CPU::new(bus);
        cpu.register_a = 10;

        cpu.load_and_run(vec![0xaa, 0x00]);

        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_5_ops_working_together() {
        let bus = Bus::new(
            test::test_rom_containing(vec![]),
            44100.0,
            |_ppu, _joypad| {},
        );
        let mut cpu = CPU::new(bus);

        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1);
    }

    #[test]
    fn test_inx_overflow() {
        let bus = Bus::new(
            test::test_rom_containing(vec![]),
            44100.0,
            |_ppu, _joypad| {},
        );
        let mut cpu = CPU::new(bus);
        cpu.register_x = 0xff;

        cpu.load_and_run(vec![0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 1);
    }

    #[test]
    fn test_lda_from_memory() {
        let bus = Bus::new(
            test::test_rom_containing(vec![]),
            44100.0,
            |_ppu, _joypad| {},
        );
        let mut cpu = CPU::new(bus);
        cpu.mem_write(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_adc_overflow_flag() {
        let bus = Bus::new(
            test::test_rom_containing(vec![]),
            44100.0,
            |_ppu, _joypad| {},
        );
        let mut cpu = CPU::new(bus);

        cpu.load_and_run(vec![0xa9, 0x7f, 0x69, 0x01, 0x00]);

        assert_eq!(cpu.register_a, 0x80);
        assert!(cpu.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.status.contains(CpuFlags::NEGATIVE));
        assert!(!cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_sbc_borrow_flag() {
        let bus = Bus::new(
            test::test_rom_containing(vec![]),
            44100.0,
            |_ppu, _joypad| {},
        );
        let mut cpu = CPU::new(bus);

        cpu.load_and_run(vec![0xa9, 0x50, 0xe9, 0xf0, 0x00]);

        assert_eq!(cpu.register_a, 0x5f); // 0x50 - 0xf0 - 1 = 0x5f
        assert!(!cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_compare_instructions() {
        let bus = Bus::new(
            test::test_rom_containing(vec![]),
            44100.0,
            |_ppu, _joypad| {},
        );
        let mut cpu = CPU::new(bus);

        cpu.load_and_run(vec![0xa9, 0x05, 0xc9, 0x05, 0x00]);

        assert!(cpu.status.contains(CpuFlags::CARRY)); // A >= M
        assert!(cpu.status.contains(CpuFlags::ZERO)); // A == M
    }
}
