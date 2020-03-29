//! Sine wave generator with frequency configuration exposed through standard
//! input.
extern crate crossbeam_channel;
extern crate jack;
extern crate sample;

use crossbeam_channel::bounded;
use std::io;
use std::str::FromStr;

use sample::{signal, Signal};

struct FMSynth {
    sample_rate: f64,
    freq: f64,
    m_ratio: f64,
    c_ratio: f64,
    m_index: f64,
    c_phase: f64,
    c_phase_step: f64,
    m_phase: f64,
    m_phase_step: f64,
    lfo_freq: f64,
    lfo_amp: f64,
    lfo_add: f64,
    lfo_phase: f64,
    amp: f64,
}

impl FMSynth {
    fn new(sample_rate: f64, freq: f64, amp: f64, m_ratio: f64, c_ratio: f64, m_index: f64) -> Self {

        // let mod_freq = signal::gen(|| [freq * m_ratio]);
        // let modulator = signal::rate(sample_rate).hz(mod_freq).sine();
        // let car_freq = signal::gen(|| [freq * c_ratio]).add_amp(modulator);
        // let carrier = signal::rate(sample_rate).hz(car_freq).sine();

        let mut synth = FMSynth {
            sample_rate,
            freq,
            m_ratio,
            c_ratio,
            m_index,
            c_phase: 0.0,
            c_phase_step: 0.0,
            m_phase: 0.0,
            m_phase_step: 0.0,
            lfo_freq: 3.0,
            lfo_amp: 4.0,
            lfo_add: 5.0,
            lfo_phase: 0.0,
            amp,
        };
        synth
    }
    fn next_stereo(&mut self) -> [f64; 2] {
        // LFO
        self.lfo_phase += (2.0 * std::f64::consts::PI * self.lfo_freq) / self.sample_rate;
        let lfo = self.lfo_phase.sin() * self.lfo_amp + self.lfo_add;
        self.m_index = lfo;

        // Modulator
        self.m_phase_step = (2.0 * std::f64::consts::PI * self.freq * self.m_ratio) / self.sample_rate;
        self.m_phase += self.m_phase_step;
        let m_sample = self.m_phase.sin() * self.freq * self.m_index;

        // Carrier
        // The frequency depends on the modulator so the phase step has to be calculated every step
        let c_freq = self.freq * self.c_ratio + m_sample;
        self.c_phase_step = (2.0 * std::f64::consts::PI * c_freq * self.c_ratio) / self.sample_rate;
        self.c_phase += self.c_phase_step;

        // The carrier output is the output of the synth
        let c_sample = self.c_phase.sin() * self.amp;
        
        [c_sample, c_sample]
    }
    fn set_freq(&mut self, freq: f64) {
        self.freq = freq;
    }
    fn control_rate_update(&mut self) {
        self.amp *= 0.98;
    }
    fn trigger(&mut self, freq: f64) {
        // Set the new frequency
        self.freq = freq;
        // Setting the amplitude triggers an attack
        self.amp = 0.5;
        // Reset all phases
        // self.lfo_phase = 0.0; // You may or may not want to reset the lfo phase based on how you use it
        self.m_phase = 0.0;
        self.c_phase = 0.0;
    }
}

fn main() {
    // 1. open a client
    let (client, _status) =
        jack::Client::new("rust_jack_sine", jack::ClientOptions::NO_START_SERVER).unwrap();

    // 2. register port
    let mut out_port_l = client
        .register_port("sine_out_l", jack::AudioOut::default())
        .unwrap();
    let mut out_port_r = client
        .register_port("sine_out_r", jack::AudioOut::default())
        .unwrap();

    // 3. define process callback handler
    let mut frequency = 220.0;
    let sample_rate = client.sample_rate();
    let frame_t = 1.0 / sample_rate as f64;
    let mut time = 0.0;
    let (tx, rx) = bounded::<[f64; 4]>(1_000_000);

    // FMSynth setup
    let mut fm_synth = FMSynth::new(sample_rate as f64, frequency, 1.0, 2.0, 1.0, 4.0);
    let mut counter = 0;

    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            // Get output buffer
            let out_l = out_port_l.as_mut_slice(ps);
            let out_r = out_port_r.as_mut_slice(ps);

            // Check frequency requests
            while let Ok(f) = rx.try_recv() {
                time = 0.0;
                frequency = f[0];
                fm_synth.set_freq(f[0]);
                fm_synth.c_ratio = f[1];
                fm_synth.m_ratio = f[2];
                fm_synth.lfo_freq = f[3];
                fm_synth.trigger(f[0]);
            }

            // Write output
            for (l, r) in out_l.iter_mut().zip(out_r.iter_mut()) {
                let frame = fm_synth.next_stereo();
                *l = frame[0] as f32;
                *r = frame[1] as f32;
                time += frame_t;
            }

            fm_synth.control_rate_update();

            // Trigger based on counter
            const COUNTER_STEP: usize = 80;
            for i in 0..16 {
                if counter == i * COUNTER_STEP {
                    fm_synth.trigger(frequency)
                }
            }

            counter = (counter+1) % (COUNTER_STEP*16);

            // Continue as normal
            jack::Control::Continue
        },
    );

    // 4. activate the client
    let active_client = client.activate_async((), process).unwrap();
    // processing starts here

    // 5. wait or do some processing while your handler is running in real time.
    println!("Enter freq c_ratio m_ratio lfo_freq");
    while let Some(f) = read_freq() {
        tx.send(f).unwrap();
    }

    // 6. Optional deactivate. Not required since active_client will deactivate on
    // drop, though explicit deactivate may help you identify errors in
    // deactivate.
    active_client.deactivate().unwrap();
}

/// Attempt to read a frequency from standard in. Will block until there is
/// user input. `None` is returned if there was an error reading from standard
/// in, or the retrieved string wasn't a compatible u16 integer.
fn read_freq() -> Option<[f64; 4]> {
    let mut user_input = String::new();
    match io::stdin().read_line(&mut user_input) {
        Ok(_) => {
            let mut values: [f64; 4] = [220.0, 1.0, 1.0, 1.0];
            let strings: Vec<&str> = user_input.split(" ").collect();
            for (i, string) in strings.into_iter().enumerate() {
                values[i] = f64::from_str(string.trim()).unwrap();
            }
            Some(values)
        },
        Err(_) => None,
    }
}
