use crunch::Chip8;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::Duration;

pub fn main() -> Result<(), String> {
    let rom = std::fs::read("test_opcode.ch8").expect("no rom found at rom.ch8");
    let mut chip = Chip8::new(&rom);

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 768, 384)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        let display = chip.display();
        for x in 0..64 {
            for y in 0..32 {
                if display.get_pixel(x, y) {
                    canvas
                        .fill_rect(Rect::new((x as i32) * 12, (y as i32) * 12, 12, 12))
                        .expect("failed to draw rect");
                }
            }
        }
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        chip.advance();
        // TODO: beep if beep > 0
        // The rest of the game loop goes here...
    }

    Ok(())
}
