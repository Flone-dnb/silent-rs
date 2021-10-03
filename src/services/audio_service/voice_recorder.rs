// External.
use sfml::audio::SoundRecorder;

// Std.
use std::sync::mpsc;

pub struct VoiceRecorder {
    sample_sender: mpsc::Sender<Vec<i16>>,
    microphone_volume_multiplier: f64,
}

impl VoiceRecorder {
    pub fn new(sample_sender: mpsc::Sender<Vec<i16>>, microphone_volume: i32) -> Self {
        VoiceRecorder {
            sample_sender,
            microphone_volume_multiplier: microphone_volume as f64 / 100.0,
        }
    }
}

impl SoundRecorder for VoiceRecorder {
    fn on_process_samples(&mut self, samples: &[i16]) -> bool {
        let mut sample_vec = Vec::from(samples);

        // apply microphone multiplier
        sample_vec.iter_mut().for_each(|sample| {
            let mut new_sample = *sample as f64 * self.microphone_volume_multiplier;
            if new_sample > std::i16::MAX as f64 {
                new_sample = std::i16::MAX as f64;
            } else if new_sample < std::i16::MIN as f64 {
                new_sample = std::i16::MIN as f64;
            }
            *sample = new_sample as i16;
        });

        self.sample_sender.send(sample_vec).unwrap();

        true
    }
}
