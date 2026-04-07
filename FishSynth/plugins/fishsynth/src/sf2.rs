use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(windows)]
use windows::Win32::Foundation::HMODULE;
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};

use soundfont::raw::{GeneratorType, SampleChunk, SampleHeader};
use soundfont::SfEnum;
use soundfont::{SoundFont2, Zone};

const WAVETABLE_SIZE: usize = 2048;

#[derive(Debug, Clone, Copy)]
pub struct Sf2Env {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

#[derive(Debug, Clone)]
pub struct Sf2Zone {
    pub key_range: (u8, u8),
    pub vel_range: (u8, u8),
    pub root_key: u8,
    pub tune_cents: i16,
    pub pan: f32,
    pub amp_gain: f32,
    pub sample_rate: f32,
    pub env: Sf2Env,
    pub table_left: Arc<Vec<f32>>,
    pub table_right: Option<Arc<Vec<f32>>>,
}

#[derive(Debug, Clone)]
pub struct Sf2Preset {
    pub zones: Vec<Arc<Sf2Zone>>,
}

#[derive(Debug, Clone)]
pub struct Sf2Bank {
    pub presets: Vec<Option<Sf2Preset>>,
    choir_aah_program: Option<u8>,
    choir_ooh_program: Option<u8>,
}

#[derive(Debug, Clone)]
struct GenValues {
    key_range: (u8, u8),
    vel_range: (u8, u8),
    root_key: Option<u8>,
    tune_cents: i16,
    pan: i16,
    attenuation_cdb: i16,
    attack_tc: i16,
    decay_tc: i16,
    sustain_cdb: i16,
    release_tc: i16,
    sample_id: Option<u16>,
    start_offset: i32,
    end_offset: i32,
    loop_start_offset: i32,
    loop_end_offset: i32,
}

impl Default for GenValues {
    fn default() -> Self {
        Self {
            key_range: (0, 127),
            vel_range: (0, 127),
            root_key: None,
            tune_cents: 0,
            pan: 0,
            attenuation_cdb: 0,
            attack_tc: 0,
            decay_tc: 0,
            sustain_cdb: 0,
            release_tc: 0,
            sample_id: None,
            start_offset: 0,
            end_offset: 0,
            loop_start_offset: 0,
            loop_end_offset: 0,
        }
    }
}

impl GenValues {
    fn apply_gen(&mut self, zone: &Zone) {
        for gen in &zone.gen_list {
            if matches!(gen.ty, SfEnum::Value(GeneratorType::KeyRange)) {
                if let Some(range) = gen.amount.as_range() {
                    self.key_range = (range.low, range.high);
                }
                continue;
            }
            if matches!(gen.ty, SfEnum::Value(GeneratorType::VelRange)) {
                if let Some(range) = gen.amount.as_range() {
                    self.vel_range = (range.low, range.high);
                }
                continue;
            }

            let amount_i16 = gen.amount.as_i16().copied().unwrap_or(0);
            if matches!(gen.ty, SfEnum::Value(GeneratorType::OverridingRootKey)) {
                if let Some(root) = gen.amount.as_u16() {
                    self.root_key = Some(*root as u8);
                }
                continue;
            }
            if matches!(gen.ty, SfEnum::Value(GeneratorType::SampleID)) {
                if let Some(sample) = gen.amount.as_u16() {
                    self.sample_id = Some(*sample);
                }
                continue;
            }
            match gen.ty {
                SfEnum::Value(GeneratorType::CoarseTune) => {
                    self.tune_cents += amount_i16.saturating_mul(100)
                }
                SfEnum::Value(GeneratorType::FineTune) => self.tune_cents += amount_i16,
                SfEnum::Value(GeneratorType::Pan) => self.pan += amount_i16,
                SfEnum::Value(GeneratorType::InitialAttenuation) => {
                    self.attenuation_cdb += amount_i16
                }
                SfEnum::Value(GeneratorType::AttackVolEnv) => self.attack_tc += amount_i16,
                SfEnum::Value(GeneratorType::DecayVolEnv) => self.decay_tc += amount_i16,
                SfEnum::Value(GeneratorType::SustainVolEnv) => self.sustain_cdb += amount_i16,
                SfEnum::Value(GeneratorType::ReleaseVolEnv) => self.release_tc += amount_i16,
                SfEnum::Value(GeneratorType::StartAddrsOffset) => {
                    self.start_offset += amount_i16 as i32
                }
                SfEnum::Value(GeneratorType::EndAddrsOffset) => self.end_offset += amount_i16 as i32,
                SfEnum::Value(GeneratorType::StartloopAddrsOffset) => {
                    self.loop_start_offset += amount_i16 as i32
                }
                SfEnum::Value(GeneratorType::EndloopAddrsOffset) => {
                    self.loop_end_offset += amount_i16 as i32
                }
                SfEnum::Value(GeneratorType::StartAddrsCoarseOffset) => {
                    self.start_offset += (amount_i16 as i32) * 32768
                }
                SfEnum::Value(GeneratorType::EndAddrsCoarseOffset) => {
                    self.end_offset += (amount_i16 as i32) * 32768
                }
                SfEnum::Value(GeneratorType::StartloopAddrsCoarseOffset) => {
                    self.loop_start_offset += (amount_i16 as i32) * 32768
                }
                SfEnum::Value(GeneratorType::EndloopAddrsCoarseOffset) => {
                    self.loop_end_offset += (amount_i16 as i32) * 32768
                }
                _ => {}
            }
        }
    }

    fn merge_from(&mut self, other: &GenValues) {
        self.key_range = other.key_range;
        self.vel_range = other.vel_range;
        if other.root_key.is_some() {
            self.root_key = other.root_key;
        }
        self.tune_cents += other.tune_cents;
        self.pan += other.pan;
        self.attenuation_cdb += other.attenuation_cdb;
        self.attack_tc += other.attack_tc;
        self.decay_tc += other.decay_tc;
        self.sustain_cdb += other.sustain_cdb;
        self.release_tc += other.release_tc;
        if other.sample_id.is_some() {
            self.sample_id = other.sample_id;
        }
        self.start_offset += other.start_offset;
        self.end_offset += other.end_offset;
        self.loop_start_offset += other.loop_start_offset;
        self.loop_end_offset += other.loop_end_offset;
    }
}

pub fn load_sf2_bank(path: &Path) -> Result<Sf2Bank, String> {
    let mut file = File::open(path).map_err(|err| format!("sf2 open failed: {err}"))?;
    let sf2 = SoundFont2::load(&mut file)
        .map_err(|err| format!("sf2 parse failed: {err:?}"))?
        .sort_presets();

    let smpl = sf2
        .sample_data
        .smpl
        .ok_or_else(|| "sf2 missing smpl chunk".to_string())?;
    let sample_data = read_sample_chunk_i16(&mut file, smpl)?;

    let mut presets = vec![None; 128];
    for preset in &sf2.presets {
        if preset.header.bank != 0 {
            continue;
        }
        let preset_index = preset.header.preset as usize;
        if preset_index >= presets.len() {
            continue;
        }
        let zones = build_preset_zones(&sf2, preset, &sample_data)?;
        presets[preset_index] = Some(Sf2Preset { zones });
    }

    let choir_aah_program = find_program_by_names(
        &sf2,
        &[
            "choir aah",
            "choir ahh",
            "choir aahs",
            "female choir",
        ],
    );
    let choir_ooh_program = find_program_by_names(
        &sf2,
        &[
            "choir ooh",
            "choir oh",
            "choir oohs",
            "male choir",
        ],
    );

    Ok(Sf2Bank {
        presets,
        choir_aah_program,
        choir_ooh_program,
    })
}

pub fn resolve_sf2_path() -> PathBuf {
    if let Some(path) = env_sf2_path().filter(|path| path.is_file()) {
        return path;
    }

    let mut candidates = Vec::new();
    if let Some(module_dir) = plugin_module_dir().or_else(exe_dir) {
        candidates.push(module_dir.join("SF").join("Ling.sf2"));

        if let Some(contents_dir) = module_dir.parent() {
            candidates.push(contents_dir.join("Resources").join("SF").join("Ling.sf2"));
        }

        if let Some(bundle_root) = find_bundle_root(&module_dir) {
            candidates.push(bundle_root.join("SF").join("Ling.sf2"));
        }
    }

    candidates.push(PathBuf::from("SF").join("Ling.sf2"));

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("SF").join("Ling.sf2"))
}

fn env_sf2_path() -> Option<PathBuf> {
    let raw = std::env::var("FISHSYNTH_SF2_PATH").ok()?;
    let path = PathBuf::from(raw);
    if path.is_dir() {
        Some(path.join("Ling.sf2"))
    } else {
        Some(path)
    }
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.to_path_buf()))
}

fn find_bundle_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = Some(start_dir);
    while let Some(dir) = current {
        if dir
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("vst3"))
        {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

#[cfg(windows)]
fn plugin_module_dir() -> Option<PathBuf> {
    let mut module = HMODULE(0);
    let addr = plugin_module_dir as *const () as *const std::ffi::c_void;
    let flags = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    let ok = unsafe { GetModuleHandleExW(flags, addr, &mut module).as_bool() };
    if !ok {
        return None;
    }

    let mut buffer = [0u16; 1024];
    let len = unsafe { GetModuleFileNameW(module, &mut buffer) } as usize;
    if len == 0 {
        return None;
    }

    let path = String::from_utf16_lossy(&buffer[..len]);
    PathBuf::from(path).parent().map(|dir| dir.to_path_buf())
}

#[cfg(not(windows))]
fn plugin_module_dir() -> Option<PathBuf> {
    None
}

pub fn load_sf2_preset_names(path: &Path) -> Result<Vec<String>, String> {
    let mut file = File::open(path).map_err(|err| format!("sf2 open failed: {err}"))?;
    let sf2 = SoundFont2::load(&mut file)
        .map_err(|err| format!("sf2 parse failed: {err:?}"))?
        .sort_presets();

    let mut names = vec![String::new(); 128];
    for preset in &sf2.presets {
        if preset.header.bank != 0 {
            continue;
        }
        let index = preset.header.preset as usize;
        if index >= names.len() {
            continue;
        }
        names[index] = clean_sf2_name(&preset.header.name);
    }

    for (idx, name) in names.iter_mut().enumerate() {
        if name.is_empty() {
            *name = format!("Preset {:03}", idx + 1);
        }
    }

    if let Some(_) = find_program_by_names(
        &sf2,
        &[
            "choir aah",
            "choir ahh",
            "choir aahs",
            "female choir",
        ],
    ) {
        if names.len() > 52 {
            names[52] = "Choir Ahhs".to_string();
        }
    }
    if let Some(_) = find_program_by_names(
        &sf2,
        &[
            "choir ooh",
            "choir oh",
            "choir oohs",
            "male choir",
        ],
    ) {
        if names.len() > 53 {
            names[53] = "Choir Oohs".to_string();
        }
    }

    Ok(names)
}

impl Sf2Bank {
    pub fn select_zone(&self, program: u8, note: u8, velocity: u8) -> Option<Arc<Sf2Zone>> {
        match program {
            52 => {
                if let Some(mapped) = self.choir_aah_program {
                    if let Some(zone) = self.select_zone_for_program(mapped, note, velocity) {
                        return Some(zone);
                    }
                }
            }
            53 => {
                if let Some(mapped) = self.choir_ooh_program {
                    if let Some(zone) = self.select_zone_for_program(mapped, note, velocity) {
                        return Some(zone);
                    }
                }
            }
            _ => {}
        }

        if let Some(zone) = self.select_zone_for_program(program, note, velocity) {
            return Some(zone);
        }

        let nearest = self.nearest_program_with_zones(program)?;
        self.select_zone_for_program(nearest, note, velocity)
    }

    fn select_zone_for_program(
        &self,
        program: u8,
        note: u8,
        velocity: u8,
    ) -> Option<Arc<Sf2Zone>> {
        let preset = self.presets.get(program as usize)?.as_ref()?;
        if preset.zones.is_empty() {
            return None;
        }

        for zone in &preset.zones {
            if note < zone.key_range.0 || note > zone.key_range.1 {
                continue;
            }
            if velocity < zone.vel_range.0 || velocity > zone.vel_range.1 {
                continue;
            }
            return Some(zone.clone());
        }

        preset.zones.first().cloned()
    }

    fn nearest_program_with_zones(&self, program: u8) -> Option<u8> {
        let mut best: Option<(u8, u8)> = None;
        for (idx, preset) in self.presets.iter().enumerate() {
            let preset = match preset {
                Some(preset) if !preset.zones.is_empty() => preset,
                _ => continue,
            };
            let _ = preset;
            let idx = idx as u8;
            let distance = if idx >= program {
                idx - program
            } else {
                program - idx
            };
            match best {
                Some((best_idx, best_dist)) => {
                    if distance < best_dist || (distance == best_dist && idx < best_idx) {
                        best = Some((idx, distance));
                    }
                }
                None => best = Some((idx, distance)),
            }
        }
        best.map(|(idx, _)| idx)
    }

    pub fn preset_env(&self, program: u8) -> Option<Sf2Env> {
        let preset = self.presets.get(program as usize)?.as_ref()?;
        preset.zones.first().map(|zone| zone.env)
    }
}

pub fn wavetable_sample(table: &[f32], phase: f32) -> f32 {
    if table.is_empty() {
        return 0.0;
    }
    let len = table.len() as f32;
    let pos = phase.fract() * len;
    let idx = pos.floor() as usize;
    let next = (idx + 1) % table.len();
    let frac = pos - idx as f32;
    table[idx] * (1.0 - frac) + table[next] * frac
}

fn build_preset_zones(
    sf2: &SoundFont2,
    preset: &soundfont::Preset,
    sample_data: &[i16],
) -> Result<Vec<Arc<Sf2Zone>>, String> {
    let preset_global = collect_global_gens(&preset.zones);
    let mut zones = Vec::new();

    for preset_zone in &preset.zones {
        let instrument_id = match preset_zone.instrument() {
            Some(id) => *id as usize,
            None => continue,
        };
        let instrument = sf2
            .instruments
            .get(instrument_id)
            .ok_or_else(|| "sf2 instrument index out of range".to_string())?;
        let instrument_global = collect_global_gens(&instrument.zones);
        let preset_gens = collect_zone_gens(preset_zone, &preset_global);

        for instrument_zone in &instrument.zones {
            let sample_id = match instrument_zone.sample() {
                Some(id) => *id as usize,
                None => continue,
            };
            let instrument_gens = collect_zone_gens(instrument_zone, &instrument_global);
            let mut combined = preset_gens.clone();
            combined.merge_from(&instrument_gens);

            let sample_header = sf2
                .sample_headers
                .get(sample_id)
                .ok_or_else(|| "sf2 sample header index out of range".to_string())?;

            let (table_left, table_right) =
                build_sample_tables(sf2, sample_data, sample_header, &combined)?;
            let root_key = combined
                .root_key
                .unwrap_or(sample_header.origpitch);
            let amp_gain = attenuation_to_gain(combined.attenuation_cdb);
            let pan = pan_to_unit(combined.pan);
            let sample_rate = sample_header.sample_rate as f32;

            let env = Sf2Env {
                attack: timecents_to_seconds(combined.attack_tc),
                decay: timecents_to_seconds(combined.decay_tc),
                sustain: sustain_cdb_to_level(combined.sustain_cdb),
                release: timecents_to_seconds(combined.release_tc),
            };

            zones.push(Arc::new(Sf2Zone {
                key_range: combined.key_range,
                vel_range: combined.vel_range,
                root_key,
                tune_cents: combined.tune_cents,
                pan,
                amp_gain,
                sample_rate,
                env,
                table_left: Arc::new(table_left),
                table_right: table_right.map(Arc::new),
            }));
        }
    }

    Ok(zones)
}

fn collect_global_gens(zones: &[Zone]) -> GenValues {
    let mut globals = GenValues::default();
    for zone in zones {
        if zone.instrument().is_none() && zone.sample().is_none() {
            globals.apply_gen(zone);
        }
    }
    globals
}

fn collect_zone_gens(zone: &Zone, base: &GenValues) -> GenValues {
    let mut gens = base.clone();
    let mut local = GenValues::default();
    local.apply_gen(zone);
    gens.merge_from(&local);
    gens
}

fn read_sample_chunk_i16(file: &mut File, chunk: SampleChunk) -> Result<Vec<i16>, String> {
    let mut buf = vec![0u8; chunk.len as usize];
    file.seek(SeekFrom::Start(chunk.offset))
        .map_err(|err| format!("sf2 seek failed: {err}"))?;
    file.read_exact(&mut buf)
        .map_err(|err| format!("sf2 read failed: {err}"))?;

    let mut samples = Vec::with_capacity(buf.len() / 2);
    for idx in 0..(buf.len() / 2) {
        let lo = buf[idx * 2];
        let hi = buf[idx * 2 + 1];
        samples.push(i16::from_le_bytes([lo, hi]));
    }
    Ok(samples)
}

fn build_wavetable(
    sample_data: &[i16],
    header: &SampleHeader,
    gens: &GenValues,
) -> Result<Vec<f32>, String> {
    let start = (header.start as i32 + gens.start_offset).max(0) as usize;
    let end = (header.end as i32 + gens.end_offset).max(start as i32 + 1) as usize;
    let mut loop_start = (header.loop_start as i32 + gens.loop_start_offset).max(start as i32);
    let mut loop_end = (header.loop_end as i32 + gens.loop_end_offset).max(loop_start + 1);

    if loop_end as usize > sample_data.len() {
        loop_end = sample_data.len() as i32;
    }
    if end > sample_data.len() {
        return Err("sf2 sample range out of bounds".to_string());
    }

    let (slice_start, slice_end) = if loop_end > loop_start {
        (loop_start as usize, loop_end as usize)
    } else {
        (start, end)
    };

    let slice = &sample_data[slice_start..slice_end];
    if slice.is_empty() {
        return Err("sf2 empty sample slice".to_string());
    }

    let mut table = vec![0.0f32; WAVETABLE_SIZE];
    let len = slice.len() as f32;
    for i in 0..WAVETABLE_SIZE {
        let pos = (i as f32 / WAVETABLE_SIZE as f32) * len;
        let idx = pos.floor() as usize;
        let next = (idx + 1).min(slice.len() - 1);
        let frac = pos - idx as f32;
        let s0 = slice[idx] as f32 / i16::MAX as f32;
        let s1 = slice[next] as f32 / i16::MAX as f32;
        table[i] = s0 * (1.0 - frac) + s1 * frac;
    }

    Ok(table)
}

fn build_sample_tables(
    sf2: &SoundFont2,
    sample_data: &[i16],
    header: &SampleHeader,
    gens: &GenValues,
) -> Result<(Vec<f32>, Option<Vec<f32>>), String> {
    let primary = build_wavetable(sample_data, header, gens)?;

    let sample_type = header.sample_type as u16;
    let is_right = (sample_type & 2) != 0;
    let is_left = (sample_type & 4) != 0;
    let is_linked = (sample_type & 8) != 0;

    if is_linked && (is_left || is_right) {
        let link_id = header.sample_link as usize;
        let link_header = sf2
            .sample_headers
            .get(link_id)
            .ok_or_else(|| "sf2 linked sample header index out of range".to_string())?;
        let linked = build_wavetable(sample_data, link_header, gens)?;
        if is_left {
            Ok((primary, Some(linked)))
        } else {
            Ok((linked, Some(primary)))
        }
    } else {
        Ok((primary, None))
    }
}

fn timecents_to_seconds(timecents: i16) -> f32 {
    let tc = timecents as f32;
    2.0_f32.powf(tc / 1200.0).max(0.0)
}

fn sustain_cdb_to_level(cdb: i16) -> f32 {
    let db = cdb as f32 / 10.0;
    10.0_f32.powf(-db / 20.0).clamp(0.0, 1.0)
}

fn attenuation_to_gain(cdb: i16) -> f32 {
    let db = cdb as f32 / 10.0;
    10.0_f32.powf(-db / 20.0).clamp(0.0, 1.0)
}

fn pan_to_unit(pan: i16) -> f32 {
    let value = pan as f32 / 1000.0;
    ((value + 1.0) * 0.5).clamp(0.0, 1.0)
}

fn clean_sf2_name(name: &str) -> String {
    name.trim_end_matches('\0').trim().to_string()
}

fn find_program_by_names(sf2: &SoundFont2, patterns: &[&str]) -> Option<u8> {
    for preset in &sf2.presets {
        if preset.header.bank != 0 {
            continue;
        }
        let name = clean_sf2_name(&preset.header.name).to_ascii_lowercase();
        if patterns.iter().any(|pattern| name.contains(pattern)) {
            return Some(preset.header.preset as u8);
        }
    }
    None
}
