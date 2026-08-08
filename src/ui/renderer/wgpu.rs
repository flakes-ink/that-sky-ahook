//! wgpu context: instance / surface / device / queue / swapchain config.

use std::ffi::c_void;
use std::ptr::NonNull;

use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use wgpu::SurfaceTargetUnsafe;

use crate::log_info;
use crate::ui::android::surface::NativeWindow;

#[derive(Debug)]
pub enum WgpuError {
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapter(wgpu::RequestAdapterError),
    RequestDevice(wgpu::RequestDeviceError),
    NoDefaultConfig,
}

/// Result of `acquire` (surface error handling).
pub enum AcquireResult {
    Frame(wgpu::SurfaceTexture),
    /// Timeout / Occluded: skip this frame.
    Skip,
    /// Outdated: reconfigure and retry.
    Outdated,
    /// Lost / Validation: needs surface recreation (Phase 16).
    Lost,
}

pub struct WgpuContext {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

impl WgpuContext {
    pub fn new(window: &NativeWindow, width: u32, height: u32) -> Result<Self, WgpuError> {
        // raw-window-handle 0.6: Android variant is `AndroidNdkWindowHandle`.
        let native_window: NonNull<c_void> =
            NonNull::new(window.as_ptr().cast()).expect("ANativeWindow is non-null");
        let raw_window_handle =
            RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(native_window));
        let raw_display_handle = RawDisplayHandle::Android(AndroidDisplayHandle::new());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        // SAFETY (Surface<'static>): the ANativeWindow refcount is held by the
        // renderer's `_window` for its whole lifetime; the display handle is a
        // stateless unit struct.
        let surface: wgpu::Surface<'static> = unsafe {
            instance
                .create_surface_unsafe(SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: Some(raw_display_handle),
                    raw_window_handle,
                })
                .map_err(WgpuError::CreateSurface)?
        };

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(WgpuError::RequestAdapter)?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(WgpuError::RequestDevice)?;

        // Pick format / present / alpha from capabilities (never hardcode).
        let config = surface
            .get_default_config(&adapter, width, height)
            .ok_or(WgpuError::NoDefaultConfig)?;

        let caps = surface.get_capabilities(&adapter);
        log_info!(
            "[rust] surface format: {:?}, present_modes: {:?}, alpha_modes: {:?}",
            caps.formats.first(),
            caps.present_modes,
            caps.alpha_modes
        );

        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Apply a new swapchain size (recreate swapchain).
    pub fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// Acquire the next frame, mapping surface errors to recoverable states.
    pub fn acquire(&self) -> AcquireResult {
        use wgpu::CurrentSurfaceTexture as C;

        match self.surface.get_current_texture() {
            C::Success(frame) | C::Suboptimal(frame) => AcquireResult::Frame(frame),
            C::Timeout | C::Occluded => AcquireResult::Skip,
            C::Outdated => AcquireResult::Outdated,
            C::Lost | C::Validation => {
                crate::log_error!("[rust] surface lost/validation error");
                AcquireResult::Lost
            }
        }
    }

    pub fn present(&self, frame: wgpu::SurfaceTexture) {
        self.queue.present(frame);
    }
}
