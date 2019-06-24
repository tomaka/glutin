#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

pub mod osmesa;
// mod rawext;

use crate::platform::ContextTraitExt;
pub use crate::platform_impl::{PlatformAttributes, RawHandle};
use crate::{
    Context, ContextCurrentState, SupportsPBuffersTrait,
    SupportsSurfacelessTrait, SupportsWindowSurfacesTrait,
};
pub use glutin_egl_sys::EGLContext;
pub use glutin_glx_sys::GLXContext;

pub use winit::platform::unix::*;
// pub use self::rawext::*;

use std::os::raw;

impl<
        CS: ContextCurrentState,
        PBS: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
    > ContextTraitExt for Context<CS, PBS, WST, ST>
{
    type Handle = RawHandle;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }

    #[inline]
    unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        self.context.get_egl_display()
    }
}
