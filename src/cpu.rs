use std::fmt::{self, Debug, Display, Formatter};

use anyhow::Result;
use bitfield::bitfield;
use bitmatch::bitmatch;
use log::trace;

use crate::bus::CpuBus;

const STACK_BASE: u16 = 0x0100;

#[derive(PartialEq, Eq, Copy, Clone)]
enum AddrMode {
    ZeroPageIndexedX,
    ZeroPageIndexedY,
    AbsoluteIndexedX,
    AbsoluteIndexedY,
    IndexedIndirectX,
    IndirectIndexedY,
    Accumulator,
    Immediate,
    ZeroPage,
    Absolute,
    Relative,
    Indirect,
}

impl Display for AddrMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            &AddrMode::ZeroPage => {
                write!(f, "d")
            }
            &AddrMode::Absolute => {
                write!(f, "a")
            }
            &AddrMode::Relative => {
                write!(f, "*+d")
            }
            &AddrMode::Indirect => {
                write!(f, "(a)")
            }
            &AddrMode::Immediate => {
                write!(f, "#i")
            }
            &AddrMode::Accumulator => {
                write!(f, "A")
            }
            &AddrMode::ZeroPageIndexedX => {
                write!(f, "d+x")
            }
            &AddrMode::ZeroPageIndexedY => {
                write!(f, "d+y")
            }
            &AddrMode::AbsoluteIndexedX => {
                write!(f, "a+x")
            }
            &AddrMode::AbsoluteIndexedY => {
                write!(f, "a+y")
            }
            &AddrMode::IndexedIndirectX => {
                write!(f, "(d+x)")
            }
            &AddrMode::IndirectIndexedY => {
                write!(f, "(d)+x")
            }
        }
    }
}

bitfield! {
    struct P(u8);
    n, set_n: 7;
    v, set_v: 6;
    b, set_b: 5, 4;
    d, set_d: 3;
    i, set_i: 2;
    z, set_z: 1;
    c, set_c: 0;
}

fn cap_if(cond: bool, c: char) -> char {
    if cond {
        c.to_ascii_uppercase()
    } else {
        c.to_ascii_lowercase()
    }
}

impl Display for P {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}{}{}{}{}",
            cap_if(self.n(), 'n'),
            cap_if(self.v(), 'v'),
            cap_if(self.b() & 0b10 > 0, 'u'),
            cap_if(self.b() & 0b01 > 0, 'b'),
            cap_if(self.d(), 'd'),
            cap_if(self.i(), 'i'),
            cap_if(self.z(), 'z'),
            cap_if(self.c(), 'c')
        )
    }
}

pub struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    s: u8,
    p: P,
    pc: u16,

    halt: bool,

    bus: CpuBus,
}

impl Debug for Cpu {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A:{:02X} X:{:02X} Y:{:02X} S:{:02X} P:{} ${:04X}",
            self.a, self.x, self.y, self.s, self.p, self.pc
        )
    }
}

impl Cpu {
    pub fn new(bus: CpuBus) -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            s: 0xFD,
            p: P(0x04),
            pc: 0,
            halt: false,
            bus,
        }
    }

    pub fn reset(&mut self) -> Result<()> {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.s = 0xFD;
        self.p = P(0x04);
        self.pc = self.bus.read_word(0xFFFC)?;
        self.bus.stalls = 0;

        Ok(())
    }

    pub fn tick(&mut self) -> Result<()> {
        self.bus.cycles = self.bus.cycles.wrapping_add(1);

        if self.bus.stalls > 0 {
            self.bus.stalls -= 1;

            return Ok(());
        }

        // TODO 割り込み

        if self.halt {
            return Ok(());
        }

        let opecode = self.bus.read(self.pc)?;

        self.pc = self.pc.wrapping_add(1);

        self.do_mnemonic(opecode)?;

        Ok(())
    }

    fn is_overflow_positive(&mut self, left: u8, right: u8) -> bool {
        let result = left.wrapping_add(right);

        let s1 = (left >> 7) > 0;
        let s2 = (right >> 7) > 0;
        let s3 = (result >> 7) > 0;

        (s1 && s2 && !s3) || (!s1 && !s2 && s3)
    }

    fn is_overflow_negative(&mut self, left: u8, right: u8) -> bool {
        let result = left.wrapping_sub(right);

        let s1 = (left >> 7) > 0;
        let s2 = (right >> 7) > 0;
        let s3 = (result >> 7) > 0;

        (!s1 && s2 && s3) || (s1 && !s2 && !s3)
    }

    fn read_operand_addr_zero_page(&mut self, index: u8) -> Result<u16> {
        let offset = self.bus.read(self.pc)?;
        self.pc = self.pc.wrapping_add(1);

        let addr = offset.wrapping_add(index);

        Ok(addr as u16)
    }

    fn read_operand_addr_absolute(&mut self, index: u8) -> Result<u16> {
        let offset = self.bus.read_word(self.pc)?;
        self.pc = self.pc.wrapping_add(2);

        let addr = offset.wrapping_add(index as u16);

        Ok(addr)
    }

    fn read_operand_addr_indirect(&self, hop_addr: u16) -> Result<u16> {
        let addr = self.bus.read_word(hop_addr)?;

        Ok(addr)
    }

    fn read_operand_addr(&mut self, mode: AddrMode) -> Result<u16> {
        match mode {
            // INST #i
            AddrMode::Immediate => {
                let addr = self.pc;
                self.pc = self.pc.wrapping_add(1);

                Ok(addr)
            }
            // INST d
            AddrMode::ZeroPage => self.read_operand_addr_zero_page(0),
            // INST a
            AddrMode::Absolute => self.read_operand_addr_absolute(0),
            // INST *+d
            AddrMode::Relative => {
                let offset = self.bus.read(self.pc)?;
                self.pc = self.pc.wrapping_add(1);

                let addr = self.pc.wrapping_add(offset as i8 as u16);

                Ok(addr)
            }
            // INST (a)
            AddrMode::Indirect => {
                let hop_addr = self.read_operand_addr_absolute(0)?;
                self.read_operand_addr_indirect(hop_addr)
            }
            // INST A
            AddrMode::Accumulator => Ok(0),
            // INST d,x
            AddrMode::ZeroPageIndexedX => self.read_operand_addr_zero_page(self.x),
            // INST d,y
            AddrMode::ZeroPageIndexedY => self.read_operand_addr_zero_page(self.y),
            // INST a,x
            AddrMode::AbsoluteIndexedX => self.read_operand_addr_absolute(self.x),
            // INST a,y
            AddrMode::AbsoluteIndexedY => self.read_operand_addr_absolute(self.y),
            // INST (d,x)
            AddrMode::IndexedIndirectX => {
                let hop_addr = self.read_operand_addr_zero_page(self.x)?;
                self.read_operand_addr_indirect(hop_addr)
            }
            // INST (d),y
            AddrMode::IndirectIndexedY => {
                let hop_addr = self.read_operand_addr_zero_page(0)?;
                let addr = self.read_operand_addr_indirect(hop_addr)?;

                Ok(addr.wrapping_add(self.y as u16))
            }
        }
    }

    fn set_z_by(&mut self, val: u8) {
        self.p.set_z(val == 0);
    }

    fn set_n_by(&mut self, val: u8) {
        self.p.set_n((val as i8) < 0);
    }

    fn set_zn_by(&mut self, val: u8) {
        self.set_z_by(val);
        self.set_n_by(val);
    }

    #[bitmatch]
    fn do_mnemonic(&mut self, opecode: u8) -> Result<()> {
        #[bitmatch]
        match opecode {
            // Control
            // +00
            // BRK
            "00000000" => self.brk(),
            // JSR a
            "00100000" => self.jsr(AddrMode::Absolute),
            // RTI
            "01000000" => self.rti(),
            // RTS
            "01100000" => self.rts(),
            // NOP #i
            "10000000" => self.nop(),
            // LDY #i
            "10100000" => self.ldy(AddrMode::Immediate),
            // CPY #i
            "11000000" => self.cpy(AddrMode::Immediate),
            // CPX #i
            "11100000" => self.cpx(AddrMode::Immediate),

            // +04
            // NOP d
            "hhh00100" if h == 0b000 || h == 0b010 || h == 0b011 => self.nop(),
            // BIT d, BIT a
            "0010m100" => self.bit(self.addr_mode_from_ctrl_mode(m)),

            // STY d, STY a, STY d,x
            "100mm100" if m != 0b11 => self.sty(self.addr_mode_from_ctrl_mode(m)),

            // LDY d, LDY a, LDY d,x, LDY a,x
            "101mm100" => self.ldy(self.addr_mode_from_ctrl_mode(m)),
            // CPY d, CPY a
            "1100m100" => self.cpy(self.addr_mode_from_ctrl_mode(m)),
            // CPX d, CPY a
            "1110m100" => self.cpx(self.addr_mode_from_ctrl_mode(m)),

            // +08
            // PHP
            "00001000" => self.php(),
            // PLP
            "00101000" => self.plp(),
            // PHA
            "01001000" => self.pha(),
            // PLA
            "01101000" => self.pla(),
            // DEY
            "10001000" => self.dey(),
            // TAY
            "10101000" => self.tay(),
            // INY
            "11001000" => self.iny(),
            // INX
            "11101000" => self.inx(),

            // +0C
            // NOP a
            "00001100" => self.nop(),
            // JMP a
            "01001100" => self.jmp(AddrMode::Absolute),
            // JMP (a)
            "01101100" => self.jmp(AddrMode::Indirect),

            // +10
            // BPL *+d
            "00010000" => self.bpl(AddrMode::Relative),
            // BMI *+d
            "00110000" => self.bmi(AddrMode::Relative),
            // BVC *+d
            "01010000" => self.bvc(AddrMode::Relative),
            // BVS *+d
            "01110000" => self.bvs(AddrMode::Relative),
            // BCC *+d
            "10010000" => self.bcc(AddrMode::Relative),
            // BCS *+d
            "10110000" => self.bcs(AddrMode::Relative),
            // BNE *+d
            "11010000" => self.bne(AddrMode::Relative),
            // BEQ *+d
            "11110000" => self.beq(AddrMode::Relative),

            // +14
            // NOP d,x
            "hhh10100" if h != 0b100 && h != 0b101 => self.nop(),

            // +18
            // CLC
            "00011000" => self.clc(),
            // SEC
            "00111000" => self.sec(),
            // CLI
            "01011000" => self.cli(),
            // SEI
            "01111000" => self.sei(),
            // TYA
            "10011000" => self.tya(),
            // CLV
            "10111000" => self.clv(),
            // CLD
            "11011000" => self.cld(),
            // SED
            "11111000" => self.sed(),

            // +1C
            // NOP a,x
            "hhh11100" if h != 0b100 && h != 0b101 => self.nop(),
            // SHY a,x
            "10011100" => self.shy(AddrMode::AbsoluteIndexedX),

            // ALU
            // ORA
            "000mmm01" => self.ora(self.addr_mode_from_alu_mode(m)),
            // AND
            "001mmm01" => self.and(self.addr_mode_from_alu_mode(m)),
            // EOR
            "010mmm01" => self.eor(self.addr_mode_from_alu_mode(m)),
            // ADC
            "011mmm01" => self.adc(self.addr_mode_from_alu_mode(m)),
            // STA
            "100mmm01" if m != 0b010 => self.sta(self.addr_mode_from_alu_mode(m)),
            // LDA
            "101mmm01" => self.lda(self.addr_mode_from_alu_mode(m)),
            // CMP
            "110mmm01" => self.cmp(self.addr_mode_from_alu_mode(m)),
            // SBC
            "111mmm01" => self.sbc(self.addr_mode_from_alu_mode(m)),

            // +09
            // NOP #i
            "10001001" => self.nop(),

            // RMW
            // +02
            // LDX #i
            "10100010" => self.ldx(AddrMode::Immediate),
            // STP
            "hhh00010" if h <= 0b011 => self.stp(),
            // NOP
            "hhh00010" if h == 0b100 || h == 0b110 || h == 0b111 => self.nop(),

            // ASL
            "000mm110" => self.asl(self.addr_mode_from_rmw_mode_x(m)),
            // ROL
            "001mm110" => self.rol(self.addr_mode_from_rmw_mode_x(m)),
            // LSR
            "010mm110" => self.lsr(self.addr_mode_from_rmw_mode_x(m)),
            // ROR
            "011mm110" => self.ror(self.addr_mode_from_rmw_mode_x(m)),

            // STX
            "100mm110" if m != 0b11 => self.stx(self.addr_mode_from_rmw_mode_y(m)),

            // LDX
            "101mm110" => self.ldx(self.addr_mode_from_rmw_mode_y(m)),
            // DEC
            "110mm110" => self.dec(self.addr_mode_from_rmw_mode_x(m)),
            // INC
            "111mm110" => self.inc(self.addr_mode_from_rmw_mode_x(m)),

            // +0A
            // ASL
            "00001010" => self.asl(AddrMode::Accumulator),
            // ROL
            "00101010" => self.rol(AddrMode::Accumulator),
            // LSR
            "01001010" => self.lsr(AddrMode::Accumulator),
            // ROR
            "01101010" => self.ror(AddrMode::Accumulator),
            // TXA
            "10001010" => self.txa(),
            // TAX
            "10101010" => self.tax(),
            // DEX
            "11001010" => self.dex(),
            // NOP
            "11101010" => self.nop(),

            // +12
            // STP
            "???10010" => self.stp(),

            // +1A
            // NOP
            "hhh11010" if h != 0b100 && h != 0b101 => self.nop(),
            // TXS
            "10011010" => self.txs(),
            // TSX
            "10111010" => self.tsx(),

            // +1E
            // SHX a,y
            "10011110" => self.shx(AddrMode::AbsoluteIndexedY),

            _ => {
                dbg!("unknown opecode {:#02X}", opecode);
                Ok(())
            }
        }
    }

    fn addr_mode_from_ctrl_mode(&self, mode: u8) -> AddrMode {
        match mode {
            0b00 => AddrMode::ZeroPage,
            0b01 => AddrMode::Absolute,
            0b10 => AddrMode::ZeroPageIndexedX,
            0b11 => AddrMode::AbsoluteIndexedX,
            _ => unimplemented!("invalid ctrl mode {:#02X}", mode),
        }
    }

    fn addr_mode_from_alu_mode(&self, mode: u8) -> AddrMode {
        match mode {
            0b000 => AddrMode::IndexedIndirectX,
            0b001 => AddrMode::ZeroPage,
            0b010 => AddrMode::Immediate,
            0b011 => AddrMode::Absolute,
            0b100 => AddrMode::IndirectIndexedY,
            0b101 => AddrMode::ZeroPageIndexedX,
            0b110 => AddrMode::AbsoluteIndexedY,
            0b111 => AddrMode::AbsoluteIndexedX,
            _ => unimplemented!("invalid alu mode {:#02X}", mode),
        }
    }

    fn addr_mode_from_rmw_mode_x(&self, mode: u8) -> AddrMode {
        match mode {
            0b00 => AddrMode::ZeroPage,
            0b01 => AddrMode::Absolute,
            0b10 => AddrMode::ZeroPageIndexedX,
            0b11 => AddrMode::AbsoluteIndexedX,
            _ => unimplemented!("invalid rmw mode x {:#02X}", mode),
        }
    }

    fn addr_mode_from_rmw_mode_y(&self, mode: u8) -> AddrMode {
        match mode {
            0b00 => AddrMode::ZeroPage,
            0b01 => AddrMode::Absolute,
            0b10 => AddrMode::ZeroPageIndexedY,
            0b11 => AddrMode::AbsoluteIndexedY,
            _ => unimplemented!("invalid rmw mode y {:#02X}", mode),
        }
    }

    fn push_8(&mut self, data: u8) -> Result<()> {
        self.s = self.s.wrapping_sub(1);
        self.bus.write(STACK_BASE + self.s as u16, data)?;

        Ok(())
    }

    fn pop_8(&mut self) -> Result<u8> {
        let result = self.bus.read(STACK_BASE + self.s as u16)?;
        self.s = self.s.wrapping_add(1);

        Ok(result)
    }

    fn push_16(&mut self, data: u16) -> Result<()> {
        self.s = self.s.wrapping_sub(2);
        self.bus.write_word(STACK_BASE + self.s as u16, data)?;

        Ok(())
    }

    fn pop_16(&mut self) -> Result<u16> {
        let result = self.bus.read_word(STACK_BASE + self.s as u16)?;
        self.s = self.s.wrapping_add(2);

        Ok(result)
    }

    fn nop(&mut self) -> Result<()> {
        trace!("{:?}: NOP", self);

        Ok(())
    }

    fn brk(&mut self) -> Result<()> {
        trace!("{:?}: BRK", self);
        // TODO 割り込み
        Ok(())
    }

    fn jsr(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;
        self.push_16(self.pc)?;
        self.pc = addr;

        trace!("{:?}: JSR {}", self, mode);

        Ok(())
    }

    fn rti(&mut self) -> Result<()> {
        self.p = P(self.pop_8()?);
        self.pc = self.pop_16()?;

        trace!("{:?}: RTI", self);

        Ok(())
    }

    fn rts(&mut self) -> Result<()> {
        self.pc = self.pop_16()?;

        trace!("{:?}: RTS", self);

        Ok(())
    }

    fn ldy(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;
        let result = self.bus.read(addr)?;

        self.y = result;
        self.set_zn_by(result);

        trace!("{:?}: LDY {}", self, mode);

        Ok(())
    }

    fn _cmp(&mut self, mode: AddrMode, left: u8) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;
        let right = self.bus.read(addr)?;
        let (result, c) = left.overflowing_sub(right);

        self.set_zn_by(result);
        self.p.set_c(c);

        Ok(())
    }

    fn cpy(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: CPY {}", self, mode);

        self._cmp(mode, self.y)
    }

    fn cpx(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: CPX {}", self, mode);

        self._cmp(mode, self.x)
    }

    fn bit(&mut self, mode: AddrMode) -> Result<()> {
        let left = self.a;
        let addr = self.read_operand_addr(mode)?;
        let right = self.bus.read(addr)?;
        let result = left & right;

        self.set_z_by(result);
        self.set_n_by(right);
        self.p.set_v(right >> 6 > 0);

        trace!("{:?}: BIT {}", self, mode);

        Ok(())
    }

    fn sty(&mut self, mode: AddrMode) -> Result<()> {
        let data = self.y;
        let addr = self.read_operand_addr(mode)?;

        self.bus.write(addr, data)?;

        trace!("{:?}: STY {}", self, mode);

        Ok(())
    }

    fn php(&mut self) -> Result<()> {
        self.push_8(self.p.0)?;

        trace!("{:?}: PHP", self);

        Ok(())
    }

    fn plp(&mut self) -> Result<()> {
        self.p = P(self.pop_8()?);

        trace!("{:?}: PLP", self);

        Ok(())
    }

    fn pha(&mut self) -> Result<()> {
        self.push_8(self.a)?;

        trace!("{:?}: PHA", self);

        Ok(())
    }

    fn pla(&mut self) -> Result<()> {
        self.a = self.pop_8()?;

        self.set_zn_by(self.a);

        trace!("{:?}: PLA", self);

        Ok(())
    }

    fn dey(&mut self) -> Result<()> {
        self.y = self.y.wrapping_sub(1);

        self.set_zn_by(self.y);

        trace!("{:?}: DEY", self);

        Ok(())
    }

    fn tay(&mut self) -> Result<()> {
        self.y = self.a;

        self.set_zn_by(self.y);

        trace!("{:?}: TAY", self);

        Ok(())
    }

    fn iny(&mut self) -> Result<()> {
        self.y = self.y.wrapping_add(1);

        self.set_zn_by(self.y);

        trace!("{:?}: INY", self);

        Ok(())
    }

    fn inx(&mut self) -> Result<()> {
        self.x = self.x.wrapping_add(1);

        self.set_zn_by(self.x);

        trace!("{:?}: INX", self);

        Ok(())
    }

    fn _jmp(&mut self, addr: u16) -> Result<()> {
        self.pc = addr;

        Ok(())
    }

    fn jmp(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: JMP {}", self, mode);

        let addr = self.read_operand_addr(mode)?;

        self._jmp(addr)
    }

    fn bpl(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;

        if !self.p.n() {
            self._jmp(addr)?;
        }

        trace!("{:?}: BPL {}", self, mode);

        Ok(())
    }

    fn bmi(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;

        if self.p.n() {
            self._jmp(addr)?;
        }

        trace!("{:?}: BMI {}", self, mode);

        Ok(())
    }

    fn bvc(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;

        if !self.p.v() {
            self._jmp(addr)?;
        }

        trace!("{:?}: BVC {}", self, mode);

        Ok(())
    }

    fn bvs(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;

        if self.p.v() {
            self._jmp(addr)?;
        }

        trace!("{:?}: BVS {}", self, mode);

        Ok(())
    }

    fn bcc(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;

        if !self.p.c() {
            self._jmp(addr)?;
        }

        trace!("{:?}: BCC {}", self, mode);

        Ok(())
    }

    fn bcs(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;

        if self.p.c() {
            self._jmp(addr)?;
        }

        trace!("{:?}: BCS {}", self, mode);

        Ok(())
    }

    fn bne(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;

        if !self.p.z() {
            self._jmp(addr)?;
        }

        trace!("{:?}: BNE {}", self, mode);

        Ok(())
    }

    fn beq(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;

        if self.p.z() {
            self._jmp(addr)?;
        }

        trace!("{:?}: BEQ {}", self, mode);

        Ok(())
    }

    fn clc(&mut self) -> Result<()> {
        self.p.set_c(false);

        trace!("{:?}: CLC", self);

        Ok(())
    }

    fn sec(&mut self) -> Result<()> {
        self.p.set_c(true);

        trace!("{:?}: SEC", self);

        Ok(())
    }

    fn cli(&mut self) -> Result<()> {
        self.p.set_i(false);

        trace!("{:?}: CLI", self);

        Ok(())
    }

    fn sei(&mut self) -> Result<()> {
        self.p.set_i(false);

        trace!("{:?}: SEI", self);

        Ok(())
    }

    fn clv(&mut self) -> Result<()> {
        self.p.set_v(false);

        trace!("{:?}: CLV", self);

        Ok(())
    }

    fn cld(&mut self) -> Result<()> {
        self.p.set_d(false);

        trace!("{:?}: CLD", self);

        Ok(())
    }

    fn sed(&mut self) -> Result<()> {
        self.p.set_d(true);

        trace!("{:?}: SED", self);

        Ok(())
    }

    fn tya(&mut self) -> Result<()> {
        self.a = self.y;

        self.set_zn_by(self.a);

        trace!("{:?}: TYA", self);

        Ok(())
    }

    fn shy(&mut self, _mode: AddrMode) -> Result<()> {
        unimplemented!("SHY");
    }

    fn _alu<Apply>(&mut self, mode: AddrMode, apply: Apply) -> Result<()>
    where
        Apply: Fn(u8, u8) -> u8,
    {
        let left = self.a;
        let addr = self.read_operand_addr(mode)?;
        let right = self.bus.read(addr)?;

        self.a = apply(left, right);

        self.set_zn_by(self.a);

        Ok(())
    }

    fn ora(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: ORA {}", self, mode);

        self._alu(mode, |left, right| left | right)
    }

    fn and(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: AND {}", self, mode);

        self._alu(mode, |left, right| left & right)
    }

    fn eor(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: EOR {}", self, mode);

        self._alu(mode, |left, right| left ^ right)
    }

    fn adc(&mut self, mode: AddrMode) -> Result<()> {
        let left = self.a;
        let addr = self.read_operand_addr(mode)?;
        let right = self.bus.read(addr)?;
        let c = self.p.c() as u8;

        let (result1, c1) = left.overflowing_add(right);
        let (result2, c2) = result1.overflowing_add(c);
        let v1 = self.is_overflow_positive(left, right);
        let v2 = self.is_overflow_positive(result1, c);

        self.a = result2;

        self.p.set_v(v1 | v2);
        self.p.set_c(c1 | c2);

        trace!("{:?}: ADC {}", self, mode);

        Ok(())
    }

    fn sta(&mut self, mode: AddrMode) -> Result<()> {
        let data = self.a;
        let addr = self.read_operand_addr(mode)?;

        self.bus.write(addr, data)?;

        trace!("{:?}: STA {}", self, mode);

        Ok(())
    }

    fn lda(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;
        let data = self.bus.read(addr)?;

        self.a = data;

        self.set_zn_by(self.a);

        trace!("{:?}: LDA {}", self, mode);

        Ok(())
    }

    fn cmp(&mut self, mode: AddrMode) -> Result<()> {
        let left = self.a;
        let addr = self.read_operand_addr(mode)?;
        let right = self.bus.read(addr)?;

        let (result, c) = left.overflowing_sub(right);

        self.set_zn_by(result);
        self.p.set_c(c);

        trace!("{:?}: CMP {}", self, mode);

        Ok(())
    }

    fn sbc(&mut self, mode: AddrMode) -> Result<()> {
        let left = self.a;
        let addr = self.read_operand_addr(mode)?;
        let right = self.bus.read(addr)?;

        let c = !self.p.c() as u8;

        let (result1, c1) = left.overflowing_sub(right);
        let (result2, c2) = result1.overflowing_sub(c);
        let v1 = self.is_overflow_negative(left, right);
        let v2 = self.is_overflow_negative(result1, c);

        self.a = result2;

        self.p.set_v(v1 | v2);
        self.p.set_c(c1 | c2);

        trace!("{:?}: SBC {}", self, mode);

        Ok(())
    }

    fn stp(&mut self) -> Result<()> {
        unimplemented!("STP");
    }

    fn _shift<Apply>(&mut self, mode: AddrMode, apply: Apply) -> Result<()>
    where
        Apply: Fn(u8) -> u8,
    {
        let addr = self.read_operand_addr(mode)?;

        let data = if mode != AddrMode::Accumulator {
            self.a
        } else {
            self.bus.read(addr)?
        };

        let result = apply(data);

        if mode == AddrMode::Accumulator {
            self.a = result;
        } else {
            self.bus.write(addr, result)?;
        };

        self.set_zn_by(result);
        self.p.set_c((data >> 7) > 0);

        Ok(())
    }

    fn asl(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: ASL {}", self, mode);

        self._shift(mode, |data| data << 1)
    }

    fn rol(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: ROL {}", self, mode);

        self._shift(mode, |data| data.rotate_left(1))
    }

    fn lsr(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: LSR {}", self, mode);

        self._shift(mode, |data| data >> 1)
    }

    fn ror(&mut self, mode: AddrMode) -> Result<()> {
        trace!("{:?}: ROR {}", self, mode);

        self._shift(mode, |data| data.rotate_right(1))
    }

    fn stx(&mut self, mode: AddrMode) -> Result<()> {
        let data = self.x;
        let addr = self.read_operand_addr(mode)?;

        self.bus.write(addr, data)?;

        trace!("{:?}: STX {}", self, mode);

        Ok(())
    }

    fn ldx(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;
        let result = self.bus.read(addr)?;

        self.x = result;
        self.set_zn_by(result);

        trace!("{:?}: LDX {}", self, mode);

        Ok(())
    }

    fn dec(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;
        let left = self.bus.read(addr)?;

        let data = left.wrapping_sub(1);

        self.bus.write(addr, data)?;

        self.set_zn_by(data);

        trace!("{:?}: DEC {}", self, mode);

        Ok(())
    }

    fn inc(&mut self, mode: AddrMode) -> Result<()> {
        let addr = self.read_operand_addr(mode)?;
        let left = self.bus.read(addr)?;

        let data = left.wrapping_add(1);

        self.bus.write(addr, data)?;

        self.set_zn_by(data);

        trace!("{:?}: INC {}", self, mode);

        Ok(())
    }

    fn txa(&mut self) -> Result<()> {
        self.a = self.x;

        self.set_zn_by(self.a);

        trace!("{:?}: TXA", self);

        Ok(())
    }

    fn tax(&mut self) -> Result<()> {
        self.x = self.a;

        self.set_zn_by(self.x);

        trace!("{:?}: TAX", self);

        Ok(())
    }

    fn dex(&mut self) -> Result<()> {
        self.x = self.x.wrapping_sub(1);

        self.set_zn_by(self.x);

        trace!("{:?}: DEX", self);

        Ok(())
    }

    fn txs(&mut self) -> Result<()> {
        self.s = self.x;

        self.set_zn_by(self.s);

        trace!("{:?}: TXS", self);

        Ok(())
    }

    fn tsx(&mut self) -> Result<()> {
        self.x = self.s;

        self.set_zn_by(self.x);

        trace!("{:?}: TSX", self);

        Ok(())
    }

    fn shx(&mut self, _mode: AddrMode) -> Result<()> {
        unimplemented!("SHX");
    }
}
