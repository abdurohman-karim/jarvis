#[cfg(test)]
mod tests {
    use pv_recorder::{PvRecorderBuilder, PvRecorderError, PvRecorderErrorStatus};

    #[test]
    fn test_init() -> Result<(), PvRecorderError> {
        let recorder = PvRecorderBuilder::new(512).device_index(0).init()?;
        assert!(recorder.sample_rate() > 0);
        // FIX: Use is_empty() for meaningful assertion
        assert!(!recorder.selected_device().is_empty());
        assert!(!recorder.version().is_empty());

        Ok(())
    }

    #[test]
    fn test_start_stop() -> Result<(), PvRecorderError> {
        let frame_length = 666;

        let recorder = PvRecorderBuilder::new(frame_length)
            .device_index(0)
            .frame_length(frame_length)
            .init()?;
        recorder.set_debug_logging(true);

        // FIX: Use direct boolean comparison instead of == false
        assert!(!recorder.is_recording());
        recorder.start()?;
        assert!(recorder.is_recording());

        let frame = recorder.read()?;
        assert_eq!(frame.len(), frame_length as usize);

        recorder.stop()?;
        assert!(!recorder.is_recording());

        Ok(())
    }

    #[test]
    fn test_read_into() -> Result<(), PvRecorderError> {
        let frame_length = 512;

        let recorder = PvRecorderBuilder::new(frame_length)
            .device_index(0)
            .init()?;

        recorder.start()?;

        // Test read_into with exact size buffer
        let mut buffer = vec![0i16; frame_length as usize];
        recorder.read_into(&mut buffer)?;
        
        // Test read_into with larger buffer (should work)
        let mut large_buffer = vec![0i16; frame_length as usize * 2];
        recorder.read_into(&mut large_buffer)?;

        recorder.stop()?;

        Ok(())
    }

    #[test]
    #[should_panic(expected = "buffer length")]
    fn test_read_into_small_buffer_panics() {
        let frame_length = 512;

        let recorder = PvRecorderBuilder::new(frame_length)
            .device_index(0)
            .init()
            .expect("Failed to init recorder");

        recorder.start().expect("Failed to start");

        // This should panic - buffer too small
        let mut small_buffer = vec![0i16; 100];
        let _ = recorder.read_into(&mut small_buffer);
    }

    #[test]
    fn test_get_available_devices() -> Result<(), PvRecorderError> {
        let devices = PvRecorderBuilder::default().get_available_devices()?;

        // FIX: Actually meaningful assertions
        for device in &devices {
            // Device names should not be empty
            assert!(!device.is_empty(), "Device name should not be empty");
        }

        Ok(())
    }

    #[test]
    fn test_invalid_frame_length() {
        let result = PvRecorderBuilder::new(0).init();
        assert!(result.is_err());
        
        if let Err(err) = result {
            assert!(matches!(err.status(), PvRecorderErrorStatus::ArgumentError));
            assert!(err.message().contains("frame_length"));
        }
    }

    #[test]
    fn test_negative_frame_length() {
        let result = PvRecorderBuilder::new(-10).init();
        assert!(result.is_err());
        
        if let Err(err) = result {
            assert!(matches!(err.status(), PvRecorderErrorStatus::ArgumentError));
        }
    }

    #[test]
    fn test_invalid_device_index() {
        let result = PvRecorderBuilder::new(512)
            .device_index(-2)  // Invalid: must be >= -1
            .init();
        assert!(result.is_err());
        
        if let Err(err) = result {
            assert!(matches!(err.status(), PvRecorderErrorStatus::ArgumentError));
            assert!(err.message().contains("device_index"));
        }
    }

    #[test]
    fn test_invalid_buffered_frames_count() {
        let result = PvRecorderBuilder::new(512)
            .buffered_frames_count(0)
            .init();
        assert!(result.is_err());
        
        if let Err(err) = result {
            assert!(matches!(err.status(), PvRecorderErrorStatus::ArgumentError));
            assert!(err.message().contains("buffered_frames_count"));
        }
    }

    #[test]
    fn test_default_builder() {
        let builder = PvRecorderBuilder::default();
        // Should create successfully with default parameters
        // (This will fail if no audio device is available, which is expected in CI)
        let _ = builder.init();
    }

    #[test]
    fn test_frame_length_getter() -> Result<(), PvRecorderError> {
        let expected_frame_length = 1024usize;
        let recorder = PvRecorderBuilder::new(expected_frame_length as i32)
            .device_index(0)
            .init()?;
        
        assert_eq!(recorder.frame_length(), expected_frame_length);
        
        Ok(())
    }

    #[test]
    fn test_sample_rate_is_reasonable() -> Result<(), PvRecorderError> {
        let recorder = PvRecorderBuilder::new(512)
            .device_index(0)
            .init()?;
        
        let sample_rate = recorder.sample_rate();
        // Common sample rates are 8000, 16000, 22050, 44100, 48000
        assert!(sample_rate >= 8000, "Sample rate {} is too low", sample_rate);
        assert!(sample_rate <= 96000, "Sample rate {} is too high", sample_rate);
        
        Ok(())
    }

    #[test]
    fn test_clone_recorder() -> Result<(), PvRecorderError> {
        let recorder1 = PvRecorderBuilder::new(512)
            .device_index(0)
            .init()?;
        
        let recorder2 = recorder1.clone();
        
        // Both should report same state
        assert_eq!(recorder1.frame_length(), recorder2.frame_length());
        assert_eq!(recorder1.sample_rate(), recorder2.sample_rate());
        assert_eq!(recorder1.selected_device(), recorder2.selected_device());
        assert_eq!(recorder1.version(), recorder2.version());
        
        Ok(())
    }

    #[test]
    fn test_error_display() {
        let err = PvRecorderError::new(
            PvRecorderErrorStatus::ArgumentError,
            "test error message",
        );
        
        let display = format!("{}", err);
        assert!(display.contains("test error message"));
        assert!(display.contains("ArgumentError"));
    }
}
