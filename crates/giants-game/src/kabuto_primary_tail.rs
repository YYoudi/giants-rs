//! Post-selection primary continuation. World/reference/transport ownership is
//! required; this component does not admit or complete a whole primary frame.
use crate::authored_camera_reference;

#[derive(Clone, Copy)]
pub struct State<K> {
    pub transient: Option<K>,
    pub transient_slot: u32,
    pub animation_slot: u32,
    pub definition: i32,
    pub buttons: u32,
    pub flags: u32,
    pub size: f32,
    pub interaction: Option<K>,
    pub reference: Option<K>,
    pub progress: [Option<K>; 2],
    pub position: [u32; 3],
    pub angle: u32,
}
#[derive(Clone, Copy)]
pub struct Input {
    /// Retained earlier caller counter, updated by animation commit when taken.
    pub current_frame: i32,
    pub earlier_behavior: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Links<K> {
    pub transient: Option<K>,
    pub interaction: Option<K>,
    pub progress: [Option<K>; 2],
}
/// Actual world backends are required. This API supplies neither a successful
/// empty renderer/UI/transport nor the event producers publishing these links.
pub trait Services {
    fn controlled(
        &self,
        session: &crate::resident_states::Session,
    ) -> Result<Option<crate::resident_states::Id>, String>;
    fn camera_ranges(&self) -> &CameraRanges;
    fn camera_aspect(&self) -> Result<f32, String>;
    fn request_camera_reference(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        reference: i32,
    ) -> Result<(), String>;
    fn set_camera_pose(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        camera: authored_camera_reference::Output,
    ) -> Result<(), String>;
    fn clear_camera(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
    ) -> Result<(), String>;
    fn widget(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        identifier: u32,
        value: u32,
    ) -> Result<(), String>;
    fn target_status(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        target: Option<crate::resident_states::Id>,
        kind: u32,
    ) -> Result<u32, String>;
    fn start_sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        identifier: u32,
    ) -> Result<(), String>;
    fn network<'a>(
        &'a self,
        session: &crate::resident_states::Session,
        owner: crate::resident_states::Id,
    ) -> Result<crate::kabuto_geometry_mode::NotificationContext<'a>, String>;
    fn notify_sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        packet: &mut u32,
    ) -> Result<(), String>;
    fn publish_position_allowed(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
    ) -> Result<bool, String>;
    fn position_clock_word(&self) -> Result<u32, String>;
    fn notify_position(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        packet: [u32; 5],
        declared_length: u32,
    ) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraRange {
    pub reference: i32,
    pub lower: i32,
    pub upper: i32,
}
/// Immutable external camera-range data. The original image is user supplied;
/// neither these records nor a native executable are distributed with the port.
pub struct CameraRanges {
    definitions: Vec<Option<Vec<CameraRange>>>,
}
impl CameraRanges {
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        Self::decode(&crate::executable_metadata::read(path)?)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        use crate::executable_metadata::{Image, Layout, word};
        let definitions = crate::kabuto_animation_cache::Definitions::decode(bytes)?;
        let image = Image::decode(bytes)?;
        let table = match image.layout {
            Layout::DebugReference => 0x160680,
            Layout::Retail => 0x15a4d8,
            Layout::Original => 0x15c5f8,
        };
        let rows = image.data.data(table, definitions.entries().len() * 28)?;
        let mut result = Vec::with_capacity(definitions.entries().len());
        for row in rows.as_chunks::<28>().0 {
            let pointer = word(row, 20)?;
            if pointer == 0 {
                result.push(None);
                continue;
            }
            let rva = pointer
                .checked_sub(image.data.base)
                .ok_or("camera range below image")?;
            let mut sequence = Vec::new();
            for index in 0..4096u32 {
                let at = rva
                    .checked_add(index * 12)
                    .ok_or("camera range address overflow")?;
                let reference = word(image.data.data(at, 4)?, 0)? as i32;
                if reference < 0 {
                    sequence.push(CameraRange {
                        reference,
                        lower: 0,
                        upper: 0,
                    });
                    break;
                }
                let values = image.data.data(at, 12)?;
                sequence.push(CameraRange {
                    reference,
                    lower: word(values, 4)? as i32,
                    upper: word(values, 8)? as i32,
                });
            }
            if sequence.last().is_none_or(|r| r.reference >= 0) {
                return Err("unterminated camera range table".into());
            }
            result.push(Some(sequence));
        }
        Ok(Self {
            definitions: result,
        })
    }
    pub fn sequence(&self, definition: usize) -> Result<Option<&[CameraRange]>, String> {
        self.definitions
            .get(definition)
            .map(|s| s.as_deref())
            .ok_or_else(|| "camera definition outside acquired table".into())
    }
}
pub trait World<K: Copy> {
    fn state(&self) -> Result<State<K>, String>;
    fn mark(&mut self, actor: K) -> Result<(), String>;
    /// Clear the current transient link and set its bound slot to the sentinel.
    fn clear_transient(&mut self) -> Result<(), String>;
    fn local_owner(&self) -> Result<bool, String>;
    fn camera_table_present(&self, definition: i32) -> Result<bool, String>;
    /// Missing rows refuse a truncated table; no fabricated negative terminator
    /// is supplied. Rows are not queried when camera input is disabled.
    fn camera_range(&self, definition: i32, row: usize) -> Result<Option<CameraRange>, String>;
    fn request_camera_reference(&mut self, reference: i32) -> Result<(), String>;
    /// Real shared reference-camera service for clip 1 / reference 7. Absence
    /// cannot be replaced by an identity pose or an unconditional success.
    fn sample_camera(&mut self) -> Result<authored_camera_reference::Output, String>;
    fn set_camera_pose(&mut self, camera: authored_camera_reference::Output) -> Result<(), String>;
    fn clear_camera(&mut self) -> Result<(), String>;
    fn class(&self, actor: K) -> Result<u32, String>;
    /// Actual availability service: zero clears flag 2, nonzero sets it. The
    /// original request word remains visible, including a raw flag mask.
    fn widget(&mut self, identifier: u32, value: u32) -> Result<(), String>;
    fn target_status(&mut self, target: Option<K>, kind: u32) -> Result<u32, String>;
    /// The two acquired scalar fields of this linked actor, without assigning
    /// them an unproved health/feeding/growth meaning or a foreign layout.
    fn progress_values(&self, actor: K) -> Result<(f32, f32), String>;
    fn start_sound(&mut self, identifier: u32) -> Result<(), String>;
    fn notifying(&self) -> Result<bool, String>;
    fn notify_sound(&mut self, packet: &mut u32) -> Result<(), String>;
    fn clear_progress(&mut self, index: usize) -> Result<(), String>;
    fn or_flags(&mut self, flags: u32) -> Result<(), String>;
    fn publish_position_allowed(&mut self) -> Result<bool, String>;
    fn position_clock_word(&self) -> Result<u32, String>;
    /// Five initialized words accompany the acquired declared length 16. This
    /// does not infer wire encoding, padding or successful delivery.
    fn notify_position(&mut self, packet: [u32; 5], declared_length: u32) -> Result<(), String>;
}

fn camera_reference<K: Copy>(
    world: &impl World<K>,
    definition: i32,
    frame: i32,
) -> Result<i32, String> {
    for row in 0..4096 {
        let Some(range) = world.camera_range(definition, row)? else {
            return Err("Kabuto camera range table is truncated".into());
        };
        if range.reference < 0 || (frame >= range.lower && frame < range.upper) {
            return Ok(range.reference);
        }
    }
    Err("Kabuto camera range table exceeds bounded profile".into())
}

/// Reads after services are live; completed stores and provider effects remain
/// on failure. The owning dispatcher must quarantine retries after an error.
pub fn advance<K: Copy>(input: Input, world: &mut impl World<K>) -> Result<(), String> {
    let s = world.state()?;
    if let Some(actor) = s.transient
        && s.transient_slot != 254
        && s.transient_slot != s.animation_slot
    {
        world.mark(actor)?;
        world.clear_transient()?;
    }
    if world.local_owner()? {
        let definition = world.state()?.definition;
        let has_table = definition >= 0 && world.camera_table_present(definition)?;
        if world.state()?.buttons & 1 == 0 {
            world.clear_camera()?;
        } else if has_table {
            let reference = camera_reference(world, definition, input.current_frame)?;
            world.request_camera_reference(reference)?;
        } else {
            let camera = world.sample_camera()?;
            world.set_camera_pose(camera)?;
        }
        let special = match world.state()?.interaction {
            Some(actor) => world.class(actor)? == 745,
            None => false,
        };
        for identifier in [0x2c, 0x2d, 0x2e] {
            let value = u32::from(world.state()?.size >= 3.);
            world.widget(identifier, value)?;
        }
        let value = world.state()?.flags & 0x80000;
        world.widget(0x2f8, value)?;
        for (identifier, index) in [(0x301, 0), (0x302, 0), (0x303, 1), (0x304, 1)] {
            let s = world.state()?;
            world.widget(
                identifier,
                u32::from(s.size >= 3. && s.progress[index].is_none()),
            )?;
        }
        for (identifier, index) in [(0x305, 0), (0x306, 0), (0x307, 1), (0x308, 1)] {
            let value = u32::from(world.state()?.progress[index].is_some());
            world.widget(identifier, value)?;
        }
        for identifier in [0x2bd, 0x2be] {
            let value = u32::from(world.state()?.size < 3.);
            world.widget(identifier, value)?;
        }
        let s = world.state()?;
        world.widget(
            0x84,
            u32::from(s.reference.is_some() && special && s.buttons & 2 != 0),
        )?;
        let flags = world.state()?.flags;
        world.widget(0xc7, u32::from(flags & 0x4800 == 0x4800))?;
        let flags = world.state()?.flags;
        world.widget(0xc8, u32::from(flags & 0x1800 == 0x1800))?;
        let s = world.state()?;
        world.widget(
            0x81,
            u32::from(s.flags & 0x800 == 0 && special && s.buttons & 2 != 0),
        )?;
        world.widget(0x82, 0)?;
        let present = world.state()?.interaction.is_some();
        world.widget(0x13, u32::from(!present || special))?;
        let s = world.state()?;
        let value = if s.flags & 0x800 != 0 {
            world.target_status(s.interaction, 2)?
        } else {
            u32::from(s.interaction.is_some() && !special)
        };
        world.widget(0x34, value)?;
    }
    for index in 0..2 {
        if let Some(actor) = world.state()?.progress[index] {
            let (value, limit) = world.progress_values(actor)?;
            if value >= limit {
                world.start_sound(0x45b)?;
                if world.notifying()? {
                    let mut packet = 0x45b;
                    world.notify_sound(&mut packet)?;
                }
                world.clear_progress(index)?;
            }
        }
    }
    if matches!(input.earlier_behavior, 12 | 13) {
        world.or_flags(0x300)?;
    }
    if world.notifying()? && world.publish_position_allowed()? {
        let s = world.state()?;
        let tick = world.position_clock_word()?;
        world.notify_position(
            [s.position[0], s.position[1], s.position[2], s.angle, tick],
            16,
        )?;
    }
    Ok(())
}
