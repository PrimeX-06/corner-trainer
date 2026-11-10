use crate::theme::ThemeMode;
use egui::{Key, Modifiers};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

/// Face identities remain stable when the six display colors are customized.
/// +X = red, -X = orange, +Y = white, -Y = yellow, +Z = green, -Z = blue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Face {
    Red,
    Orange,
    White,
    Yellow,
    Green,
    Blue,
}

impl Face {
    /// Opposite faces in their fixed left/right presentation order.
    pub const PAIRS: [[Self; 2]; 3] = [
        [Self::Yellow, Self::White],
        [Self::Green, Self::Blue],
        [Self::Orange, Self::Red],
    ];
    pub const ALL: [Self; 6] = [
        Self::PAIRS[0][0],
        Self::PAIRS[0][1],
        Self::PAIRS[1][0],
        Self::PAIRS[1][1],
        Self::PAIRS[2][0],
        Self::PAIRS[2][1],
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Red => "红色",
            Self::Orange => "橙色",
            Self::White => "白色",
            Self::Yellow => "黄色",
            Self::Green => "绿色",
            Self::Blue => "蓝色",
        }
    }
    pub fn axis(self) -> usize {
        self.index() / 2
    }
    pub fn sign(self) -> i32 {
        if self.index().is_multiple_of(2) {
            1
        } else {
            -1
        }
    }
    pub fn opposite(self) -> Self {
        match self {
            Self::Red => Self::Orange,
            Self::Orange => Self::Red,
            Self::White => Self::Yellow,
            Self::Yellow => Self::White,
            Self::Green => Self::Blue,
            Self::Blue => Self::Green,
        }
    }
    pub fn pair(self) -> [Self; 2] {
        match self {
            Self::Yellow | Self::White => Self::PAIRS[0],
            Self::Green | Self::Blue => Self::PAIRS[1],
            Self::Orange | Self::Red => Self::PAIRS[2],
        }
    }
}

pub const STANDARD: [[u8; 3]; 6] = [
    [226, 57, 67],
    [255, 139, 40],
    [246, 248, 250],
    [255, 213, 44],
    [26, 174, 112],
    [46, 109, 233],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub key: Key,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Binding {
    pub fn new(key: Key) -> Self {
        Self {
            key,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }
    pub fn matches(self, key: Key, m: Modifiers) -> bool {
        self.key == key
            && self.ctrl == (m.ctrl || m.command)
            && self.alt == m.alt
            && self.shift == m.shift
    }
    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_owned());
        }
        if self.alt {
            parts.push("Alt".to_owned());
        }
        if self.shift {
            parts.push("Shift".to_owned());
        }
        parts.push(match self.key {
            Key::Space => "Space".to_owned(),
            other => other.name().to_owned(),
        });
        parts.join(" + ")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub bindings: [Binding; 4],
    pub custom_palette: bool,
    pub colors: [[u8; 3]; 6],
    /// Allowed colors on the physical bottom (-Y), indexed by Face identity.
    pub bottom_colors: [bool; 6],
    pub review_mistakes: bool,
    pub theme: ThemeMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            bindings: [
                Binding::new(Key::A),
                Binding::new(Key::F),
                Binding::new(Key::Space),
                Binding::new(Key::T),
            ],
            custom_palette: false,
            colors: STANDARD,
            bottom_colors: [true; 6],
            review_mistakes: true,
            theme: ThemeMode::System,
        }
    }
}

impl Settings {
    pub fn palette(&self) -> &[[u8; 3]; 6] {
        if self.custom_palette {
            &self.colors
        } else {
            &STANDARD
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.bottom_colors.iter().any(|selected| *selected) {
            return Err("请至少选择一种底面颜色。".into());
        }
        for i in 0..4 {
            if self.bindings[i].key == Key::Escape {
                return Err("Esc 用于关闭设置，请选择其他按键。".into());
            }
            for j in i + 1..4 {
                if self.bindings[i] == self.bindings[j] {
                    return Err("快捷键不能重复，请为每个操作设置不同的按键。".into());
                }
            }
        }
        if self.custom_palette {
            for i in 0..6 {
                for j in i + 1..6 {
                    let distance: i32 = self.colors[i]
                        .iter()
                        .zip(self.colors[j])
                        .map(|(a, b)| (*a as i32 - b as i32).pow(2))
                        .sum();
                    if distance < 400 {
                        return Err("六种颜色需要能够区分，请调整过于接近的颜色。".into());
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub micros: u64,
    pub had_error: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    #[serde(default)]
    pub schema_version: u32,
    pub settings: Settings,
    pub records: Vec<Record>,
    pub wrong_clicks: u64,
    pub mistakes: [u32; 8],
    pub slow_review: [u32; 8],
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            schema_version: 3,
            settings: Settings::default(),
            records: Vec::new(),
            wrong_clicks: 0,
            mistakes: [0; 8],
            slow_review: [0; 8],
        }
    }
}

impl Saved {
    pub fn migrate(&mut self) {
        if !self.settings.bottom_colors.iter().any(|selected| *selected) {
            self.settings.bottom_colors = [true; 6];
        }
        if self.schema_version < 2 {
            let old = [
                Binding::new(Key::Num1),
                Binding::new(Key::Num2),
                Binding::new(Key::Space),
                Binding::new(Key::S),
            ];
            if self.settings.bindings == old {
                self.settings.bindings = Settings::default().bindings;
            }
            self.schema_version = 2;
        }
        if self.schema_version < 3 {
            self.schema_version = 3;
        }
    }
    /// Nearest-rank 80th percentile of completed solves, in microseconds.
    pub fn slow_threshold(&self) -> Option<u64> {
        if self.records.is_empty() {
            return None;
        }
        let mut times: Vec<_> = self.records.iter().map(|r| r.micros).collect();
        let rank = times.len() - times.len() / 5;
        let (_, threshold, _) = times.select_nth_unstable(rank - 1);
        Some(*threshold)
    }
    fn remember_slow(&mut self, corner: usize, micros: u64) {
        if let Some(threshold) = self.slow_threshold()
            && micros > threshold
        {
            // Doubling the threshold reaches the same maximum bonus as mistakes.
            let bonus = ((micros - threshold) as u128 * 36)
                .div_ceil(threshold.max(1) as u128)
                .min(36) as u32;
            self.slow_review[corner] = self.slow_review[corner].max(bonus);
        }
    }
    fn review_weights(&self) -> [u32; 8] {
        std::array::from_fn(|i| {
            if self.settings.review_mistakes {
                1 + (self.mistakes[i].min(12) * 3 + self.slow_review[i].min(36)).min(36)
            } else {
                1
            }
        })
    }
    pub fn accuracy(&self) -> Option<f64> {
        let attempts = self.records.len() as u64 + self.wrong_clicks;
        if attempts == 0 {
            None
        } else {
            Some(self.records.len() as f64 / attempts as f64)
        }
    }
    pub fn best(&self) -> Option<f64> {
        self.records
            .iter()
            .map(|r| r.micros)
            .min()
            .map(|v| v as f64 / 1_000_000.0)
    }
    pub fn accuracy_100(&self) -> Option<f64> {
        if self.records.len() < 100 {
            return None;
        }
        Some(
            self.records[self.records.len() - 100..]
                .iter()
                .filter(|r| !r.had_error)
                .count() as f64
                / 100.0,
        )
    }
    pub fn mo3(&self) -> Option<f64> {
        if self.records.len() < 3 {
            return None;
        }
        Some(
            self.records[self.records.len() - 3..]
                .iter()
                .map(|r| r.micros as f64)
                .sum::<f64>()
                / 3_000_000.0,
        )
    }
    pub fn mean(&self) -> Option<f64> {
        if self.records.is_empty() {
            None
        } else {
            Some(
                self.records.iter().map(|r| r.micros as f64).sum::<f64>()
                    / self.records.len() as f64
                    / 1_000_000.0,
            )
        }
    }
    /// WCA style: discard ceil(5% * n) at each end of the latest n solves.
    pub fn average(&self, n: usize) -> Option<f64> {
        if n < 3 || self.records.len() < n {
            return None;
        }
        let mut values: Vec<u64> = self.records[self.records.len() - n..]
            .iter()
            .map(|r| r.micros)
            .collect();
        values.sort_unstable();
        let trim = n.div_ceil(20);
        Some(
            values[trim..n - trim]
                .iter()
                .map(|v| *v as f64)
                .sum::<f64>()
                / (n - 2 * trim) as f64
                / 1_000_000.0,
        )
    }
    pub fn clear(&mut self, include_mistakes: bool) {
        self.records.clear();
        self.wrong_clicks = 0;
        if include_mistakes {
            self.mistakes = [0; 8];
            self.slow_review = [0; 8];
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub corner: usize,
    /// Stickers on the displayed positive X, Y, Z faces.
    pub faces: [Face; 3],
    pub hidden: usize,
    pub options: [Face; 2],
}

impl Question {
    pub fn bottom(&self) -> Face {
        self.faces[1].opposite()
    }
    pub fn correct(&self) -> Face {
        self.faces[self.hidden]
    }
    #[cfg(test)]
    pub fn generate(rng: &mut impl Rng, saved: &Saved) -> Self {
        Self::generate_next(rng, saved, None)
    }
    pub fn generate_next(rng: &mut impl Rng, saved: &Saved, previous: Option<&Self>) -> Self {
        let weights = saved.review_weights();
        let selected_count = saved
            .settings
            .bottom_colors
            .iter()
            .filter(|selected| **selected)
            .count();
        let candidates: Vec<_> = (0..8)
            .flat_map(|corner| {
                orientations(corner)
                    .into_iter()
                    .map(move |faces| (corner, faces))
            })
            .filter(|(corner, faces)| {
                (selected_count == 0 || saved.settings.bottom_colors[faces[1].opposite().index()])
                    && (!saved.settings.review_mistakes
                        || !previous.is_some_and(|q| q.corner == *corner && q.faces == *faces))
            })
            .collect();
        let mut ticket = rng.random_range(
            0..candidates
                .iter()
                .map(|(corner, _)| weights[*corner])
                .sum::<u32>(),
        );
        let (corner, faces) = *candidates
            .iter()
            .find(|(corner, _)| {
                if ticket < weights[*corner] {
                    true
                } else {
                    ticket -= weights[*corner];
                    false
                }
            })
            .unwrap();
        let hidden = if selected_count == 1 {
            [0, 2][rng.random_range(0..2)]
        } else {
            rng.random_range(0..3)
        };
        let options = faces[hidden].pair();
        Self {
            corner,
            faces,
            hidden,
            options,
        }
    }
}

/// A signed permutation is a physical cube rotation only when its determinant
/// is +1. Each of the eight corners therefore has three visible orientations.
pub fn orientations(corner: usize) -> Vec<[Face; 3]> {
    let source = [
        if corner & 1 == 0 {
            Face::Red
        } else {
            Face::Orange
        },
        if corner & 2 == 0 {
            Face::White
        } else {
            Face::Yellow
        },
        if corner & 4 == 0 {
            Face::Green
        } else {
            Face::Blue
        },
    ];
    const PERMUTATIONS: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    PERMUTATIONS
        .iter()
        .filter_map(|p| {
            let inversions = (0..3)
                .flat_map(|i| (i + 1..3).map(move |j| (i, j)))
                .filter(|(i, j)| p[*i] > p[*j])
                .count();
            let parity = if inversions % 2 == 0 { 1 } else { -1 };
            (parity * source.iter().map(|f| f.sign()).product::<i32>() == 1)
                .then(|| p.map(|i| source[i]))
        })
        .collect()
}

pub struct Game {
    pub saved: Saved,
    pub question: Option<Question>,
    pub started_at: Option<Instant>,
    pub wrong_marks: [bool; 2],
}

#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    Ignored,
    Wrong,
    Correct,
}

impl Game {
    pub fn new(saved: Saved) -> Self {
        Self {
            saved,
            question: None,
            started_at: None,
            wrong_marks: [false; 2],
        }
    }
    pub fn running(&self) -> bool {
        self.question.is_some()
    }
    pub fn elapsed(&self, now: Instant) -> Duration {
        self.started_at
            .map(|s| now.saturating_duration_since(s))
            .unwrap_or_default()
    }
    pub fn start(&mut self, rng: &mut impl Rng, now: Instant) {
        self.question = Some(Question::generate_next(
            rng,
            &self.saved,
            self.question.as_ref(),
        ));
        self.wrong_marks = [false; 2];
        self.started_at = Some(now);
    }
    pub fn stop(&mut self) {
        self.question = None;
        self.started_at = None;
        self.wrong_marks = [false; 2];
    }
    pub fn answer(&mut self, option: usize, rng: &mut impl Rng, now: Instant) -> Answer {
        let Some(q) = self.question.as_ref() else {
            return Answer::Ignored;
        };
        if option >= 2 {
            return Answer::Ignored;
        }
        if q.options[option] != q.correct() {
            self.wrong_marks[option] = true;
            self.saved.wrong_clicks = self.saved.wrong_clicks.saturating_add(1);
            self.saved.mistakes[q.corner] = self.saved.mistakes[q.corner].saturating_add(1);
            Answer::Wrong
        } else {
            let corner = q.corner;
            let micros = self.elapsed(now).as_micros().min(u64::MAX as u128) as u64;
            self.saved.remember_slow(corner, micros);
            self.saved.records.push(Record {
                micros,
                had_error: self.wrong_marks.iter().any(|b| *b),
            });
            self.start(rng, now);
            Answer::Correct
        }
    }
}

pub fn time_text(seconds: Option<f64>) -> String {
    seconds
        .map(|s| format!("{s:.3}"))
        .unwrap_or_else(|| "—".into())
}

pub struct Store {
    pub path: PathBuf,
    pub warning: Option<String>,
}

impl Store {
    pub fn load() -> (Self, Saved) {
        #[cfg(feature = "qa")]
        if std::env::args().any(|arg| arg == "--qa") {
            return (
                Self {
                    path: PathBuf::from("qa/data.json"),
                    warning: None,
                },
                Saved::default(),
            );
        }
        let dir = directories::ProjectDirs::from("", "", "CornerTrainer")
            .map(|d| d.data_local_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".corner-trainer"));
        let mut store = Self {
            path: dir.join("data.json"),
            warning: None,
        };
        let saved = match std::fs::read(&store.path) {
            Ok(bytes) => match serde_json::from_slice::<Saved>(&bytes) {
                Ok(mut s) => {
                    s.migrate();
                    if s.settings.validate().is_err() {
                        s.settings = Settings::default();
                        store.warning = Some("已恢复有效的默认设置。".into());
                    }
                    s
                }
                Err(_) => {
                    let stamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let backup = dir.join(format!("data-backup-{stamp}.json"));
                    match std::fs::copy(&store.path, backup) {
                        Ok(_) => store.warning = Some("历史数据无法读取，原文件已备份。".into()),
                        Err(e) => store.warning = Some(format!("历史数据无法读取：{e}")),
                    }
                    Saved::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Saved::default(),
            Err(e) => {
                store.warning = Some(format!("读取成绩失败：{e}"));
                Saved::default()
            }
        };
        (store, saved)
    }
    pub fn save(&mut self, saved: &Saved) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            std::fs::create_dir_all(self.path.parent().unwrap())?;
            let temp = self.path.with_extension("tmp");
            let bytes = serde_json::to_vec_pretty(saved)?;
            use std::io::Write;
            let mut file = std::fs::File::create(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(temp, &self.path)?;
            Ok(())
        })();
        if let Err(e) = result {
            self.warning = Some(format!("保存成绩失败：{e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn all_24_orientations_preserve_cube_handedness() {
        let mut seen = std::collections::HashSet::new();
        for corner in 0..8 {
            let views = orientations(corner);
            assert_eq!(views.len(), 3);
            for view in views {
                let ids = view.map(Face::index);
                assert!(seen.insert(ids));
                let inversions = (0..3)
                    .flat_map(|i| (i + 1..3).map(move |j| (i, j)))
                    .filter(|(i, j)| view[*i].axis() > view[*j].axis())
                    .count();
                assert_eq!(
                    (if inversions % 2 == 0 { 1 } else { -1 })
                        * view.iter().map(|f| f.sign()).product::<i32>(),
                    1
                );
            }
        }
        assert_eq!(seen.len(), 24);
    }

    #[test]
    fn each_visible_pair_and_slot_has_exactly_one_physical_answer() {
        let all: Vec<_> = (0..8).flat_map(orientations).collect();
        for faces in &all {
            for hidden in 0..3 {
                let valid: Vec<_> = all
                    .iter()
                    .filter(|other| {
                        (0..3)
                            .filter(|i| *i != hidden)
                            .all(|i| faces[i] == other[i])
                    })
                    .collect();
                assert_eq!(valid.len(), 1);
                assert_eq!(valid[0][hidden], faces[hidden]);
            }
        }
    }

    #[test]
    fn wrong_answer_keeps_question_and_clock_correct_answer_records_total() {
        let mut rng = StdRng::seed_from_u64(8);
        let mut game = Game::new(Saved::default());
        let start = Instant::now();
        game.start(&mut rng, start);
        let q = game.question.clone().unwrap();
        let right = q.options.iter().position(|f| *f == q.correct()).unwrap();
        assert_eq!(
            game.answer(1 - right, &mut rng, start + Duration::from_millis(300)),
            Answer::Wrong
        );
        assert_eq!(game.question.as_ref(), Some(&q));
        assert_eq!(game.started_at, Some(start));
        assert!(game.saved.records.is_empty());
        assert_eq!(game.saved.mistakes[q.corner], 1);
        assert_eq!(
            game.answer(right, &mut rng, start + Duration::from_micros(1_234_567)),
            Answer::Correct
        );
        assert_eq!(game.saved.records[0].micros, 1_234_567);
        assert!(game.saved.records[0].had_error);
        assert_eq!(game.wrong_marks, [false; 2]);
        assert_eq!(game.saved.accuracy(), Some(0.5));
        assert_eq!(
            game.elapsed(start + Duration::from_micros(1_234_567)),
            Duration::ZERO
        );
        game.stop();
        assert_eq!(game.saved.records.len(), 1);
        assert_eq!(game.answer(0, &mut rng, start), Answer::Ignored);
    }

    #[test]
    fn review_increases_corner_probability_and_off_is_uniform() {
        let mut saved = Saved::default();
        saved.mistakes[3] = 5;
        let mut rng = StdRng::seed_from_u64(48);
        let mut counts = [0u32; 8];
        for _ in 0..16_000 {
            counts[Question::generate(&mut rng, &saved).corner] += 1;
        }
        assert!(counts[3] > 10_000);
        saved.settings.review_mistakes = false;
        counts = [0; 8];
        for _ in 0..16_000 {
            let q = Question::generate(&mut rng, &saved);
            counts[q.corner] += 1;
            assert_eq!(q.options[0].opposite(), q.options[1]);
            assert!(q.options.contains(&q.correct()));
        }
        assert!(counts.iter().all(|n| (1700..2300).contains(n)));
    }

    #[test]
    fn averages_trim_latest_window_and_resets_preserve_selected_data() {
        let mut saved = Saved {
            records: (1..=100)
                .map(|i| Record {
                    micros: i * 1_000_000,
                    had_error: false,
                })
                .collect(),
            ..Default::default()
        };
        saved.mistakes[1] = 8;
        saved.slow_review[2] = 18;
        assert_eq!(saved.average(12), Some(94.5));
        assert_eq!(saved.average(100), Some(50.5));
        assert_eq!(saved.average(101), None);
        assert_eq!(saved.best(), Some(1.0));
        saved.clear(false);
        assert_eq!(saved.mistakes[1], 8);
        assert_eq!(saved.slow_review[2], 18);
        assert!(saved.records.is_empty());
        saved.clear(true);
        assert_eq!(saved.mistakes, [0; 8]);
        assert_eq!(saved.slow_review, [0; 8]);
    }

    #[test]
    fn settings_roundtrip_and_reject_conflicting_keys_and_colors() {
        let mut settings = Settings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings.bindings, loaded.bindings);
        assert_eq!(loaded.bottom_colors, [true; 6]);
        settings.bottom_colors = [false; 6];
        assert!(settings.validate().is_err());
        settings.bottom_colors[Face::Green.index()] = true;
        let loaded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(settings.bottom_colors, loaded.bottom_colors);
        settings.bindings[1] = settings.bindings[0];
        assert!(settings.validate().is_err());
        settings = Settings::default();
        settings.custom_palette = true;
        settings.colors[1] = settings.colors[0];
        assert!(settings.validate().is_err());
    }

    #[test]
    fn every_bottom_color_subset_preserves_physical_faces_and_continuous_play() {
        let mut rng = StdRng::seed_from_u64(104);
        for mask in 1u8..64 {
            for review in [false, true] {
                let mut saved = Saved::default();
                saved.settings.bottom_colors = std::array::from_fn(|i| mask & (1 << i) != 0);
                saved.settings.review_mistakes = review;
                saved.mistakes = [12, 2, 8, 0, 1, 6, 4, 3];
                saved.slow_review = [36, 0, 12, 36, 6, 18, 0, 9];
                let mut game = Game::new(saved);
                let now = Instant::now();
                game.start(&mut rng, now);
                let mut hidden_faces = [false; 3];
                for round in 0..128 {
                    let q = game.question.clone().unwrap();
                    hidden_faces[q.hidden] = true;
                    assert!(game.saved.settings.bottom_colors[q.bottom().index()]);
                    assert_eq!(q.faces[1], q.bottom().opposite());
                    if mask.count_ones() == 1 {
                        assert_ne!(q.hidden, 1);
                        assert!(!game.saved.settings.bottom_colors[q.faces[1].index()]);
                    }
                    assert!(orientations(q.corner).contains(&q.faces));
                    assert!(matches!(
                        q.options,
                        [Face::Yellow, Face::White]
                            | [Face::Green, Face::Blue]
                            | [Face::Orange, Face::Red]
                    ));
                    let right = q
                        .options
                        .iter()
                        .position(|face| *face == q.correct())
                        .unwrap();
                    assert_eq!(game.answer(1 - right, &mut rng, now), Answer::Wrong);
                    assert_eq!(game.question.as_ref(), Some(&q));
                    assert_eq!(
                        game.answer(right, &mut rng, now + Duration::from_millis(round)),
                        Answer::Correct
                    );
                    if review {
                        assert_ne!(game.question.as_ref().unwrap().faces, q.faces);
                    }
                }
                assert_eq!(hidden_faces, [true, mask.count_ones() > 1, true]);
            }
        }
    }

    #[test]
    fn selected_bottom_colors_are_uniform_when_review_is_disabled() {
        let mut saved = Saved::default();
        saved.settings.review_mistakes = false;
        saved.settings.bottom_colors = [false; 6];
        for face in [Face::White, Face::Yellow, Face::Red] {
            saved.settings.bottom_colors[face.index()] = true;
        }
        let mut rng = StdRng::seed_from_u64(105);
        let mut counts = [0; 6];
        for _ in 0..6000 {
            counts[Question::generate(&mut rng, &saved).bottom().index()] += 1;
        }
        for face in Face::ALL {
            if saved.settings.bottom_colors[face.index()] {
                assert!((1800..2200).contains(&counts[face.index()]));
            } else {
                assert_eq!(counts[face.index()], 0);
            }
        }
    }

    #[test]
    fn missing_or_empty_bottom_selection_recovers_without_losing_other_settings() {
        let mut saved: Saved = serde_json::from_value(serde_json::json!({
            "schema_version": 3,
            "settings": {"theme": "Dark"},
            "records": [{"micros": 1234, "had_error": false}],
            "mistakes": [3, 0, 0, 0, 0, 0, 0, 0]
        }))
        .unwrap();
        assert_eq!(saved.settings.bottom_colors, [true; 6]);
        saved.settings.bottom_colors = [false; 6];
        saved.settings.bindings[0] = Binding::new(Key::Q);
        saved.migrate();
        assert_eq!(saved.settings.bottom_colors, [true; 6]);
        assert_eq!(saved.settings.theme, ThemeMode::Dark);
        assert_eq!(saved.settings.bindings[0].key, Key::Q);
        assert_eq!(saved.records[0].micros, 1234);
        assert_eq!(saved.mistakes[0], 3);
    }

    #[test]
    fn mo3_uses_all_three_latest_times_and_100_accuracy_uses_latest_completed_rounds() {
        let mut saved = Saved::default();
        assert_eq!(saved.mo3(), None);
        assert_eq!(saved.accuracy_100(), None);
        saved.records = (0..105)
            .map(|i| Record {
                micros: (i + 1) * 1_000_000,
                had_error: i % 5 == 0,
            })
            .collect();
        assert_eq!(saved.mo3(), Some(104.0));
        assert_eq!(saved.accuracy_100(), Some(0.8));
        saved.records.pop();
        saved.records.push(Record {
            micros: 500_000,
            had_error: true,
        });
        assert_eq!(saved.accuracy_100(), Some(0.79));
    }

    #[test]
    fn review_preserves_three_colors_and_changes_repeated_corner_orientation() {
        let mut saved = Saved::default();
        saved.mistakes[0] = 12;
        let mut rng = StdRng::seed_from_u64(77);
        let mut previous = Question::generate(&mut rng, &saved);
        let mut repeated = 0;
        for _ in 0..1000 {
            let next = Question::generate_next(&mut rng, &saved, Some(&previous));
            if previous.corner == next.corner {
                repeated += 1;
                assert_ne!(previous.faces, next.faces);
                let mut a = previous.faces.map(Face::index);
                let mut b = next.faces.map(Face::index);
                a.sort();
                b.sort();
                assert_eq!(a, b);
                assert!(orientations(next.corner).contains(&next.faces));
            }
            previous = next;
        }
        assert!(repeated > 500);
    }

    #[test]
    fn migration_updates_old_defaults_and_preserves_scores_mistakes_and_custom_keys() {
        let old = [
            Binding::new(Key::Num1),
            Binding::new(Key::Num2),
            Binding::new(Key::Space),
            Binding::new(Key::S),
        ];
        let mut saved = Saved {
            schema_version: 0,
            ..Default::default()
        };
        saved.settings.bindings = old;
        saved.records.push(Record {
            micros: 123_000,
            had_error: true,
        });
        saved.mistakes[4] = 9;
        saved.wrong_clicks = 7;
        saved.migrate();
        assert_eq!(saved.settings.bindings, Settings::default().bindings);
        assert_eq!(saved.records[0].micros, 123_000);
        assert_eq!(saved.mistakes[4], 9);
        assert_eq!(saved.wrong_clicks, 7);
        saved.schema_version = 0;
        saved.settings.bindings[0] = Binding::new(Key::Q);
        saved.migrate();
        assert_eq!(saved.settings.bindings[0].key, Key::Q);
        let json = serde_json::json!({"settings":{"bindings":old},"records":[{"micros":4321,"had_error":false}],"mistakes":[0,1,0,0,0,0,0,0]});
        let mut loaded: Saved = serde_json::from_value(json).unwrap();
        assert_eq!(loaded.schema_version, 0);
        loaded.migrate();
        assert_eq!(loaded.schema_version, 3);
        assert_eq!(loaded.slow_review, [0; 8]);
        assert_eq!(loaded.records[0].micros, 4321);
        assert_eq!(loaded.settings.bindings[3].key, Key::T);
    }
    #[test]
    fn slow_review_uses_historical_percentile_and_grows_with_time_up_to_a_shared_cap() {
        let mut saved = Saved {
            records: (1..=10)
                .map(|seconds| Record {
                    micros: seconds * 1_000_000,
                    had_error: false,
                })
                .collect(),
            ..Default::default()
        };
        assert_eq!(saved.slow_threshold(), Some(8_000_000));
        saved.remember_slow(2, 8_000_000);
        assert_eq!(saved.slow_review[2], 0);
        saved.remember_slow(2, 10_000_000);
        assert_eq!(saved.slow_review[2], 9);
        saved.remember_slow(2, 12_000_000);
        assert_eq!(saved.slow_review[2], 18);
        saved.remember_slow(2, 16_000_000);
        assert_eq!(saved.slow_review[2], 36);
        saved.remember_slow(2, u64::MAX);
        saved.mistakes[2] = u32::MAX;
        assert_eq!(saved.review_weights()[2], 37);
        saved.settings.review_mistakes = false;
        assert_eq!(saved.review_weights(), [1; 8]);
        let reloaded: Saved =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert_eq!(reloaded.slow_review, saved.slow_review);
        saved.records.clear();
        assert_eq!(saved.slow_threshold(), None);
        saved.remember_slow(1, 5_000_000);
        assert_eq!(saved.slow_review[1], 0);
    }
    #[test]
    fn completed_slow_round_is_reviewed_and_stopped_round_is_discarded() {
        let mut saved = Saved::default();
        saved.records.push(Record {
            micros: 1_000_000,
            had_error: false,
        });
        let mut rng = StdRng::seed_from_u64(78);
        let mut game = Game::new(saved);
        let start = Instant::now();
        game.start(&mut rng, start);
        let question = game.question.clone().unwrap();
        let right = question
            .options
            .iter()
            .position(|f| *f == question.correct())
            .unwrap();
        assert_eq!(
            game.answer(right, &mut rng, start + Duration::from_millis(1500)),
            Answer::Correct
        );
        assert_eq!(game.saved.slow_review[question.corner], 18);
        assert_eq!(game.saved.records.len(), 2);
        let scores = game.saved.slow_review;
        game.stop();
        assert_eq!(game.saved.slow_review, scores);
        assert_eq!(game.saved.records.len(), 2);
    }
    #[test]
    fn slower_corner_weights_increase_actual_frequency_and_disabled_review_is_uniform() {
        let mut saved = Saved::default();
        saved.slow_review[0] = 9;
        saved.slow_review[1] = 18;
        saved.slow_review[2] = 36;
        let mut rng = StdRng::seed_from_u64(79);
        let mut counts = [0u32; 8];
        for _ in 0..16_000 {
            counts[Question::generate(&mut rng, &saved).corner] += 1;
        }
        assert!(counts[2] > counts[1] && counts[1] > counts[0]);
        assert!(counts[0] > counts[3] * 5);
        saved.settings.review_mistakes = false;
        counts = [0; 8];
        for _ in 0..16_000 {
            counts[Question::generate(&mut rng, &saved).corner] += 1;
        }
        assert!(counts.iter().all(|count| (1700..2300).contains(count)));
    }
}
