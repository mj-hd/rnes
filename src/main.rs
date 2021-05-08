use pixels::{Pixels, SurfaceTexture};
use rnes::{nes::Nes, rom::Rom};
use std::{
    env,
    fs::File,
    io::BufReader,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

enum NesThreadEvent {}

enum UiThreadEvent {
    Render(Vec<u8>),
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let size = LogicalSize::new(160, 144);
    let window = WindowBuilder::new()
        .with_title("nes")
        .with_inner_size(size)
        .with_min_inner_size(size)
        .build(&event_loop)
        .unwrap();

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(256, 240, surface_texture).unwrap();

    let (nes_sender, nes_receiver) = mpsc::channel::<NesThreadEvent>();
    let (ui_sender, ui_receiver) = mpsc::sync_channel::<UiThreadEvent>(1);

    let args = env::args().collect::<Vec<String>>();

    let mut reader = BufReader::new(File::open(args[1].clone()).unwrap());
    let rom = Rom::new(&mut reader).unwrap();

    {
        thread::spawn(move || {
            let mut nes = Nes::new(rom).unwrap();

            nes.reset().unwrap();

            loop {
                let time = Instant::now();

                for _ in 0..89342 {
                    nes.tick().unwrap();
                }

                match nes_receiver.try_recv() {
                    Ok(event) => match event {},
                    _ => {}
                };

                let buffer = nes.render().unwrap();

                let _ = ui_sender.try_send(UiThreadEvent::Render(buffer));

                let elapsed = time.elapsed().as_millis();

                let (wait, c) = ((1000 / 60) as u128).overflowing_sub(elapsed);

                if !c {
                    thread::sleep(Duration::from_millis(wait as u64));
                }
            }
        });
    }

    {
        let mut time = Instant::now();

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::RedrawRequested(_) => {
                    pixels.render().unwrap();
                }
                Event::MainEventsCleared => match ui_receiver.recv() {
                    Ok(event) => match event {
                        UiThreadEvent::Render(buffer) => {
                            pixels.get_frame().copy_from_slice(buffer.as_slice());
                        }
                    },
                    _ => {}
                },
                _ => {}
            }

            match *control_flow {
                ControlFlow::Exit => {}
                _ => {
                    if time.elapsed() >= Duration::from_millis(1000 / 60) {
                        time = Instant::now();

                        window.request_redraw();
                    }

                    if input.update(&event) {
                        if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }

                        if let Some(size) = input.window_resized() {
                            pixels.resize(size.width, size.height);
                        }
                    }

                    *control_flow = ControlFlow::Poll;
                }
            }
        });
    }
}
