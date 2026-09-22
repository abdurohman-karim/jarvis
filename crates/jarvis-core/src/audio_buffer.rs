use std::collections::VecDeque;

pub struct AudioRingBuffer {
    buffer: VecDeque<Vec<i16>>,
    max_frames: usize,
}

impl AudioRingBuffer {
    // Create buffer that holds `seconds` worth of audio at given frame_size and sample_rate
    pub fn new(seconds: f32, frame_size: usize, sample_rate: usize) -> Self {
        let frames_per_second = sample_rate / frame_size;
        let max_frames = (frames_per_second as f32 * seconds) as usize;
        
        Self {
            buffer: VecDeque::with_capacity(max_frames),
            max_frames,
        }
    }
    
    // Push a frame, dropping oldest if full
    pub fn push(&mut self, frame: &[i16]) {
        if self.buffer.len() >= self.max_frames {
            self.buffer.pop_front();
        }
        self.buffer.push_back(frame.to_vec());
    }
    
    // Drain all buffered frames into a single vec
    pub fn drain_all(&mut self) -> Vec<Vec<i16>> {
        self.buffer.drain(..).collect()
    }
    
    // Get frame count
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_newest_frames() {
        // 1 second at 4 frames/second = 4 frames
        let mut buf = AudioRingBuffer::new(1.0, 4, 16);
        for i in 0..6i16 {
            buf.push(&[i; 4]);
        }
        assert_eq!(buf.len(), 4);
        let frames = buf.drain_all();
        assert_eq!(frames.iter().map(|f| f[0]).collect::<Vec<_>>(), vec![2, 3, 4, 5]);
        assert_eq!(buf.len(), 0, "drain empties the buffer");
    }

    #[test]
    fn clear_drops_everything() {
        let mut buf = AudioRingBuffer::new(1.0, 4, 16);
        buf.push(&[1; 4]);
        buf.clear();
        assert_eq!(buf.len(), 0);
        assert!(buf.drain_all().is_empty());
    }
}
