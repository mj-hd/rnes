use anyhow::{bail, Result};

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
