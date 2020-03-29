use nannou_audio as audio;
use nannou_audio::Buffer;
use std::f64::consts::PI;

pub const NUM_SINES: usize = 1000;

pub struct AudioInterface {
  stream: audio::Stream<Audio>,
  next_free_sine: usize,
  amp_changes: Vec<(usize, f32)>,
  freq_changes: Vec<(usize, f64)>,
}

impl AudioInterface {
  pub fn new() -> Self {
    // Initialise the audio API so we can spawn an audio stream.
    let audio_host = audio::Host::new();
    println!("Audio host init");

    println!("Default output: \"{:?}\"", audio_host.default_output_device().unwrap().name());
    
    if let Ok(devices) = audio_host.output_devices() {
        for device in devices {
            println!("Devices: {:?}", device.name());
        }
    }
    // Devices: Ok("hw:CARD=Pro,DEV=1")
    // Devices: Ok("plughw:CARD=Pro,DEV=1")
    // Devices: Ok("dmix:CARD=Pro,DEV=1")
    // Devices: Ok("default:CARD=Pro")
    // Devices: Ok("sysdefault:CARD=Pro")
    // Devices: Ok("front:CARD=Pro,DEV=0")


    let output_device = find_output_device(&audio_host, "jack")
        .expect("no output devices available on the system");
    println!("Selected Output Device: {:?}", output_device.name());
    
    // Initialise the state that we want to live on the audio thread.
    let model = Audio::new();
    let stream = audio_host
        .new_output_stream(model)
        .render(audio)
        .sample_rate(44100)
        .frames_per_buffer(512)
        .device(output_device)
        .build()
        .expect("Unable to build audio stream.");

    AudioInterface {
      stream,
      next_free_sine: 0,
      amp_changes: vec![],
      freq_changes: vec![],
    }
  }
  pub fn get_new_sine(&mut self) -> usize {
    let index = self.next_free_sine;
    self.next_free_sine = (self.next_free_sine + 1) % NUM_SINES;
    return index;
  }
  pub fn set_sine_freq(&mut self, index: usize, freq: f64) {
    self.freq_changes.push((index, freq));
    // self.stream
    //   .send(move |audio| {
    //       audio.set_sine_freq(index, freq);
    //   })
    //   .ok();
  }
  pub fn set_sine_amp(&mut self, index: usize, amp: f32) {
    self.amp_changes.push((index, amp));
    // self.stream
    //   .send(move |audio| {
    //       audio.set_sine_amp(index, amp);
    //   })
    //   .ok();
  }
  pub fn update(&mut self) {
    let amp_changes = self.amp_changes.clone();
    let freq_changes = self.freq_changes.clone();
    self.stream
      .send(move |audio| {
        for (i, amp) in amp_changes {
          audio.set_sine_amp(i, amp);
        }
        for (i, freq) in freq_changes {
          audio.set_sine_freq(i, freq);
        }
          
      })
      .ok();
    self.amp_changes.clear();
    self.freq_changes.clear();
  }
}

pub struct Audio {
  sines: [Sine; NUM_SINES],
}

impl Audio {
  pub fn new() -> Self {
    Audio {
      sines: [Sine::new(); NUM_SINES],
    }
  }
  
  pub fn set_sine_freq(&mut self, index: usize, freq: f64) {
    self.sines[index].hz = freq;
  }
  pub fn set_sine_amp(&mut self, index: usize, amp: f32) {
    self.sines[index].amp = amp;
  }
}

// A function that renders the given `Audio` to the given `Buffer`.
pub fn audio(audio: &mut Audio, buffer: &mut Buffer) {
  let sample_rate = buffer.sample_rate() as f64;
  let volume = 0.5;
  for frame in buffer.frames_mut() {
    let mut sample: f32 = 0.0;
    for sine in audio.sines.iter_mut() {
      let sine_amp = sine.next_sample(sample_rate);
      sample += sine_amp;
    }
    for channel in frame {
      *channel = sample * volume;
    }
  }
  // Do block update.
  for sine in audio.sines.iter_mut() {
    sine.update();
  }
}

#[derive(Copy, Clone)]
struct Sine {
  phase: f64,
  hz: f64,
  amp: f32,
  current_amp: f32,
}

impl Sine {
  pub fn new() -> Self {
    Sine {
      phase: 0.0,
      hz: 220.0,
      amp: 0.0,
      current_amp: 0.0,
    }
  }

  fn next_sample(&mut self, sample_rate: f64) -> f32 {
    let sine_amp = (2.0 * PI * self.phase).sin() as f32;
    self.phase += self.hz / sample_rate;
    self.phase %= sample_rate;
    return sine_amp * self.current_amp;
  }

  fn update(&mut self) {
    self.amp *= 0.95;
    self.current_amp = self.current_amp * 0.95 + self.amp * 0.05;
  }
}


/// From https://github.com/museumsvictoria/spatial_audio_server/ by MindBuffer
/// Given a target device name, find the device within the host and return it.
///
/// If no device with the given name can be found, or if the given `target_name` is empty, the
/// default will be returned.
///
/// Returns `None` if no output devices could be found.
fn find_output_device(host: &audio::Host, target_name: &str) -> Option<audio::Device> {
  if target_name.is_empty() {
      host.default_output_device()
  } else {
      host.output_devices()
          .ok()
          .into_iter()
          .flat_map(std::convert::identity)
          .find(|d| d.name().map(|n| n.contains(&target_name)).unwrap_or(false))
          .or_else(|| host.default_output_device())
  }
}