use anyhow::Result;

pub struct Apu {}

impl Apu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn read_square_ch1_control1(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_square_ch1_control2(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_square_ch1_freq1(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_square_ch1_freq2(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_square_ch2_control1(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_square_ch2_control2(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_square_ch2_freq1(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_square_ch2_freq2(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_sign_control(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_sign_freq1(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_sign_freq2(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_noise_control(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_noise_rand(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_noise_duration(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_dpcm_control1(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_dpcm_control2(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_dpcm_control3(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_dpcm_control4(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn read_voice_control(&self) -> Result<u8> {
        Ok(0)
    }

    pub fn write_square_ch1_control1(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_square_ch1_control2(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_square_ch1_freq1(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_square_ch1_freq2(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_square_ch2_control1(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_square_ch2_control2(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_square_ch2_freq1(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_square_ch2_freq2(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_sign_control(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_sign_freq1(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_sign_freq2(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_noise_control(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_noise_rand(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_noise_duration(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_dpcm_control1(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_dpcm_control2(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_dpcm_control3(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_dpcm_control4(&mut self, data: u8) -> Result<()> {
        Ok(())
    }

    pub fn write_voice_control(&mut self, data: u8) -> Result<()> {
        Ok(())
    }
}
