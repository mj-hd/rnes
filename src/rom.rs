use anyhow::{bail, Context, Result};
use bitfield::bitfield;
use core::fmt;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{
  fmt::{Debug, Formatter},
  fs::File,
  io::BufReader,
  io::Read,
};

bitfield! {
  pub struct Flag1(u8);
  impl Debug;
  u16, mapper_type_low, _: 7, 4;
  four_screen_mode, _: 3;
  has_trainer, _: 2;
  has_battery, _: 1;
  mirroring, _: 0;
}

bitfield! {
  pub struct Flag2(u8);
  impl Debug;
  u16, mapper_type_middle, _: 7, 4;
  u8, into ConsoleType, console_type, _: 1, 0;
}

bitfield! {
  struct MapperSubmapper(u8);
  impl Debug;
  u16, mapper_type_high, _: 7, 4;
  u8, into SubmapperType, submapper_type, _: 3, 0;
}

bitfield! {
  struct PrgChrRomNum(u8);
  impl Debug;
  prg_high, _: 7, 4;
  chr_high, _: 3, 0;
}

bitfield! {
  struct PrgRamEepromSize(u8);
  impl Debug;
  nvram_shift_count, _: 7, 4;
  ram_shift_count, _: 3, 0;
}

bitfield! {
  struct CharRamSize(u8);
  impl Debug;
  nvram_shift_count, _: 7, 4;
  ram_shift_count, _: 3, 0;
}

bitfield! {
  struct CpuPpuTiming(u8);
  impl Debug;
  u8, into CpuPpuTimingMode, mode, _: 1, 0;
}

bitfield! {
  pub struct VsSystemType(u8);
  impl Debug;
  hardware_type, _: 7, 4;
  ppu_type, _: 3, 0;
}

bitfield! {
  pub struct ExtendedConsoleType(u8);
  impl Debug;
  console_type, _: 3, 0;
}

bitfield! {
  struct DefaultExpansionDevice(u8);
  u8, into ExpansionDeviceType, device_type, _: 5, 0;
}

#[derive(FromPrimitive, Debug)]
pub enum ConsoleType {
  NesFc = 0,
  VsSystem = 1,
  Playchoice10 = 2,
  Extended = 3,
  Unknown,
}

impl From<u8> for ConsoleType {
  fn from(v: u8) -> Self {
    FromPrimitive::from_u8(v).unwrap_or(ConsoleType::Unknown)
  }
}

#[derive(FromPrimitive, Debug)]
pub enum MapperType {
  Mmc0 = 0,
  Mmc1 = 1,
  Unknown,
}

#[derive(FromPrimitive, Debug)]
pub enum SubmapperType {
  Unknown,
}

impl From<u8> for SubmapperType {
  fn from(v: u8) -> Self {
    FromPrimitive::from_u8(v).unwrap_or(SubmapperType::Unknown)
  }
}

#[derive(FromPrimitive, Debug)]
pub enum CpuPpuTimingMode {
  Rp2C02 = 0,
  Rp2C07 = 1,
  MultipleRegion = 2,
  Umc6527p = 3,
  Unknown,
}

impl From<u8> for CpuPpuTimingMode {
  fn from(v: u8) -> Self {
    FromPrimitive::from_u8(v).unwrap_or(CpuPpuTimingMode::Unknown)
  }
}

#[derive(FromPrimitive, Debug)]
pub enum ExpansionDeviceType {
  Unspecified = 0x00,
  // TODO 必要になったら実装
}

impl From<u8> for ExpansionDeviceType {
  fn from(v: u8) -> Self {
    FromPrimitive::from_u8(v).unwrap_or(ExpansionDeviceType::Unspecified)
  }
}

pub struct Rom {
  pub prg_size: usize,
  pub chr_size: usize,
  pub flag1: Flag1,
  pub flag2: Flag2,
  pub mapper: MapperType,
  pub submapper: SubmapperType,
  pub prg_ram_size: usize,
  pub prg_nvram_size: usize,
  pub chr_ram_size: usize,
  pub chr_nvram_size: usize,
  pub timing_mode: CpuPpuTimingMode,
  pub vs_system_type: VsSystemType,
  pub extended_console_type: ExtendedConsoleType,
  pub expansion_device_type: ExpansionDeviceType,

  data: Vec<u8>,
}

impl Default for Rom {
  fn default() -> Self {
    Self {
      prg_size: 0,
      chr_size: 0,
      flag1: Flag1(0),
      flag2: Flag2(0),
      mapper: MapperType::Unknown,
      submapper: SubmapperType::Unknown,
      prg_ram_size: 0,
      prg_nvram_size: 0,
      chr_ram_size: 0,
      chr_nvram_size: 0,
      timing_mode: CpuPpuTimingMode::Unknown,
      vs_system_type: VsSystemType(0),
      extended_console_type: ExtendedConsoleType(0),
      expansion_device_type: ExpansionDeviceType::Unspecified,

      data: Vec::new(),
    }
  }
}

impl Debug for Rom {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.debug_struct("Rom")
      .field("prg_size", &self.prg_size)
      .field("chr_size", &self.chr_size)
      .field("flag1", &self.flag1)
      .field("flag2", &self.flag2)
      .field("mapper", &self.mapper)
      .field("submapper", &self.submapper)
      .field("prg_ram_size", &self.prg_ram_size)
      .field("prg_nvram_size", &self.prg_nvram_size)
      .field("chr_ram_size", &self.chr_ram_size)
      .field("chr_nvram_size", &self.chr_nvram_size)
      .field("timing_mode", &self.timing_mode)
      .field("vs_system_type", &self.vs_system_type)
      .field("extended_console_type", &self.extended_console_type)
      .field("expansion_device_type", &self.expansion_device_type)
      .field("data", &self.data.len())
      .field("prg", &self.prg().len())
      .field("chr", &self.chr().len())
      .field("trainer", &self.trainer())
      .field("misc", &self.misc().len())
      .finish()
  }
}

impl Rom {
  pub fn new(reader: &mut BufReader<File>) -> Result<Rom> {
    let mut rom = Rom::default();

    reader.read_to_end(&mut rom.data)?;

    if rom.data[0x0000..0x0004] != b"NES\x1A"[..] {
      bail!("missing NES 2.0 header");
    }

    let mut prg_num = rom.data[0x0004] as usize;
    let mut chr_num = rom.data[0x0005] as usize;

    rom.flag1 = Flag1(rom.data[0x0006]);
    rom.flag2 = Flag2(rom.data[0x0007]);

    let mapper_submapper = MapperSubmapper(rom.data[0x0008]);

    rom.submapper = mapper_submapper.submapper_type();

    let mut mapper = rom.flag1.mapper_type_low();
    mapper += rom.flag2.mapper_type_middle() << 4;
    mapper += mapper_submapper.mapper_type_high() << 8;
    rom.mapper = FromPrimitive::from_u16(mapper).context("unknown mapper type")?;

    let prg_chr_rom_num = PrgChrRomNum(rom.data[0x0009]);

    prg_num += (prg_chr_rom_num.prg_high() as usize) << 8;
    chr_num += (prg_chr_rom_num.chr_high() as usize) << 8;

    rom.prg_size = prg_num * 16 * 1024;
    rom.chr_size = chr_num * 8 * 1024;

    let prg_ram_eeprom_size = PrgRamEepromSize(rom.data[0x000A]);

    if prg_ram_eeprom_size.ram_shift_count() > 0 {
      rom.prg_ram_size = 64 << prg_ram_eeprom_size.ram_shift_count();
    }

    if prg_ram_eeprom_size.nvram_shift_count() > 0 {
      rom.prg_nvram_size = 64 << prg_ram_eeprom_size.nvram_shift_count();
    }

    let chr_ram_size = CharRamSize(rom.data[0x000B]);

    if chr_ram_size.ram_shift_count() > 0 {
      rom.chr_ram_size = 64 << chr_ram_size.ram_shift_count();
    }

    if chr_ram_size.nvram_shift_count() > 0 {
      rom.chr_nvram_size = 64 << chr_ram_size.nvram_shift_count();
    }

    let cpu_ppu_timing = CpuPpuTiming(rom.data[0x000C]);

    rom.timing_mode = cpu_ppu_timing.mode();

    match rom.flag2.console_type() {
      ConsoleType::VsSystem => {
        rom.vs_system_type = VsSystemType(rom.data[0x000D]);
      }
      ConsoleType::Extended => {
        rom.extended_console_type = ExtendedConsoleType(rom.data[0x000D]);
      }
      _ => {}
    };

    let default_expansion_device = DefaultExpansionDevice(rom.data[0x000F]);

    rom.expansion_device_type = default_expansion_device.device_type();

    Ok(rom)
  }

  fn trainer_offset(&self) -> usize {
    0x0010
  }

  pub fn trainer(&self) -> Option<&[u8]> {
    let offset = self.trainer_offset();

    if self.flag1.has_trainer() {
      Some(&self.data[offset..(offset + 0x0200)])
    } else {
      None
    }
  }

  fn prg_offset(&self) -> usize {
    self.trainer_offset() + if self.flag1.has_trainer() { 0x0200 } else { 0 }
  }

  pub fn prg(&self) -> &[u8] {
    let offset = self.prg_offset();

    &self.data[offset..(offset + self.prg_size)]
  }

  fn chr_offset(&self) -> usize {
    self.prg_offset() + self.prg_size
  }

  pub fn chr(&self) -> &[u8] {
    let offset = self.chr_offset();

    &self.data[offset..(offset + self.chr_size)]
  }

  fn misc_offset(&self) -> usize {
    self.chr_offset() + self.chr_size
  }

  pub fn misc(&self) -> &[u8] {
    let offset = self.misc_offset();

    &self.data[offset..]
  }
}
