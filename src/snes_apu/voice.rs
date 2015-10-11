use super::dsp::Dsp;
// TODO: This is a placeholder before I start generalizing traits
// from the old code.
use super::apu::Apu;
use super::envelope::Envelope;
use super::brr_block_decoder::BrrBlockDecoder;
use super::dsp_helpers;

pub struct VoiceOutput {
    left_out: i32,
    right_out: i32,
    last_voice_out: i32
}

pub struct Voice {
    dsp: *mut Dsp,
    emulator: *mut Apu,

    envelope: Box<Envelope>,

    vol_left: u8,
    vol_right: u8,
    pitch_low: u8,
    pitch_high: u8,
    source: u8,
    pitch_mod: bool,
    noise_on: bool,
    echo_on: bool,

    sample_start_address: u32,
    loop_start_address: u32,
    brr_block_decoder: Box<BrrBlockDecoder>,
    sample_address: u32,
    sample_pos: i32,
    current_sample: i32,
    next_sample: i32
}

impl Voice {
    pub fn new(dsp: *mut Dsp, emulator: *mut Apu) -> Voice {
        Voice {
            dsp: dsp,
            emulator: emulator,

            envelope: Box::new(Envelope::new(dsp)),

            vol_left: 0,
            vol_right: 0,
            pitch_low: 0,
            pitch_high: 0,
            source: 0,
            pitch_mod: false,
            noise_on: false,
            echo_on: false,

            sample_start_address: 0,
            loop_start_address: 0,
            brr_block_decoder: Box::new(BrrBlockDecoder::new()),
            sample_address: 0,
            sample_pos: 0,
            current_sample: 0,
            next_sample: 0
        }
    }

    #[inline]
    fn dsp(&self) -> &mut Dsp {
        unsafe {
            &mut (*self.dsp)
        }
    }

    #[inline]
    fn emulator(&self) -> &mut Apu {
        unsafe {
            &mut (*self.emulator)
        }
    }

    pub fn render_sample(&mut self, last_voice_out: i32, noise: i32) -> VoiceOutput {
        let mut pitch = ((self.pitch_high as i32) << 8) | (self.pitch_low as i32);
        if self.pitch_mod {
            pitch += ((last_voice_out >> 5) * pitch) >> 10;
        }
        if pitch < 0 {
            pitch = 0;
        }
        if pitch > 0x3fff {
            pitch = 0x3fff;
        }

        let p1 = self.sample_pos;
        let p2 = 0x1000 - p1;
        let mut sample = if self.noise_on {
            (noise * 2) as i32
        } else {
            dsp_helpers::clamp((self.current_sample * p2 + self.next_sample * p1) >> 12) & !1
        };

        self.envelope.tick();
        let env_level = self.envelope.level;

        sample = ((sample * env_level) >> 11) & !1;

        if self.brr_block_decoder.is_end && self.brr_block_decoder.is_looping {
            self.envelope.key_off();
            self.envelope.level = 0;
        }

        self.sample_pos = self.sample_pos + pitch;
        while self.sample_pos >= 0x1000 {
            self.sample_pos -= 0x1000;
            self.read_next_sample();

            if self.brr_block_decoder.is_finished() {
                if self.brr_block_decoder.is_end && self.brr_block_decoder.is_looping {
                    self.read_entry();
                    self.sample_address = self.loop_start_address;
                }
                self.read_next_block();
            }
        }

        VoiceOutput {
            left_out: dsp_helpers::multiply_volume(sample, self.vol_left),
            right_out: dsp_helpers::multiply_volume(sample, self.vol_right),
            last_voice_out: sample
        }
    }

    pub fn key_on(&mut self) {
        self.read_entry();
        self.sample_address = self.sample_start_address;
        self.brr_block_decoder.reset(0, 0);
        self.read_next_block();
        self.sample_pos = 0;
        self.next_sample = 0;
        self.read_next_sample();
        self.envelope.key_on();
    }

    pub fn key_off(&mut self) {
        self.envelope.key_off();
    }

    fn read_entry(&mut self) {
        self.sample_start_address = self.dsp().read_source_dir_start_address(self.source);
        self.loop_start_address = self.dsp().read_source_dir_loop_address(self.source);
    }

    fn read_next_block(&mut self) {
        let mut buf = [0; 9];
        for i in 0..9 {
            buf[i] = self.emulator().read_byte(self.sample_address + (i as u32));
        }
        self.brr_block_decoder.read(&buf);
        self.sample_address += 9;
    }

    fn read_next_sample(&mut self) {
        self.current_sample = self.next_sample;
        self.next_sample = self.brr_block_decoder.read_next_sample() as i32;
    }
}