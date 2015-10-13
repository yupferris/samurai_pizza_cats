extern crate emu;
extern crate snes_apu;

use std::iter;
use std::env;
use std::io::{Result, Error, ErrorKind, Write, stdout, stdin};
use std::thread;
use std::sync::{Arc, Mutex};

use emu::audio_driver::AudioDriver;
use emu::core_audio_driver::CoreAudioDriver;

use snes_apu::apu::{Apu, BUFFER_LEN};
use snes_apu::spc::{Spc, Emulator};

fn main() {
    if let Err(e) = play_spc_files() {
        println!("ERROR: {}", e);
        std::process::exit(1);
    }
}

fn play_spc_files() -> Result<()> {
    for file_name in try!(get_file_names()) {
        try!(play_spc_file(&file_name));
    }
    Ok(())
}

fn get_file_names() -> Result<iter::Skip<env::Args>> {
    let args = env::args();
    match args.len() {
        1 => Err(Error::new(ErrorKind::Other, "No file(s) specified")),
        _ => Ok(args.skip(1))
    }
}

fn play_spc_file(file_name: &String) -> Result<()> {
    let spc = try!(Spc::load(file_name));

    println!("SPC: {}", file_name);
    println!(" Version Minor: {}", spc.version_minor);
    println!(" PC: {}", spc.pc);
    println!(" A: {}", spc.a);
    println!(" X: {}", spc.x);
    println!(" Y: {}", spc.y);
    println!(" PSW: {}", spc.psw);
    println!(" SP: {}", spc.sp);

    if let Some(ref id666_tag) = spc.id666_tag {
        println!(" ID666 tag present:");
        println!("  Song title: {}", id666_tag.song_title);
        println!("  Game title: {}", id666_tag.game_title);
        println!("  Dumper name: {}", id666_tag.dumper_name);
        println!("  Comments: {}", id666_tag.comments);
        println!("  Date dumped (MM/DD/YYYY): {}", id666_tag.date_dumped);
        println!("  Seconds to play before fading out: {}", id666_tag.seconds_to_play_before_fading_out);
        println!("  Fade out length: {}ms", id666_tag.fade_out_length);
        println!("  Artist name: {}", id666_tag.artist_name);
        println!("  Default channel disables: {}", id666_tag.default_channel_disables);
        println!("  Dumping emulator: {}", match id666_tag.dumping_emulator {
            Emulator::Unknown => "Unknown",
            Emulator::ZSnes => "ZSnes",
            Emulator::Snes9x => "Snes9x"
        });
    } else {
        println!(" No ID666 tag present.");
    }

    let mut apu = Apu::new();
    apu.set_state(&spc);

    let mut driver = CoreAudioDriver::new();
    driver.set_sample_rate(32000);
    let mut left = [0; BUFFER_LEN];
    let mut right = [0; BUFFER_LEN];
    driver.set_render_callback(Some(Box::new(move |buffer, num_frames| {
        apu.render(&mut left, &mut right, num_frames as i32);
        for i in 0..num_frames {
            let j = i * 2;
            buffer[j + 0] = left[i] as f32 / 32768.0;
            buffer[j + 1] = right[i] as f32 / 32768.0;
        }
    })));

    println!("Return quits.");
    try!(wait_for_key_press_with_busy_icon());

    Ok(())
}

// TODO: This function is super thread-safe but can panic XD
fn wait_for_key_press_with_busy_icon() -> Result<()> {
    let is_done = Arc::new(Mutex::new(false));

    let thread_is_done = is_done.clone();
    let handle = thread::spawn(move || {
        let chars = ['-', '/', '|', '\\'];
        let mut char_index = 0;
        while !*(thread_is_done.lock().unwrap()) {
            print!("\r[{}]", chars[char_index]);
            stdout().flush().unwrap();
            char_index = (char_index + 1) % chars.len();

            thread::sleep_ms(5);
        }
        print!("\r   \r");
    });

    let mut s = String::new();
    try!(stdin().read_line(&mut s));
    *is_done.lock().unwrap() = true;
    handle.join().unwrap();

    Ok(())
}