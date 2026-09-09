use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

/// Holds state for an ongoing simultaneous playback & recording session.
#[derive(Clone)]
pub struct RecordingSession {
    pub is_recording: Arc<AtomicBool>,
    pub should_play_audio: Arc<AtomicBool>,
    pub pcm_samples: Arc<Mutex<Vec<i16>>>,
    pub current_peak: Arc<Mutex<f32>>,
    pub sample_rate: u32,
    pub channels: u16,
    pub started_at: Arc<Mutex<Option<Instant>>>,
}

impl RecordingSession {
    pub fn new(sample_rate: u32, channels: u16, play_audio: bool) -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(true)),
            should_play_audio: Arc::new(AtomicBool::new(play_audio)),
            pcm_samples: Arc::new(Mutex::new(Vec::with_capacity(sample_rate as usize * channels as usize * 180))),
            current_peak: Arc::new(Mutex::new(0.0)),
            sample_rate,
            channels,
            started_at: Arc::new(Mutex::new(Some(Instant::now()))),
        }
    }

    /// Appends new PCM samples to the recording buffer and calculates the current peak meter.
    pub fn push_samples(&self, samples: &[i16]) {
        if !self.is_recording.load(Ordering::Relaxed) {
            return;
        }

        let mut max_abs: f32 = 0.0;
        for &s in samples {
            let abs = (s.abs() as f32) / 32768.0;
            if abs > max_abs {
                max_abs = abs;
            }
        }

        if let Ok(mut peak) = self.current_peak.lock() {
            *peak = (*peak * 0.7) + (max_abs * 0.3); // smooth peak
        }

        if let Ok(mut buf) = self.pcm_samples.lock() {
            buf.extend_from_slice(samples);
        }
    }

    pub fn duration_seconds(&self) -> f32 {
        if let Ok(buf) = self.pcm_samples.lock() {
            if self.sample_rate > 0 && self.channels > 0 {
                return (buf.len() as f32) / (self.sample_rate as f32 * self.channels as f32);
            }
        }
        0.0
    }

    pub fn stop(&self) {
        self.is_recording.store(false, Ordering::Relaxed);
    }

    pub fn finish_and_take_samples(&self) -> Vec<i16> {
        self.stop();
        if let Ok(mut buf) = self.pcm_samples.lock() {
            std::mem::take(&mut *buf)
        } else {
            Vec::new()
        }
    }
}
