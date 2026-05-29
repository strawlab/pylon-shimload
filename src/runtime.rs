use std::sync::{Mutex, OnceLock};

use crate::{shim_loader, PylonError, PylonResult};

#[derive(Debug)]
struct RuntimeState {
    leases: usize,
    initialized: bool,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            leases: 0,
            initialized: false,
        }
    }
}

static RUNTIME_STATE: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

fn state() -> &'static Mutex<RuntimeState> {
    RUNTIME_STATE.get_or_init(|| Mutex::new(RuntimeState::new()))
}

fn lock_state() -> std::sync::MutexGuard<'static, RuntimeState> {
    match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) fn acquire_runtime() -> PylonResult<RuntimeLease> {
    let mut st = lock_state();
    if !st.initialized {
        let shim = shim_loader::shim_or_err()?;
        unsafe { (shim.pylon_initialize)() };
        st.initialized = true;
    }
    st.leases += 1;
    Ok(RuntimeLease { active: true })
}

fn release_runtime_internal() {
    let mut st = lock_state();
    if st.leases == 0 {
        return;
    }

    st.leases -= 1;
    if st.leases == 0 && st.initialized {
        unsafe { (shim_loader::shim().pylon_terminate)(1) };
        st.initialized = false;
    }
}

pub(crate) fn runtime_version() -> PylonResult<crate::PylonVersion> {
    let _lease = acquire_runtime()?;

    let mut major = 0u32;
    let mut minor = 0u32;
    let mut subminor = 0u32;
    let mut build = 0u32;

    unsafe {
        (shim_loader::shim().pylon_get_version)(&mut major, &mut minor, &mut subminor, &mut build)
    };

    Ok(crate::PylonVersion {
        major,
        minor,
        subminor,
        build,
    })
}

pub fn shutdown() -> PylonResult<()> {
    let mut st = lock_state();
    if st.leases != 0 {
        return Err(PylonError::Msg(
            "Cannot shutdown runtime while handles are alive".to_string(),
        ));
    }
    if st.initialized {
        unsafe { (shim_loader::shim().pylon_terminate)(1) };
        st.initialized = false;
    }
    Ok(())
}

/// Keeps the Pylon runtime alive for explicit lifecycle management.
///
/// Obtained via [`crate::pylon::init`]; see that function's documentation
/// for when this type is needed.
pub struct RuntimeGuard {
    _lease: RuntimeLease,
}

impl RuntimeGuard {
    pub fn new() -> PylonResult<Self> {
        Ok(Self {
            _lease: acquire_runtime()?,
        })
    }
}

pub(crate) struct RuntimeLease {
    active: bool,
}

impl Clone for RuntimeLease {
    fn clone(&self) -> Self {
        if self.active {
            let mut st = lock_state();
            st.leases += 1;
        }
        Self {
            active: self.active,
        }
    }
}

impl Drop for RuntimeLease {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
            release_runtime_internal();
        }
    }
}
