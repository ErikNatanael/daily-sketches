# FM Synth using the sample crate

I'm running into som issues that are teaching me a lot of how threading works as well as audio graphs.

## Changing parameters

Building a static synth with static parameters works well, but changing the parameters (freq, c_ratio, m_ratio, m_index, amp etc) is more difficult. Because the synth has to be allocated in the main thread and then moved into the `jack::ClosureProcessHandler` everything has to be `Send` (https://docs.rs/jack/0.6.2/jack/struct.ClosureProcessHandler.html).
The first suggestion from Mitch was Rc<Cell<>>, but this doesn't work because the `Rc` isn't thread safe so it can't be sent to the new thread. `Cell` and `RefCell` both are not `Sync`, which they would need to be in order to be used within `Arc`. An `Arc<Mutex<f64>>` works of course, but having a mutex right in the audio processing loop is bad practice.