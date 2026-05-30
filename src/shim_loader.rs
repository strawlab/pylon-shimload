/* Loads the shim shared library at runtime and resolves the
 * exported API table eagerly.  The shim is loaded once and lives for the
 * lifetime of the process.
 *
 * Symbol names mirror the C interface in include/pylon-cxx-shim.h.
 */

use std::ffi::{c_char, c_int, c_void};
use std::sync::OnceLock;

use crate::{PylonError, ShimError, EXPECTED_CABI_VERSION};

// Convenience alias used in type signatures below.
type Err = *const c_char; // NULL = ok, non-NULL = malloc'd error string

/// All function pointers resolved from the shim.  Fields are `pub(crate)`
/// so that `lib.rs` can call them directly.
#[repr(C)]
pub(crate) struct ShimApi {
    pub abi_version: u32,
    pub struct_size: u32,

    // --- memory ---
    pub pylon_cxx_free_str: unsafe extern "C" fn(*mut c_char),
    pub pylon_cxx_free_ptr: unsafe extern "C" fn(*mut c_void),

    // --- lifecycle ---
    pub pylon_initialize: unsafe extern "C" fn(),
    pub pylon_terminate: unsafe extern "C" fn(c_int),
    pub pylon_get_version: unsafe extern "C" fn(*mut u32, *mut u32, *mut u32, *mut u32),

    // --- TlFactory ---
    pub tl_factory_create_first_device: unsafe extern "C" fn(*mut *mut c_void) -> Err,
    pub tl_factory_create_device: unsafe extern "C" fn(*const c_void, *mut *mut c_void) -> Err,
    pub tl_factory_enumerate_devices:
        unsafe extern "C" fn(*mut *mut *mut c_void, *mut usize) -> Err,

    // --- CInstantCamera ---
    pub instant_camera_destroy: unsafe extern "C" fn(*mut c_void),
    pub instant_camera_get_device_info:
        unsafe extern "C" fn(*const c_void, *mut *mut c_void) -> Err,
    pub instant_camera_open: unsafe extern "C" fn(*mut c_void) -> Err,
    pub instant_camera_is_open: unsafe extern "C" fn(*const c_void, *mut c_int) -> Err,
    pub instant_camera_close: unsafe extern "C" fn(*mut c_void) -> Err,
    pub instant_camera_start_grabbing: unsafe extern "C" fn(*mut c_void) -> Err,
    pub instant_camera_stop_grabbing: unsafe extern "C" fn(*mut c_void) -> Err,
    pub instant_camera_is_grabbing: unsafe extern "C" fn(*const c_void) -> c_int,
    pub instant_camera_start_grabbing_with_strategy:
        unsafe extern "C" fn(*mut c_void, c_int) -> Err,
    pub instant_camera_start_grabbing_with_count: unsafe extern "C" fn(*mut c_void, u32) -> Err,
    pub instant_camera_start_grabbing_with_count_and_strategy:
        unsafe extern "C" fn(*mut c_void, u32, c_int) -> Err,
    pub instant_camera_retrieve_result:
        unsafe extern "C" fn(*mut c_void, u32, *mut c_void, c_int, *mut c_int) -> Err,
    pub instant_camera_get_node_map: unsafe extern "C" fn(*const c_void, *mut *const c_void) -> Err,
    pub instant_camera_get_tl_node_map:
        unsafe extern "C" fn(*const c_void, *mut *const c_void) -> Err,
    pub instant_camera_get_stream_grabber_node_map:
        unsafe extern "C" fn(*const c_void, *mut *const c_void) -> Err,
    pub instant_camera_get_event_grabber_node_map:
        unsafe extern "C" fn(*const c_void, *mut *const c_void) -> Err,
    pub instant_camera_get_instant_camera_node_map:
        unsafe extern "C" fn(*const c_void, *mut *const c_void) -> Err,
    // Present on all non-Windows platforms regardless of the `stream` feature —
    // this field must always be included to match the C ABI struct layout.
    #[cfg(not(target_os = "windows"))]
    pub instant_camera_wait_object_fd: unsafe extern "C" fn(*const c_void, *mut c_int) -> Err,
    // Present on Windows regardless of the `stream` feature — same reason.
    #[cfg(target_os = "windows")]
    pub instant_camera_wait_object: unsafe extern "C" fn(*const c_void, *mut *mut c_void) -> Err,

    // --- NodeMap ---
    pub node_map_load: unsafe extern "C" fn(*const c_void, *const c_char, usize, c_int) -> Err,
    pub node_map_save: unsafe extern "C" fn(*const c_void, *const c_char, usize) -> Err,
    pub node_map_load_from_string:
        unsafe extern "C" fn(*const c_void, *const c_char, usize, c_int) -> Err,
    pub node_map_save_to_string: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> Err,
    pub node_map_get_boolean_parameter:
        unsafe extern "C" fn(*const c_void, *const c_char, usize, *mut *mut c_void) -> Err,
    pub node_map_get_integer_parameter:
        unsafe extern "C" fn(*const c_void, *const c_char, usize, *mut *mut c_void) -> Err,
    pub node_map_get_float_parameter:
        unsafe extern "C" fn(*const c_void, *const c_char, usize, *mut *mut c_void) -> Err,
    pub node_map_get_enum_parameter:
        unsafe extern "C" fn(*const c_void, *const c_char, usize, *mut *mut c_void) -> Err,
    pub node_map_get_command_parameter:
        unsafe extern "C" fn(*const c_void, *const c_char, usize, *mut *mut c_void) -> Err,

    // --- CBooleanParameter ---
    pub boolean_parameter_destroy: unsafe extern "C" fn(*mut c_void),
    pub boolean_node_get_value: unsafe extern "C" fn(*const c_void, *mut c_int) -> Err,
    pub boolean_node_set_value: unsafe extern "C" fn(*mut c_void, c_int) -> Err,

    // --- CIntegerParameter ---
    pub integer_parameter_destroy: unsafe extern "C" fn(*mut c_void),
    pub integer_node_get_unit: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> Err,
    pub integer_node_get_value: unsafe extern "C" fn(*const c_void, *mut i64) -> Err,
    pub integer_node_get_min: unsafe extern "C" fn(*const c_void, *mut i64) -> Err,
    pub integer_node_get_max: unsafe extern "C" fn(*const c_void, *mut i64) -> Err,
    pub integer_node_set_value: unsafe extern "C" fn(*mut c_void, i64) -> Err,

    // --- CFloatParameter ---
    pub float_parameter_destroy: unsafe extern "C" fn(*mut c_void),
    pub float_node_get_unit: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> Err,
    pub float_node_get_value: unsafe extern "C" fn(*const c_void, *mut f64) -> Err,
    pub float_node_get_min: unsafe extern "C" fn(*const c_void, *mut f64) -> Err,
    pub float_node_get_max: unsafe extern "C" fn(*const c_void, *mut f64) -> Err,
    pub float_node_set_value: unsafe extern "C" fn(*mut c_void, f64) -> Err,

    // --- CEnumParameter ---
    pub enum_parameter_destroy: unsafe extern "C" fn(*mut c_void),
    pub enum_node_get_value: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> Err,
    pub enum_node_settable_values:
        unsafe extern "C" fn(*const c_void, *mut *mut *mut c_char, *mut usize) -> Err,
    pub enum_node_free_settable_values: unsafe extern "C" fn(*mut *mut c_char, usize),
    pub enum_node_set_value: unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> Err,

    // --- CCommandParameter ---
    pub command_parameter_destroy: unsafe extern "C" fn(*mut c_void),
    pub command_node_execute: unsafe extern "C" fn(*mut c_void, c_int) -> Err,

    // --- CGrabResultPtr ---
    pub new_grab_result_ptr: unsafe extern "C" fn(*mut *mut c_void) -> Err,
    pub grab_result_ptr_destroy: unsafe extern "C" fn(*mut c_void),
    pub grab_result_grab_succeeded: unsafe extern "C" fn(*const c_void, *mut c_int) -> Err,
    pub grab_result_error_description: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> Err,
    pub grab_result_error_code: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_width: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_height: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_offset_x: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_offset_y: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_padding_x: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_padding_y: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_buffer: unsafe extern "C" fn(*const c_void, *mut *const u8, *mut usize) -> Err,
    pub grab_result_payload_size: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_buffer_size: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_block_id: unsafe extern "C" fn(*const c_void, *mut u64) -> Err,
    pub grab_result_time_stamp: unsafe extern "C" fn(*const c_void, *mut u64) -> Err,
    pub grab_result_stride: unsafe extern "C" fn(*const c_void, *mut usize) -> Err,
    pub grab_result_image_size: unsafe extern "C" fn(*const c_void, *mut u32) -> Err,
    pub grab_result_get_chunk_data_node_map:
        unsafe extern "C" fn(*const c_void, *mut *const c_void) -> Err,

    // --- CDeviceInfo ---
    pub device_info_destroy: unsafe extern "C" fn(*mut c_void),
    pub device_info_clone: unsafe extern "C" fn(*const c_void, *mut *mut c_void) -> Err,
    pub device_info_get_property_names:
        unsafe extern "C" fn(*const c_void, *mut *mut *mut c_char, *mut usize) -> Err,
    pub device_info_free_property_names: unsafe extern "C" fn(*mut *mut c_char, usize),
    pub device_info_get_property_value:
        unsafe extern "C" fn(*const c_void, *const c_char, usize, *mut *mut c_char) -> Err,
    pub device_info_get_model_name: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> Err,

    // --- WaitObject (Windows) ---
    // Always present on Windows to match the C ABI layout.
    #[cfg(target_os = "windows")]
    pub wait_object_destroy: unsafe extern "C" fn(*mut c_void),
    #[cfg(target_os = "windows")]
    pub wait_object_wait: unsafe extern "C" fn(*const c_void, u64, *mut c_int) -> Err,
}

pub(crate) struct Shim {
    api: &'static ShimApi,
    _lib: libloading::Library, // kept alive for the lifetime of all fn pointers
}

impl std::ops::Deref for Shim {
    type Target = ShimApi;

    fn deref(&self) -> &Self::Target {
        &self.api
    }
}

// SAFETY: all fn pointers come from a Library that lives for 'static.
unsafe impl Send for Shim {}
unsafe impl Sync for Shim {}

static SHIM: OnceLock<Shim> = OnceLock::new();

/// Loads the shim library and resolves all symbols, panicking on error.
fn init_shim() -> Shim {
    init_shim_result()
        .unwrap_or_else(|e| panic!("{} failed to load shim: {}", env!["CARGO_PKG_NAME"], e))
}

/// Loads the shim library and resolves all symbols, returning an error if any step fails.
fn init_shim_result() -> Result<Shim, PylonError> {
    let path = match std::env::var_os("PYLON_CABI") {
        Some(val) => val,
        None => {
            // Use platform-specific default prefix and suffix (e.g.
            // `libpylon-cabi.so` on linux, `libpylon-cabi.dylib`, and
            // 'pylon-cabi.dll` on windows ).
            libloading::library_filename("pylon-cabi")
        }
    };
    let path_ref = path.as_os_str();

    // SAFETY: we are loading a shared library that the maintainer has
    // published.  The caller is responsible for providing a valid path.
    let lib = unsafe { libloading::Library::new(path_ref) }.map_err(|err| match err {
        libloading::Error::DlOpen { source } => PylonError::DlOpenFailed {
            path: path_ref.to_os_string(),
            source: format!("{}", source),
        },
        other_err => PylonError::new(format!(
            "Unexpected error loading shim library: {other_err:?}"
        )),
    })?;
    let pylon_get_api = {
        let symbol = "pylon_shim_get_api";
        let s: libloading::Symbol<unsafe extern "C" fn() -> *const ShimApi> = unsafe {
            lib.get(symbol).map_err(|err| {
                PylonError::ShimError(ShimError::SymbolLoadFailed {
                    path: path_ref.to_os_string(),
                    symbol: symbol.to_string(),
                    err_str: format!("{err:?}"),
                })
            })?
        };
        *s
    };

    let api_ptr = unsafe { pylon_get_api() };
    if api_ptr.is_null() {
        return Err(PylonError::ShimError(ShimError::NullApi {
            path: path_ref.to_os_string(),
        }));
    }
    let api = unsafe { &(*api_ptr) };

    let expected_struct_size = std::mem::size_of::<ShimApi>() as u32;
    if api.struct_size < expected_struct_size {
        return Err(PylonError::ShimError(ShimError::ApiTableTooSmall {
            path: path_ref.to_os_string(),
            got: api.struct_size,
            need: expected_struct_size,
        }));
    }

    let actual_version = api.abi_version;
    if actual_version != EXPECTED_CABI_VERSION {
        return Err(PylonError::ShimError(ShimError::IncompatibleAbiVersion {
            path: path_ref.to_os_string(),
            got: actual_version,
            need: EXPECTED_CABI_VERSION,
        }));
    }

    Ok(Shim {
        api: &api,
        _lib: lib,
    })
}

/// Returns a reference to the lazily-loaded shim.  Panics if the shim
/// cannot be found or if any required symbol is missing.
pub(crate) fn shim() -> &'static Shim {
    SHIM.get_or_init(init_shim)
}

/// Returns a reference to the lazily-loaded shim.  Returns error if the shim
/// cannot be found or if any required symbol is missing.
pub(crate) fn shim_or_err() -> Result<&'static Shim, PylonError> {
    match SHIM.get() {
        Some(s) => Ok(s),
        None => {
            let shim = init_shim_result()?;
            SHIM.set(shim)
                .map_err(|_| PylonError::new("Failed to set shim".to_string()))?;
            Ok(SHIM.get().unwrap())
        }
    }
}

/// Converts a shim error return value into a PylonResult.
/// Frees the error string via the shim's own free function.
pub(crate) fn check_err(err: *const std::ffi::c_char) -> Result<(), String> {
    if err.is_null() {
        Ok(())
    } else {
        let s = unsafe { std::ffi::CStr::from_ptr(err) }
            .to_string_lossy()
            .into_owned();
        unsafe { (shim().pylon_cxx_free_str)(err as *mut _) };
        Err(s)
    }
}

/// Reads a malloc-allocated C string returned by the shim, frees it,
/// and returns the Rust String.
pub(crate) unsafe fn take_str(ptr: *mut std::ffi::c_char) -> String {
    let s = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
    (shim().pylon_cxx_free_str)(ptr);
    s
}
