#![deny(unsafe_code)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use euclid::default::Size2D;
use surfman::{
    Adapter, Connection, Context, ContextAttributeFlags, ContextAttributes, Device, Error, GLApi,
    GLVersion, NativeWidget, Surface, SurfaceAccess, SurfaceInfo, SurfaceType,
};

mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

    pub use Gles2 as Gl;
}

/// A Verso rendering context, which holds all of the information needed
/// to render Servo's layout, and bridges WebRender and glutin.
#[derive(Clone)]
pub struct RenderingContext(Rc<RenderingContextData>);

struct RContext {}

impl RContext {
    // /// Create a rendering context instance.
    // pub fn create(
    //     connection: &Connection,
    //     adapter: &Adapter,
    //     surface_type: SurfaceType<NativeWidget>,
    // ) -> Result<Self, Error> {
    //     todo!()
    // }
}

struct RenderingContextData {
    device: RefCell<Device>,
    context: RefCell<Context>,
}

impl Drop for RenderingContextData {
    fn drop(&mut self) {
        let device = &mut self.device.borrow_mut();
        let context = &mut self.context.borrow_mut();
        let _ = device.destroy_context(context);
    }
}

impl RenderingContext {
    /// Create a rendering context instance.
    pub fn create(
        connection: &Connection,
        adapter: &Adapter,
        surface_type: SurfaceType<NativeWidget>,
    ) -> Result<Self, Error> {
        let mut device = connection.create_device(adapter)?;
        let flags = ContextAttributeFlags::ALPHA
            | ContextAttributeFlags::DEPTH
            | ContextAttributeFlags::STENCIL;
        let version = match connection.gl_api() {
            GLApi::GLES => GLVersion { major: 3, minor: 0 },
            GLApi::GL => GLVersion { major: 3, minor: 2 },
        };
        let context_attributes = ContextAttributes { flags, version };
        let context_descriptor = device.create_context_descriptor(&context_attributes)?;
        let mut context = device.create_context(&context_descriptor, None)?;
        let surface_access = SurfaceAccess::GPUOnly;
        let surface = device.create_surface(&context, surface_access, surface_type)?;
        device
            .bind_surface_to_context(&mut context, surface)
            .map_err(|(err, mut surface)| {
                let _ = device.destroy_surface(&mut context, &mut surface);
                err
            })?;

        device.make_context_current(&context)?;

        let device = RefCell::new(device);
        let context = RefCell::new(context);
        let data = RenderingContextData { device, context };
        Ok(RenderingContext(Rc::new(data)))
    }

    /// Create a surface based on provided surface type.
    pub fn create_surface(
        &self,
        surface_type: SurfaceType<NativeWidget>,
    ) -> Result<Surface, Error> {
        let device = &mut self.0.device.borrow_mut();
        let context = &self.0.context.borrow();
        let surface_access = SurfaceAccess::GPUOnly;
        device.create_surface(context, surface_access, surface_type)
    }

    /// Destroy a surface. A surface must call this before dropping.
    pub fn destroy_surface(&self, mut surface: Surface) -> Result<(), Error> {
        let device = &self.0.device.borrow();
        let context = &mut self.0.context.borrow_mut();
        device.destroy_surface(context, &mut surface)
    }

    // /// Create a surface texture.
    // pub fn create_surface_texture(&self, surface: Surface) -> Result<SurfaceTexture, Error> {
    //     let device = &self.0.device.borrow();
    //     let context = &mut self.0.context.borrow_mut();
    //     device
    //         .create_surface_texture(context, surface)
    //         .map_err(|(error, _)| error)
    // }
    //
    // /// Destroy a surface texture.
    // pub fn destroy_surface_texture(
    //     &self,
    //     surface_texture: SurfaceTexture,
    // ) -> Result<Surface, Error> {
    //     let device = &self.0.device.borrow();
    //     let context = &mut self.0.context.borrow_mut();
    //     device
    //         .destroy_surface_texture(context, surface_texture)
    //         .map_err(|(error, _)| error)
    // }

    /// Make GL context current.
    pub fn make_gl_context_current(&self) -> Result<(), Error> {
        let device = &self.0.device.borrow();
        let context = &self.0.context.borrow();
        device.make_context_current(context)
    }

    // /// Get the swap chain of the rendering context.
    // pub fn swap_chain(&self) -> Result<&SwapChain<Device>, Error> {
    //     self.0.swap_chain.as_ref().ok_or(Error::WidgetAttached)
    // }

    /// Resize the rendering context.
    pub fn resize(&self, size: Size2D<i32>) -> Result<(), Error> {
        let device = &mut self.0.device.borrow_mut();
        let context = &mut self.0.context.borrow_mut();
        let mut surface = device.unbind_surface_from_context(context)?.unwrap();
        device.resize_surface(context, &mut surface, size)?;
        device
            .bind_surface_to_context(context, surface)
            .map_err(|(err, mut surface)| {
                let _ = device.destroy_surface(context, &mut surface);
                err
            })
    }

    /// Present the surface of the rendering context.
    pub fn present(&self) -> Result<(), Error> {
        let device = &mut self.0.device.borrow_mut();
        let context = &mut self.0.context.borrow_mut();
        let mut surface = device.unbind_surface_from_context(context)?.unwrap();
        device.present_surface(context, &mut surface)?;
        device
            .bind_surface_to_context(context, surface)
            .map_err(|(err, mut surface)| {
                let _ = device.destroy_surface(context, &mut surface);
                err
            })
    }

    /// Invoke a closure with the surface associated with the current front buffer.
    /// This can be used to create a surfman::SurfaceTexture to blit elsewhere.
    pub fn with_front_buffer<F: FnOnce(&Device, Surface) -> Surface>(&self, f: F) {
        let device = &mut self.0.device.borrow_mut();
        let context = &mut self.0.context.borrow_mut();
        let surface = device
            .unbind_surface_from_context(context)
            .unwrap()
            .unwrap();
        let surface = f(device, surface);
        device.bind_surface_to_context(context, surface).unwrap();
    }

    // /// Get the device of the rendering context.
    // pub fn device(&self) -> std::cell::Ref<Device> {
    //     self.0.device.borrow()
    // }

    /// Get the conntection of the rendering context.
    pub fn connection(&self) -> Connection {
        let device = &self.0.device.borrow();
        device.connection()
    }

    // /// Get the adapter of the rendering context.
    // pub fn adapter(&self) -> Adapter {
    //     let device = &self.0.device.borrow();
    //     device.adapter()
    // }

    // /// Get the native context of the rendering context.
    // pub fn native_context(&self) -> NativeContext {
    //     let device = &self.0.device.borrow();
    //     let context = &self.0.context.borrow();
    //     device.native_context(context)
    // }

    // pub fn native_device(&self) -> NativeDevice {
    //     let device = &self.0.device.borrow();
    //     device.native_device()
    // }
    //
    // pub fn context_attributes(&self) -> ContextAttributes {
    //     let device = &self.0.device.borrow();
    //     let context = &self.0.context.borrow();
    //     let descriptor = &device.context_descriptor(context);
    //     device.context_descriptor_attributes(descriptor)
    // }
    //
    /// Get the context surface info of the rendering context.
    pub fn context_surface_info(&self) -> Result<Option<SurfaceInfo>, Error> {
        let device = &self.0.device.borrow();
        let context = &self.0.context.borrow();
        device.context_surface_info(context)
    }
    //
    // pub fn surface_info(&self, surface: &Surface) -> SurfaceInfo {
    //     let device = &self.0.device.borrow();
    //     device.surface_info(surface)
    // }
    //
    // pub fn surface_texture_object(&self, surface: &SurfaceTexture) -> u32 {
    //     let device = &self.0.device.borrow();
    //     device.surface_texture_object(surface)
    // }
    //
    /// Get the proc address of the rendering context.
    pub fn get_proc_address(&self, name: &str) -> *const c_void {
        let device = &self.0.device.borrow();
        let context = &self.0.context.borrow();
        device.get_proc_address(context, name)
    }
    //
    // pub fn unbind_native_surface_from_context(&self) -> Result<(), Error> {
    //     let device = self.0.device.borrow_mut();
    //     let mut context = self.0.context.borrow_mut();
    //     let mut surface = device.unbind_surface_from_context(&mut context)?.unwrap();
    //     device.destroy_surface(&mut context, &mut surface)?;
    //     Ok(())
    // }
    //
    // pub fn bind_native_surface_to_context(&self, native_widget: NativeWidget) -> Result<(), Error> {
    //     let mut device = self.0.device.borrow_mut();
    //     let mut context = self.0.context.borrow_mut();
    //     let surface_access = SurfaceAccess::GPUOnly;
    //     let surface_type = SurfaceType::Widget { native_widget };
    //     let surface = device.create_surface(&context, surface_access, surface_type)?;
    //     device
    //         .bind_surface_to_context(&mut context, surface)
    //         .map_err(|(err, mut surface)| {
    //             let _ = device.destroy_surface(&mut context, &mut surface);
    //             err
    //         })?;
    //     device.make_context_current(&context)?;
    //     Ok(())
    // }
}
