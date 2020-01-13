use crate::config::{Config, SwapInterval};
use crate::platform_impl;

use glutin_interface::{NativePixmap, NativePixmapSource, NativeWindow, NativeWindowSource};
use winit_types::dpi;
use winit_types::error::Error;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
/// A [`Surface`]'s type. Returned from calling
/// [`SurfaceTypeTrait::surface_type()`] on the type specializing your
/// [`Surface`].
///
/// [`Surface`]: crate::surface::Surface
/// [`SurfaceTypeTrait::surface_type()`]: crate::surface::SurfaceTypeTrait::surface_type()
pub enum SurfaceType {
    /// A [`Window`](crate::surface::Window)
    Window,
    /// A [`PBuffer`](crate::surface::PBuffer)
    PBuffer,
    /// A [`Pixmap`](crate::surface::Pixmap)
    Pixmap,
}

/// A trait implemented on all allowed specializations of [`Surface`].
///
/// [`Surface`]s can only be specialized by the [`PBuffer`], [`Pixmap`], and
/// [`Window`] types.
///
/// [`Surface`]: crate::surface::Surface
/// [`Window`]: crate::surface::Window
/// [`PBuffer`]: crate::surface::PBuffer
/// [`Pixmap`]: crate::surface::Pixmap
pub trait SurfaceTypeTrait {
    /// Returns the [`SurfaceType`] of the specialization.
    ///
    /// [`SurfaceType`]: crate::surface::SurfaceType
    fn surface_type() -> SurfaceType;
}

#[derive(Copy, Clone, Debug)]
pub enum Window {}
#[derive(Copy, Clone, Debug)]
pub enum PBuffer {}
#[derive(Copy, Clone, Debug)]
pub enum Pixmap {}

impl SurfaceTypeTrait for Window {
    fn surface_type() -> SurfaceType {
        SurfaceType::Window
    }
}

impl SurfaceTypeTrait for PBuffer {
    fn surface_type() -> SurfaceType {
        SurfaceType::PBuffer
    }
}

impl SurfaceTypeTrait for Pixmap {
    fn surface_type() -> SurfaceType {
        SurfaceType::Pixmap
    }
}

#[derive(Debug)]
pub struct Surface<T: SurfaceTypeTrait>(pub(crate) platform_impl::Surface<T>);

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    pub fn get_config(&self) -> Config {
        self.0.get_config()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.0.make_not_current()
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new_pixmap<NPS: NativePixmapSource>(
        conf: &Config,
        nps: &NPS,
        wb: NPS::PixmapBuilder,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        platform_impl::Surface::<Pixmap>::new(conf.as_ref(), nps, wb)
            .map(|(pix, surf)| (pix, Surface(surf)))
    }

    #[inline]
    pub unsafe fn new_from_existing_pixmap<NP: NativePixmap>(conf: &Config, np: &NP) -> Result<Self, Error> {
        platform_impl::Surface::<Pixmap>::new_existing(conf.as_ref(), np).map(Surface)
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new_pbuffer(conf: &Config, size: dpi::PhysicalSize<u32>) -> Result<Self, Error> {
        platform_impl::Surface::<PBuffer>::new(conf.as_ref(), size).map(Surface)
    }
}

impl Surface<Window> {
    #[inline]
    pub unsafe fn new_window<NWS: NativeWindowSource>(
        conf: &Config,
        nws: &NWS,
        wb: NWS::WindowBuilder,
    ) -> Result<(NWS::Window, Self), Error> {
        platform_impl::Surface::<Window>::new(conf.as_ref(), nws, wb)
            .map(|(win, surf)| (win, Surface(surf)))
    }

    #[inline]
    pub unsafe fn new_from_existing_window<NW: NativeWindow>(conf: &Config, nw: &NW) -> Result<Self, Error> {
        platform_impl::Surface::<Window>::new_existing(conf.as_ref(), nw).map(Surface)
    }

    /// Swaps the buffers in case of double or triple buffering.
    ///
    /// You should call this function every time you have finished rendering, or
    /// the image may not be displayed on the screen.
    ///
    /// **Warning**: if the swap interval when creating the surface was not
    /// `DontWait` or your graphics driver decided to override your requested
    /// behaviour, this function may block. Please refer to [`SwapInterval`].
    ///
    /// [`SwapInterval`]: crate::config::SwapInterval
    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        self.0.swap_buffers()
    }

    /// Similiar to [`Surface::swap_buffers`] but allows specifying damage
    /// rectangles.
    ///
    /// [`Surface::swap_buffers`]: crate::surface::Surface::swap_buffers()
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        self.0.swap_buffers_with_damage(rects)
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize<u32>) {
        #![cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        self.0.update_after_resize(size);
    }

    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        // FIXME
        unimplemented!()
        //self.0.modify_swap_interval()
    }
}
