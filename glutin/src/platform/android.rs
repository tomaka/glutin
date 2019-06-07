#![cfg(any(target_os = "android"))]

use crate::platform::ContextTraitExt;
use crate::{Context, ContextCurrentState};
use crate::{SupportsPBuffersTrait, SupportsWindowSurfacesTrait, SupportsSurfacelessTrait};
pub use glutin_egl_sys::EGLContext;

pub use winit::platform::android::*;

use std::os::raw;

impl<CS: ContextCurrentState, PBS: SupportsPBuffersTrait, WST: SupportsWindowSurfacesTrait, ST: SupportsSurfacelessTrait> ContextTraitExt for Context<CS, PBS, WST, ST> {
    type Handle = EGLContext;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }

    #[inline]
    unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        Some(self.context.get_egl_display())
    }
}
