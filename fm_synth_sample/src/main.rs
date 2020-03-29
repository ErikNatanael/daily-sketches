//! Sine wave generator with frequency configuration exposed through standard
//! input.
extern crate crossbeam_channel;
extern crate jack;
extern crate sample;

use crossbeam_channel::bounded;
use std::io;
use std::str::FromStr;

use sample::{signal, Signal};
use sample::frame::Frame;

struct FMSynth {
    sample_rate: f64,
    freq: f64,
    m_ratio: f64,
    c_ratio: f64,
    m_index: f64,
    // modulator: Box<dyn Signal<Frame = sample::frame::Mono<f64>>>,
    carrier: Box<dyn Signal<Frame = sample::frame::Mono<f64>>>,
    amp: f64,
}

impl FMSynth {
    fn new(sample_rate: f64, freq: f64, amp: f64, m_ratio: f64, c_ratio: f64, m_index: f64) -> Self {

        let freq_sig = signal::gen(move || [freq]);
        let freq_sig2 = signal::gen(move || [freq]);
        let freq_sig3 = signal::gen(move || [freq]);
        let index_sig = signal::gen(move || [m_index]);
        let mut mod_freq = signal::gen(move || [m_ratio]).mul_amp(freq_sig);
        let mut modulator = signal::rate(sample_rate as f64).hz(mod_freq).sine().mul_amp(freq_sig2).mul_amp(index_sig);
        let mut car_freq = signal::gen(move || [c_ratio]).mul_amp(freq_sig3).add_amp(modulator);
        let mut carrier = signal::rate(sample_rate as f64).hz(car_freq).sine();

        let mut synth = FMSynth {
            sample_rate,
            freq,
            m_ratio,
            c_ratio,
            m_index,
            // modulator: Box::new(modulator), // cannot be used here because the modulator is moved
            carrier: Box::new(carrier),
            amp,
        };
        synth
    }
    fn next_stereo(&mut self) -> [f64; 2] {
        let sample = self.carrier.next();
        
        [sample[0], sample[0]]
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
    let m_ratio = 2.5;
    let c_ratio = 1.0;
    let m_index = 6.0;
    // Question: How can I use one freq_sig that I can later change to influence all the places where it should be?
    // If I try to do that now I get a "use of moved value" error
    let freq_sig = signal::gen(move || [frequency]);
    let freq_sig2 = signal::gen(move || [frequency]);
    let freq_sig3 = signal::gen(move || [frequency]);
    let index_sig = signal::gen(move || [m_index]);
    let mut mod_freq = signal::gen(move || [m_ratio]).mul_amp(freq_sig);
    let mut modulator = signal::rate(sample_rate as f64).hz(mod_freq).sine().mul_amp(freq_sig2).mul_amp(index_sig);
    let mut car_freq = signal::gen(move || [c_ratio]).mul_amp(freq_sig3).add_amp(modulator);
    let mut carrier = signal::rate(sample_rate as f64).hz(car_freq).sine();

    let mut counter = 0;

    // Trying to use the FMSynth struct in the jack client process results in an error:
    //     |
    // 155 |     let active_client = client.activate_async((), process).unwrap();
    // |                                ^^^^^^^^^^^^^^ `(dyn sample::Signal<Frame = [f64; 1]> + 'static)` cannot be shared between threads safely
    // |
    // = help: the trait `std::marker::Sync` is not implemented for `(dyn sample::Signal<Frame = [f64; 1]> + 'static)`
    // = note: required because of the requirements on the impl of `std::marker::Sync` for `std::ptr::Unique<(dyn sample::Signal<Frame = [f64; 1]> + 'static)>`
    // = note: required because it appears within the type `std::boxed::Box<(dyn sample::Signal<Frame = [f64; 1]> + 'static)>`
    // = note: required because it appears within the type `FMSynth`
    // = note: required because it appears within the type `[closure@fm_synth_sample/src/main.rs:111:9: 151:10 out_port_l:jack::Port<jack::AudioOut>, out_port_r:jack::Port<jack::AudioOut>, rx:crossbeam_channel::Receiver<[f64; 4]>, time:f64, frequency:f64, carrier:sample::signal::Sine<sample::signal::Hz<sample::signal::AddAmp<sample::signal::MulAmp<sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:105:36: 105:53 c_ratio:f64], [f64; 1]>, sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:101:33: 101:52 frequency:f64], [f64; 1]>>, sample::signal::MulAmp<sample::signal::MulAmp<sample::signal::Sine<sample::signal::Hz<sample::signal::MulAmp<sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:103:36: 103:53 m_ratio:f64], [f64; 1]>, sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:100:33: 100:52 frequency:f64], [f64; 1]>>>>, sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:100:33: 100:52 frequency:f64], [f64; 1]>>, sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:102:33: 102:50 m_index:f64], [f64; 1]>>>>>, fm_synth:FMSynth, frame_t:f64, counter:usize]`
    // = note: required because it appears within the type `jack::ClosureProcessHandler<[closure@fm_synth_sample/src/main.rs:111:9: 151:10 out_port_l:jack::Port<jack::AudioOut>, out_port_r:jack::Port<jack::AudioOut>, rx:crossbeam_channel::Receiver<[f64; 4]>, time:f64, frequency:f64, carrier:sample::signal::Sine<sample::signal::Hz<sample::signal::AddAmp<sample::signal::MulAmp<sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:105:36: 105:53 c_ratio:f64], [f64; 1]>, sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:101:33: 101:52 frequency:f64], [f64; 1]>>, sample::signal::MulAmp<sample::signal::MulAmp<sample::signal::Sine<sample::signal::Hz<sample::signal::MulAmp<sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:103:36: 103:53 m_ratio:f64], [f64; 1]>, sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:100:33: 100:52 frequency:f64], [f64; 1]>>>>, sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:100:33: 100:52 frequency:f64], [f64; 1]>>, sample::signal::Gen<[closure@fm_synth_sample/src/main.rs:102:33: 102:50 m_index:f64], [f64; 1]>>>>>, fm_synth:FMSynth, frame_t:f64, counter:usize]>`
    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            // Get output buffer
            let out_l = out_port_l.as_mut_slice(ps);
            let out_r = out_port_r.as_mut_slice(ps);

            // Check frequency requests
            while let Ok(f) = rx.try_recv() {
                time = 0.0;
                frequency = f[0];
                // Question: How can I change the parameters of the synth here, e.g. frequency?
                // fm_synth.set_freq(f[0]);
                // fm_synth.c_ratio = f[1];
                // fm_synth.m_ratio = f[2];
                // fm_synth.lfo_freq = f[3];
                // fm_synth.trigger(f[0]);
            }

            // Write output
            for (l, r) in out_l.iter_mut().zip(out_r.iter_mut()) {
                let frame = carrier.next();
                // let struct_frame = fm_synth.next_stereo();
                *l = frame[0] as f32;
                *r = frame[0] as f32;
                time += frame_t;
            }

            // fm_synth.control_rate_update();

            // Trigger based on counter
            const COUNTER_STEP: usize = 80;
            for i in 0..16 {
                if counter == i * COUNTER_STEP {
                    // fm_synth.trigger(frequency)
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
