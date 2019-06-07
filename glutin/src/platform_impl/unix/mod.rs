#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

mod wayland;
mod x11;

use self::x11::X11Context;
use crate::{
    Api, ContextCurrentState, ContextError, CreationError, GlAttributes,
    NotCurrent, PixelFormat, PixelFormatRequirements,
};
pub use x11::utils as x11_utils;

use crate::platform::unix::x11::XConnection;
use crate::platform::unix::EventLoopExtUnix;
use winit::dpi;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use std::marker::PhantomData;
use std::os::raw;
use std::sync::Arc;

/// Context handles available on Unix-like platforms.
#[derive(Clone, Debug)]
pub enum RawHandle {
    /// Context handle for a glx context.
    Glx(glutin_glx_sys::GLXContext),
    /// Context handle for a egl context.
    Egl(glutin_egl_sys::EGLContext),
}

#[derive(Debug)]
pub enum ContextType {
    X11,
    Wayland,
    OsMesa,
}

#[derive(Debug)]
pub enum Context {
    X11(x11::Context),
    Wayland(wayland::Context),
    OsMesa(osmesa::OsMesaContext),
}

impl Context {
    fn is_compatible(
        c: &Option<&Context>,
        ct: ContextType,
    ) -> Result<(), CreationError> {
        if let Some(c) = *c {
            match ct {
                ContextType::OsMesa => match *c {
                    Context::OsMesa(_) => Ok(()),
                    _ => {
                        let msg = "Cannot share an OSMesa context with a non-OSMesa context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
                ContextType::X11 => match *c {
                    Context::X11(_) => Ok(()),
                    _ => {
                        let msg = "Cannot share an X11 context with a non-X11 context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
                ContextType::Wayland => match *c {
                    Context::Wayland(_) => Ok(()),
                    _ => {
                        let msg = "Cannot share a Wayland context with a non-Wayland context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
            }
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn new_windowed<T>(
        wb: WindowBuilder,
        el: &EventLoop<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        plat_attr: &PlatformAttributes,
    ) -> Result<(Window, Self), CreationError> {
        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;

            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::Wayland(ref ctx) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new(wb, el, pf_reqs, &gl_attr, plat_attr)
                .map(|(win, context)| (win, Context::Wayland(context)))
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::X11(ref ctx) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new(wb, el, pf_reqs, &gl_attr, plat_attr)
                .map(|(win, context)| (win, Context::X11(context)))
        }
    }

    #[inline]
    pub fn new_headless<T>(
        el: &EventLoop<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        plat_attr: &PlatformAttributes,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        Self::new_headless_impl(el, pf_reqs, gl_attr, plat_attr, Some(size))
    }

    pub fn new_headless_impl<T>(
        el: &EventLoop<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        plat_attr: &PlatformAttributes,
        size: Option<dpi::PhysicalSize>,
    ) -> Result<Self, CreationError> {
        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::Wayland(ref ctx) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new_headless(&el, pf_reqs, &gl_attr, plat_attr, size)
                .map(|ctx| Context::Wayland(ctx))
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::X11(ref ctx) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new_headless(&el, pf_reqs, &gl_attr, plat_attr, size)
                .map(|ctx| Context::X11(ctx))
        }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.make_current(),
            Context::Wayland(ref ctx) => ctx.make_current(),
            Context::OsMesa(ref ctx) => ctx.make_current(),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.make_not_current(),
            Context::Wayland(ref ctx) => ctx.make_not_current(),
            Context::OsMesa(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::X11(ref ctx) => ctx.is_current(),
            Context::Wayland(ref ctx) => ctx.is_current(),
            Context::OsMesa(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match *self {
            Context::X11(ref ctx) => ctx.get_api(),
            Context::Wayland(ref ctx) => ctx.get_api(),
            Context::OsMesa(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match *self {
            Context::X11(ref ctx) => match *ctx.raw_handle() {
                X11Context::Glx(ref ctx) => RawHandle::Glx(ctx.raw_handle()),
                X11Context::Egl(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
            },
            Context::Wayland(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
            Context::OsMesa(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
        }
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match *self {
            Context::X11(ref ctx) => ctx.get_egl_display(),
            Context::Wayland(ref ctx) => ctx.get_egl_display(),
            _ => None,
        }
    }

    #[inline]
    pub fn resize(&self, width: u32, height: u32) {
        match *self {
            Context::X11(_) => (),
            Context::Wayland(ref ctx) => ctx.resize(width, height),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match *self {
            Context::X11(ref ctx) => ctx.get_proc_address(addr),
            Context::Wayland(ref ctx) => ctx.get_proc_address(addr),
            Context::OsMesa(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.swap_buffers(),
            Context::Wayland(ref ctx) => ctx.swap_buffers(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match *self {
            Context::X11(ref ctx) => ctx.get_pixel_format(),
            Context::Wayland(ref ctx) => ctx.get_pixel_format(),
            _ => unreachable!(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct PlatformAttributes {
    /// X11 only: set internally to insure a certain visual xid is used when
    /// choosing the fbconfig.
    pub(crate) x11_visual_xid: Option<std::os::raw::c_ulong>,
}
