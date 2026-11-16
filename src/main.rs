mod animation;
mod art;
mod audio;
mod background;
mod camera;
mod demo;
mod display;
mod enemy;
mod gamepad;
mod input;
mod music;
mod physics;
mod player;
mod sfx;
mod sprites;
mod synth;
mod tiles;
mod timing;
mod video;

use macroquad::miniquad::conf::Icon;
use macroquad::prelude::*;

use audio::Speaker;
use demo::Demo;
use display::Display;
use gamepad::Gamepads;
use input::Controller;
use sfx::Sfx;
use timing::FixedTimestep;
use video::{Framebuffer, SCREEN_H, SCREEN_W};

/// The window and dock icon: raw RGBA at the three sizes the window system
/// asks for, made from the same pixel art as `docs/images/icon.svg`.
fn window_icon() -> Icon {
    Icon {
        small: *include_bytes!("../assets/icon-16.rgba"),
        medium: *include_bytes!("../assets/icon-32.rgba"),
        big: *include_bytes!("../assets/icon-64.rgba"),
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Glint".to_owned(),
        window_width: SCREEN_W as i32 * 3,
        window_height: SCREEN_H as i32 * 3,
        icon: Some(window_icon()),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut clock = FixedTimestep::new();
    let mut fb = Framebuffer::new();
    let mut display = Display::new();
    let mut demo = Demo::new();
    let mut pad = Controller::new();
    let mut gamepads = Gamepads::new();
    // Every sound effect is synthesized once here, before the game starts.
    let mut speaker = Speaker::new(&Sfx::ALL.map(Sfx::notes)).await;
    // So is the music: all four voices are mixed into one loop.
    speaker.start_music(&music::theme()).await;

    // Debug aid: GLINT_SNAPSHOT=out.png saves the frame after 60 ticks and
    // exits. GLINT_SNAPSHOT_TICKS changes the tick count, and
    // GLINT_SNAPSHOT_HOLD is a controller byte to hold down throughout
    // (128 is Right), so a snapshot can show the hero somewhere other than
    // the start.
    let number = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<u32>().ok());
    let snapshot = std::env::var("GLINT_SNAPSHOT").ok();
    let snapshot_ticks = number("GLINT_SNAPSHOT_TICKS").unwrap_or(60);
    let snapshot_hold = number("GLINT_SNAPSHOT_HOLD").unwrap_or(0) as u8;
    let mut ticks_run = 0u32;

    loop {
        // 1. Update: zero, one or several fixed ticks, depending on how much
        //    real time the last frame took.
        for _ in 0..clock.advance(get_frame_time()) {
            // Latch the controller once per tick, then let the game react.
            // Keyboard and gamepad are merged: either can press a button.
            pad.update(input::read_keyboard() | gamepads.read() | snapshot_hold);
            demo.update(&pad);
            // The game only says which sounds it wants; play them here.
            for sfx in demo.take_sounds() {
                speaker.play(sfx as usize);
            }
            ticks_run += 1;
        }

        // M mutes and unmutes. This belongs to the host program rather than
        // the console, so it reads the keyboard directly.
        if is_key_pressed(KeyCode::M) {
            speaker.toggle_mute();
        }
        // The game says whether music should be heard; it falls silent on pause.
        speaker.set_music_playing(demo.music_playing());

        // 2. Draw: the game paints palette indices into the framebuffer...
        demo.draw(&mut fb);
        // 3. ...and the display converts them to RGB and shows them.
        display.present(&fb);

        if let Some(path) = &snapshot {
            if ticks_run >= snapshot_ticks {
                display.save_png(path);
                break;
            }
        }

        next_frame().await;
    }
}
