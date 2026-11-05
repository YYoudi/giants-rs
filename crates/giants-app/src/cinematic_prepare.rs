//! CPU-only use of the real scene preparers. No App, plugins, GPU or audio.
use crate::{
    actor_scene, attachment_scene, cinematic_camera, cinematic_scene, gun_scene,
    rigid_attachment_scene, spear_scene, sword_scene,
};
use std::{collections::BTreeMap, io::Write, path::Path};

pub fn owners(
    cinematic: &cinematic_camera::Cinematic,
    gun: &gun_scene::Scene,
    staff: &attachment_scene::Scene,
    spears: &spear_scene::Scene,
    sword: Option<&sword_scene::Scene>,
) -> Result<cinematic_scene::AttachmentOwners, &'static str> {
    let mut result = cinematic_scene::AttachmentOwners::new(cinematic.segments().len());
    for director in 0..cinematic.segments().len() {
        result.add(director, gun.owned_slots(director), "gun")?;
        result.add(director, staff.owned_slots(director), "staff")?;
        result.add(director, spears.owned_slots(director), "spear")?;
        if let Some(s) = sword {
            result.add(director, s.owned_slots(director), "sword")?;
        }
    }
    Ok(result)
}

pub fn audit(root: &Path, cases: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()?;
    if parent.starts_with(root) {
        return Err("Audit output must stay outside read-only data".into());
    }
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in std::fs::read_to_string(cases)?.lines() {
        let (world, seq) = line
            .split_once('\t')
            .ok_or("Audit requires world/sequence TSV")?;
        if ![world, seq]
            .into_iter()
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        {
            return Err("Unsafe audit identifier".into());
        }
        grouped.entry(world.into()).or_default().push(seq.into());
    }
    let mut out = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?,
    );
    println!(
        "cinematic_consumer_audit schema_version=2 cpu_only=true app_created=false gpu_initialized=false audio_initialized=false"
    );
    let voices = giants_game::stream_resources::Voices::load(root)?;
    for (world, sequences) in grouped {
        let source = giants_game::world_source::resolve(root, &world)?;
        let config = giants_formats::world::parse_world(&std::fs::read(
            root.join("Bin").join(&source.catalog),
        )?)?;
        let resources =
            giants_game::scene_resources::SceneResources::load(root, &source.catalog, "English")?;
        for sequence in sequences {
            println!("cinematic_consumer_audit_case world={world} sequence={sequence}");
            let mut stage = "director";
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                if !config
                    .cinematics
                    .iter()
                    .any(|c| c.name.eq_ignore_ascii_case(&sequence))
                {
                    return Err("Sequence is not declared in this world".into());
                }
                let bytes = std::fs::read(giants_game::flick::sequence_path(root, &sequence)?)?;
                stage = "declarations";
                let metadata = giants_formats::flick::discover_dependencies(&bytes)?;
                let mut counts = BTreeMap::<u32, usize>::new();
                let mut numeric = 0;
                let mut checked = 0;
                let mut missing = 0;
                for command in &metadata.commands {
                    *counts.entry(command.opcode).or_default() += 1;
                    numeric += usize::from(
                        giants_game::cinematic_sound::Request::decode(command)?.is_some(),
                    );
                    if matches!(command.opcode, 44 | 52) {
                        checked += 1;
                        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                            let name = command.label.as_deref().ok_or("stream label absent")?;
                            std::fs::metadata(voices.resolve(name)?)?;
                            Ok(())
                        })();
                        if let Err(reason) = result {
                            missing += 1;
                            println!(
                                "cinematic_audit_stream_missing offset={} opcode={} reason={reason:?}",
                                command.offset, command.opcode
                            );
                        }
                    }
                }
                let effects = giants_formats::flick::effect_commands(&bytes)?;
                let count = |opcode| counts.get(&opcode).copied().unwrap_or(0);
                println!(
                    "cinematic_audit_declarations actors={} active_effect_requests={} voice_requests={} subtitle_calls={} sync_requests={} numeric_sound_requests={} unclassified_commands_28_30={} credits_commands={} declared_effect_slots={} runtime_profile_supported={}",
                    metadata.actors.len(),
                    effects.iter().filter(|e| e.slot >= 0).count(),
                    count(44),
                    count(16),
                    count(52),
                    numeric,
                    count(28) + count(30),
                    count(47) + count(49) + count(51) + count(53),
                    metadata.opaque_blocks,
                    metadata.runtime_profile_supported,
                );
                for (opcode, count) in counts {
                    println!("cinematic_audit_command opcode={opcode} count={count}");
                }
                println!(
                    "cinematic_audit_stream_resources checked={checked} missing={missing} scope=file_presence_only"
                );
                match giants_game::cinematic_fx::Bindings::load(&resources, &bytes) {
                    Ok(_) => println!(
                        "cinematic_audit_mission_effect_binding result=bound live_execution_verified=false"
                    ),
                    Err(reason) => println!(
                        "cinematic_audit_mission_effect_binding result=refused reason={reason:?}"
                    ),
                }
                stage = "director";
                let animation = giants_formats::anm::inline_flk(&bytes)?;
                let (cinematic, _) = cinematic_camera::Cinematic::load(
                    root,
                    cinematic_camera::Load {
                        tick: Some(animation.timeline[0]),
                        benchmark: false,
                        directors: 1,
                        sequence: Some(&sequence),
                        full_chain: false,
                        preview: true,
                    },
                )?;
                stage = "spear";
                let spears = spear_scene::Scene::load(root, &cinematic, None, &config)?;
                stage = "gun";
                let gun = gun_scene::Scene::load(root, &cinematic, None, &config)?;
                stage = "staff";
                let staff = attachment_scene::Scene::load(root, &cinematic, None)?;
                stage = "bodies";
                let actors = actor_scene::Scene::load(root, &cinematic, None, &resources, &config)?;
                let sword = match sword_scene::Scene::load(root, &cinematic, None, &config) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        eprintln!(
                            "cinematic_specialized_consumer_deferred consumer=sword reason={e:?}"
                        );
                        None
                    }
                };
                stage = "ownership";
                let owners = owners(&cinematic, &gun, &staff, &spears, sword.as_ref())?;
                stage = "rigid_and_effects";
                let _rigid = rigid_attachment_scene::Scene::load(
                    root, &cinematic, None, &resources, &actors, &owners, &config,
                )?;
                Ok(())
            })();
            match result {
                Ok(()) => writeln!(
                    out,
                    "prepared\t{world}\t{sequence}\tall_actor_consumers\tno_gpu_no_audio_no_scene_fidelity"
                )?,
                Err(e) => writeln!(out, "error\t{world}\t{sequence}\t{stage}\t{e:?}")?,
            }
            out.flush()?;
        }
    }
    println!(
        "cinematic_consumer_audit finished=true gpu_initialized=false audio_initialized=false scene_complete=false"
    );
    Ok(())
}
