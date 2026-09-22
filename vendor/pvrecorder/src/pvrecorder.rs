/*
    Copyright 2021-2023 Picovoice Inc.

    You may not use this file except in compliance with the license. A copy of the license is located in the "LICENSE"
    file accompanying this source.

    Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on
    an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
    specific language governing permissions and limitations under the License.
*/

use std::ffi::CStr;
use std::path::Path;
use std::ptr::{addr_of_mut, NonNull};
use std::sync::Arc;
use std::{cmp::PartialEq, path::PathBuf};

use libc::{c_char, c_int};
use libloading::{Library, Symbol};

use crate::util::pv_library_path;

#[cfg(unix)]
use libloading::os::unix::Symbol as RawSymbol;
#[cfg(windows)]
use libloading::os::windows::Symbol as RawSymbol;

#[repr(C)]
struct CPvRecorder {}

/// Status codes returned by the PvRecorder C library.
#[repr(C)]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub enum PvRecorderStatus {
    SUCCESS = 0,
    OUT_OF_MEMORY = 1,
    INVALID_ARGUMENT = 2,
    INVALID_STATE = 3,
    BACKEND_ERROR = 4,
    DEVICE_ALREADY_INITIALIZED = 5,
    DEVICE_NOT_INITIALIZED = 6,
    IO_ERROR = 7,
    RUNTIME_ERROR = 8,
}

// FIX: Use c_int instead of bool for FFI safety
type PvRecorderInitFn = unsafe extern "C" fn(
    frame_length: i32,
    device_index: i32,
    buffered_frames_count: i32,
    object: *mut *mut CPvRecorder,
) -> PvRecorderStatus;
type PvRecorderDeleteFn = unsafe extern "C" fn(object: *mut CPvRecorder);
type PvRecorderStartFn = unsafe extern "C" fn(object: *mut CPvRecorder) -> PvRecorderStatus;
type PvRecorderStopFn = unsafe extern "C" fn(object: *mut CPvRecorder) -> PvRecorderStatus;
type PvRecorderReadFn =
    unsafe extern "C" fn(object: *mut CPvRecorder, pcm: *mut i16) -> PvRecorderStatus;
// FIX: Changed bool -> c_int for ABI safety
type PvRecorderSetDebugLoggingFn =
    unsafe extern "C" fn(object: *mut CPvRecorder, is_debug_logging: c_int);
// FIX: Changed bool -> c_int for ABI safety
type PvRecorderGetIsRecordingFn = unsafe extern "C" fn(object: *mut CPvRecorder) -> c_int;
type PvRecorderGetSelectedDeviceFn =
    unsafe extern "C" fn(object: *mut CPvRecorder) -> *const c_char;
type PvRecorderGetAvailableDevicesFn = unsafe extern "C" fn(
    device_list_length: *mut i32,
    device_list: *mut *mut *mut c_char,
) -> PvRecorderStatus;
type PvRecorderFreeAvailableDevicesList =
    unsafe extern "C" fn(device_list_length: i32, device_list: *mut *mut c_char);

type PvRecorderSampleRate = unsafe extern "C" fn() -> i32;
type PvRecorderVersion = unsafe extern "C" fn() -> *const c_char;

/// Categorization of errors that can occur with PvRecorder.
#[derive(Clone, Debug)]
pub enum PvRecorderErrorStatus {
    /// Error returned by the underlying C library.
    LibraryError(PvRecorderStatus),
    /// Failed to load the dynamic library or a symbol from it.
    LibraryLoadError,
    /// Invalid argument passed to a function.
    ArgumentError,
    /// Other uncategorized error.
    OtherError,
}

/// Error type for PvRecorder operations.
#[derive(Clone, Debug)]
pub struct PvRecorderError {
    status: PvRecorderErrorStatus,
    message: String,
}

impl PvRecorderError {
    pub fn new(status: PvRecorderErrorStatus, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    // FIX: Added getter for status
    /// Returns the error status category.
    #[must_use]
    pub fn status(&self) -> &PvRecorderErrorStatus {
        &self.status
    }

    // FIX: Added getter for message
    /// Returns the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for PvRecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.message, self.status)
    }
}

impl std::error::Error for PvRecorderError {}

const DEFAULT_DEVICE_INDEX: i32 = -1;
const DEFAULT_FRAME_LENGTH: i32 = 512;
const DEFAULT_BUFFERED_FRAMES_COUNT: i32 = 50;

/// Builder for creating [`PvRecorder`] instances.
///
/// # Example
/// ```no_run
/// use pv_recorder::PvRecorderBuilder;
///
/// let recorder = PvRecorderBuilder::new(512)
///     .device_index(0)
///     .init()
///     .expect("Failed to create recorder");
///
/// recorder.start().expect("Failed to start recording");
/// let samples = recorder.read().expect("Failed to read samples");
/// recorder.stop().expect("Failed to stop recording");
/// ```
pub struct PvRecorderBuilder {
    frame_length: i32,
    device_index: i32,
    buffered_frames_count: i32,
    library_path: PathBuf,
}

impl Default for PvRecorderBuilder {
    fn default() -> Self {
        Self::new(DEFAULT_FRAME_LENGTH)
    }
}

impl PvRecorderBuilder {
    /// Creates a new builder with the specified frame length.
    ///
    /// # Arguments
    /// * `frame_length` - Number of audio samples per frame. Must be greater than 0.
    #[must_use]
    pub fn new(frame_length: i32) -> Self {
        Self {
            frame_length,
            device_index: DEFAULT_DEVICE_INDEX,
            buffered_frames_count: DEFAULT_BUFFERED_FRAMES_COUNT,
            library_path: pv_library_path(),
        }
    }

    /// Sets the frame length (number of samples per read).
    // FIX: Changed to take owned self for more ergonomic chaining
    #[must_use]
    pub fn frame_length(mut self, frame_length: i32) -> Self {
        self.frame_length = frame_length;
        self
    }

    /// Sets the audio device index.
    ///
    /// Use -1 (default) for the system default device, or a specific index
    /// from [`get_available_devices`](Self::get_available_devices).
    #[must_use]
    pub fn device_index(mut self, device_index: i32) -> Self {
        self.device_index = device_index;
        self
    }

    /// Sets the number of frames to buffer internally.
    #[must_use]
    pub fn buffered_frames_count(mut self, buffered_frames_count: i32) -> Self {
        self.buffered_frames_count = buffered_frames_count;
        self
    }

    /// Sets a custom path to the pvrecorder dynamic library.
    #[must_use]
    pub fn library_path(mut self, library_path: &Path) -> Self {
        self.library_path = library_path.into();
        self
    }

    /// Initializes and returns a new [`PvRecorder`] instance.
    ///
    /// # Errors
    /// Returns an error if:
    /// - `frame_length` is not greater than 0
    /// - `device_index` is less than -1
    /// - `buffered_frames_count` is not greater than 0
    /// - The library fails to load
    /// - The device fails to initialize
    pub fn init(&self) -> Result<PvRecorder, PvRecorderError> {
        // FIX: Corrected error message - was "greater than or equal to 0"
        if self.frame_length <= 0 {
            return Err(PvRecorderError::new(
                PvRecorderErrorStatus::ArgumentError,
                format!(
                    "frame_length must be greater than 0, got: {}",
                    self.frame_length
                ),
            ));
        }

        if self.device_index < -1 {
            return Err(PvRecorderError::new(
                PvRecorderErrorStatus::ArgumentError,
                format!(
                    "device_index must be >= -1, got: {}",
                    self.device_index
                ),
            ));
        }

        if self.buffered_frames_count <= 0 {
            return Err(PvRecorderError::new(
                PvRecorderErrorStatus::ArgumentError,
                format!(
                    "buffered_frames_count must be greater than 0, got: {}",
                    self.buffered_frames_count
                ),
            ));
        }

        let recorder_inner = PvRecorderInner::init(
            self.frame_length,
            self.device_index,
            self.buffered_frames_count,
            &self.library_path,
        );
        recorder_inner.map(|inner| PvRecorder {
            inner: Arc::new(inner),
        })
    }

    /// Returns a list of available audio input devices.
    ///
    /// The index of each device in the returned vector can be used with
    /// [`device_index`](Self::device_index).
    pub fn get_available_devices(&self) -> Result<Vec<String>, PvRecorderError> {
        PvRecorderInner::get_available_devices(&self.library_path)
    }
}

/// Audio recorder for capturing microphone input.
///
/// # Thread Safety
/// `PvRecorder` is `Send` and can be moved between threads. It is also `Sync`,
/// but care should be taken when calling `read()` from multiple threads
/// simultaneously as audio samples may be split across threads unpredictably.
///
/// # Example
/// ```no_run
/// use pv_recorder::PvRecorderBuilder;
///
/// let recorder = PvRecorderBuilder::default().init()?;
/// println!("Using device: {}", recorder.selected_device());
/// println!("Sample rate: {} Hz", recorder.sample_rate());
///
/// recorder.start()?;
/// while recorder.is_recording() {
///     let samples = recorder.read()?;
///     // Process samples...
/// }
/// # Ok::<(), pv_recorder::PvRecorderError>(())
/// ```
#[derive(Clone)]
pub struct PvRecorder {
    inner: Arc<PvRecorderInner>,
}

impl std::fmt::Debug for PvRecorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PvRecorder")
            .field("frame_length", &self.frame_length())
            .field("sample_rate", &self.sample_rate())
            .field("selected_device", &self.selected_device())
            .field("version", &self.version())
            .field("is_recording", &self.is_recording())
            .finish()
    }
}

impl PvRecorder {
    /// Starts recording audio from the selected device.
    ///
    /// # Errors
    /// Returns an error if the recorder is already started or the device fails.
    pub fn start(&self) -> Result<(), PvRecorderError> {
        self.inner.start()
    }

    /// Stops recording audio.
    ///
    /// # Errors
    /// Returns an error if the recorder is not started.
    pub fn stop(&self) -> Result<(), PvRecorderError> {
        self.inner.stop()
    }

    /// Reads one frame of audio samples.
    ///
    /// This method blocks until a full frame is available.
    ///
    /// # Returns
    /// A vector of `i16` samples with length equal to [`frame_length`](Self::frame_length).
    ///
    /// # Errors
    /// Returns an error if the recorder is not started or a read error occurs.
    pub fn read(&self) -> Result<Vec<i16>, PvRecorderError> {
        self.inner.read()
    }

    /// Reads audio samples into the provided buffer.
    ///
    /// This is more efficient than [`read`](Self::read) as it avoids allocation.
    ///
    /// # Panics
    /// Panics if `buffer.len() < self.frame_length()`.
    pub fn read_into(&self, buffer: &mut [i16]) -> Result<(), PvRecorderError> {
        self.inner.read_into(buffer)
    }

    /// Enables or disables debug logging.
    pub fn set_debug_logging(&self, is_debug_logging_enabled: bool) {
        self.inner.set_debug_logging(is_debug_logging_enabled)
    }

    /// Returns the number of samples per frame.
    #[must_use]
    pub fn frame_length(&self) -> usize {
        self.inner.frame_length() as usize
    }

    /// Returns whether the recorder is currently recording.
    #[must_use]
    pub fn is_recording(&self) -> bool {
        self.inner.is_recording()
    }

    /// Returns the sample rate in Hz (typically 16000).
    #[must_use]
    pub fn sample_rate(&self) -> usize {
        self.inner.sample_rate() as usize
    }

    /// Returns the name of the selected audio device.
    // FIX: Return &str instead of String to avoid allocation
    #[must_use]
    pub fn selected_device(&self) -> &str {
        &self.inner.selected_device
    }

    /// Returns the version string of the pvrecorder library.
    // FIX: Return &str instead of String to avoid allocation
    #[must_use]
    pub fn version(&self) -> &str {
        &self.inner.version
    }
}

unsafe fn load_library_fn<T>(
    library: &Library,
    function_name: &[u8],
) -> Result<RawSymbol<T>, PvRecorderError> {
    // SAFETY: Caller ensures the library outlives the returned symbol.
    // The function signature T must match the actual function in the library.
    unsafe {
        library
            .get(function_name)
            .map(|s: Symbol<T>| s.into_raw())
            .map_err(|err| {
                PvRecorderError::new(
                    PvRecorderErrorStatus::LibraryLoadError,
                    format!(
                        "Failed to load function symbol from pvrecorder library: {}",
                        err
                    ),
                )
            })
    }
}

fn check_fn_call_status(
    status: PvRecorderStatus,
    function_name: &str,
) -> Result<(), PvRecorderError> {
    match status {
        PvRecorderStatus::SUCCESS => Ok(()),
        _ => Err(PvRecorderError::new(
            PvRecorderErrorStatus::LibraryError(status),
            format!(
                "Function '{}' in the pvrecorder library failed",
                function_name
            ),
        )),
    }
}

struct PvRecorderInnerVTable {
    pv_recorder_init: RawSymbol<PvRecorderInitFn>,
    pv_recorder_delete: RawSymbol<PvRecorderDeleteFn>,
    pv_recorder_start: RawSymbol<PvRecorderStartFn>,
    pv_recorder_stop: RawSymbol<PvRecorderStopFn>,
    pv_recorder_read: RawSymbol<PvRecorderReadFn>,
    pv_recorder_set_debug_logging: RawSymbol<PvRecorderSetDebugLoggingFn>,
    pv_recorder_get_is_recording: RawSymbol<PvRecorderGetIsRecordingFn>,
    pv_recorder_get_selected_device: RawSymbol<PvRecorderGetSelectedDeviceFn>,
    pv_recorder_get_available_devices: RawSymbol<PvRecorderGetAvailableDevicesFn>,
    pv_recorder_free_available_devices: RawSymbol<PvRecorderFreeAvailableDevicesList>,
    pv_recorder_sample_rate: RawSymbol<PvRecorderSampleRate>,
    pv_recorder_version: RawSymbol<PvRecorderVersion>,

    _lib_guard: Library,
}

impl PvRecorderInnerVTable {
    pub fn new(lib: Library) -> Result<Self, PvRecorderError> {
        // SAFETY: The library is held by this struct via `_lib_guard`,
        // ensuring all symbols remain valid for the struct's lifetime.
        unsafe {
            Ok(Self {
                pv_recorder_init: load_library_fn(&lib, b"pv_recorder_init")?,
                pv_recorder_delete: load_library_fn(&lib, b"pv_recorder_delete")?,
                pv_recorder_start: load_library_fn(&lib, b"pv_recorder_start")?,
                pv_recorder_stop: load_library_fn(&lib, b"pv_recorder_stop")?,
                pv_recorder_read: load_library_fn(&lib, b"pv_recorder_read")?,
                pv_recorder_set_debug_logging: load_library_fn(
                    &lib,
                    b"pv_recorder_set_debug_logging",
                )?,
                pv_recorder_get_is_recording: load_library_fn(
                    &lib,
                    b"pv_recorder_get_is_recording",
                )?,
                pv_recorder_get_selected_device: load_library_fn(
                    &lib,
                    b"pv_recorder_get_selected_device",
                )?,
                pv_recorder_get_available_devices: load_library_fn(
                    &lib,
                    b"pv_recorder_get_available_devices",
                )?,
                pv_recorder_free_available_devices: load_library_fn(
                    &lib,
                    b"pv_recorder_free_available_devices",
                )?,
                pv_recorder_sample_rate: load_library_fn(&lib, b"pv_recorder_sample_rate")?,
                pv_recorder_version: load_library_fn(&lib, b"pv_recorder_version")?,

                _lib_guard: lib,
            })
        }
    }
}

struct PvRecorderInner {
    // FIX: Use NonNull for better safety semantics
    cpvrecorder: NonNull<CPvRecorder>,
    frame_length: i32,
    sample_rate: i32,
    selected_device: String,
    version: String,
    vtable: PvRecorderInnerVTable,
}

impl PvRecorderInner {
    pub fn init(
        frame_length: i32,
        device_index: i32,
        buffered_frames_count: i32,
        library_path: &Path,
    ) -> Result<Self, PvRecorderError> {
        // FIX: Removed duplicate validation - builder already validates

        let lib = unsafe { Library::new(library_path) }.map_err(|err| {
            PvRecorderError::new(
                PvRecorderErrorStatus::LibraryLoadError,
                format!("Failed to load pvrecorder dynamic library: {}", err),
            )
        })?;
        let vtable = PvRecorderInnerVTable::new(lib)?;

        let mut cpvrecorder_ptr = std::ptr::null_mut();

        unsafe {
            let status = (vtable.pv_recorder_init)(
                frame_length,
                device_index,
                buffered_frames_count,
                addr_of_mut!(cpvrecorder_ptr),
            );
            check_fn_call_status(status, "pv_recorder_init")?;
        }

        // FIX: Added NULL check after init
        let cpvrecorder = NonNull::new(cpvrecorder_ptr).ok_or_else(|| {
            PvRecorderError::new(
                PvRecorderErrorStatus::OtherError,
                "pv_recorder_init returned SUCCESS but pointer is null",
            )
        })?;

        let selected_device = unsafe {
            let selected_device_c = (vtable.pv_recorder_get_selected_device)(cpvrecorder.as_ptr());
            String::from(CStr::from_ptr(selected_device_c).to_str().map_err(|_| {
                PvRecorderError::new(
                    PvRecorderErrorStatus::OtherError,
                    "Failed to convert selected device string",
                )
            })?)
        };

        let sample_rate = unsafe { (vtable.pv_recorder_sample_rate)() };

        let version = unsafe {
            let version_c = (vtable.pv_recorder_version)();
            String::from(CStr::from_ptr(version_c).to_str().map_err(|_| {
                PvRecorderError::new(
                    PvRecorderErrorStatus::OtherError,
                    "Failed to convert version string",
                )
            })?)
        };

        Ok(Self {
            cpvrecorder,
            frame_length,
            sample_rate,
            selected_device,
            version,
            vtable,
        })
    }

    fn start(&self) -> Result<(), PvRecorderError> {
        let status = unsafe { (self.vtable.pv_recorder_start)(self.cpvrecorder.as_ptr()) };
        check_fn_call_status(status, "pv_recorder_start")
    }

    fn stop(&self) -> Result<(), PvRecorderError> {
        let status = unsafe { (self.vtable.pv_recorder_stop)(self.cpvrecorder.as_ptr()) };
        check_fn_call_status(status, "pv_recorder_stop")
    }

    fn read(&self) -> Result<Vec<i16>, PvRecorderError> {
        let mut frame = vec![0; self.frame_length() as usize];
        self.read_into(&mut frame)?;
        Ok(frame)
    }

    fn read_into(&self, buffer: &mut [i16]) -> Result<(), PvRecorderError> {
        assert!(
            buffer.len() >= self.frame_length() as usize,
            "buffer length {} is less than frame_length {}",
            buffer.len(),
            self.frame_length()
        );
        let status =
            unsafe { (self.vtable.pv_recorder_read)(self.cpvrecorder.as_ptr(), buffer.as_mut_ptr()) };
        check_fn_call_status(status, "pv_recorder_read")
    }

    fn set_debug_logging(&self, is_debug_logging_enabled: bool) {
        // FIX: Convert bool to c_int for FFI safety
        unsafe {
            (self.vtable.pv_recorder_set_debug_logging)(
                self.cpvrecorder.as_ptr(),
                is_debug_logging_enabled as c_int,
            )
        };
    }

    fn frame_length(&self) -> i32 {
        self.frame_length
    }

    fn is_recording(&self) -> bool {
        // FIX: Convert c_int to bool
        unsafe { (self.vtable.pv_recorder_get_is_recording)(self.cpvrecorder.as_ptr()) != 0 }
    }

    fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    pub fn get_available_devices<P: AsRef<Path>>(
        library_path: P,
    ) -> Result<Vec<String>, PvRecorderError> {
        let lib = unsafe { Library::new(library_path.as_ref()) }.map_err(|err| {
            PvRecorderError::new(
                PvRecorderErrorStatus::LibraryLoadError,
                format!("Failed to load pvrecorder dynamic library: {}", err),
            )
        })?;

        let vtable = PvRecorderInnerVTable::new(lib)?;

        let mut device_list = Vec::new();
        let mut device_list_length = 0;

        unsafe {
            let mut device_list_ptr: *mut c_char = std::ptr::null_mut();
            let mut device_list_ptr_ptr: *mut *mut c_char = addr_of_mut!(device_list_ptr);

            let status = (vtable.pv_recorder_get_available_devices)(
                addr_of_mut!(device_list_length),
                addr_of_mut!(device_list_ptr_ptr),
            );
            check_fn_call_status(status, "pv_recorder_get_available_devices")?;

            for i in 0..device_list_length as usize {
                let device = CStr::from_ptr(*device_list_ptr_ptr.add(i));
                device_list.push(String::from(device.to_str().map_err(|_| {
                    PvRecorderError::new(
                        PvRecorderErrorStatus::OtherError,
                        "Failed to convert device strings",
                    )
                })?));
            }

            (vtable.pv_recorder_free_available_devices)(device_list_length, device_list_ptr_ptr);
        }
        Ok(device_list)
    }
}

// SAFETY: The underlying C library (pvrecorder) is thread-safe for all operations
// on a single recorder instance. The raw pointer `cpvrecorder` is encapsulated
// and only accessed through the vtable function pointers. The NonNull wrapper
// ensures the pointer is always valid, and the Arc wrapper in PvRecorder ensures
// proper shared ownership semantics.
unsafe impl Send for PvRecorderInner {}
unsafe impl Sync for PvRecorderInner {}

impl Drop for PvRecorderInner {
    fn drop(&mut self) {
        unsafe {
            (self.vtable.pv_recorder_delete)(self.cpvrecorder.as_ptr());
        }
    }
}
