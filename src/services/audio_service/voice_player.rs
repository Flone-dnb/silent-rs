// External.
use sfml::audio::SoundStream;
use sfml::system::Time;

// Std.
use std::collections::VecDeque;
use std::sync::mpsc;

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
            // remove old chunk
            if self.sample_chunks.len() == 1 {
                self.sample_chunks.clear();
                // now wait for new data (below)
            } else {
                self.sample_chunks.pop_front();

                while self.sample_receiver.try_recv().is_ok() {
                    // read more chunks
                    let res = self.sample_receiver.recv();
                    if let Err(e) = res {
                        panic!("error: {} at [{}, {}]", e, file!(), line!());
                    }
                    self.sample_chunks.push_back(res.unwrap());

                    if self.sample_chunks.back().unwrap().len() == 0 {
                        // zero-sized chunk means end of voice message
                        // finished
                        self.sample_chunks.clear();
                        return (&mut self.finish_chunk, false);
                    }
                }
            }
        }

        if self.sample_chunks.len() == 0 {
            loop {
                // wait, we need data to play
                let res = self.sample_receiver.recv();
                if let Err(e) = res {
                    panic!("error: {} at [{}, {}]", e, file!(), line!());
                }
                self.sample_chunks.push_back(res.unwrap());

                if self.sample_chunks.back().unwrap().len() == 0 {
                    // zero-sized chunk means end of voice message
                    // finished
                    self.sample_chunks.clear();
                    return (&mut self.finish_chunk, false);
                }

                if self.sample_receiver.try_recv().is_err() {
                    break; // no more data for now
                }
            }
        }

        println!("ready chunks: {}", self.sample_chunks.len());
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
