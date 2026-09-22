use std::collections::VecDeque;

// Ring buffer of audio frames with a fixed capacity.
//
// Frames arrive ~31 times per second and are dropped as often, so buffers are recycled:
// an evicted (or drained) frame goes back to a free list and is refilled in place instead
// of allocating a fresh Vec each time.
pub struct AudioRingBuffer {
    buffer: VecDeque<Vec<i16>>,
    spare: Vec<Vec<i16>>,
    max_frames: usize,
}

impl AudioRingBuffer {
    // Create buffer that holds `seconds` worth of audio at given frame_size and sample_rate
    pub fn new(seconds: f32, frame_size: usize, sample_rate: usize) -> Self {
        let frames_per_second = sample_rate / frame_size;
        let max_frames = ((frames_per_second as f32 * seconds) as usize).max(1);

        Self {
            buffer: VecDeque::with_capacity(max_frames),
            spare: Vec::with_capacity(max_frames),
            max_frames,
        }
    }

    // Push a frame, dropping oldest if full
    pub fn push(&mut self, frame: &[i16]) {
        if self.buffer.len() >= self.max_frames {
            if let Some(evicted) = self.buffer.pop_front() {
                self.spare.push(evicted);
            }
        }

        let mut slot = self.spare.pop().unwrap_or_else(|| Vec::with_capacity(frame.len()));
        slot.clear();
        slot.extend_from_slice(frame);
        self.buffer.push_back(slot);
    }

    // Take all buffered frames out; `recycle` returns them for reuse
    pub fn drain_all(&mut self) -> Vec<Vec<i16>> {
        self.buffer.drain(..).collect()
    }

    // Give frames obtained from drain_all back to the buffer's free list
    pub fn recycle(&mut self, frames: Vec<Vec<i16>>) {
        for frame in frames {
            if self.spare.len() < self.max_frames {
                self.spare.push(frame);
            }
        }
    }

    // Get frame count
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn clear(&mut self) {
        while let Some(frame) = self.buffer.pop_front() {
            if self.spare.len() < self.max_frames {
                self.spare.push(frame);
            }
        }
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
    fn recycles_buffers_instead_of_reallocating() {
        let mut buf = AudioRingBuffer::new(1.0, 4, 16);
        for i in 0..4i16 {
            buf.push(&[i; 4]);
        }
        let frames = buf.drain_all();
        let ptrs: Vec<*const i16> = frames.iter().map(|f| f.as_ptr()).collect();
        buf.recycle(frames);

        // the same allocations must come back
        for i in 0..4i16 {
            buf.push(&[i + 10; 4]);
        }
        let reused = buf.drain_all();
        assert!(reused.iter().all(|f| ptrs.contains(&f.as_ptr())), "buffers were reallocated");
        assert_eq!(reused.iter().map(|f| f[0]).collect::<Vec<_>>(), vec![10, 11, 12, 13]);
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
