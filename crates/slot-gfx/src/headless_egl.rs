//! A GLES 2 context with nothing on screen, through EGL, for tests on Linux: the same programs
//! the handheld runs, drawn by whatever GLES the machine has (Mesa's, on a desktop).

use std::ffi::{c_char, c_void, CString};

use libloading::Library;

use crate::surface::{GfxError, Surface, OUT_H, OUT_W};

const EGL_NONE: i32 = 0x3038;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_HEIGHT: i32 = 0x3056;
const EGL_WIDTH: i32 = 0x3057;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
const EGL_OPENGL_ES_API: u32 = 0x30A0;
const EGL_PBUFFER_BIT: i32 = 0x0001;
const EGL_OPENGL_ES2_BIT: i32 = 0x0004;
const EGL_PLATFORM_SURFACELESS_MESA: u32 = 0x31DD;

type Ptr = *mut c_void;

pub struct HeadlessSurface {
    // Never unloaded: GL objects made in the context may outlive the surface.
    lib: std::mem::ManuallyDrop<Library>,
    gles: std::mem::ManuallyDrop<Library>,
    display: Ptr,
}

fn open(name: &str) -> Result<Library, GfxError> {
    unsafe { Library::new(name) }.map_err(|e| GfxError::Context(format!("{name}: {e}")))
}

impl HeadlessSurface {
    pub fn new() -> Result<Self, GfxError> {
        let lib = open("libEGL.so.1").or_else(|_| open("libEGL.so"))?;
        let gles = open("libGLESv2.so.2").or_else(|_| open("libGLESv2.so"))?;
        unsafe {
            let get = |name: &str| -> Result<*const c_void, GfxError> {
                lib.get::<unsafe extern "C" fn()>(name.as_bytes())
                    .map(|s| *s as *const c_void)
                    .map_err(|e| GfxError::Context(format!("{name}: {e}")))
            };
            let get_proc: unsafe extern "C" fn(*const c_char) -> *const c_void =
                std::mem::transmute(get("eglGetProcAddress")?);
            let platform = CString::new("eglGetPlatformDisplayEXT").unwrap();
            let platform_display = get_proc(platform.as_ptr());
            let mut display: Ptr = std::ptr::null_mut();
            if !platform_display.is_null() {
                let f: unsafe extern "C" fn(u32, Ptr, *const i32) -> Ptr = std::mem::transmute(platform_display);
                display = f(EGL_PLATFORM_SURFACELESS_MESA, std::ptr::null_mut(), std::ptr::null());
            }
            if display.is_null() {
                let f: unsafe extern "C" fn(Ptr) -> Ptr = std::mem::transmute(get("eglGetDisplay")?);
                display = f(std::ptr::null_mut());
            }
            if display.is_null() {
                return Err(GfxError::Context("no egl display".into()));
            }
            let initialize: unsafe extern "C" fn(Ptr, *mut i32, *mut i32) -> u32 =
                std::mem::transmute(get("eglInitialize")?);
            if initialize(display, std::ptr::null_mut(), std::ptr::null_mut()) == 0 {
                return Err(GfxError::Context("eglInitialize".into()));
            }
            let bind_api: unsafe extern "C" fn(u32) -> u32 = std::mem::transmute(get("eglBindAPI")?);
            bind_api(EGL_OPENGL_ES_API);
            let choose: unsafe extern "C" fn(Ptr, *const i32, *mut Ptr, i32, *mut i32) -> u32 =
                std::mem::transmute(get("eglChooseConfig")?);
            let attrs = [
                EGL_SURFACE_TYPE,
                EGL_PBUFFER_BIT,
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES2_BIT,
                EGL_NONE,
            ];
            let mut config: Ptr = std::ptr::null_mut();
            let mut found = 0;
            if choose(display, attrs.as_ptr(), &mut config, 1, &mut found) == 0 || found == 0 {
                return Err(GfxError::Context("no es2 pbuffer config".into()));
            }
            let pbuffer: unsafe extern "C" fn(Ptr, Ptr, *const i32) -> Ptr =
                std::mem::transmute(get("eglCreatePbufferSurface")?);
            let size = [EGL_WIDTH, OUT_W as i32, EGL_HEIGHT, OUT_H as i32, EGL_NONE];
            let surface = pbuffer(display, config, size.as_ptr());
            let create: unsafe extern "C" fn(Ptr, Ptr, Ptr, *const i32) -> Ptr =
                std::mem::transmute(get("eglCreateContext")?);
            let ctx_attrs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
            let context = create(display, config, std::ptr::null_mut(), ctx_attrs.as_ptr());
            if context.is_null() {
                return Err(GfxError::Context("eglCreateContext".into()));
            }
            let make_current: unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> u32 =
                std::mem::transmute(get("eglMakeCurrent")?);
            if make_current(display, surface, surface, context) == 0 {
                return Err(GfxError::Context("eglMakeCurrent".into()));
            }
            Ok(HeadlessSurface {
                lib: std::mem::ManuallyDrop::new(lib),
                gles: std::mem::ManuallyDrop::new(gles),
                display,
            })
        }
    }
}

impl Surface for HeadlessSurface {
    fn make_current(&mut self) -> Result<(), GfxError> {
        Ok(())
    }

    fn window_size(&self) -> (u32, u32) {
        (OUT_W, OUT_H)
    }

    fn swap(&mut self) -> Result<(), GfxError> {
        Ok(())
    }

    fn proc_address(&self, name: &str) -> *const c_void {
        let Ok(c) = CString::new(name) else {
            return std::ptr::null();
        };
        if let Ok(f) = unsafe { self.gles.get::<unsafe extern "C" fn()>(name.as_bytes()) } {
            return *f as *const c_void;
        }
        unsafe {
            match self.lib.get::<unsafe extern "C" fn(*const c_char) -> *const c_void>(b"eglGetProcAddress") {
                Ok(f) => f(c.as_ptr()),
                Err(_) => std::ptr::null(),
            }
        }
    }
}

impl Drop for HeadlessSurface {
    fn drop(&mut self) {
        let _ = self.display;
    }
}
