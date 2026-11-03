//! Platform service for the measured D Windows monochrome font profile.
//! Owns only a memory DC and its GDI objects; never selects into a game/window DC.
use crate::font_atlas::BITMAP_BYTES;

pub struct Mask {
    pub bits: [u8; BITMAP_BYTES],
    pub extent: [i32; 2],
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::{ffi::c_void, ptr};
    type Handle = *mut c_void;

    #[repr(C)]
    struct LogFont {
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u8,
        underline: u8,
        strike: u8,
        charset: u8,
        out_precision: u8,
        clip_precision: u8,
        quality: u8,
        pitch: u8,
        face: [u8; 32],
    }
    #[repr(C)]
    struct BitmapInfo {
        header_size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bpp: u16,
        compression: u32,
        image_size: u32,
        xppm: i32,
        yppm: i32,
        used: u32,
        important: u32,
        colors: [u32; 2],
    }
    #[repr(C)]
    #[derive(Default)]
    struct Size {
        x: i32,
        y: i32,
    }
    const _: () = assert!(std::mem::size_of::<LogFont>() == 60);
    const _: () = assert!(std::mem::size_of::<BitmapInfo>() == 48);
    const _: () = assert!(std::mem::size_of::<Size>() == 8);
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateCompatibleDC(dc: Handle) -> Handle;
        fn CreateDIBSection(
            dc: Handle,
            info: *const BitmapInfo,
            usage: u32,
            bits: *mut *mut u8,
            section: Handle,
            offset: u32,
        ) -> Handle;
        fn CreateFontIndirectA(request: *const LogFont) -> Handle;
        fn SelectObject(dc: Handle, object: Handle) -> Handle;
        fn DeleteObject(object: Handle) -> i32;
        fn DeleteDC(dc: Handle) -> i32;
        fn SetBkMode(dc: Handle, mode: i32) -> i32;
        fn SetTextColor(dc: Handle, color: u32) -> u32;
        fn SetTextAlign(dc: Handle, align: u32) -> u32;
        fn TextOutA(dc: Handle, x: i32, y: i32, text: *const u8, count: i32) -> i32;
        fn GetTextExtentPoint32A(dc: Handle, text: *const u8, count: i32, size: *mut Size) -> i32;
        fn GetTextFaceA(dc: Handle, count: i32, face: *mut u8) -> i32;
        fn GdiFlush() -> i32;
    }

    fn valid(handle: Handle) -> bool {
        !handle.is_null() && handle as isize != -1
    }

    /// Thread-confined by its raw handles. Use a non-send engine resource.
    /// The selected face is resolved by Windows for the explicit empty-face
    /// request; a face seen on another machine is not substituted here.
    pub struct Font {
        dc: Handle,
        bitmap: Handle,
        font: Handle,
        old_bitmap: Handle,
        old_font: Handle,
        bits: *mut u8,
        face: String,
    }
    impl Font {
        pub fn new() -> Result<Self, &'static str> {
            let mut this = Self {
                dc: ptr::null_mut(),
                bitmap: ptr::null_mut(),
                font: ptr::null_mut(),
                old_bitmap: ptr::null_mut(),
                old_font: ptr::null_mut(),
                bits: ptr::null_mut(),
                face: String::new(),
            };
            // SAFETY: all pointed-to structs and output buffers live across
            // their calls. GDI owns successful objects until this value drops;
            // Drop also handles each partially constructed state.
            unsafe {
                this.dc = CreateCompatibleDC(ptr::null_mut());
                if !valid(this.dc) {
                    return Err("font memory DC creation failed");
                }
                let info = BitmapInfo {
                    header_size: 40,
                    width: 128,
                    height: -64,
                    planes: 1,
                    bpp: 1,
                    compression: 0,
                    image_size: 0,
                    xppm: 0,
                    yppm: 0,
                    used: 0,
                    important: 0,
                    colors: [0, 0xffffff],
                };
                this.bitmap =
                    CreateDIBSection(this.dc, &info, 0, &mut this.bits, ptr::null_mut(), 0);
                if !valid(this.bitmap) || this.bits.is_null() {
                    return Err("font monochrome bitmap creation failed");
                }
                this.old_bitmap = SelectObject(this.dc, this.bitmap);
                if !valid(this.old_bitmap) {
                    return Err("font bitmap selection failed");
                }
                if SetBkMode(this.dc, 1) == 0
                    || SetTextColor(this.dc, 0xffffff) == u32::MAX
                    || SetTextAlign(this.dc, 0x18) == u32::MAX
                {
                    return Err("font DC profile setup failed");
                }
                let request = LogFont {
                    height: 24,
                    width: 0,
                    escapement: 0,
                    orientation: 0,
                    weight: 400,
                    italic: 0,
                    underline: 0,
                    strike: 0,
                    charset: 0,
                    out_precision: 0,
                    clip_precision: 0,
                    quality: 2,
                    pitch: 0x22,
                    face: [0; 32],
                };
                this.font = CreateFontIndirectA(&request);
                if !valid(this.font) {
                    return Err("font profile creation failed");
                }
                this.old_font = SelectObject(this.dc, this.font);
                if !valid(this.old_font) {
                    return Err("font selection failed");
                }
                let mut face = [0u8; 128];
                let count = GetTextFaceA(this.dc, face.len() as i32, face.as_mut_ptr());
                if count <= 0 || count as usize >= face.len() {
                    return Err("font face query failed");
                }
                this.face = String::from_utf8_lossy(&face[..count as usize]).into_owned();
            }
            Ok(this)
        }
        pub fn face(&self) -> &str {
            &self.face
        }
        pub fn render(&mut self, code: u8) -> Result<Mask, &'static str> {
            if code < 32 {
                return Err("font control glyph construction not established");
            }
            let mut extent = Size::default();
            let mut bits = [0u8; BITMAP_BYTES];
            // SAFETY: self uniquely owns the selected 128x64 top-down 1bpp
            // bitmap (DWORD-aligned stride16, 1024 bytes). Flush before CPU
            // writes/reads; pointers remain valid until Drop deletes it.
            unsafe {
                if GdiFlush() == 0 {
                    return Err("font pre-draw GDI flush failed");
                }
                ptr::write_bytes(self.bits, 0, BITMAP_BYTES);
                if GetTextExtentPoint32A(self.dc, &code, 1, &mut extent) == 0
                    || TextOutA(self.dc, 0, 32, &code, 1) == 0
                    || GdiFlush() == 0
                {
                    return Err("font glyph rasterization failed");
                }
                ptr::copy_nonoverlapping(self.bits, bits.as_mut_ptr(), BITMAP_BYTES);
            }
            Ok(Mask {
                bits,
                extent: [extent.x, extent.y],
            })
        }
    }
    impl Drop for Font {
        fn drop(&mut self) {
            // SAFETY: these are exclusively owned GDI handles. Restore
            // successful prior selections before deleting owned objects.
            unsafe {
                if valid(self.dc) {
                    GdiFlush();
                    if valid(self.old_font) {
                        SelectObject(self.dc, self.old_font);
                    }
                    if valid(self.old_bitmap) {
                        SelectObject(self.dc, self.old_bitmap);
                    }
                    if valid(self.font) {
                        DeleteObject(self.font);
                    }
                    if valid(self.bitmap) {
                        DeleteObject(self.bitmap);
                    }
                    DeleteDC(self.dc);
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;
    pub struct Font;
    impl Font {
        pub fn new() -> Result<Self, &'static str> {
            Err("native Windows font profile requires a platform rasterizer")
        }
        pub fn face(&self) -> &str {
            ""
        }
        pub fn render(&mut self, _: u8) -> Result<Mask, &'static str> {
            Err("native Windows font profile requires a platform rasterizer")
        }
    }
}
pub use platform::Font;
