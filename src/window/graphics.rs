use windows::{
    core::{InParam, Interface},
    Win32::Graphics::Dxgi::{DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET},
    Win32::{
        Foundation::{HINSTANCE, POINT},
        Graphics::{
            Direct2D::{
                D2D1CreateFactory, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1,
                D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED,
            },
            Direct3D::{D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                D3D11_SDK_VERSION,
            },
            DirectWrite::{DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED},
            Dxgi::IDXGIDevice,
        },
        System::WinRT::Composition::{ICompositionDrawingSurfaceInterop, ICompositorInterop},
    },
    UI::Composition::{CompositionDrawingSurface, CompositionGraphicsDevice, Compositor},
};

thread_local! {
    static DWRITE_FACTORY: windows::core::Result<IDWriteFactory> = create_dwrite_factory();
    static D3D11_DEVICE: windows::core::Result<ID3D11Device> = create_d3d11_device();
    static D2D1_DEVICE: windows::core::Result<ID2D1Device> = create_d2d1_device();
}

fn create_dwrite_factory() -> windows::core::Result<IDWriteFactory> {
    let dwrite_factory =
        unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory::IID) }?;
    Ok(dwrite_factory.cast()?)
}

pub fn dwrite_factory() -> windows::core::Result<IDWriteFactory> {
    DWRITE_FACTORY.with(|v| v.clone())
}

fn create_d3d11_device() -> windows::core::Result<ID3D11Device> {
    fn create_device(driver_type: D3D_DRIVER_TYPE) -> Result<ID3D11Device, windows::core::Error> {
        let mut device: Option<ID3D11Device> = None;
        unsafe {
            D3D11CreateDevice(
                InParam::null(),
                driver_type,
                HINSTANCE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                &[],
                D3D11_SDK_VERSION,
                &mut device,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }?;
        Ok(device.unwrap())
    }

    let device = create_device(D3D_DRIVER_TYPE_HARDWARE);
    let device = if device.is_ok() {
        device
    } else {
        create_device(D3D_DRIVER_TYPE_WARP)
    };
    device
}

pub fn d3d11_device() -> windows::core::Result<ID3D11Device> {
    D3D11_DEVICE.with(|v| v.clone())
}

fn create_d2d1_device() -> Result<ID2D1Device, windows::core::Error> {
    let dxdevice: IDXGIDevice = D3D11_DEVICE.with(|v| v.clone())?.cast()?;
    let options = D2D1_FACTORY_OPTIONS::default();
    let factory: ID2D1Factory1 =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &options) }?;
    let d2device = unsafe { factory.CreateDevice(&dxdevice) }?;
    Ok(d2device)
}

pub fn d2d1_device() -> windows::core::Result<ID2D1Device> {
    D2D1_DEVICE.with(|v| v.clone())
}

pub fn create_composition_graphics_device(
    compositor: &Compositor,
) -> crate::Result<CompositionGraphicsDevice> {
    let interop_compositor: ICompositorInterop = compositor.cast()?;
    let d2device = d2d1_device()?;
    let graphic_device = unsafe { interop_compositor.CreateGraphicsDevice(&d2device) }?;
    Ok(graphic_device)
}

//
// TODO: Actually handle the device reset situation
//
pub fn check_for_device_removed<T>(
    result: windows::core::Result<T>,
) -> windows::core::Result<Option<T>> {
    match result {
        Err(ref e)
            if e.code() == DXGI_ERROR_DEVICE_REMOVED || e.code() == DXGI_ERROR_DEVICE_RESET =>
        {
            Ok(None)
        }
        _ => result.map(|v| Some(v)),
    }
}

pub fn draw<F: Fn(ID2D1DeviceContext, POINT) -> crate::Result<()>>(
    surface: &CompositionDrawingSurface,
    f: F,
) -> crate::Result<()> {
    let mut updateoffset = POINT { x: 0, y: 0 };
    let surface_interop: ICompositionDrawingSurfaceInterop = surface.cast()?;
    let context: Option<ID2D1DeviceContext> = check_for_device_removed(unsafe {
        surface_interop.BeginDraw(std::ptr::null(), &mut updateoffset)
    })?;
    if let Some(context) = context {
        f(context, updateoffset)?;
        unsafe { surface_interop.EndDraw() }?;
    }
    Ok(())
}
