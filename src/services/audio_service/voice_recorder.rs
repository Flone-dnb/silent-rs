// External.
use sfml::audio::SoundRecorder;

// Std.
use std::sync::mpsc;

pub struct VoiceRecorder {
    sample_sender: mpsc::Sender<Vec<i16>>,
}

impl VoiceRecorder {
    pub fn new(sample_sender: mpsc::Sender<Vec<i16>>) -> Self {
        VoiceRecorder { sample_sender }
    }
}

impl SoundRecorder for VoiceRecorder {
    fn on_process_samples(&mut self, samples: &[i16]) -> bool {
        self.sample_sender.send(Vec::from(samples)).unwrap();

        true
    }
}
