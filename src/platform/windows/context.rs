#![cfg(target_os = "windows")]

use std::ptr;

use winapi::shared::windef::HWND;
use winit;

use Api;
use ContextError;
use CreationError;
use GlAttributes;
use GlRequest;
use PixelFormat;
use PixelFormatRequirements;

use api::egl;
use api::egl::ffi::egl::Egl;
use api::egl::Context as EglContext;
use api::wgl::Context as WglContext;
use os::windows::WindowExt;
use platform::RawHandle;

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

pub enum Context {
    /// A regular window
    Egl(EglContext),
    Wgl(WglContext),
    /// A regular window, but invisible.
    HiddenWindowEgl(winit::Window, EglContext),
    HiddenWindowWgl(winit::Window, WglContext),
    /// An EGL pbuffer.
    EglPbuffer(EglContext),
}

impl Context {
    /// See the docs in the crate root file.
    pub unsafe fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
        egl: Option<&Egl>,
    ) -> Result<(winit::Window, Self), CreationError> {
        let window = window_builder.build(events_loop)?;
        let ctxt = Self::new_separate(
            &window,
            pf_reqs,
            gl_attr,
            egl,
        )?;

        Ok((window, ctxt))
    }

    pub unsafe fn new_separate(
        window: &winit::Window,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
        egl: Option<&Egl>,
    ) -> Result<Self, CreationError> {
        let w = window.get_hwnd() as HWND;
        match gl_attr.version {
            GlRequest::Specific(Api::OpenGlEs, (_major, _minor)) => {
                match (gl_attr.sharing, egl) {
                    // We must use WGL.
                    (Some(&Context::HiddenWindowWgl(_, _)), _)
                        | (Some(&Context::Wgl(_)), _)
                        | (None, None) => {
                        let gl_attr_wgl = gl_attr.clone().map_sharing(|ctxt| {
                            match *ctxt {
                                Context::HiddenWindowWgl(_, ref c)
                                    | Context::Wgl(ref c) => c.get_hglrc(),
                                _ => unreachable!(),
                            }
                        });
                        WglContext::new(&pf_reqs, &gl_attr_wgl, w).map(Context::Wgl)
                    }
                    // We must use EGL.
                    (Some(_), Some(egl)) => {
                        let gl_attr_egl = gl_attr.clone().map_sharing(|ctxt| {
                            match *ctxt {
                                Context::Egl(ref c)
                                    | Context::EglPbuffer(ref c)
                                    | Context::HiddenWindowEgl(_, ref c) => c,
                                _ => unreachable!(),
                            }
                        });

                        EglContext::new(
                            egl.clone(),
                            &pf_reqs,
                            &gl_attr_egl,
                            egl::NativeDisplay::Other(Some(ptr::null())),
                        ).and_then(|p| p.finish(w)).map(|c| Context::Egl(c))
                    }
                    // Try EGL, fallback to WGL.
                    (None, Some(egl)) => {
                        let gl_attr_egl = gl_attr.clone().map_sharing(|_| unreachable!());
                        let gl_attr_wgl = gl_attr.clone().map_sharing(|_| unreachable!());

                        if let Ok(c) = EglContext::new(
                            egl.clone(),
                            &pf_reqs,
                            &gl_attr_egl,
                            egl::NativeDisplay::Other(Some(ptr::null())),
                        ).and_then(|p| p.finish(w))
                        {
                            Ok(Context::Egl(c))
                        } else {
                            WglContext::new(&pf_reqs, &gl_attr_wgl, w).map(Context::Wgl)
                        }
                    }
                    _ => panic!(),
                }
            }
            _ => {
                let gl_attr_wgl = gl_attr.clone().map_sharing(|ctxt| {
                    match *ctxt {
                        Context::HiddenWindowWgl(_, ref c)
                            | Context::Wgl(ref c) => c.get_hglrc(),
                        _ => panic!(),
                    }
                });
                WglContext::new(&pf_reqs, &gl_attr_wgl, w).map(Context::Wgl)
            }
        }
    }

    #[inline]
    pub unsafe fn new_context(
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        _shareable_with_windowed_contexts: bool,
        egl: Option<&Egl>,
    ) -> Result<Self, CreationError> {
        // if EGL is available, we try using EGL first
        // if EGL returns an error, we try the hidden window method
        match (gl_attr.sharing, egl) {
            (None, Some(egl))
                | (Some(&Context::Egl(_)), Some(egl))
                | (Some(&Context::HiddenWindowEgl(_, _)), Some(egl))
                | (Some(&Context::EglPbuffer(_)), Some(egl)) => {

                let gl_attr_egl = gl_attr.clone().map_sharing(|ctxt| {
                    match *ctxt {
                        Context::Egl(ref c)
                            | Context::EglPbuffer(ref c)
                            | Context::HiddenWindowEgl(_, ref c) => c,
                        _ => unreachable!(),
                    }
                });

                let native_display = egl::NativeDisplay::Other(None);
                let context = EglContext::new(egl.clone(), pf_reqs, &gl_attr_egl, native_display)
                    .and_then(|prototype| prototype.finish_pbuffer((1, 1)))
                    .map(|ctxt| Context::EglPbuffer(ctxt));

                if let Ok(context) = context {
                    return Ok(context);
                }
            }
            _ => (),
        }

        let window_builder = winit::WindowBuilder::new().with_visibility(false);
        Self::new(window_builder, &el, pf_reqs, gl_attr, egl).map(|(window, context)| match context
        {
            Context::Egl(context) => Context::HiddenWindowEgl(window, context),
            Context::Wgl(context) => Context::HiddenWindowWgl(window, context),
            _ => unreachable!(),
        })
    }

    #[inline]
    pub fn resize(&self, _width: u32, _height: u32) {
        // Method is for API consistency.
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => c.make_current(),
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.make_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => c.is_current(),
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.is_current(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => c.get_proc_address(addr),
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match *self {
            Context::Wgl(ref c) => c.swap_buffers(),
            Context::Egl(ref c) => c.swap_buffers(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => c.get_api(),
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.get_api(),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match *self {
            Context::Wgl(ref c) => c.get_pixel_format(),
            Context::Egl(ref c) => c.get_pixel_format(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => {
                RawHandle::Wgl(c.get_hglrc())
            }
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => RawHandle::Egl(c.raw_handle()),
        }
    }
}
