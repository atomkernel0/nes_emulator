use crate::cpu::AddressingMode;
use std::collections::HashMap;

pub struct OpCode {
    pub code: u8,
    pub mnemonic: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl OpCode {
    fn new(code: u8, mnemonic: &'static str, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        OpCode {
            code: code,
            mnemonic: mnemonic,
            len: len,
            cycles: cycles,
            mode: mode,
        }
    }
}

lazy_static! {
    pub static ref CPU_OPS_CODES: Vec<OpCode> = vec![
        OpCode::new(0x00, "BRK", 1, 7, AddressingMode::NoneAddressing),
        OpCode::new(0xea, "NOP", 1, 2, AddressingMode::NoneAddressing),

        /* Arithmetic */
        OpCode::new(0x69, "ADC", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x65, "ADC", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x75, "ADC", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x6d, "ADC", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x7d, "ADC", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X),
        OpCode::new(0x79, "ADC", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y),
        OpCode::new(0x61, "ADC", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x71, "ADC", 2, 5/*+1 if page crossed*/, AddressingMode::Indirect_Y),

        OpCode::new(0xe9, "SBC", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xe5, "SBC", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xf5, "SBC", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xed, "SBC", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xfd, "SBC", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X),
        OpCode::new(0xf9, "SBC", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y),
        OpCode::new(0xe1, "SBC", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xf1, "SBC", 2, 5/*+1 if page crossed*/, AddressingMode::Indirect_Y),

        OpCode::new(0x29, "AND", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x2d, "AND", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x3d, "AND", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X),
        OpCode::new(0x39, "AND", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y),
        OpCode::new(0x21, "AND", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x31, "AND", 2, 5/*+1 if page crossed*/, AddressingMode::Indirect_Y),

        OpCode::new(0x49, "EOR", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x45, "EOR", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x55, "EOR", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x4d, "EOR", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x5d, "EOR", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X),
        OpCode::new(0x59, "EOR", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y),
        OpCode::new(0x41, "EOR", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x51, "EOR", 2, 5/*+1 if page crossed*/, AddressingMode::Indirect_Y),

        OpCode::new(0x09, "ORA", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x05, "ORA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x15, "ORA", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x0d, "ORA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x1d, "ORA", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X),
        OpCode::new(0x19, "ORA", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y),
        OpCode::new(0x01, "ORA", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x11, "ORA", 2, 5/*+1 if page crossed*/, AddressingMode::Indirect_Y),

        /* Shifts */
        OpCode::new(0x0a, "ASL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x06, "ASL", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x16, "ASL", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x0e, "ASL", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x1e, "ASL", 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0x4a, "LSR", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x46, "LSR", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x56, "LSR", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x4e, "LSR", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x5e, "LSR", 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0x2a, "ROL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x26, "ROL", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x36, "ROL", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x2e, "ROL", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x3e, "ROL", 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0x6a, "ROR", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x66, "ROR", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x76, "ROR", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x6e, "ROR", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x7e, "ROR", 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0xe6, "INC", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xf6, "INC", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0xee, "INC", 3, 6, AddressingMode::Absolute),
        OpCode::new(0xfe, "INC", 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0xe8, "INX", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xc8, "INY", 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0xc6, "DEC", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xd6, "DEC", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0xce, "DEC", 3, 6, AddressingMode::Absolute),
        OpCode::new(0xde, "DEC", 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0xca, "DEX", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x88, "DEY", 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0xc9, "CMP", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xc5, "CMP", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xd5, "CMP", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xcd, "CMP", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xdd, "CMP", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X),
        OpCode::new(0xd9, "CMP", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y),
        OpCode::new(0xc1, "CMP", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xd1, "CMP", 2, 5/*+1 if page crossed*/, AddressingMode::Indirect_Y),

        OpCode::new(0xc0, "CPY", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xc4, "CPY", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xcc, "CPY", 3, 4, AddressingMode::Absolute),

        OpCode::new(0xe0, "CPX", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xe4, "CPX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xec, "CPX", 3, 4, AddressingMode::Absolute),


        /* Branching */

        OpCode::new(0x4c, "JMP", 3, 3, AddressingMode::NoneAddressing), //AddressingMode that acts as Immidiate
        OpCode::new(0x6c, "JMP", 3, 5, AddressingMode::NoneAddressing), //AddressingMode:Indirect with 6502 bug

        OpCode::new(0x20, "JSR", 3, 6, AddressingMode::NoneAddressing),
        OpCode::new(0x60, "RTS", 1, 6, AddressingMode::NoneAddressing),

        OpCode::new(0x40, "RTI", 1, 6, AddressingMode::NoneAddressing),

        OpCode::new(0xd0, "BNE", 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x70, "BVS", 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x50, "BVC", 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x30, "BMI", 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0xf0, "BEQ", 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0xb0, "BCS", 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x90, "BCC", 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),
        OpCode::new(0x10, "BPL", 2, 2 /*(+1 if branch succeeds +2 if to a new page)*/, AddressingMode::NoneAddressing),

        OpCode::new(0x24, "BIT", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x2c, "BIT", 3, 4, AddressingMode::Absolute),


        /* Stores, Loads */
        OpCode::new(0xa9, "LDA", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xa5, "LDA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xb5, "LDA", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xad, "LDA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xbd, "LDA", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X),
        OpCode::new(0xb9, "LDA", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y),
        OpCode::new(0xa1, "LDA", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xb1, "LDA", 2, 5/*+1 if page crossed*/, AddressingMode::Indirect_Y),

        OpCode::new(0xa2, "LDX", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xa6, "LDX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xb6, "LDX", 2, 4, AddressingMode::ZeroPage_Y),
        OpCode::new(0xae, "LDX", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xbe, "LDX", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y),

        OpCode::new(0xa0, "LDY", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xa4, "LDY", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xb4, "LDY", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xac, "LDY", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xbc, "LDY", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X),


        OpCode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x8d, "STA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x9d, "STA", 3, 5, AddressingMode::Absolute_X),
        OpCode::new(0x99, "STA", 3, 5, AddressingMode::Absolute_Y),
        OpCode::new(0x81, "STA", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x91, "STA", 2, 6, AddressingMode::Indirect_Y),

        OpCode::new(0x86, "STX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x96, "STX", 2, 4, AddressingMode::ZeroPage_Y),
        OpCode::new(0x8e, "STX", 3, 4, AddressingMode::Absolute),

        OpCode::new(0x84, "STY", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x94, "STY", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x8c, "STY", 3, 4, AddressingMode::Absolute),


        /* Flags clear */

        OpCode::new(0xD8, "CLD", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x58, "CLI", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xb8, "CLV", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x18, "CLC", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x38, "SEC", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x78, "SEI", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xf8, "SED", 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0xaa, "TAX", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xa8, "TAY", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xba, "TSX", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x8a, "TXA", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x9a, "TXS", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x98, "TYA", 1, 2, AddressingMode::NoneAddressing),

        /* Stack */
        OpCode::new(0x48, "PHA", 1, 3, AddressingMode::NoneAddressing),
        OpCode::new(0x68, "PLA", 1, 4, AddressingMode::NoneAddressing),
        OpCode::new(0x08, "PHP", 1, 3, AddressingMode::NoneAddressing),
        OpCode::new(0x28, "PLP", 1, 4, AddressingMode::NoneAddressing),

        /* Illegals Opcodes (used by many NES games) */
        
        // AAC (ANC) - AND byte with accumulator. If result is negative then carry is set.
        OpCode::new(0x0B, "AAC", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x2B, "AAC", 2, 2, AddressingMode::Immediate),

        // AAX (SAX) - AND X register with accumulator and store result in memory
        OpCode::new(0x87, "AAX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x97, "AAX", 2, 4, AddressingMode::ZeroPage_Y),
        OpCode::new(0x83, "AAX", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x8F, "AAX", 3, 4, AddressingMode::Absolute),

        // ARR - AND byte with accumulator, then rotate one bit right in accumulator
        OpCode::new(0x6B, "ARR", 2, 2, AddressingMode::Immediate),

        // ASR (ALR) - AND byte with accumulator, then shift right one bit in accumulator
        OpCode::new(0x4B, "ASR", 2, 2, AddressingMode::Immediate),

        // ATX (LXA) - AND byte with accumulator, then transfer accumulator to X register
        OpCode::new(0xAB, "ATX", 2, 2, AddressingMode::Immediate),

        // AXA (SHA) - AND X register with accumulator then AND result with 7 and store in memory
        OpCode::new(0x9F, "AXA", 3, 5, AddressingMode::Absolute_Y),
        OpCode::new(0x93, "AXA", 2, 6, AddressingMode::Indirect_Y),

        // AXS (SBX) - AND X register with accumulator and store result in X register, then subtract byte from X register
        OpCode::new(0xCB, "AXS", 2, 2, AddressingMode::Immediate),

        // DCP - Subtract 1 from memory (without borrow)
        OpCode::new(0xC7, "DCP", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xD7, "DCP", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0xCF, "DCP", 3, 6, AddressingMode::Absolute),
        OpCode::new(0xDF, "DCP", 3, 7, AddressingMode::Absolute_X),
        OpCode::new(0xDB, "DCP", 3, 7, AddressingMode::Absolute_Y),
        OpCode::new(0xC3, "DCP", 2, 8, AddressingMode::Indirect_X),
        OpCode::new(0xD3, "DCP", 2, 8, AddressingMode::Indirect_Y),

        // DOP (NOP) - No operation (double NOP)
        OpCode::new(0x04, "DOP", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x14, "DOP", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x34, "DOP", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x44, "DOP", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x54, "DOP", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x64, "DOP", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x74, "DOP", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x80, "DOP", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x82, "DOP", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x89, "DOP", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xC2, "DOP", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xD4, "DOP", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xE2, "DOP", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xF4, "DOP", 2, 4, AddressingMode::ZeroPage_X),

        // ISC (ISB) - Increase memory by one, then subtract memory from accumulator (with borrow)
        OpCode::new(0xE7, "ISC", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xF7, "ISC", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0xEF, "ISC", 3, 6, AddressingMode::Absolute),
        OpCode::new(0xFF, "ISC", 3, 7, AddressingMode::Absolute_X),
        OpCode::new(0xFB, "ISC", 3, 7, AddressingMode::Absolute_Y),
        OpCode::new(0xE3, "ISC", 2, 8, AddressingMode::Indirect_X),
        OpCode::new(0xF3, "ISC", 2, 8, AddressingMode::Indirect_Y),

        // KIL (JAM) - Stop program counter (processor lock up)
        OpCode::new(0x02, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x12, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x22, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x32, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x42, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x52, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x62, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x72, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x92, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xB2, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xD2, "KIL", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xF2, "KIL", 1, 2, AddressingMode::NoneAddressing),

        // LAR (LAE) - AND memory with stack pointer, transfer result to accumulator, X register and stack pointer
        OpCode::new(0xBB, "LAR", 3, 4, AddressingMode::Absolute_Y),

        // LAX - Load accumulator and X register with memory
        OpCode::new(0xA7, "LAX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB7, "LAX", 2, 4, AddressingMode::ZeroPage_Y),
        OpCode::new(0xAF, "LAX", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBF, "LAX", 3, 4, AddressingMode::Absolute_Y),
        OpCode::new(0xA3, "LAX", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xB3, "LAX", 2, 5, AddressingMode::Indirect_Y),

        // NOP - No operation (opcodes illegaux supplémentaires)
        OpCode::new(0x1A, "NOP", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x3A, "NOP", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x5A, "NOP", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x7A, "NOP", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xDA, "NOP", 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xFA, "NOP", 1, 2, AddressingMode::NoneAddressing),

        // RLA - Rotate one bit left in memory, then AND accumulator with memory
        OpCode::new(0x27, "RLA", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x37, "RLA", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x2F, "RLA", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x3F, "RLA", 3, 7, AddressingMode::Absolute_X),
        OpCode::new(0x3B, "RLA", 3, 7, AddressingMode::Absolute_Y),
        OpCode::new(0x23, "RLA", 2, 8, AddressingMode::Indirect_X),
        OpCode::new(0x33, "RLA", 2, 8, AddressingMode::Indirect_Y),

        // RRA - Rotate one bit right in memory, then add memory to accumulator (with carry)
        OpCode::new(0x67, "RRA", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x77, "RRA", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x6F, "RRA", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x7F, "RRA", 3, 7, AddressingMode::Absolute_X),
        OpCode::new(0x7B, "RRA", 3, 7, AddressingMode::Absolute_Y),
        OpCode::new(0x63, "RRA", 2, 8, AddressingMode::Indirect_X),
        OpCode::new(0x73, "RRA", 2, 8, AddressingMode::Indirect_Y),

        // SBC - The same as the legal opcode $E9 (SBC #byte)
        OpCode::new(0xEB, "SBC", 2, 2, AddressingMode::Immediate),

        // SLO - Shift left one bit in memory, then OR accumulator with memory
        OpCode::new(0x07, "SLO", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x17, "SLO", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x0F, "SLO", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x1F, "SLO", 3, 7, AddressingMode::Absolute_X),
        OpCode::new(0x1B, "SLO", 3, 7, AddressingMode::Absolute_Y),
        OpCode::new(0x03, "SLO", 2, 8, AddressingMode::Indirect_X),
        OpCode::new(0x13, "SLO", 2, 8, AddressingMode::Indirect_Y),

        // SRE - Shift right one bit in memory, then EOR accumulator with memory
        OpCode::new(0x47, "SRE", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x57, "SRE", 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x4F, "SRE", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x5F, "SRE", 3, 7, AddressingMode::Absolute_X),
        OpCode::new(0x5B, "SRE", 3, 7, AddressingMode::Absolute_Y),
        OpCode::new(0x43, "SRE", 2, 8, AddressingMode::Indirect_X),
        OpCode::new(0x53, "SRE", 2, 8, AddressingMode::Indirect_Y),

        // SXA (SHX) - AND X register with the high byte of the target address + 1. Store the result in memory
        OpCode::new(0x9E, "SXA", 3, 5, AddressingMode::Absolute_Y),

        // SYA (SHY) - AND Y register with the high byte of the target address + 1. Store the result in memory
        OpCode::new(0x9C, "SYA", 3, 5, AddressingMode::Absolute_X),

        // TOP (NOP) - No operation (triple NOP)
        OpCode::new(0x0C, "TOP", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x1C, "TOP", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0x3C, "TOP", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0x5C, "TOP", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0x7C, "TOP", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0xDC, "TOP", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0xFC, "TOP", 3, 4, AddressingMode::Absolute_X),

        // XAA (ANE) - Exact operation unknown
        OpCode::new(0x8B, "XAA", 2, 2, AddressingMode::Immediate),

        // XAS (SHS) - AND X register with accumulator and store result in stack pointer
        OpCode::new(0x9B, "XAS", 3, 5, AddressingMode::Absolute_Y),

    ];


    pub static ref OPCODES_MAP: HashMap<u8, &'static OpCode> = {
        let mut map = HashMap::new();
        for cpuop in &*CPU_OPS_CODES {
            map.insert(cpuop.code, cpuop);
        }
        map
    };
}
