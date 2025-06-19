pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod joypad;
pub mod opcodes;
pub mod ppu;
pub mod render;
pub mod trace;

use bus::Bus;
use cartridge::Rom;
use cpu::CPU;
use ppu::NesPPU;
use render::frame::Frame;
use sdl2::audio::AudioSpecDesired;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

const AUDIO_SAMPLE_RATE: f64 = 44100.0;

fn main() {
    // --- SDL2 Initialization ---
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    // -- Window Configuration --
    let window = video_subsystem
        .window("NES Emulator", (256.0 * 2.0) as u32, (240.0 * 2.0) as u32)
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(2.0, 2.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    // -- Audio Configuration --
    let desired_spec = AudioSpecDesired {
        freq: Some(AUDIO_SAMPLE_RATE as i32),
        channels: Some(1),   // mono
        samples: Some(1024), // default
    };

    let audio_queue = audio_subsystem
        .open_queue::<f32, _>(None, &desired_spec)
        .unwrap();
    audio_queue.resume();

    // --- ROM Loading ---
    let bytes: Vec<u8> = std::fs::read("mario_usa.nes").unwrap();
    let rom = Rom::new(&bytes).unwrap();
    let mut frame = Frame::new();

    // --- Key Mapping ---
    let mut key_map = HashMap::new();
    key_map.insert(Keycode::Down, joypad::JoypadButton::DOWN);
    key_map.insert(Keycode::Up, joypad::JoypadButton::UP);
    key_map.insert(Keycode::Right, joypad::JoypadButton::RIGHT);
    key_map.insert(Keycode::Left, joypad::JoypadButton::LEFT);
    key_map.insert(Keycode::Space, joypad::JoypadButton::SELECT);
    key_map.insert(Keycode::Return, joypad::JoypadButton::START);
    key_map.insert(Keycode::A, joypad::JoypadButton::BUTTON_A);
    key_map.insert(Keycode::S, joypad::JoypadButton::BUTTON_B);

    // --- Reset Logic ---
    let should_reset = Arc::new(Mutex::new(false));
    let should_reset_clone = should_reset.clone();

    // --- Main Loop ---
    let bus = Bus::new(
        rom,
        AUDIO_SAMPLE_RATE,
        move |ppu: &NesPPU, joypad: &mut joypad::Joypad| {
            render::render(ppu, &mut frame);

            texture
                .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                    for y in 0..240 {
                        for x in 0..256 {
                            let offset = y * 256 * 3 + x * 3;
                            let buffer_offset = y * pitch + x * 3;
                            buffer[buffer_offset] = frame.data[offset];
                            buffer[buffer_offset + 1] = frame.data[offset + 1];
                            buffer[buffer_offset + 2] = frame.data[offset + 2];
                        }
                    }
                })
                .unwrap();

            canvas.copy(&texture, None, None).unwrap();
            canvas.present();

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => std::process::exit(0),

                    Event::KeyDown { keycode, .. } => {
                        if let Some(key) = keycode {
                            match key {
                                Keycode::R => *should_reset_clone.lock().unwrap() = true,
                                _ => {
                                    if let Some(button) = key_map.get(&key) {
                                        joypad.set_button_pressed_status(*button, true);
                                    }
                                }
                            }
                        }
                    }
                    Event::KeyUp { keycode, .. } => {
                        if let Some(key) = keycode {
                            if let Some(button) = key_map.get(&key) {
                                joypad.set_button_pressed_status(*button, false);
                            }
                        }
                    }
                    _ => {}
                }
            }
        },
    );

    let mut cpu = CPU::new(bus);
    cpu.reset();

    // --- Start emulator ---
    loop {
        // Audio sync: The desired hardware buffer size is 1024 samples * 4 bytes/sample = 4096 bytes.
        // To keep latency low, we pause the emulator if the queue size exceeds twice that (8192 bytes).
        while audio_queue.size() > 8192 {
            std::thread::sleep(std::time::Duration::from_micros(10));
        }

        if *should_reset.lock().unwrap() {
            cpu.reset();
            *should_reset.lock().unwrap() = false;
        }

        cpu.step();

        if let Some(sample) = cpu.collect_audio_sample() {
            let _ = audio_queue.queue_audio(&[sample]);
        }
    }
}
