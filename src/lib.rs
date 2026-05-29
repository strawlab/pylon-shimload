//! Runtime-loaded Rust bindings for Basler Pylon cameras.
//!
//! This crate loads a C ABI shim library at runtime. The shim library links
//! against the Pylon C++ SDK. This allows the crate to be used without linking
//! to the Pylon library at compile time, and also allows it to be used with
//! different versions of the Pylon library without recompilation. Set the
//! `PYLON_CABI` environment variable to point at the shim if it is not
//! installed in a standard library location.

use std::ffi::{c_char, c_int, c_void};

mod runtime_impl;
mod shim_loader;

#[cfg(all(not(target_os = "windows"), feature = "stream"))]
mod stream_unix;

#[cfg(feature = "stream")]
use std::cell::RefCell;

#[cfg(all(target_os = "windows", feature = "stream"))]
use std::thread::JoinHandle;

#[cfg(all(target_os = "windows", feature = "stream"))]
mod stream_windows;

pub(crate) const EXPECTED_CABI_VERSION: u32 = 1;

// =========================================================================
// Error type
// =========================================================================

/// Errors returned by this crate.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PylonError {
    /// A general error message produced by this crate.
    Msg(String),
    /// A shim call returned an error string.
    ShimCallFailed {
        /// Logical shim operation name.
        op: &'static str,
        /// Rust callsite where the failing shim function was invoked.
        callsite: String,
        /// Error message returned by the shim.
        err_str: String,
    },
    /// A shim call reported success but returned invalid output.
    InvalidShimOutput {
        /// Logical shim operation name.
        op: &'static str,
        /// Description of which output invariant failed.
        detail: String,
    },
    /// The shim library could not be opened.
    DlOpenFailed {
        /// The attempted shim library path.
        path: std::ffi::OsString,
        source: String,
    },
    /// A lower-level error reported while loading or validating the shim.
    ShimError(ShimError),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
/// Errors encountered while loading the C ABI shim.
pub enum ShimError {
    /// A required symbol could not be loaded from the shim.
    SymbolLoadFailed {
        /// The shim library path.
        path: std::ffi::OsString,
        /// The missing or invalid symbol name.
        symbol: String,
        /// The dynamic loader error string.
        err_str: String,
    },
    /// The shim returned a null API table pointer.
    NullApi {
        /// The shim library path.
        path: std::ffi::OsString,
    },
    /// The shim API table is smaller than this crate expects.
    ApiTableTooSmall {
        /// The shim library path.
        path: std::ffi::OsString,
        /// The API table size reported by the shim.
        got: u32,
        /// The minimum API table size required by this crate.
        need: u32,
    },
    /// The shim ABI version does not match this crate.
    IncompatibleAbiVersion {
        /// The shim library path.
        path: std::ffi::OsString,
        /// The ABI version reported by the shim.
        got: u32,
        /// The ABI version required by this crate.
        need: u32,
    },
}

impl PylonError {
    fn new(msg: String) -> Self {
        PylonError::Msg(msg)
    }
}

impl From<std::str::Utf8Error> for PylonError {
    fn from(_: std::str::Utf8Error) -> PylonError {
        PylonError::new("Cannot convert C++ string to UTF-8".to_string())
    }
}

impl From<std::io::Error> for PylonError {
    fn from(orig: std::io::Error) -> PylonError {
        PylonError::new(orig.to_string())
    }
}

impl std::fmt::Display for PylonError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PylonError::Msg(msg) => write!(f, "PylonError({})", msg),
            PylonError::ShimCallFailed {
                op,
                callsite,
                err_str,
            } => write!(
                f,
                "PylonError(ShimCallFailed {op} at {callsite}: {err_str})"
            ),
            PylonError::InvalidShimOutput { op, detail } => {
                write!(f, "PylonError(InvalidShimOutput {op}: {detail})")
            }
            PylonError::DlOpenFailed { path,  source } => write!(
                f,
                "There was a problem opening the `pylon-cabi` shim library. The path was specified \
                as {path:?}. You can force a specific path \
                to the shim library using the `PYLON_CABI` environment variable. Shim libraries can \
                be downloaded from https://strawlab.org/assets/pylon-cabi/precompiled/ or built \
                from source. You need v{EXPECTED_CABI_VERSION} of the shim library for this version of `pylon-shimload`.\
                \n\nThe source of the error was:\n\n\
                {source}"
            ),
            PylonError::ShimError(err) => write!(f, "PylonError(ShimError: {err:?})"),
        }
    }
}

impl std::error::Error for PylonError {}

/// Convenient result type used throughout the crate.
pub type PylonResult<T> = Result<T, PylonError>;

// =========================================================================
// Public enums
// =========================================================================

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// How timeout conditions are reported by grab operations.
pub enum TimeoutHandling {
    /// Return `Ok(false)` when the timeout expires.
    Return = 0,
    /// Convert the timeout into an error reported by Pylon.
    ThrowException = 1,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Buffer handling strategy used while grabbing.
pub enum GrabStrategy {
    /// Deliver every image in order.
    OneByOne = 0,
    /// Keep only the most recent image.
    LatestImageOnly = 1,
    /// Keep a rolling set of recent images.
    LatestImages = 2,
    /// Wait for the next image that arrives after the call.
    UpcomingImage = 3,
}

// =========================================================================
// Internal helpers (shim-backed)
// =========================================================================

#[inline]
#[track_caller]
fn shim_check_op(op: &'static str, err: *const c_char) -> PylonResult<()> {
    shim_loader::check_err(err).map_err(|err_str| {
        let loc = std::panic::Location::caller();
        PylonError::ShimCallFailed {
            op,
            callsite: format!("{}:{}", loc.file(), loc.line()),
            err_str,
        }
    })
}

#[inline]
#[track_caller]
fn shim_check(err: *const c_char) -> PylonResult<()> {
    shim_check_op("shim_call", err)
}

#[inline]
fn ensure_non_null<T>(op: &'static str, what: &'static str, ptr: *const T) -> PylonResult<()> {
    if ptr.is_null() {
        Err(PylonError::InvalidShimOutput {
            op,
            detail: format!("{what} was null"),
        })
    } else {
        Ok(())
    }
}

macro_rules! shim_call {
    ($shim:expr, $func:ident($($arg:expr),* $(,)?)) => {{
        let err = unsafe { (($shim).$func)($($arg),*) };
        shim_check_for!($func, err)
    }};
}

macro_rules! shim_check_for {
    ($op:ident, $err:expr) => {
        shim_check_op(stringify!($op), $err)
    };
}

macro_rules! ensure_non_null_for {
    ($op:ident, $what:literal, $ptr:expr) => {
        ensure_non_null(stringify!($op), $what, $ptr)
    };
}

// =========================================================================
// Public types
// =========================================================================

/// Pylon runtime lifecycle management.
///
/// Most callers do not need this module — the runtime initializes
/// automatically on the first call to [`enumerate_devices`],
/// [`create_first_device`], or [`create_device`], and shuts down when
/// the last [`InstantCamera`] or [`DeviceInfo`] is dropped.
///
/// Use this module when you need explicit control: early initialization,
/// keeping the runtime alive between handle lifetimes, or deterministic
/// teardown.
pub mod runtime {
    use crate::{runtime_impl, PylonResult, PylonVersion};

    /// Keeps the Pylon runtime alive for explicit lifecycle management.
    ///
    /// Most callers do not need this type.  The runtime is initialized
    /// automatically on the first call to [`crate::enumerate_devices`],
    /// [`crate::create_first_device`], or [`crate::create_device`], and is
    /// kept alive as long as any [`crate::InstantCamera`] or
    /// [`crate::DeviceInfo`] remains in scope.
    ///
    /// Obtain one via [`init`].
    pub use crate::runtime_impl::RuntimeGuard;

    /// Explicitly initializes the Pylon runtime and returns a guard that
    /// keeps it alive until dropped.
    ///
    /// Calling this is **not required** before normal use — the runtime
    /// initializes automatically on the first call to
    /// [`crate::enumerate_devices`], [`crate::create_first_device`], or
    /// [`crate::create_device`].
    ///
    /// Use `runtime::init()` when you need to:
    /// * Detect a missing Pylon installation at startup, before opening any
    ///   camera.
    /// * Keep the runtime alive across a window where no camera handles
    ///   exist.
    /// * Drive deterministic teardown via [`shutdown`].
    ///
    /// The guard releases its hold on the runtime when dropped; if it is the
    /// last holder, the runtime is terminated at that point.
    pub fn init() -> PylonResult<RuntimeGuard> {
        RuntimeGuard::new()
    }

    /// Returns the version of the loaded Pylon runtime.
    ///
    /// Initializes the runtime if it has not been initialized yet.
    pub fn version() -> PylonResult<PylonVersion> {
        runtime_impl::runtime_version()
    }

    /// Explicitly terminates the Pylon runtime.
    ///
    /// Returns an error if any [`crate::InstantCamera`], [`crate::DeviceInfo`],
    /// or [`RuntimeGuard`] is still alive.  Intended for use alongside
    /// [`init`] when deterministic teardown is required.
    ///
    /// Under normal usage the runtime terminates automatically when the last
    /// handle is dropped, so calling this is not necessary.
    pub fn shutdown() -> PylonResult<()> {
        runtime_impl::shutdown()
    }
}

/// Pylon version information.
#[derive(Debug)]
pub struct PylonVersion {
    /// Major version number.
    pub major: u32,
    /// Minor version number.
    pub minor: u32,
    /// Patch-level version number.
    pub subminor: u32,
    /// Build number.
    pub build: u32,
}

/// Returns the version of the loaded Pylon runtime.
pub fn version() -> Result<PylonVersion, PylonError> {
    runtime::version()
}

/// Enumerates all currently available camera devices.
pub fn enumerate_devices() -> PylonResult<Vec<DeviceInfo>> {
    let runtime = runtime_impl::acquire_runtime()?;
    let mut raw_arr: *mut *mut c_void = std::ptr::null_mut();
    let mut count: usize = 0;
    shim_call!(
        shim_loader::shim(),
        tl_factory_enumerate_devices(&mut raw_arr, &mut count)
    )?;
    if count > 0 {
        ensure_non_null_for!(tl_factory_enumerate_devices, "device array", raw_arr)?;
    }
    let mut result = Vec::with_capacity(count);
    unsafe {
        for i in 0..count {
            result.push(DeviceInfo {
                ptr: *raw_arr.add(i),
                runtime: runtime.clone(),
            });
        }
        (shim_loader::shim().pylon_cxx_free_ptr)(raw_arr as *mut _);
    }
    Ok(result)
}

/// Creates the first available camera device.
pub fn create_first_device() -> PylonResult<InstantCamera> {
    let runtime = runtime_impl::acquire_runtime()?;
    let mut ptr: *mut c_void = std::ptr::null_mut();
    shim_call!(
        shim_loader::shim(),
        tl_factory_create_first_device(&mut ptr)
    )?;
    ensure_non_null_for!(tl_factory_create_first_device, "camera pointer", ptr)?;
    Ok(InstantCamera::from_ptr(runtime, ptr))
}

/// Creates a camera handle from previously discovered device info.
pub fn create_device(device_info: &DeviceInfo) -> PylonResult<InstantCamera> {
    let runtime = runtime_impl::acquire_runtime()?;
    let mut ptr: *mut c_void = std::ptr::null_mut();
    shim_call!(
        shim_loader::shim(),
        tl_factory_create_device(device_info.ptr, &mut ptr)
    )?;
    ensure_non_null_for!(tl_factory_create_device, "camera pointer", ptr)?;
    Ok(InstantCamera::from_ptr(runtime, ptr))
}

// -------------------------------------------------------------------------
// InstantCamera
// -------------------------------------------------------------------------

/// Camera handle used to open, configure, and grab from a device.
pub struct InstantCamera {
    ptr: *mut c_void,
    #[cfg(all(not(target_os = "windows"), feature = "stream"))]
    pub(crate) fd: RefCell<Option<tokio::io::unix::AsyncFd<std::os::unix::io::RawFd>>>,
    #[cfg(all(target_os = "windows", feature = "stream"))]
    pub(crate) wait_thread: RefCell<Option<JoinHandle<()>>>,
    /// Kept last so that runtime lease outlives native pointer drops.
    _runtime: runtime_impl::RuntimeLease,
}

unsafe impl Send for InstantCamera {}

impl InstantCamera {
    fn from_ptr(runtime: runtime_impl::RuntimeLease, ptr: *mut c_void) -> Self {
        InstantCamera {
            ptr,
            _runtime: runtime,
            #[cfg(all(not(target_os = "windows"), feature = "stream"))]
            fd: RefCell::new(None),
            #[cfg(all(target_os = "windows", feature = "stream"))]
            wait_thread: RefCell::new(None),
        }
    }

    /// Returns metadata for the attached device.
    pub fn device_info(&self) -> PylonResult<DeviceInfo> {
        {
            let mut out: *mut c_void = std::ptr::null_mut();
            shim_call!(
                shim_loader::shim(),
                instant_camera_get_device_info(self.ptr, &mut out)
            )?;
            ensure_non_null_for!(instant_camera_get_device_info, "device info pointer", out)?;
            Ok(DeviceInfo {
                ptr: out,
                runtime: self._runtime.clone(),
            })
        }
    }

    /// Opens the camera.
    pub fn open(&self) -> PylonResult<()> {
        shim_call!(shim_loader::shim(), instant_camera_open(self.ptr))
    }

    /// Returns whether the camera is open.
    pub fn is_open(&self) -> PylonResult<bool> {
        {
            let mut out: c_int = 0;
            shim_call!(
                shim_loader::shim(),
                instant_camera_is_open(self.ptr, &mut out)
            )?;
            Ok(out != 0)
        }
    }

    /// Closes the camera.
    pub fn close(&self) -> PylonResult<()> {
        shim_call!(shim_loader::shim(), instant_camera_close(self.ptr))
    }

    /// Starts grabbing using the supplied options.
    pub fn start_grabbing(&self, options: &GrabOptions) -> PylonResult<()> {
        {
            // We assign the wait-object fd here for using it in the stream.
            #[cfg(all(not(target_os = "windows"), feature = "stream"))]
            {
                if tokio::runtime::Handle::try_current().is_ok() {
                    self.fd.replace(Some(tokio::io::unix::AsyncFd::new(
                        self.get_grab_result_fd()?,
                    )?));
                }
            }

            let s = shim_loader::shim();
            match (options.count, options.strategy) {
                (Some(count), Some(strategy)) => shim_call!(
                    s,
                    instant_camera_start_grabbing_with_count_and_strategy(
                        self.ptr,
                        count,
                        strategy as c_int
                    )
                ),
                (Some(count), None) => {
                    shim_call!(s, instant_camera_start_grabbing_with_count(self.ptr, count))
                }
                (None, Some(strategy)) => shim_call!(
                    s,
                    instant_camera_start_grabbing_with_strategy(self.ptr, strategy as c_int)
                ),
                (None, None) => shim_call!(s, instant_camera_start_grabbing(self.ptr)),
            }
        }
    }

    /// Stops any active grab operation.
    pub fn stop_grabbing(&self) -> PylonResult<()> {
        {
            shim_call!(shim_loader::shim(), instant_camera_stop_grabbing(self.ptr))?;
            #[cfg(all(not(target_os = "windows"), feature = "stream"))]
            self.fd.replace(None);
            #[cfg(all(target_os = "windows", feature = "stream"))]
            self.wait_thread.replace(None);
            Ok(())
        }
    }

    /// Returns `true` while the camera is actively grabbing.
    pub fn is_grabbing(&self) -> bool {
        unsafe { (shim_loader::shim().instant_camera_is_grabbing)(self.ptr) != 0 }
    }

    /// Waits for the next grab result.
    ///
    /// Returns `Ok(true)` when a result was retrieved.
    pub fn retrieve_result(
        &self,
        timeout_ms: u32,
        grab_result: &mut GrabResult,
        timeout_handling: TimeoutHandling,
    ) -> PylonResult<bool> {
        {
            let mut grabbed: c_int = 0;
            let err = unsafe {
                (shim_loader::shim().instant_camera_retrieve_result)(
                    self.ptr,
                    timeout_ms,
                    grab_result.ptr,
                    timeout_handling as c_int,
                    &mut grabbed,
                )
            };
            shim_check_for!(instant_camera_retrieve_result, err)?;
            Ok(grabbed != 0)
        }
    }

    #[cfg(all(not(target_os = "windows"), feature = "stream"))]
    /// Returns the wait-object file descriptor used by the `stream` feature.
    pub fn get_grab_result_fd(&self) -> PylonResult<std::os::unix::io::RawFd> {
        {
            let mut fd: c_int = -1;
            shim_call!(
                shim_loader::shim(),
                instant_camera_wait_object_fd(self.ptr, &mut fd)
            )?;
            Ok(fd)
        }
    }

    #[cfg(all(target_os = "windows", feature = "stream"))]
    pub(crate) fn get_grab_result_wait_object(&self) -> PylonResult<WaitObject> {
        {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            shim_call!(
                shim_loader::shim(),
                instant_camera_wait_object(self.ptr, &mut ptr)
            )?;
            ensure_non_null_for!(instant_camera_wait_object, "wait object pointer", ptr)?;
            Ok(WaitObject(ptr))
        }
    }
}

impl Drop for InstantCamera {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (shim_loader::shim().instant_camera_destroy)(self.ptr) }
        }
    }
}

// --- NodeMap ---------------------------------------------------------------

/// Borrowed GenICam node map tied to the lifetime of its parent object.
pub struct NodeMap<'parent> {
    ptr: *const c_void,
    _marker: std::marker::PhantomData<&'parent ()>,
}

impl InstantCamera {
    /// Returns the camera node map.
    pub fn node_map<'a>(&'a self) -> PylonResult<NodeMap<'a>> {
        {
            let mut ptr: *const c_void = std::ptr::null();
            let err =
                unsafe { (shim_loader::shim().instant_camera_get_node_map)(self.ptr, &mut ptr) };
            shim_check_for!(instant_camera_get_node_map, err)?;
            ensure_non_null_for!(instant_camera_get_node_map, "node map pointer", ptr)?;
            Ok(NodeMap {
                ptr,
                _marker: std::marker::PhantomData,
            })
        }
    }
    /// Returns the transport-layer node map.
    pub fn tl_node_map<'a>(&'a self) -> PylonResult<NodeMap<'a>> {
        {
            let mut ptr: *const c_void = std::ptr::null();
            let err =
                unsafe { (shim_loader::shim().instant_camera_get_tl_node_map)(self.ptr, &mut ptr) };
            shim_check_for!(instant_camera_get_tl_node_map, err)?;
            ensure_non_null_for!(instant_camera_get_tl_node_map, "tl node map pointer", ptr)?;
            Ok(NodeMap {
                ptr,
                _marker: std::marker::PhantomData,
            })
        }
    }
    /// Returns the stream-grabber node map.
    pub fn stream_grabber_node_map<'a>(&'a self) -> PylonResult<NodeMap<'a>> {
        {
            let mut ptr: *const c_void = std::ptr::null();
            let err = unsafe {
                (shim_loader::shim().instant_camera_get_stream_grabber_node_map)(self.ptr, &mut ptr)
            };
            shim_check_for!(instant_camera_get_stream_grabber_node_map, err)?;
            ensure_non_null(
                stringify!(instant_camera_get_stream_grabber_node_map),
                "stream grabber node map pointer",
                ptr,
            )?;
            Ok(NodeMap {
                ptr,
                _marker: std::marker::PhantomData,
            })
        }
    }
    /// Returns the event-grabber node map.
    pub fn event_grabber_node_map<'a>(&'a self) -> PylonResult<NodeMap<'a>> {
        {
            let mut ptr: *const c_void = std::ptr::null();
            let err = unsafe {
                (shim_loader::shim().instant_camera_get_event_grabber_node_map)(self.ptr, &mut ptr)
            };
            shim_check_for!(instant_camera_get_event_grabber_node_map, err)?;
            ensure_non_null(
                stringify!(instant_camera_get_event_grabber_node_map),
                "event grabber node map pointer",
                ptr,
            )?;
            Ok(NodeMap {
                ptr,
                _marker: std::marker::PhantomData,
            })
        }
    }
    /// Returns the instant-camera node map.
    pub fn instant_camera_node_map<'a>(&'a self) -> PylonResult<NodeMap<'a>> {
        {
            let mut ptr: *const c_void = std::ptr::null();
            let err = unsafe {
                (shim_loader::shim().instant_camera_get_instant_camera_node_map)(self.ptr, &mut ptr)
            };
            shim_check_for!(instant_camera_get_instant_camera_node_map, err)?;
            ensure_non_null(
                stringify!(instant_camera_get_instant_camera_node_map),
                "instant camera node map pointer",
                ptr,
            )?;
            Ok(NodeMap {
                ptr,
                _marker: std::marker::PhantomData,
            })
        }
    }
}

impl<'parent> NodeMap<'parent> {
    /// Loads feature settings from a file.
    pub fn load<P: AsRef<std::path::Path>>(&self, path: P, validate: bool) -> PylonResult<()> {
        {
            let filename = path_to_string(path)?;
            let err = unsafe {
                (shim_loader::shim().node_map_load)(
                    self.ptr,
                    filename.as_ptr() as *const c_char,
                    filename.len(),
                    validate as c_int,
                )
            };
            shim_check(err)
        }
    }
    /// Saves feature settings to a file.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> PylonResult<()> {
        {
            let filename = path_to_string(path)?;
            let err = unsafe {
                (shim_loader::shim().node_map_save)(
                    self.ptr,
                    filename.as_ptr() as *const c_char,
                    filename.len(),
                )
            };
            shim_check(err)
        }
    }
    /// Loads feature settings from a serialized string.
    pub fn load_from_string(&self, features: String, validate: bool) -> PylonResult<()> {
        {
            let err = unsafe {
                (shim_loader::shim().node_map_load_from_string)(
                    self.ptr,
                    features.as_ptr() as *const c_char,
                    features.len(),
                    validate as c_int,
                )
            };
            shim_check(err)
        }
    }
    /// Serializes the node map into a string.
    pub fn save_to_string(&self) -> PylonResult<String> {
        {
            let mut out: *mut c_char = std::ptr::null_mut();
            let err = unsafe { (shim_loader::shim().node_map_save_to_string)(self.ptr, &mut out) };
            shim_check_for!(node_map_save_to_string, err)?;
            ensure_non_null_for!(node_map_save_to_string, "serialized node map string", out)?;
            Ok(unsafe { shim_loader::take_str(out) })
        }
    }

    fn get_param_raw(
        &self,
        name: &str,
        getter: unsafe extern "C" fn(
            *const c_void,
            *const c_char,
            usize,
            *mut *mut c_void,
        ) -> *const c_char,
    ) -> PylonResult<*mut c_void> {
        {
            let mut out: *mut c_void = std::ptr::null_mut();
            let err = unsafe {
                getter(
                    self.ptr,
                    name.as_ptr() as *const c_char,
                    name.len(),
                    &mut out,
                )
            };
            shim_check(err)?;
            ensure_non_null_for!(node_map_get_parameter, "parameter pointer", out)?;
            Ok(out)
        }
    }

    /// Returns a boolean node by name.
    pub fn boolean_node(&self, name: &str) -> PylonResult<BooleanNode> {
        Ok(BooleanNode {
            name: name.to_string(),
            ptr: self.get_param_raw(name, shim_loader::shim().node_map_get_boolean_parameter)?,
        })
    }
    /// Returns an integer node by name.
    pub fn integer_node(&self, name: &str) -> PylonResult<IntegerNode> {
        Ok(IntegerNode {
            name: name.to_string(),
            ptr: self.get_param_raw(name, shim_loader::shim().node_map_get_integer_parameter)?,
        })
    }
    /// Returns a floating-point node by name.
    pub fn float_node(&self, name: &str) -> PylonResult<FloatNode> {
        Ok(FloatNode {
            name: name.to_string(),
            ptr: self.get_param_raw(name, shim_loader::shim().node_map_get_float_parameter)?,
        })
    }
    /// Returns an enum node by name.
    pub fn enum_node(&self, name: &str) -> PylonResult<EnumNode> {
        Ok(EnumNode {
            name: name.to_string(),
            ptr: self.get_param_raw(name, shim_loader::shim().node_map_get_enum_parameter)?,
        })
    }
    /// Returns a command node by name.
    pub fn command_node(&self, name: &str) -> PylonResult<CommandNode> {
        Ok(CommandNode {
            name: name.to_string(),
            ptr: self.get_param_raw(name, shim_loader::shim().node_map_get_command_parameter)?,
        })
    }
}

// =========================================================================
// GrabOptions
// =========================================================================

#[derive(Default)]
/// Options passed to [`InstantCamera::start_grabbing`].
pub struct GrabOptions {
    count: Option<u32>,
    strategy: Option<GrabStrategy>,
}

impl GrabOptions {
    /// Limits the grab to a fixed number of images.
    pub fn count(self, count: u32) -> GrabOptions {
        Self {
            count: Some(count),
            ..self
        }
    }
    /// Selects the buffer handling strategy.
    pub fn strategy(self, strategy: GrabStrategy) -> GrabOptions {
        Self {
            strategy: Some(strategy),
            ..self
        }
    }
}

// =========================================================================
// Parameter nodes
// =========================================================================

/// Boolean-valued GenICam parameter.
pub struct BooleanNode {
    name: String,
    ptr: *mut c_void,
}

impl BooleanNode {
    /// Returns the node name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the current value.
    pub fn value(&self) -> PylonResult<bool> {
        {
            let mut out: c_int = 0;
            let err = unsafe { (shim_loader::shim().boolean_node_get_value)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out != 0)
        }
    }
    /// Sets the current value.
    pub fn set_value(&mut self, value: bool) -> PylonResult<()> {
        shim_check(unsafe {
            (shim_loader::shim().boolean_node_set_value)(self.ptr, value as c_int)
        })
    }
}

impl Drop for BooleanNode {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (shim_loader::shim().boolean_parameter_destroy)(self.ptr) }
        }
    }
}

/// Integer-valued GenICam parameter.
pub struct IntegerNode {
    name: String,
    ptr: *mut c_void,
}

impl IntegerNode {
    /// Returns the node name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the engineering unit string.
    pub fn unit(&self) -> PylonResult<String> {
        {
            let mut out: *mut c_char = std::ptr::null_mut();
            let err = unsafe { (shim_loader::shim().integer_node_get_unit)(self.ptr, &mut out) };
            shim_check_for!(integer_node_get_unit, err)?;
            ensure_non_null_for!(integer_node_get_unit, "unit string", out)?;
            Ok(unsafe { shim_loader::take_str(out) })
        }
    }
    /// Returns the current value.
    pub fn value(&self) -> PylonResult<i64> {
        {
            let mut out = 0i64;
            let err = unsafe { (shim_loader::shim().integer_node_get_value)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out)
        }
    }
    /// Returns the minimum allowed value.
    pub fn min(&self) -> PylonResult<i64> {
        {
            let mut out = 0i64;
            let err = unsafe { (shim_loader::shim().integer_node_get_min)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out)
        }
    }
    /// Returns the maximum allowed value.
    pub fn max(&self) -> PylonResult<i64> {
        {
            let mut out = 0i64;
            let err = unsafe { (shim_loader::shim().integer_node_get_max)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out)
        }
    }
    /// Sets the current value.
    pub fn set_value(&mut self, value: i64) -> PylonResult<()> {
        shim_check(unsafe { (shim_loader::shim().integer_node_set_value)(self.ptr, value) })
    }
}

impl Drop for IntegerNode {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (shim_loader::shim().integer_parameter_destroy)(self.ptr) }
        }
    }
}

/// Floating-point GenICam parameter.
pub struct FloatNode {
    name: String,
    ptr: *mut c_void,
}

impl FloatNode {
    /// Returns the node name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the engineering unit string.
    pub fn unit(&self) -> PylonResult<String> {
        {
            let mut out: *mut c_char = std::ptr::null_mut();
            let err = unsafe { (shim_loader::shim().float_node_get_unit)(self.ptr, &mut out) };
            shim_check_for!(float_node_get_unit, err)?;
            ensure_non_null_for!(float_node_get_unit, "unit string", out)?;
            Ok(unsafe { shim_loader::take_str(out) })
        }
    }
    /// Returns the current value.
    pub fn value(&self) -> PylonResult<f64> {
        {
            let mut out = 0f64;
            let err = unsafe { (shim_loader::shim().float_node_get_value)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out)
        }
    }
    /// Returns the minimum allowed value.
    pub fn min(&self) -> PylonResult<f64> {
        {
            let mut out = 0f64;
            let err = unsafe { (shim_loader::shim().float_node_get_min)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out)
        }
    }
    /// Returns the maximum allowed value.
    pub fn max(&self) -> PylonResult<f64> {
        {
            let mut out = 0f64;
            let err = unsafe { (shim_loader::shim().float_node_get_max)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out)
        }
    }
    /// Sets the current value.
    pub fn set_value(&mut self, value: f64) -> PylonResult<()> {
        shim_check(unsafe { (shim_loader::shim().float_node_set_value)(self.ptr, value) })
    }
}

impl Drop for FloatNode {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (shim_loader::shim().float_parameter_destroy)(self.ptr) }
        }
    }
}

/// Enum-valued GenICam parameter.
pub struct EnumNode {
    name: String,
    ptr: *mut c_void,
}

impl EnumNode {
    /// Returns the node name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the currently selected entry.
    pub fn value(&self) -> PylonResult<String> {
        {
            let mut out: *mut c_char = std::ptr::null_mut();
            let err = unsafe { (shim_loader::shim().enum_node_get_value)(self.ptr, &mut out) };
            shim_check_for!(enum_node_get_value, err)?;
            ensure_non_null_for!(enum_node_get_value, "enum value string", out)?;
            Ok(unsafe { shim_loader::take_str(out) })
        }
    }
    /// Returns the values currently accepted by the node.
    pub fn settable_values(&self) -> PylonResult<Vec<String>> {
        {
            let mut arr: *mut *mut c_char = std::ptr::null_mut();
            let mut count: usize = 0;
            let err = unsafe {
                (shim_loader::shim().enum_node_settable_values)(self.ptr, &mut arr, &mut count)
            };
            shim_check_for!(enum_node_settable_values, err)?;
            if count > 0 {
                ensure_non_null_for!(enum_node_settable_values, "settable values array", arr)?;
            }
            let mut result = Vec::with_capacity(count);
            unsafe {
                for i in 0..count {
                    let s = std::ffi::CStr::from_ptr(*arr.add(i))
                        .to_string_lossy()
                        .into_owned();
                    result.push(s);
                }
                (shim_loader::shim().enum_node_free_settable_values)(arr, count);
            }
            Ok(result)
        }
    }
    /// Selects a new enum entry by name.
    pub fn set_value(&mut self, value: &str) -> PylonResult<()> {
        shim_check(unsafe {
            (shim_loader::shim().enum_node_set_value)(
                self.ptr,
                value.as_ptr() as *const c_char,
                value.len(),
            )
        })
    }
}

impl Drop for EnumNode {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (shim_loader::shim().enum_parameter_destroy)(self.ptr) }
        }
    }
}

/// Command-like GenICam parameter.
pub struct CommandNode {
    name: String,
    ptr: *mut c_void,
}

impl CommandNode {
    /// Returns the node name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Executes the command.
    pub fn execute(&self, verify: bool) -> PylonResult<()> {
        shim_check(unsafe { (shim_loader::shim().command_node_execute)(self.ptr, verify as c_int) })
    }
}

impl Drop for CommandNode {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (shim_loader::shim().command_parameter_destroy)(self.ptr) }
        }
    }
}

// =========================================================================
// GrabResult
// =========================================================================

/// Reusable container for a single grab result.
pub struct GrabResult {
    ptr: *mut c_void,
}

unsafe impl Send for GrabResult {}

impl GrabResult {
    /// Allocates an empty grab result.
    pub fn new() -> PylonResult<Self> {
        {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let err = unsafe { (shim_loader::shim().new_grab_result_ptr)(&mut ptr) };
            shim_check(err)?;
            Ok(GrabResult { ptr })
        }
    }

    /// Returns whether the grab completed successfully.
    pub fn grab_succeeded(&self) -> PylonResult<bool> {
        {
            let mut out: c_int = 0;
            let err =
                unsafe { (shim_loader::shim().grab_result_grab_succeeded)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out != 0)
        }
    }
    /// Returns the failure message for an unsuccessful grab.
    pub fn error_description(&self) -> PylonResult<String> {
        {
            let mut out: *mut c_char = std::ptr::null_mut();
            let err =
                unsafe { (shim_loader::shim().grab_result_error_description)(self.ptr, &mut out) };
            shim_check_for!(grab_result_error_description, err)?;
            ensure_non_null_for!(
                grab_result_error_description,
                "error description string",
                out
            )?;
            Ok(unsafe { shim_loader::take_str(out) })
        }
    }
    /// Returns the camera-specific error code for an unsuccessful grab.
    pub fn error_code(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            let err = unsafe { (shim_loader::shim().grab_result_error_code)(self.ptr, &mut out) };
            shim_check(err)?;
            Ok(out)
        }
    }
    /// Returns the image width in pixels.
    pub fn width(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe { (shim_loader::shim().grab_result_width)(self.ptr, &mut out) })?;
            Ok(out)
        }
    }
    /// Returns the image height in pixels.
    pub fn height(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe { (shim_loader::shim().grab_result_height)(self.ptr, &mut out) })?;
            Ok(out)
        }
    }
    /// Returns the horizontal image offset.
    pub fn offset_x(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe { (shim_loader::shim().grab_result_offset_x)(self.ptr, &mut out) })?;
            Ok(out)
        }
    }
    /// Returns the vertical image offset.
    pub fn offset_y(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe { (shim_loader::shim().grab_result_offset_y)(self.ptr, &mut out) })?;
            Ok(out)
        }
    }
    /// Returns the horizontal padding in bytes.
    pub fn padding_x(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe { (shim_loader::shim().grab_result_padding_x)(self.ptr, &mut out) })?;
            Ok(out)
        }
    }
    /// Returns the vertical padding in bytes.
    pub fn padding_y(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe { (shim_loader::shim().grab_result_padding_y)(self.ptr, &mut out) })?;
            Ok(out)
        }
    }
    /// Returns the image buffer.
    pub fn buffer(&self) -> PylonResult<&[u8]> {
        {
            let mut buf: *const u8 = std::ptr::null();
            let mut len: usize = 0;
            let err =
                unsafe { (shim_loader::shim().grab_result_buffer)(self.ptr, &mut buf, &mut len) };
            shim_check_for!(grab_result_buffer, err)?;
            if len > 0 {
                ensure_non_null_for!(grab_result_buffer, "image buffer", buf)?;
            }
            Ok(unsafe { std::slice::from_raw_parts(buf, len) })
        }
    }
    /// Returns the payload size in bytes.
    pub fn payload_size(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe {
                (shim_loader::shim().grab_result_payload_size)(self.ptr, &mut out)
            })?;
            Ok(out)
        }
    }
    /// Returns the backing buffer capacity in bytes.
    pub fn buffer_size(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe {
                (shim_loader::shim().grab_result_buffer_size)(self.ptr, &mut out)
            })?;
            Ok(out)
        }
    }
    /// Returns the block identifier.
    pub fn block_id(&self) -> PylonResult<u64> {
        {
            let mut out = 0u64;
            shim_check(unsafe { (shim_loader::shim().grab_result_block_id)(self.ptr, &mut out) })?;
            Ok(out)
        }
    }
    /// Returns the camera timestamp.
    pub fn time_stamp(&self) -> PylonResult<u64> {
        {
            let mut out = 0u64;
            shim_check(unsafe {
                (shim_loader::shim().grab_result_time_stamp)(self.ptr, &mut out)
            })?;
            Ok(out)
        }
    }
    /// Returns the image stride in bytes.
    pub fn stride(&self) -> PylonResult<usize> {
        {
            let mut out = 0usize;
            shim_check(unsafe { (shim_loader::shim().grab_result_stride)(self.ptr, &mut out) })?;
            Ok(out)
        }
    }
    /// Returns the image size in bytes.
    pub fn image_size(&self) -> PylonResult<u32> {
        {
            let mut out = 0u32;
            shim_check(unsafe {
                (shim_loader::shim().grab_result_image_size)(self.ptr, &mut out)
            })?;
            Ok(out)
        }
    }
    /// Returns the chunk-data node map for this result.
    pub fn chunk_data_node_map(&self) -> PylonResult<NodeMap<'_>> {
        {
            let mut ptr: *const c_void = std::ptr::null();
            let err = unsafe {
                (shim_loader::shim().grab_result_get_chunk_data_node_map)(self.ptr, &mut ptr)
            };
            shim_check_for!(grab_result_get_chunk_data_node_map, err)?;
            ensure_non_null(
                stringify!(grab_result_get_chunk_data_node_map),
                "chunk data node map pointer",
                ptr,
            )?;
            Ok(NodeMap {
                ptr,
                _marker: std::marker::PhantomData,
            })
        }
    }
}

impl Drop for GrabResult {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (shim_loader::shim().grab_result_ptr_destroy)(self.ptr) }
        }
    }
}

// =========================================================================
// DeviceInfo
// =========================================================================

/// Immutable information about a discovered device.
pub struct DeviceInfo {
    pub(crate) ptr: *mut c_void,
    runtime: runtime_impl::RuntimeLease,
}

unsafe impl Send for DeviceInfo {}

impl Clone for DeviceInfo {
    fn clone(&self) -> DeviceInfo {
        {
            let mut out: *mut c_void = std::ptr::null_mut();
            let err = unsafe { (shim_loader::shim().device_info_clone)(self.ptr, &mut out) };
            shim_loader::check_err(err)
                .map_err(PylonError::new)
                .expect("device_info_clone should not fail");
            DeviceInfo {
                ptr: out,
                runtime: self.runtime.clone(),
            }
        }
    }
}

impl Drop for DeviceInfo {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { (shim_loader::shim().device_info_destroy)(self.ptr) }
        }
    }
}

impl DeviceInfo {
    /// Returns the device model name.
    pub fn model_name(&self) -> PylonResult<String> {
        {
            let mut out: *mut c_char = std::ptr::null_mut();
            let err =
                unsafe { (shim_loader::shim().device_info_get_model_name)(self.ptr, &mut out) };
            shim_check_for!(device_info_get_model_name, err)?;
            ensure_non_null_for!(device_info_get_model_name, "model name string", out)?;
            Ok(unsafe { shim_loader::take_str(out) })
        }
    }
}

/// Shared access to name/value device properties.
pub trait HasProperties {
    /// Returns the available property names.
    fn property_names(&self) -> PylonResult<Vec<String>>;

    /// Returns the value for a single property.
    fn property_value(&self, name: &str) -> PylonResult<String>;
}

impl HasProperties for DeviceInfo {
    fn property_names(&self) -> PylonResult<Vec<String>> {
        {
            let mut arr: *mut *mut c_char = std::ptr::null_mut();
            let mut count: usize = 0;
            let err = unsafe {
                (shim_loader::shim().device_info_get_property_names)(self.ptr, &mut arr, &mut count)
            };
            shim_check_for!(device_info_get_property_names, err)?;
            if count > 0 {
                ensure_non_null_for!(device_info_get_property_names, "property names array", arr)?;
            }
            let mut result = Vec::with_capacity(count);
            unsafe {
                for i in 0..count {
                    let s = std::ffi::CStr::from_ptr(*arr.add(i))
                        .to_string_lossy()
                        .into_owned();
                    result.push(s);
                }
                (shim_loader::shim().device_info_free_property_names)(arr, count);
            }
            Ok(result)
        }
    }

    fn property_value(&self, name: &str) -> PylonResult<String> {
        {
            let mut out: *mut c_char = std::ptr::null_mut();
            let err = unsafe {
                (shim_loader::shim().device_info_get_property_value)(
                    self.ptr,
                    name.as_ptr() as *const c_char,
                    name.len(),
                    &mut out,
                )
            };
            shim_check_for!(device_info_get_property_value, err)?;
            ensure_non_null_for!(device_info_get_property_value, "property value string", out)?;
            Ok(unsafe { shim_loader::take_str(out) })
        }
    }
}

// =========================================================================
// WaitObject (Windows stream)
// =========================================================================

#[cfg(all(target_os = "windows", feature = "stream"))]
pub struct WaitObject(pub(crate) *mut c_void);

#[cfg(all(target_os = "windows", feature = "stream"))]
unsafe impl Send for WaitObject {}

#[cfg(all(target_os = "windows", feature = "stream"))]
impl Drop for WaitObject {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { (shim_loader::shim().wait_object_destroy)(self.0) }
        }
    }
}

#[cfg(all(target_os = "windows", feature = "stream"))]
impl WaitObject {
    pub fn wait(&self, timeout: u64) -> PylonResult<bool> {
        {
            let mut out: c_int = 0;
            let err = unsafe { (shim_loader::shim().wait_object_wait)(self.0, timeout, &mut out) };
            shim_check(err)?;
            Ok(out != 0)
        }
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn path_to_string<P: AsRef<std::path::Path>>(path: P) -> PylonResult<String> {
    match path.as_ref().to_str() {
        Some(filename) => Ok(filename.into()),
        None => Err(PylonError::new("Cannot convert path to UTF-8".to_string())),
    }
}
