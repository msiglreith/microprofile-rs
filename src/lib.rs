
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate microprofile_sys;

#[cfg(feature = "vulkan")]
extern crate vk_sys as vk;

#[cfg(any(feature = "dx11", feature = "dx12"))]
extern crate winapi;

use std::ffi::CString;
use std::ops::*;

pub struct Profiler { _hidden: () }

lazy_static! {
    static ref PROFILER: Profiler = {
        unsafe { microprofile_sys::MicroProfileInit(); }
        Profiler { _hidden: () }
    };
}

pub struct LocalCounter<'a> {
    token: microprofile_sys::MicroProfileToken,
    value: i64,
    marker: std::marker::PhantomData<&'a Profiler>,
}

pub struct Counter<'a> {
    token: microprofile_sys::MicroProfileToken,
    name: CString,
    marker: std::marker::PhantomData<&'a Profiler>,
}

impl<'a> AddAssign<i64> for Counter<'a> {
    fn add_assign(&mut self, v: i64) {
        unsafe { microprofile_sys::MicroProfileCounterAdd(self.token, v); }
    }
}

impl<'a> SubAssign<i64> for Counter<'a> {
    fn sub_assign(&mut self, v: i64) {
        unsafe { microprofile_sys::MicroProfileCounterAdd(self.token, -v); }
    }
}

impl<'a> Counter<'a> {
    pub fn set(&mut self, v: i64) {
        unsafe { microprofile_sys::MicroProfileCounterSet(self.token, v); }
    }

    pub fn config(&mut self) { // TODO
        unsafe {
            microprofile_sys::MicroProfileCounterConfig(
                self.name.as_ptr(),
                0,
                0,
                0,
            );
        }
    }
}

pub struct Group<'a>(CString, std::marker::PhantomData<&'a Profiler>);

impl<'a> Group<'a> {
    fn as_ptr(&self) -> *const libc::c_char {
        self.0.as_ptr()
    }

    pub fn get_cpu_scope(&self, name: &str, color: Color) -> CpuScope<'a> {
        let token = unsafe {
            microprofile_sys::MicroProfileGetToken(
                self.as_ptr(),
                CString::new(name).unwrap().as_ptr(),
                color.into(),
                microprofile_sys::MicroProfileTokenTypeCpu
            )
        };

        CpuScope {
            token: token,
            tick: None,
            marker: std::marker::PhantomData,
        }
    }

    pub fn get_gpu_scope<'b>(&self, name: &str, log: &'b GpuThreadLog<'a>, color: Color) -> GpuScope<'a, 'b> {
        let token = unsafe {
            microprofile_sys::MicroProfileGetToken(
                self.as_ptr(),
                CString::new(name).unwrap().as_ptr(),
                color.into(),
                microprofile_sys::MicroProfileTokenTypeGpu
            )
        };

        GpuScope {
            token: token,
            tick: None,
            log: log,
        }
    }
}

pub struct Category<'a>(CString, std::marker::PhantomData<&'a Profiler>);

impl<'a> Category<'a> {
    fn as_ptr(&self) -> *const libc::c_char {
        self.0.as_ptr()
    }

    pub fn enable(&self, enable: bool) {
        if enable {
            unsafe { microprofile_sys::MicroProfileEnableCategory(self.as_ptr()); }
        } else {
            unsafe { microprofile_sys::MicroProfileDisableCategory(self.as_ptr()); }
        }
    }

    pub fn define_group(&self, name: &str, color: Color) -> Group<'a> {
        let group = Group(CString::new(name).unwrap(), std::marker::PhantomData);

        unsafe {
            microprofile_sys::MicroProfileRegisterGroup(
                group.as_ptr(),
                self.as_ptr(),
                color.into(),
            );
        }

        group
    }
}

pub trait Scope {
    fn enter(&mut self);
    fn leave(&mut self);
}

pub struct CpuScope<'a> {
    token: microprofile_sys::MicroProfileToken,
    tick: Option<u64>,
    marker: std::marker::PhantomData<&'a Profiler>,
}

impl<'a> Scope for CpuScope<'a> {
    fn enter(&mut self) {
        assert!(self.tick.is_none(), "Unable to re-enter scope");
        self.tick = Some(unsafe { microprofile_sys::MicroProfileEnterInternal(self.token) });
    }

    fn leave(&mut self) {
        assert!(self.tick.is_some(), "Must enter the scope before leaving");
        unsafe { microprofile_sys::MicroProfileLeaveInternal(self.token, self.tick.unwrap()); }
        self.tick = None;
    }
}

pub struct GpuScope<'a: 'b, 'b> {
    token: microprofile_sys::MicroProfileToken,
    tick: Option<u64>,
    log: &'b GpuThreadLog<'a>,
}

impl<'a, 'b> Scope for GpuScope<'a, 'b> {
    fn enter(&mut self) {
        assert!(self.tick.is_none(), "Unable to re-enter scope");
        self.tick = Some(unsafe { microprofile_sys::MicroProfileGpuEnterInternal(self.log.0, self.token) });
    }

    fn leave(&mut self) {
        assert!(self.tick.is_some(), "Must enter the scope before leaving");
        unsafe { microprofile_sys::MicroProfileGpuLeaveInternal(self.log.0, self.token, self.tick.unwrap()); }
        self.tick = None;
    }
}

pub struct SmartScope<'b, T>(&'b mut T) where T: Scope + 'b;

impl<'b, T> SmartScope<'b, T> where T: Scope {
    pub fn new(scope: &'b mut T) -> Self {
        scope.enter();
        SmartScope(scope)
    }
}

impl<'b, T> Drop for SmartScope<'b, T> where T: Scope {
    fn drop(&mut self) {
        self.0.leave()
    }
}

#[derive(Copy, Clone)]
pub struct Color(pub u8, pub u8, pub u8);

impl Into<u32> for Color {
    fn into(self) -> u32 {
        (self.0 as u32) << 16 | (self.1 as u32) << 8 | self.2 as u32
    }
}

pub enum GpuContext {
    #[cfg(feature = "dx11")]
    D3D11(*mut winapi::ID3D11DeviceContext),
    #[cfg(feature = "dx12")]
    D3D12(*mut winapi::ID3D12GraphicsCommandList),
    #[cfg(feature = "vulkan")]
    Vulkan(vk::CommandBuffer),
    #[cfg(feature = "gl")]
    GL,
    None,
}

pub struct GpuThreadLog<'a>(*mut microprofile_sys::MicroProfileThreadLogGpu, std::marker::PhantomData<&'a Profiler>);

impl<'a> GpuThreadLog<'a> {
    pub fn reset(&mut self) {
        unsafe { microprofile_sys::MicroProfileThreadLogGpuReset(self.0); }
    }
}

impl<'a> Drop for GpuThreadLog<'a> {
    fn drop(&mut self) {
        unsafe { microprofile_sys::MicroProfileThreadLogGpuFree(self.0); }
    }
}

impl Profiler {
    pub fn global() -> &'static Profiler {
        &PROFILER
    }
    #[cfg(feature = "gl")]
    pub fn init_gl<F>(&mut self, f: F) where F: Fn(&str) -> *mut libc::c_void {
        use std::ffi::CStr;
        use libc::*;

        extern fn init_gl_wrapper<F>(string: *const c_char, closure: *mut c_void) -> *mut c_void where F: Fn(&str) -> *mut c_void {
            let opt_closure = closure as *mut F;
            let string = unsafe { CStr::from_ptr(string) };
            unsafe {
                let ret = (*opt_closure)(string.to_str().unwrap());
                ret as *mut _
            }
        }

        let user = &f as *const _ as *mut c_void;
        unsafe { microprofile_sys::MicroProfileGpuInitGL(init_gl_wrapper::<F>, user); }
        self.gpu_context = GpuContext::OpenGL;
    }

    #[cfg(feature = "dx11")]
    pub fn init_dx11(&self, device: *mut winapi::ID3D11Device, immediate_context: *mut winapi::ID3D11DeviceContext) {
        unsafe { microprofile_sys::MicroProfileGpuInitD3D11(device as *mut libc::c_void, immediate_context as *mut libc::c_void); }
    }

    #[cfg(feature = "dx12")]
    pub fn init_dx12(&self, node_count: u32) {
        unsafe { microprofile_sys::MicroProfileGpuInitD3D12(std::ptr::null_mut(), node_count, std::ptr::null_mut()); } // TODO
    }

    #[cfg(feature = "vulkan")]
    pub fn init_vulkan(&self, devices: &mut [vk::Device], physical_devices: &mut [vk::PhysicalDevice], queues: &mut [vk::Queue], queue_family: u32, node_count: u32) {
        unsafe { microprofile_sys::MicroProfileGpuInitVulkan(devices.as_mut_ptr(), physical_devices.as_mut_ptr(), queues.as_mut_ptr(), queue_family, node_count); } // TODO
    }

    pub fn enable_all_groups(&self, enable: bool) {
        unsafe { microprofile_sys::MicroProfileSetEnableAllGroups(enable as i32); }
    }

    pub fn enable_all_meta_counters(&self, enable: bool) {
        unsafe { microprofile_sys::MicroProfileSetForceMetaCounters(enable as i32); }
    }

    pub fn flip(&self, gpu_context: GpuContext) {
        let context = match gpu_context {
            #[cfg(feature = "dx11")]
            GpuContext::D3D11(cx) => cx as *mut libc::c_void,
            #[cfg(feature = "dx12")]
            GpuContext::D3D12(cx) => cx as *mut libc::c_void,
            #[cfg(feature = "vulkan")]
            GpuContext::Vulkan(cx) => cx as *mut libc::c_void,
            _ => std::ptr::null_mut(),
        };

        unsafe { microprofile_sys::MicroProfileFlip(context); }
    }

    pub fn begin_thread(&self, name: &str) {
        unsafe { microprofile_sys::MicroProfileOnThreadCreate(CString::new(name).unwrap().as_ptr()); }
    }

    pub fn end_thread(&self) {
        unsafe { microprofile_sys::MicroProfileOnThreadExit(); }
    }

    pub fn begin_context_switch_trace(&self) {
        unsafe { microprofile_sys::MicroProfileStartContextSwitchTrace(); }
    }

    pub fn end_context_switch_trace(&self) {
        unsafe { microprofile_sys::MicroProfileStopContextSwitchTrace(); }
    }

    pub fn define_counter(&self, name: &str) -> Counter {
        let name = CString::new(name).unwrap();
        let token = unsafe { microprofile_sys::MicroProfileGetCounterToken(name.as_ptr()) };
        Counter {
            token: token,
            name: name,
            marker: std::marker::PhantomData,
         }
    }

    pub fn define_local_counter(&self, name: &str) -> LocalCounter {
        let token = unsafe { microprofile_sys::MicroProfileGetCounterToken(CString::new(name).unwrap().as_ptr()) };
        LocalCounter {
            token: token,
            value: 0,
            marker: std::marker::PhantomData,
        }
    }

    pub fn define_category<'a>(&'a self, name: &str) -> Category<'a> {
        Category(CString::new(name).unwrap(), std::marker::PhantomData)
    }

    pub fn alloc_gpu_thread_log<'a>(&'a self) -> GpuThreadLog<'a> {
        let log = unsafe { microprofile_sys::MicroProfileThreadLogGpuAlloc() };
        GpuThreadLog(log, std::marker::PhantomData)
    }

    pub fn get_webserver_port(&self) -> u16 {
        unsafe { microprofile_sys::MicroProfileWebServerPort() as u16 }
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        unsafe { microprofile_sys::MicroProfileShutdown(); }
    }
}
