use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use crate::{apu::Apu, mmc::Mmc, ppu::Ppu};

pub struct CpuBus {
  mmc: Rc<RefCell<Box<dyn Mmc>>>,
  ppu: Rc<RefCell<Ppu>>,
  apu: Rc<RefCell<Apu>>,

  pub wram: [u8; 0x0800],
}

impl CpuBus {
  pub fn new(mmc: Rc<RefCell<Box<dyn Mmc>>>, ppu: Rc<RefCell<Ppu>>, apu: Rc<RefCell<Apu>>) -> Self {
    Self {
      mmc,
      ppu,
      apu,
      wram: [0; 0x0800],
    }
  }

  pub fn read_word(&self, addr: u16) -> Result<u16> {
    let low = self.read(addr)?;
    let high = self.read(addr + 1)?;

    Ok(((high as u16) << 8) | (low as u16))
  }

  pub fn read(&self, addr: u16) -> Result<u8> {
    let addr = match addr {
      0x0800..=0x1FFF => (addr - 0x0800) % 0x0800,
      0x2008..=0x3FFF => 0x2000 + (addr - 0x2008) % 0x0008,
      _ => addr,
    };

    match addr {
      0x0000..=0x07FF => Ok(self.wram[addr as usize]),
      0x2000 => self.ppu.borrow().read_control1(),
      0x2001 => self.ppu.borrow().read_control2(),
      0x2002 => self.ppu.borrow().read_status(),
      0x2003 => self.ppu.borrow().read_oam_addr(),
      0x2004 => self.ppu.borrow().read_oam_data(),
      0x2005 => self.ppu.borrow().read_scroll(),
      0x2006 => self.ppu.borrow().read_vram_addr(),
      0x2007 => self.ppu.borrow().read_vram_data(),
      0x4000 => self.apu.borrow().read_square_ch1_control1(),
      0x4001 => self.apu.borrow().read_square_ch1_control2(),
      0x4002 => self.apu.borrow().read_square_ch1_freq1(),
      0x4003 => self.apu.borrow().read_square_ch1_freq2(),
      0x4004 => self.apu.borrow().read_square_ch2_control1(),
      0x4005 => self.apu.borrow().read_square_ch2_control2(),
      0x4006 => self.apu.borrow().read_square_ch2_freq1(),
      0x4007 => self.apu.borrow().read_square_ch2_freq2(),
      0x4008 => self.apu.borrow().read_sign_control(),
      0x400A => self.apu.borrow().read_sign_freq1(),
      0x400B => self.apu.borrow().read_sign_freq2(),
      0x400C => self.apu.borrow().read_noise_control(),
      0x400E => self.apu.borrow().read_noise_rand(),
      0x400F => self.apu.borrow().read_noise_duration(),
      0x4010 => self.apu.borrow().read_dpcm_control1(),
      0x4011 => self.apu.borrow().read_dpcm_control2(),
      0x4012 => self.apu.borrow().read_dpcm_control3(),
      0x4013 => self.apu.borrow().read_dpcm_control4(),
      0x4014 => self.ppu.borrow().read_oam_dma(),
      0x4015 => self.apu.borrow().read_voice_control(),
      addr => self.mmc.borrow().read_cpu(addr),
    }
  }

  pub fn write_word(&mut self, addr: u16, data: u16) -> Result<()> {
    let low = (data & 0x00FF) as u8;
    let high = (data >> 8) as u8;

    self.write(addr, low)?;
    self.write(addr + 1, high)?;

    Ok(())
  }

  pub fn write(&mut self, addr: u16, data: u8) -> Result<()> {
    let addr = match addr {
      0x0800..=0x1FFF => (addr - 0x0800) % 0x0800,
      0x2008..=0x3FFF => 0x2000 + (addr - 0x2008) % 0x0008,
      _ => addr,
    };

    match addr {
      0x0000..=0x07FF => {
        self.wram[addr as usize] = data;
        Ok(())
      }
      0x2000 => self.ppu.borrow_mut().write_control1(data),
      0x2001 => self.ppu.borrow_mut().write_control2(data),
      0x2002 => self.ppu.borrow_mut().write_status(data),
      0x2003 => self.ppu.borrow_mut().write_oam_addr(data),
      0x2004 => self.ppu.borrow_mut().write_oam_data(data),
      0x2005 => self.ppu.borrow_mut().write_scroll(data),
      0x2006 => self.ppu.borrow_mut().write_vram_addr(data),
      0x2007 => self.ppu.borrow_mut().write_vram_data(data),
      0x4000 => self.apu.borrow_mut().write_square_ch1_control1(data),
      0x4001 => self.apu.borrow_mut().write_square_ch1_control2(data),
      0x4002 => self.apu.borrow_mut().write_square_ch1_freq1(data),
      0x4003 => self.apu.borrow_mut().write_square_ch1_freq2(data),
      0x4004 => self.apu.borrow_mut().write_square_ch2_control1(data),
      0x4005 => self.apu.borrow_mut().write_square_ch2_control2(data),
      0x4006 => self.apu.borrow_mut().write_square_ch2_freq1(data),
      0x4007 => self.apu.borrow_mut().write_square_ch2_freq2(data),
      0x4008 => self.apu.borrow_mut().write_sign_control(data),
      0x400A => self.apu.borrow_mut().write_sign_freq1(data),
      0x400B => self.apu.borrow_mut().write_sign_freq2(data),
      0x400C => self.apu.borrow_mut().write_noise_control(data),
      0x400E => self.apu.borrow_mut().write_noise_rand(data),
      0x400F => self.apu.borrow_mut().write_noise_duration(data),
      0x4010 => self.apu.borrow_mut().write_dpcm_control1(data),
      0x4011 => self.apu.borrow_mut().write_dpcm_control2(data),
      0x4012 => self.apu.borrow_mut().write_dpcm_control3(data),
      0x4013 => self.apu.borrow_mut().write_dpcm_control4(data),
      0x4014 => self.ppu.borrow_mut().write_oam_dma(data),
      0x4015 => self.apu.borrow_mut().write_voice_control(data),
      0x4020..=0xFFFF => self.mmc.borrow_mut().write_cpu(addr, data),
      _ => Ok(()),
    }
  }
}

pub struct PpuBus {
  mmc: Rc<RefCell<Box<dyn Mmc>>>,
  pub vram: [u8; 0x0800],
  pub palette: [u8; 0x0020],
}

impl PpuBus {
  pub fn new(mmc: Rc<RefCell<Box<dyn Mmc>>>) -> Self {
    Self {
      mmc,
      vram: [0; 0x0800],
      palette: [0; 0x0020],
    }
  }

  pub fn read_word(&self, addr: u16) -> Result<u16> {
    let low = self.read(addr)?;
    let high = self.read(addr + 1)?;

    Ok(((high as u16) << 8) | (low as u16))
  }

  pub fn read(&self, addr: u16) -> Result<u8> {
    let addr = match addr {
      0x2800..=0x3EFF => 0x2000 + (addr - 0x2800) % 0x0800,
      0x3F20..=0x3FFF => 0x3F00 + addr - 0x3F20,
      0x4000..=0xFFFF => addr - 0x4000,
      _ => addr,
    };

    match addr {
      0x0000..=0x1FFF => self.mmc.borrow().read_ppu(addr),
      0x2000..=0x27FF => Ok(self.vram[(addr - 0x2000) as usize]),
      0x3F00..=0x3F1F => Ok(self.palette[(addr - 0x3F00) as usize]),
      _ => Ok(0),
    }
  }

  pub fn write_word(&mut self, addr: u16, data: u16) -> Result<()> {
    let low = (data & 0x00FF) as u8;
    let high = (data >> 8) as u8;

    self.write(addr, low)?;
    self.write(addr + 1, high)?;

    Ok(())
  }

  pub fn write(&mut self, addr: u16, data: u8) -> Result<()> {
    let addr = match addr {
      0x2800..=0x3EFF => 0x2000 + (addr - 0x2800) % 0x0800,
      0x3F20..=0x3FFF => 0x3F00 + addr - 0x3F20,
      0x4000..=0xFFFF => addr - 0x4000,
      _ => addr,
    };

    match addr {
      0x0000..=0x1FFF => self.mmc.borrow_mut().write_ppu(addr, data),
      0x2000..=0x27FF => {
        self.vram[(addr - 0x2000) as usize] = data;
        Ok(())
      }
      0x3F00..=0x3F1F => {
        self.palette[(addr - 0x3F00) as usize] = data;
        Ok(())
      }
      _ => Ok(()),
    }
  }
}
