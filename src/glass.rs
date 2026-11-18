//! Optical edging for the transparent window, cached as a Direct2D bitmap.

use std::cell::RefCell;

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Direct2D::{Common::*, *};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};
use windows::core::Result;

#[derive(Clone, Copy, PartialEq)]
pub struct Style {
    pub dark: bool,
    pub controls: [u8; 5],
}

/// At zero transparency, the tint leaves a faint hint of the desktop.
pub fn tint_opacity(dark: bool, transparency: u8) -> f32 {
    const MIN_TRANSPARENCY_OPACITY: f32 = 0.98;
    if dark {
        let remaining = 1.0 - f32::from(transparency.min(200)) / 200.0;
        0.10 + (MIN_TRANSPARENCY_OPACITY - 0.10) * remaining.powf(2.3)
    } else {
        let remaining = 1.0 - f32::from(transparency.min(100)) / 100.0;
        0.18 + (MIN_TRANSPARENCY_OPACITY - 0.18) * remaining
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Geometry {
    size: [f32; 2],
    pixels: D2D_SIZE_U,
    pointer: [f32; 2],
    radius: f32,
}

impl Geometry {
    fn get(rt: &ID2D1HwndRenderTarget, radius: f32) -> Option<Self> {
        unsafe {
            let size = rt.GetSize();
            if size.width < 2.0 || size.height < 2.0 {
                return None;
            }
            let hwnd = rt.GetHwnd();
            let mut window = RECT::default();
            GetWindowRect(hwnd, &mut window).ok()?;
            let mut cursor = POINT::default();
            let pointer = if GetCursorPos(&mut cursor).is_ok()
                && cursor.x >= window.left
                && cursor.x < window.right
                && cursor.y >= window.top
                && cursor.y < window.bottom
            {
                let quantize = |pos, start, end| {
                    let fraction = (pos - start) as f32 / (end - start) as f32;
                    ((fraction * 2.0 - 1.0) * 24.0).round() / 24.0
                };
                [
                    quantize(cursor.x, window.left, window.right),
                    quantize(cursor.y, window.top, window.bottom),
                ]
            } else {
                [0.0; 2]
            };
            let (mut dpi_x, mut dpi_y) = (96.0, 96.0);
            rt.GetDpi(&mut dpi_x, &mut dpi_y);
            let dip = dpi_x.max(1.0) / 96.0;
            let scale = dip.min(1.25).min((1_700_000.0 / (size.width * size.height)).sqrt());
            Some(Self {
                size: [size.width, size.height],
                pixels: D2D_SIZE_U {
                    width: (size.width * scale).ceil() as u32,
                    height: (size.height * scale).ceil() as u32,
                },
                pointer,
                radius,
            })
        }
    }
}

#[repr(C)]
struct Uniforms {
    lens: [f32; 4],
    optics: [f32; 4],
    pointer: [f32; 4],
}

impl Uniforms {
    fn new(geometry: Geometry, style: Style) -> Self {
        let [_, refraction, distortion, dispersion, edge] = style.controls.map(|v| v as f32 / 100.0);
        let [width, height] = geometry.size;
        let radius = geometry.radius.min(width / 2.0).min(height / 2.0);
        let limit = width.min(height) * 0.47;
        let baseline = (radius * 1.26).clamp(10.0, 28.0).min(height * 0.37).min(limit);
        let bevel = if edge <= 0.3 {
            baseline * edge / 0.3
        } else {
            baseline + (limit - baseline) * (edge - 0.3) / 0.7
        };
        Self {
            lens: [width, height, radius, bevel],
            optics: [refraction, distortion, dispersion, if style.dark { 0.0 } else { 1.0 }],
            pointer: [geometry.pointer[0], geometry.pointer[1], 0.0, 0.0],
        }
    }
}

struct GlassDevice {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    shader: ID3D11ComputeShader,
    constants: ID3D11Buffer,
}

impl GlassDevice {
    fn new() -> Result<Self> {
        unsafe {
            let (mut device, mut context) = (None, None);
            let mut create = |driver| {
                D3D11CreateDevice(
                    None,
                    driver,
                    Default::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    Some(&mut context),
                )
            };
            create(D3D_DRIVER_TYPE_HARDWARE).or_else(|_| create(D3D_DRIVER_TYPE_WARP))?;
            let device = device.unwrap();
            let (mut shader, mut constants) = (None, None);
            device.CreateComputeShader(
                include_bytes!(concat!(env!("OUT_DIR"), "/liquid-glass.cso")),
                None,
                Some(&mut shader),
            )?;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: size_of::<Uniforms>() as u32,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut constants),
            )?;
            Ok(Self {
                device,
                context: context.unwrap(),
                shader: shader.unwrap(),
                constants: constants.unwrap(),
            })
        }
    }

    fn draw(&self, rt: &ID2D1HwndRenderTarget, geometry: Geometry, style: Style) -> Result<ID2D1Bitmap> {
        let uniforms = Uniforms::new(geometry, style);
        let D2D_SIZE_U { width, height } = geometry.pixels;
        let texture = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_UNORDERED_ACCESS.0 as u32,
            ..Default::default()
        };
        let (mut output, mut staging, mut uav) = (None, None, None);
        unsafe {
            self.device.CreateTexture2D(&texture, None, Some(&mut output))?;
            self.device.CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    ..texture
                },
                None,
                Some(&mut staging),
            )?;
            let (output, staging) = (output.unwrap(), staging.unwrap());
            self.device.CreateUnorderedAccessView(&output, None, Some(&mut uav))?;
            let ctx = &self.context;
            ctx.UpdateSubresource(&self.constants, 0, None, (&raw const uniforms).cast(), 0, 0);
            ctx.CSSetShader(&self.shader, None);
            ctx.CSSetConstantBuffers(0, Some(&[Some(self.constants.clone())]));
            ctx.CSSetUnorderedAccessViews(0, 1, Some(&uav), None);
            ctx.Dispatch(width.div_ceil(8), height.div_ceil(8), 1);
            ctx.CSSetUnorderedAccessViews(0, 1, Some(&None), None);
            ctx.CopyResource(&staging, &output);
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            ctx.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
            let bitmap = rt.CreateBitmap(
                geometry.pixels,
                Some(mapped.pData),
                mapped.RowPitch,
                &D2D1_BITMAP_PROPERTIES {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.0,
                    dpiY: 96.0,
                },
            );
            ctx.Unmap(&staging, 0);
            bitmap
        }
    }
}

thread_local! {
    static DEVICE: RefCell<Option<Result<GlassDevice>>> = const { RefCell::new(None) };
}

#[derive(Default)]
pub struct Cache(Option<(Geometry, Style, Option<ID2D1Bitmap>)>);

impl Cache {
    pub fn paint(&mut self, rt: &ID2D1HwndRenderTarget, style: Option<Style>, radius: f32) -> bool {
        let Some(style) = style else {
            self.0 = None;
            return false;
        };
        let Some(geometry) = Geometry::get(rt, radius) else {
            return false;
        };
        if self
            .0
            .as_ref()
            .is_none_or(|(last, optics, _)| *last != geometry || *optics != style)
        {
            let bitmap = DEVICE.with(|device| {
                let mut device = device.borrow_mut();
                let device = device.get_or_insert_with(GlassDevice::new);
                match device {
                    Ok(device) => device.draw(rt, geometry, style).ok(),
                    Err(_) => None,
                }
            });
            self.0 = Some((geometry, style, bitmap));
        }
        if let Some((_, _, Some(bitmap))) = &self.0 {
            unsafe {
                rt.DrawBitmap(
                    bitmap,
                    Some(&D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: geometry.size[0],
                        bottom: geometry.size[1],
                    }),
                    1.0,
                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                    None,
                );
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tint_opacity;

    #[test]
    fn glass_tint_allows_the_desktop_through_at_every_setting() {
        for dark in [false, true] {
            let low = tint_opacity(dark, 200);
            let default = tint_opacity(dark, 100);
            let high = tint_opacity(dark, 0);
            assert!((0.0..0.3).contains(&low));
            assert!((high - 0.98).abs() < 0.001);
            assert!(low <= default && default < high && high < 1.0);
        }
    }
}
