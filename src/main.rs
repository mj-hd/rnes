use rnes::{nes::Nes, rom::Rom};
use std::{env::args, fs::File, io::BufReader};

fn main() {
  let args = args().collect::<Vec<String>>();
  let mut reader = BufReader::new(File::open(args[1].clone()).unwrap());
  let rom = Rom::new(&mut reader).unwrap();
  let nes = Nes::new(rom);
}
