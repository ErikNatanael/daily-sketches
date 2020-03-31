# FM Synth using the dsp-chain crate and jack

I ran into a problem where the dsp::Graph API expects interleaved channels in the form of &mut [[l0, r0], [l1, r1], [l2, r2] [.., ..]], but JACK gives me each channel separately as a &mut [l0, l1, l2, ..] etc. The easiest way to solve this it seems is to use a temporary buffer and iteratively fetch new audio data to fill the temporary buffer, making sure to save frames from previous calls to the audio callback is the temporary buffer isn't depleted.
