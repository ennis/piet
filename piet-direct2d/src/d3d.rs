use std::ptr::null_mut;

// TODO figure out whether to export this or to move `raw_pixels` into this module.
pub use winapi::shared::dxgi::DXGI_MAP_READ;

use winapi::shared::dxgi::{IDXGIDevice, IDXGISurface};
use winapi::shared::dxgiformat::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM};
use winapi::shared::dxgitype::DXGI_SAMPLE_DESC;
use winapi::shared::winerror::{HRESULT, SUCCEEDED};
use winapi::um::d3d11::{D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_BIND_FLAG, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_FLAG, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX, D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, D3D11_RESOURCE_MISC_FLAG};
use winapi::um::d3dcommon::D3D_DRIVER_TYPE_HARDWARE;
use winapi::Interface;

use std::ptr;
use wio::com::ComPtr;

#[derive(Debug)]
pub struct Error(HRESULT);

pub struct D3D11Device(ComPtr<ID3D11Device>);
pub struct D3D11DeviceContext(ComPtr<ID3D11DeviceContext>);
pub struct D3D11Texture2D(ComPtr<ID3D11Texture2D>);
pub struct DxgiDevice(ComPtr<IDXGIDevice>);

// According to Microsoft's docs, D3D11Device does its own synchronization. It also says that "only
// one thread can call a ID3D11DeviceContext at a time", so D3D11DeviceContext is definitely not
// Sync, but probably Send (or else they would have said something).
//
// https://docs.microsoft.com/en-us/windows/win32/direct3d11/overviews-direct3d-11-render-multi-thread-intro
unsafe impl Send for D3D11Device {}
unsafe impl Sync for D3D11Device {}
unsafe impl Send for D3D11DeviceContext {}

pub enum TextureMode {
    Target,
    Read,
    /// Creates a texture that supports CreateSharedHandle
    Shared
}

impl TextureMode {
    pub fn usage(&self) -> D3D11_USAGE {
        match self {
            TextureMode::Target => D3D11_USAGE_DEFAULT,
            TextureMode::Read => D3D11_USAGE_STAGING,
            TextureMode::Shared => D3D11_USAGE_DEFAULT
        }
    }

    pub fn bind_flags(&self) -> D3D11_BIND_FLAG {
        match self {
            TextureMode::Target => D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET,
            TextureMode::Shared => D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET,
            TextureMode::Read => 0,
        }
    }

    pub fn cpu_access_flags(&self) -> D3D11_CPU_ACCESS_FLAG {
        match self {
            TextureMode::Target => 0,
            TextureMode::Shared => 0,
            TextureMode::Read => D3D11_CPU_ACCESS_READ,
        }
    }

    pub fn misc_flags(&self) -> D3D11_RESOURCE_MISC_FLAG {
        match self {
            TextureMode::Shared => D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX | D3D11_RESOURCE_MISC_SHARED_NTHANDLE,
            _ => 0
        }
    }
}

unsafe fn wrap<T, U, F>(hr: HRESULT, ptr: *mut T, f: F) -> Result<U, Error>
where
    F: Fn(ComPtr<T>) -> U,
    T: Interface,
{
    if SUCCEEDED(hr) {
        Ok(f(ComPtr::from_raw(ptr)))
    } else {
        Err(Error(hr))
    }
}

impl D3D11Device {
    pub fn inner(&self) -> &ComPtr<ID3D11Device> {
        &self.0
    }

    // This function only supports a fraction of available options.
    pub fn create() -> Result<(D3D11Device, D3D11DeviceContext), Error> {
        unsafe {
            let mut ptr = null_mut();
            let mut ctx_ptr = null_mut();
            let hr = D3D11CreateDevice(
                null_mut(), /* adapter */
                D3D_DRIVER_TYPE_HARDWARE,
                null_mut(), /* module */
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                null_mut(), /* feature levels */
                0,
                D3D11_SDK_VERSION,
                &mut ptr,
                null_mut(), /* feature level */
                &mut ctx_ptr,
            );
            let device = wrap(hr, ptr, D3D11Device)?;
            let device_ctx = wrap(hr, ctx_ptr, D3D11DeviceContext)?;
            Ok((device, device_ctx))
        }
    }

    pub fn as_dxgi(&self) -> Option<DxgiDevice> {
        self.0.cast().ok().map(DxgiDevice)
    }

    pub fn create_texture(
        &self,
        width: u32,
        height: u32,
        mode: TextureMode,
    ) -> Result<D3D11Texture2D, Error> {
        unsafe {
            let mut ptr = null_mut();
            let desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: mode.usage(),
                BindFlags: mode.bind_flags(),
                CPUAccessFlags: mode.cpu_access_flags(),
                MiscFlags: mode.misc_flags(),
            };
            let hr = self.0.CreateTexture2D(&desc, null_mut(), &mut ptr);
            wrap(hr, ptr, D3D11Texture2D)
        }
    }
}

impl D3D11DeviceContext {
    pub fn inner(&self) -> &ComPtr<ID3D11DeviceContext> {
        &self.0
    }
}

impl D3D11Texture2D {
    pub fn inner(&self) -> &ComPtr<ID3D11Texture2D> {
        &self.0
    }

    pub fn as_dxgi(&self) -> ComPtr<IDXGISurface> {
        self.0.cast().unwrap()
    }

    pub fn as_raw(&self) -> *mut ID3D11Texture2D {
        self.0.as_raw()
    }
}

impl DxgiDevice {
    pub fn as_raw(&self) -> *mut IDXGIDevice {
        self.0.as_raw()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "HRESULT error {}", self.0)
    }
}

impl std::error::Error for Error {}
