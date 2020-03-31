//! Sine wave generator with frequency configuration exposed through standard
//! input.
extern crate crossbeam_channel;
extern crate dsp;
extern crate jack;

use crossbeam_channel::bounded;
// use std::cell::Cell;
use std::io;
// use std::rc::Rc;
use std::str::FromStr;
// use std::sync::{Arc, Mutex};

use dsp::{sample::ToFrameSliceMut, Frame, FromSample, Graph, Node, Sample, Walker};

type Output = f32;

type Phase = f64;
type Frequency = f64;
type Volume = f32;

const CHANNELS: usize = 2;
const FRAMES: u32 = 64;
const SAMPLE_HZ: f64 = 44_100.0;
const LOWEST_BUFFER_SIZE: usize = 16;

// struct FMSynth {
//     sample_rate: f64,
//     freq: Arc<Mutex<f64>>,
//     m_ratio: f64,
//     c_ratio: f64,
//     m_index: f64,
//     // modulator: Box<dyn Signal<Frame = sample::frame::Mono<f64>>>,
//     carrier: Box<dyn Signal<Frame = sample::frame::Mono<f64>> + Send + Sync>,
//     amp: f64,
// }

// impl FMSynth {
//     fn new(sample_rate: f64, freq: f64, amp: f64, m_ratio: f64, c_ratio: f64, m_index: f64) -> Self {
//         let sync_freq = Arc::new(Mutex::new(freq));
//         let freq_sig1 = signal::gen(move || [*sync_freq.lock().unwrap()]);
//         let freq_sig2 = signal::gen(move || [*sync_freq.lock().unwrap()]);
//         let freq_sig3 = signal::gen(move || [*sync_freq.lock().unwrap()]);
//         let index_sig = signal::gen(move || [m_index]);
//         let mut mod_freq = signal::gen(move || [m_ratio]).mul_amp(freq_sig1);
//         let mut modulator = signal::rate(sample_rate as f64).hz(mod_freq).sine().mul_amp(freq_sig2).mul_amp(index_sig);
//         let mut car_freq = signal::gen(move || [c_ratio]).mul_amp(freq_sig3).add_amp(modulator);
//         let mut carrier = signal::rate(sample_rate as f64).hz(car_freq).sine();

//         let mut synth = FMSynth {
//             sample_rate,
//             freq: sync_freq,
//             m_ratio,
//             c_ratio,
//             m_index,
//             // modulator: Box::new(modulator), // cannot be used here because the modulator is moved
//             carrier: Box::new(carrier),
//             amp,
//         };
//         synth
//     }
//     fn next_stereo(&mut self) -> [f64; 2] {
//         let sample = self.carrier.next();

//         [sample[0], sample[0]]
//     }
//     fn set_freq(&mut self, freq: f64) {
//         *self.freq.lock().unwrap() = freq;
//     }
//     fn control_rate_update(&mut self) {
//         self.amp *= 0.98;
//     }
//     fn trigger(&mut self, freq: f64) {
//         // Set the new frequency
//         *self.freq.lock().unwrap() = freq;
//         // Setting the amplitude triggers an attack
//         self.amp = 0.5;
//         // Reset all phases
//         // self.lfo_phase = 0.0; // You may or may not want to reset the lfo phase based on how you use it
//     }
// }

fn main() {
    // 1. open a client
    let (client, _status) =
        jack::Client::new("rust_jack_fm", jack::ClientOptions::NO_START_SERVER).unwrap();

    // 2. register port
    let mut out_port_l = client
        .register_port("out_l", jack::AudioOut::default())
        .unwrap();
    let mut out_port_r = client
        .register_port("out_r", jack::AudioOut::default())
        .unwrap();

    // The current maximum size that will every be passed to the process callback.
    // NB: Buffer size can be changed via Client::set_buffer_size()
    let buffer_size = client.buffer_size();

    // Get the system ports
    let ports = client.ports(Some("system:playback_.*"), None, jack::PortFlags::empty());

    // 3. define process callback handler
    let mut frequency = 220.0;
    let sample_rate = client.sample_rate();
    let frame_t = 1.0 / sample_rate as f64;
    let mut time = 0.0;
    let (tx, rx) = bounded::<[f64; 4]>(1_000_000);

    // DSP-GRAPH setup
    // Construct our dsp graph.
    let mut graph = Graph::new();
    // Construct our fancy Synth and add it to the graph!
    let synth = graph.add_node(DspNode::Synth);

    const A5_HZ: Frequency = 440.0;
    // Connect a few oscillators to the synth.
    let (_, oscillator_a) = graph.add_input(DspNode::Oscillator(0.0, A5_HZ, 0.2), synth);
    let (_, oscillator_b) = graph.add_input(DspNode::Oscillator(0.0, A5_HZ * 4.0 / 3.0, 0.1), synth);
    let (_, oscillator_c) = graph.add_input(DspNode::Oscillator(0.0, A5_HZ * 5.0 / 6.0, 0.15), synth);

    // If adding a connection between two nodes would create a cycle, Graph will return an Err.
    if let Err(err) = graph.add_connection(synth, oscillator_a) {
        println!(
            "Testing for cycle error: {:?}",
            err
        );
    }

    // Set the synth as the master node for the graph.
    graph.set_master(Some(synth));

    let mut counter = 0;

    let mut temp_buffer = [[0.0_f32; CHANNELS]; LOWEST_BUFFER_SIZE];
    let mut temp_buffer_index: usize = 0;

    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            // Get output buffer
            let out_l = out_port_l.as_mut_slice(ps);
            let out_r = out_port_r.as_mut_slice(ps);

            // Check frequency requests
            while let Ok(f) = rx.try_recv() {
                time = 0.0;
                frequency = f[0];
                // Traverse inputs or outputs of a node with the following pattern.
                let mut inputs = graph.inputs(synth);
                while let Some(input_idx) = inputs.next_node(&graph) {
                    if let DspNode::Oscillator(_, ref mut pitch, _) = graph[input_idx] {
                        // Pitch down our oscillators for fun.
                        *pitch = frequency;
                    }
                }
                // Question: How can I change the parameters of the synth here, e.g. frequency?
                // fm_synth.set_freq(f[0]);
                // fm_synth.c_ratio = f[1];
                // fm_synth.m_ratio = f[2];
                // fm_synth.lfo_freq = f[3];
                // fm_synth.trigger(f[0]);
            }

            // Combine out_l and out_r from [l0, l1, l2, ..] and [r0, r1, r2, ..] to
            // [[l0, r0], [l1, r1], [l2, r2] [.., ..]]

            let current_buffer_size = out_l.len();

            // Create a buffer to store the audio data for this tick
            let mut output_buffers = [out_l, out_r];

            for i in 0..current_buffer_size {
                if temp_buffer_index >= LOWEST_BUFFER_SIZE {
                    // Get new samples if the temporary buffer is depleted
                    dsp::slice::equilibrium(&mut temp_buffer);
                    graph.audio_requested(&mut temp_buffer, sample_rate as f64);
                    temp_buffer_index = 0;
                }
                // Write the interleaved samples [[l, r] ..] to each output buffer
                for ch_ix in 0..CHANNELS {
                    let output_channel = &mut output_buffers[ch_ix];
                    output_channel[i] = temp_buffer[temp_buffer_index][ch_ix];
                }
                // Increase the index into the temporary buffer
                temp_buffer_index += 1;
            }
            
            
            // Write interleaved samples to non-interleaved `output_buffers`, one channel at a time.
            // for ch_ix in 0..CHANNELS {
            //     let output_channel = &mut output_buffers[ch_ix];
            //     for (frame, output_sample) in interleaved.chunks(n_channels).zip(output_channel) {
            //         *output_sample = frame[ch_ix];
            //     }
            // }

            // // Write output
            // for (l, r) in out_l.iter_mut().zip(out_r.iter_mut()) {
            //     let frame = carrier.next();
            //     // let frame = fm_synth.next_stereo();
            //     *l = frame[0] as f32;
            //     *r = frame[0] as f32;
            //     time += frame_t;
            // }

            // fm_synth.control_rate_update();

            // Trigger based on counter
            const COUNTER_STEP: usize = 80;
            for i in 0..16 {
                if counter == i * COUNTER_STEP {
                    // fm_synth.trigger(frequency)
                }
            }

            counter = (counter + 1) % (COUNTER_STEP * 16);

            // Continue as normal
            jack::Control::Continue
        },
    );

    // 4. activate the client
    let active_client = client.activate_async((), process).unwrap();
    // processing starts here

    // Connect the client to the system outputs automatically.
    // It seems like this has to be done after the client is activated, doing it just after creating the ports doesn't work.
    // TODO: Get the local port names automatically from the client or by putting the client and port names in variables.
    let res = active_client
        .as_client()
        .connect_ports_by_name("rust_jack_fm:out_l", &ports[0]);
    match res {
        Ok(_) => (),
        Err(e) => println!("Unable to connect to port {} with error {}", ports[0], e),
    }
    match active_client
        .as_client()
        .connect_ports_by_name("rust_jack_fm:out_r", &ports[1])
    {
        Ok(_) => (),
        Err(e) => println!("Unable to connect to port {} with error {}", ports[1], e),
    }

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
        }
        Err(_) => None,
    }
}

// DSP STUFF

/// Our type for which we will implement the `Dsp` trait.
#[derive(Debug)]
enum DspNode {
    /// Synth will be our demonstration of a master GraphNode.
    Synth,
    /// Oscillator will be our generator type of node, meaning that we will override
    /// the way it provides audio via its `audio_requested` method.
    Oscillator(Phase, Frequency, Volume),
}

impl Node<[Output; CHANNELS]> for DspNode {
    /// Here we'll override the audio_requested method and generate a sine wave.
    fn audio_requested(&mut self, buffer: &mut [[Output; CHANNELS]], sample_hz: f64) {
        match *self {
            DspNode::Synth => (),
            DspNode::Oscillator(ref mut phase, frequency, volume) => {
                dsp::slice::map_in_place(buffer, |_| {
                    let val = sine_wave(*phase, volume);
                    *phase += frequency / sample_hz;
                    Frame::from_fn(|_| val)
                });
            }
        }
    }
}

/// Return a sine wave for the given phase.
fn sine_wave<S: Sample>(phase: Phase, volume: Volume) -> S
where
    S: Sample + FromSample<f32>,
{
    use std::f64::consts::PI;
    ((phase * PI * 2.0).sin() as f32 * volume).to_sample::<S>()
}
