//! Independent bounded data readers. No game runtime, graphics API or native DLL.
pub mod flick;
pub mod footprint;
pub mod gb2;
pub mod gbs;
pub mod gtext;
pub mod gti;
pub mod gzp;
pub mod mission;
pub mod music;
pub mod object_spec;
pub mod pixels;
pub mod sound_catalog;
pub mod texture_table;
pub mod tga;
pub mod wav;
pub mod weapons;
pub mod world;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}

pub(crate) fn slice(data: &[u8], at: usize, size: usize) -> Result<&[u8], Error> {
    data.get(at..at.checked_add(size).ok_or(Error("offset overflow"))?)
        .ok_or(Error("truncated data"))
}
pub(crate) fn u32_at(data: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(slice(data, at, 4)?.try_into().unwrap()))
}
pub mod anm;
pub mod cast;
pub mod class_program;
pub mod effect_image;
pub mod effect_program;
pub mod global_data;
pub mod mission_metadata;
pub mod skn;
pub mod world_tables;

pub mod world_bounds;
