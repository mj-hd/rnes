use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
};

use anyhow::{Context, Result};
use log::debug;

use crate::{apu::Apu, joypad::Joypad, mmc::Mmc, ppu::Ppu};

pub enum CpuBusEvent {
    RequestDma(u16, u8),
}

pub struct CpuBus {
    mmc: Rc<RefCell<Box<dyn Mmc>>>,
    ppu: Rc<RefCell<Ppu>>,
    apu: Rc<RefCell<Apu>>,
    joypad1: Rc<RefCell<Joypad>>,
    joypad2: Rc<RefCell<Joypad>>,

    event: Receiver<CpuBusEvent>,
    ppu_bus_sender: Sender<PpuBusEvent>,

    pub cycles: u8,
    pub stalls: u16,
    pub wram: [u8; 0x0800],
}

impl CpuBus {
    pub fn new(
        mmc: Rc<RefCell<Box<dyn Mmc>>>,
        ppu: Rc<RefCell<Ppu>>,
        apu: Rc<RefCell<Apu>>,
        joypad1: Rc<RefCell<Joypad>>,
        joypad2: Rc<RefCell<Joypad>>,
        event: Receiver<CpuBusEvent>,
        ppu_bus_sender: Sender<PpuBusEvent>,
    ) -> Self {
        Self {
            mmc,
            ppu,
            apu,
            joypad1,
            joypad2,
            ppu_bus_sender,
            event,
            cycles: 0,
            stalls: 0,
            wram: [0xFF; 0x0800],
        }
    }

    pub fn tick(&mut self) -> Result<()> {
        match self.event.try_recv() {
            Ok(event) => match event {
                CpuBusEvent::RequestDma(addr, oam_addr) => {
                    debug!("RECEIVED REQUEST DMA: {:#04X}", oam_addr);

                    let mut result = Vec::with_capacity(0x0100);

                    for i in addr..(addr + 0x0100) {
                        result.push(self.read(i)?);
                    }

                    self.ppu_bus_sender
                        .send(PpuBusEvent::Dma(result, oam_addr))
                        .context("failed to send ppu event")?;

                    self.stalls += 513 + if self.cycles % 2 == 0 { 0 } else { 1 };

                    Ok(())
                }
            },
            _ => Ok(()),
        }
    }

    pub fn nmi(&self) -> bool {
        let mut ppu = self.ppu.borrow_mut();

        if ppu.nmi {
            ppu.nmi = false;

            return true;
        }

        false
    }

    pub fn read_word(&self, addr: u16) -> Result<u16> {
        let low = self.read(addr)?;
        let high = self.read(addr.wrapping_add(1))?;

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
            0x2000 => self.ppu.borrow().read_ctrl(),
            0x2001 => self.ppu.borrow().read_mask(),
            0x2002 => self.ppu.borrow_mut().read_status(),
            0x2004 => self.ppu.borrow().read_oam_data(),
            0x2007 => self.ppu.borrow_mut().read_vram_data(),
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
            0x4016 => self.joypad1.borrow_mut().read(),
            0x4017 => self.joypad2.borrow_mut().read(),
            addr => self.mmc.borrow().read_cpu(addr),
        }
    }

    pub fn write_word(&mut self, addr: u16, data: u16) -> Result<()> {
        let low = (data & 0x00FF) as u8;
        let high = (data >> 8) as u8;

        self.write(addr, low)?;
        self.write(addr.wrapping_add(1), high)?;

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
            0x2000 => self.ppu.borrow_mut().write_ctrl(data),
            0x2001 => self.ppu.borrow_mut().write_mask(data),
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
            0x4016 => self.joypad1.borrow_mut().write(data),
            0x4017 => self.joypad2.borrow_mut().write(data),
            0x4020..=0xFFFF => self.mmc.borrow_mut().write_cpu(addr, data),
            _ => Ok(()),
        }
    }
}

pub enum PpuBusEvent {
    Dma(Vec<u8>, u8),
}

pub struct PpuBus {
    mmc: Rc<RefCell<Box<dyn Mmc>>>,
    event: Receiver<PpuBusEvent>,
    cpu_bus_sender: Sender<CpuBusEvent>,
    pub vram: [u8; 0x0800],
    pub palette: [u8; 0x0020],
    pub oam: [u8; 0x0100],
}

impl PpuBus {
    pub fn new(
        mmc: Rc<RefCell<Box<dyn Mmc>>>,
        event: Receiver<PpuBusEvent>,
        cpu_bus_sender: Sender<CpuBusEvent>,
    ) -> Self {
        Self {
            mmc,
            event,
            cpu_bus_sender,
            vram: [0xFF; 0x0800],
            palette: [0; 0x0020],
            oam: [0; 0x0100],
        }
    }

    pub fn tick(&mut self) -> Result<()> {
        match self.event.try_recv() {
            Ok(event) => match event {
                PpuBusEvent::Dma(data, oam_addr) => {
                    debug!("RECEIVED DMA: {:#04X}", oam_addr);

                    for i in 0..data.len() {
                        let addr = i + oam_addr as usize;
                        self.oam[addr] = data[i];
                    }
                }
            },
            _ => {}
        }

        Ok(())
    }

    pub fn request_dma(&mut self, cpu_addr: u16, oam_addr: u8) -> Result<()> {
        debug!("SEND REQUEST DMA: {:#04X}", oam_addr);

        self.cpu_bus_sender
            .send(CpuBusEvent::RequestDma(cpu_addr, oam_addr))
            .context("failed to send cpu event")
    }

    pub fn read_word(&self, addr: u16) -> Result<u16> {
        let low = self.read(addr)?;
        let high = self.read(addr + 1)?;

        Ok(((high as u16) << 8) | (low as u16))
    }

    pub fn read(&self, addr: u16) -> Result<u8> {
        let addr = match addr {
            0x2800..=0x3EFF => 0x2000 + (addr - 0x2800) % 0x0800,
            0x3F10..=0x3F1F if addr % 4 == 0 => addr - 0x0010,
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
            0x3F10..=0x3F1F if addr % 4 == 0 => addr - 0x0010,
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
