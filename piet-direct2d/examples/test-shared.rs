//! Basic example of sharing a surface between D2D and D3D.

use std::path::Path;

use winapi::shared::dxgi::{DXGI_MAP_READ, IDXGIKeyedMutex};
use piet::{samples, RenderContext};
use piet_direct2d::{D2DRenderContext, D2DText};
use winapi::shared::dxgi1_2::{IDXGIResource1, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE};
use std::ptr;
use piet::kurbo::Size;

const HIDPI: f32 = 2.0;
const FILE_PREFIX: &str = "d2d-test-";

/*
    pub unsafe fn acquire(&mut self) {
        self.keyed_mutex.AcquireSync(KEYED_MUTEX_D2D_KEY, 0);
    }

    pub unsafe fn release(&mut self) {
        self.keyed_mutex.ReleaseSync(KEYED_MUTEX_EXTERNAL_KEY);
    }
 */

const KEYED_MUTEX_D2D_KEY: u64 = 0;
const KEYED_MUTEX_EXTERNAL_KEY: u64 = 1;


fn main() {
    let size = Size::new(800.0,600.0);
    // Create the D2D factory
    let d2d = piet_direct2d::D2DFactory::new().unwrap();
    let dwrite = piet_direct2d::DwriteFactory::new().unwrap();
    let text = D2DText::new_with_shared_fonts(dwrite, None);

    // Initialize a D3D Device
    let (d3d, d3d_ctx) = piet_direct2d::d3d::D3D11Device::create().unwrap();

    // Create the D2D Device and Context
    let mut device = unsafe { d2d.create_device(d3d.as_dxgi().unwrap().as_raw()).unwrap() };
    let mut context = device.create_device_context().unwrap();

    // Create a texture to render to
    let tex = d3d
        .create_texture(
            size.width as u32,
            size.height as u32,
            piet_direct2d::d3d::TextureMode::Shared,
        )
        .unwrap();

    // Open shared handle
    let dxgi_resource = tex.inner().cast::<IDXGIResource1>().unwrap();
    let mut shared_handle = ptr::null_mut();
    let hr = unsafe {
        dxgi_resource.CreateSharedHandle(
            ptr::null(),
            DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE,
            ptr::null(),
            &mut shared_handle,
        )
    };
    dbg!(hr);
    let keyed_mutex = tex.inner().cast::<IDXGIKeyedMutex>().unwrap();

    // Bind the backing texture to a D2D Bitmap
    let target = unsafe { context.create_bitmap_from_dxgi(&tex.as_dxgi(), HIDPI).unwrap() };

    context.set_target(&target);
    context.set_dpi_scale(HIDPI);
    context.begin_draw();
    let mut piet_context = D2DRenderContext::new(&d2d, text, &mut context);

    unsafe {
        let hr = keyed_mutex.AcquireSync(KEYED_MUTEX_D2D_KEY, 0);
        eprintln!("AcquireSync(KEYED_MUTEX_D2D_KEY) = {:08x}", hr);
    }
    // ---------------------------------------------------------------------------------------------
    // draw with Piet/D2D

    unsafe {
        keyed_mutex.ReleaseSync(KEYED_MUTEX_EXTERNAL_KEY);
        eprintln!("ReleaseSync(KEYED_MUTEX_EXTERNAL_KEY) = {:08x}", hr);
        keyed_mutex.AcquireSync(KEYED_MUTEX_EXTERNAL_KEY, 0);
        eprintln!("ReleaseSync(KEYED_MUTEX_EXTERNAL_KEY) = {:08x}", hr);
    }

    // ---------------------------------------------------------------------------------------------
    // draw with external API

    // 3. Once the D3D context is accessible, can create a shared texture "by hand", no need to modify piet
    // 4. must call end_draw before ReleaseSync to external
    //      -> problem: since the RenderContext borrows the DeviceContext, must destroy the RenderContext
    //      -> modify piet so that the RenderContext can temporarily release control of the target
    //          -> ctx.begin_external_painting(|| { ... });
    // 5. how to expose the native D3D context?
    //      -> currently it's created in druid-shell/platform/window.rs, and basically dropped once the swapchain is created
    //      -> create it in application instead
    //      -> expose it via a platform-specific trait
    // 6. must expose the D2DDeviceContext
    //      -> do the same as for the D3D context
    // 
    // Once all of this is done, 


    unsafe {
        keyed_mutex.ReleaseSync(KEYED_MUTEX_D2D_KEY);
        eprintln!("ReleaseSync(KEYED_MUTEX_D2D_KEY) = {:08x}", hr);
        let hr = keyed_mutex.AcquireSync(KEYED_MUTEX_D2D_KEY, 0);
        eprintln!("AcquireSync(KEYED_MUTEX_D2D_KEY) = {:08x}", hr);
    }

    piet_context.finish().unwrap();
    std::mem::drop(piet_context);
    context.end_draw().unwrap();

    let temp_texture = d3d.create_texture(
        size.width as u32,
        size.height as u32,
        piet_direct2d::d3d::TextureMode::Read,
    ).unwrap();

    // Get the data so we can write it to a file
    // TODO: Have a safe way to accomplish this :D
    let pixel_count = (size.width * size.height) as usize * 4;
    let mut raw_pixels = vec![0_u8; pixel_count];
    unsafe {
        d3d_ctx
            .inner()
            .CopyResource(temp_texture.as_raw() as *mut _, tex.as_raw() as *mut _);
        d3d_ctx.inner().Flush();

        let surface = temp_texture.as_dxgi();
        let mut mapped_rect = std::mem::zeroed();
        let _hr = surface.Map(&mut mapped_rect, DXGI_MAP_READ);
        for y in 0..size.height as usize {
            let src = mapped_rect
                .pBits
                .offset(mapped_rect.Pitch as isize * y as isize);
            let dst = raw_pixels
                .as_mut_ptr()
                .offset(size.width as isize * 4 * y as isize);
            std::ptr::copy_nonoverlapping(src, dst, size.width as usize * 4);
        }
    }

    image::save_buffer(
        "text-shared.png",
        &raw_pixels,
        size.width as u32,
        size.height as u32,
        image::ColorType::Rgba8,
    ).unwrap();
}
