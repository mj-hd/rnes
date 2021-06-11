use anyhow::{bail, Result};
use bitfield::bitfield;
use bitmatch::bitmatch;
use log::debug;

use crate::rom::{MapperType, Rom};

pub trait Mmc {
    fn read_cpu(&self, addr: u16) -> Result<u8>;
    fn write_cpu(&mut self, addr: u16, data: u8) -> Result<()>;
    fn read_ppu(&self, addr: u16) -> Result<u8>;
    fn write_ppu(&mut self, addr: u16, data: u8) -> Result<()>;
}

pub fn new_mmc(rom: Rom) -> Result<Box<dyn Mmc>> {
    match rom.mapper {
        MapperType::Mmc0 => Ok(Box::new(Mmc0::new(rom))),
        MapperType::Mmc1 => Ok(Box::new(Mmc1::new(rom))),
        _ => bail!("unknown mapper {:?}", rom.mapper),
    }
}

pub struct Mmc0 {
    rom: Rom,

    prg_ram: [u8; 0x2000],
}

impl Mmc0 {
    pub fn new(rom: Rom) -> Self {
        Self {
            rom,
            prg_ram: [0; 0x2000],
        }
    }
}

impl Mmc for Mmc0 {
    fn read_cpu(&self, addr: u16) -> Result<u8> {
        let addr = if self.rom.prg_size <= 0x4000 && addr >= 0xC000 {
            addr - 0x4000
        } else {
            addr
        };

        match addr {
            0x6000..=0x7FFF => Ok(self.prg_ram[(addr - 0x6000) as usize]),
            0x8000..=0xFFFF => Ok(self.rom.prg()[(addr - 0x8000) as usize]),
            _ => Ok(0),
        }
    }

    fn write_cpu(&mut self, addr: u16, data: u8) -> Result<()> {
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram[(addr - 0x6000) as usize] = data;

                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn read_ppu(&self, addr: u16) -> Result<u8> {
        match addr {
            0x0000..=0x1FFF => Ok(self.rom.chr()[addr as usize]),
            _ => Ok(0),
        }
    }

    fn write_ppu(&mut self, addr: u16, data: u8) -> Result<()> {
        Ok(())
    }
}

bitfield! {
    struct Mmc1Control(u8);
    chr_rom_bank, _: 4;
    prg_rom_bank, _: 3, 2;
    mirror, _: 1, 0;
}

bitfield! {
    struct Mmc1PrgBank(u8);
    prg_ram_enable, _: 4;
    prg_rom_bank, _: 3, 0;
}

pub struct Mmc1 {
    rom: Rom,

    prg_ram: [u8; 0x2000],

    latch: u8,
    counter: usize,

    control: Mmc1Control,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: Mmc1PrgBank,
}

impl Mmc1 {
    pub fn new(rom: Rom) -> Self {
        Self {
            rom,

            prg_ram: [0; 0x2000],

            latch: 0,
            counter: 0,

            control: Mmc1Control(0),
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: Mmc1PrgBank(0),
        }
    }

    fn reset_load(&mut self) {
        self.latch = 0;
        self.counter = 0;
    }

    fn copy_register(&mut self, addr: u16) {
        match addr {
            0x0000 => {
                self.control = Mmc1Control(self.latch);
            }
            0x2000 => {
                self.chr_bank_0 = self.latch;
            }
            0x4000 => {
                self.chr_bank_1 = self.latch;
            }
            0x6000 => {
                self.prg_bank = Mmc1PrgBank(self.latch);
            }
            _ => {}
        }
    }

    #[bitmatch]
    fn write_load(&mut self, addr: u16, data: u8) {
        #[bitmatch]
        let "r??????d" = data;

        if r > 0 {
            self.reset_load();

            return;
        }

        self.latch <<= 1;

        if d > 0 {
            self.latch |= 1;
        }

        self.counter += 1;

        if self.counter == 5 {
            self.copy_register(addr & 0b01100000);
            self.reset_load();
        }
    }

    fn read_prg_bank_32kb(&self, addr: u16) -> u8 {
        let bank = (self.prg_bank.prg_rom_bank() & 0b1110) as u16 >> 1;
        let offset = addr - 0x8000;
        self.rom.prg()[(bank * 0x8000 + offset) as usize]
    }

    fn read_prg_bank_first_fixed(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xBFFF => self.rom.prg()[(addr - 0x8000) as usize],
            0xC000..=0xFFFF => {
                let bank = self.prg_bank.prg_rom_bank() as u16;
                let offset = (addr - 0xC000) as u16;
                self.rom.prg()[(bank * 0x4000 + offset) as usize]
            }
            _ => {
                debug!("index out of range");
                0
            }
        }
    }

    fn read_prg_bank_last_fixed(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xBFFF => {
                let bank = self.prg_bank.prg_rom_bank() as u16;
                let offset = (addr - 0x8000) as u16;
                self.rom.prg()[(bank * 0x4000 + offset) as usize]
            }
            0xC000..=0xFFFF => {
                let neg_offset = 0xFFFF - addr;
                self.rom.prg()[self.rom.prg_size - neg_offset as usize]
            }
            _ => {
                debug!("index out of range");
                0
            }
        }
    }

    fn read_prg_bank(&self, addr: u16) -> u8 {
        match self.control.prg_rom_bank() {
            0 | 1 => self.read_prg_bank_32kb(addr),
            2 => self.read_prg_bank_first_fixed(addr),
            3 => self.read_prg_bank_last_fixed(addr),
            _ => {
                debug!("unknown prg rom bank control");
                0
            }
        }
    }

    fn read_chr_bank_8kb(&self, addr: u16) -> u8 {
        let bank = (self.chr_bank_0 & 0b1110) as u16 >> 1;
        let offset = addr;
        self.rom.chr()[(bank * 0x2000 + offset) as usize]
    }

    fn read_chr_bank_4kb(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x0FFF => {
                let bank = self.chr_bank_0 as u16;
                let offset = addr;
                self.rom.chr()[(bank * 0x1000 + offset) as usize]
            }
            0x1000..=0x1FFF => {
                let bank = self.chr_bank_1 as u16;
                let offset = addr - 0x1000;
                self.rom.chr()[(bank * 0x1000 + offset) as usize]
            }
            _ => {
                debug!("index out of range");
                0
            }
        }
    }

    fn read_chr_bank(&self, addr: u16) -> u8 {
        match self.control.chr_rom_bank() {
            false => self.read_chr_bank_8kb(addr),
            true => self.read_chr_bank_4kb(addr),
        }
    }
}

impl Mmc for Mmc1 {
    fn read_cpu(&self, addr: u16) -> Result<u8> {
        match addr {
            0x6000..=0x7FFF => Ok(self.prg_ram[(addr - 0x6000) as usize]),
            0x8000..=0xFFFF => Ok(self.read_prg_bank(addr)),
            _ => Ok(0),
        }
    }

    fn write_cpu(&mut self, addr: u16, data: u8) -> Result<()> {
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram[(addr - 0x6000) as usize] = data;

                Ok(())
            }
            0x8000..=0xFFFF => {
                self.write_load(addr, data);

                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn read_ppu(&self, addr: u16) -> Result<u8> {
        Ok(self.read_chr_bank(addr))
    }

    fn write_ppu(&mut self, addr: u16, data: u8) -> Result<()> {
        Ok(())
    }
}
