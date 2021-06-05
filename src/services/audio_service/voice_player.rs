// External.
use sfml::audio::SoundStream;
use sfml::system::Time;

// Std.
use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

// Custom
use crate::global_params::*;

pub struct VoicePlayer {
    sample_receiver: mpsc::Receiver<Vec<i16>>,
    sample_rate: u32,
    sample_chunks: VecDeque<Vec<i16>>,
    finish_chunk: Vec<i16>,
}

impl VoicePlayer {
    pub fn new(sample_receiver: mpsc::Receiver<Vec<i16>>, sample_rate: u32) -> Self {
        VoicePlayer {
            sample_receiver,
            sample_rate,
            sample_chunks: VecDeque::new(),
            finish_chunk: vec![0i16; 1],
        }
    }
}

impl SoundStream for VoicePlayer {
    /// Returns `(chunk, keep_playing)`, where `chunk` is the chunk of audio samples,
    /// and `keep_playing` tells the streaming loop whether to keep playing or to stop.
    fn get_data(&mut self) -> (&mut [i16], bool) {
        if self.sample_chunks.len() > 0 {
            self.sample_chunks.pop_front();
        }

        if self.sample_chunks.len() == 0 {
            // wait, we need data to play
            let res = self
                .sample_receiver
                .recv_timeout(Duration::from_secs(MAX_WAIT_TIME_IN_VOICE_PLAYER_SEC));
            if let Err(e) = res {
                match e {
                    mpsc::RecvTimeoutError::Timeout => {
                        // finish
                        self.sample_chunks.clear();
                        return (&mut self.finish_chunk, false);
                    }
                    _ => {
                        panic!("error: {} at [{}, {}]", e, file!(), line!());
                    }
                }
            }
            self.sample_chunks.push_back(res.unwrap());

            if self.sample_chunks.back().unwrap().len() == 0 {
                // zero-sized chunk means end of voice message
                // finished
                self.sample_chunks.clear();
                return (&mut self.finish_chunk, false);
            }
        }

        (&mut self.sample_chunks[0], true)
    }
    fn seek(&mut self, _offset: Time) {
        // dont need
    }
    fn channel_count(&self) -> u32 {
        1
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
