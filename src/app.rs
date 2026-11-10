use crate::{
    cube::{CubeRenderer, IdleCube},
    digits,
    model::{Answer, Binding, Face, Game, Settings, Store, time_text},
    native::{self, NativeMaterial},
    theme::{Theme, ThemeMode},
    ui::*,
};
use egui::{
    Align2, Color32, Event, FontData, FontDefinitions, FontFamily, Id, Key, Pos2, Rect, Sense,
    Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};
use rand::rngs::ThreadRng;
use std::time::{Duration, Instant};

const SETTINGS_ANIMATION_TIME: f32 = 0.2;
const SETTINGS_BLUR_SIGMA: f32 = 5.0;

pub struct Trainer {
    game: Game,
    store: Store,
    rng: ThreadRng,
    renderer: CubeRenderer,
    idle_cube: IdleCube,
    bottom_menu_rect: Option<Rect>,
    bottom_menu_dismissed: bool,
    settings_open: bool,
    settings_progress: f32,
    draft: Settings,
    capture_key: Option<usize>,
    settings_error: Option<String>,
    reset_choice: Option<bool>,
    scroll_to_latest: bool,
    theme: Theme,
    system_dark: bool,
    theme_checked: Instant,
    native: NativeMaterial,
    acrylic: Option<egui::TextureHandle>,
    snapshot_pending: Option<Instant>,
    #[cfg(feature = "qa")]
    qa_frame: Option<u32>,
    #[cfg(feature = "qa")]
    qa_captures: Vec<std::thread::JoinHandle<()>>,
}

impl Trainer {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "noto-cjk".into(),
            FontData::from_static(include_bytes!("../assets/NotoSansCJKsc-Regular.otf")).into(),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .push("noto-cjk".into());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push("noto-cjk".into());
        if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/segoeui.ttf") {
            fonts
                .font_data
                .insert("segoe-ui".into(), FontData::from_owned(bytes).into());
            fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, "segoe-ui".into());
        }
        cc.egui_ctx.set_fonts(fonts);
        let (store, saved) = Store::load();
        let system_dark = native::system_dark();
        let theme = Theme::resolve(saved.settings.theme, system_dark);
        theme.apply(&cc.egui_ctx);
        let mut material = NativeMaterial::new(cc);
        material.apply(theme.dark);
        Self {
            draft: saved.settings.clone(),
            game: Game::new(saved),
            store,
            rng: rand::rng(),
            renderer: CubeRenderer::default(),
            idle_cube: IdleCube::default(),
            bottom_menu_rect: None,
            bottom_menu_dismissed: false,
            settings_open: false,
            settings_progress: 0.0,
            capture_key: None,
            settings_error: None,
            reset_choice: None,
            scroll_to_latest: false,
            theme,
            system_dark,
            theme_checked: Instant::now(),
            native: material,
            acrylic: None,
            snapshot_pending: None,
            #[cfg(feature = "qa")]
            qa_captures: Vec::new(),
            #[cfg(feature = "qa")]
            qa_frame: std::env::args().any(|a| a == "--qa").then_some(
                if std::env::args().any(|a| a == "--qa-settings-animation") {
                    1100
                } else if std::env::args().any(|a| a == "--qa-bottom-colors") {
                    900
                } else if std::env::args().any(|a| a == "--qa-settings") {
                    565
                } else {
                    0
                },
            ),
        }
    }
    fn refresh_theme(&mut self, ctx: &egui::Context) {
        if self.theme_checked.elapsed() >= Duration::from_secs(1) {
            self.system_dark = native::system_dark();
            self.theme_checked = Instant::now();
        }
        let mode = if self.settings_open {
            self.draft.theme
        } else {
            self.game.saved.settings.theme
        };
        let next = Theme::resolve(mode, self.system_dark);
        if next != self.theme {
            self.theme = next;
            next.apply(ctx);
            if self.settings_open {
                self.capture_background(ctx);
            }
        }
        self.native.apply(self.theme.dark);
    }
    fn settings_blocking(&self) -> bool {
        self.settings_open || self.settings_progress > 0.0
    }
    fn open_settings(&mut self, ctx: &egui::Context) {
        if self.settings_progress == 0.0 {
            ctx.animate_bool_with_time(Id::new("settings-transition"), false, 0.0);
        }
        let reuse_background = self.settings_progress > 0.0 && self.acrylic.is_some();
        self.bottom_menu_rect = None;
        self.bottom_menu_dismissed = false;
        self.draft = self.game.saved.settings.clone();
        self.settings_open = true;
        self.capture_key = None;
        self.settings_error = None;
        self.reset_choice = None;
        if !reuse_background {
            self.capture_background(ctx);
        }
    }
    fn capture_background(&mut self, ctx: &egui::Context) {
        self.acrylic = None;
        self.snapshot_pending = Some(Instant::now());
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
            "acrylic-background".to_owned(),
        )));
    }
    fn close_settings(&mut self) {
        self.settings_open = false;
        self.capture_key = None;
        if self.settings_progress == 0.0 {
            self.acrylic = None;
        }
        self.snapshot_pending = None;
    }
    fn toggle(&mut self) {
        self.bottom_menu_rect = None;
        self.bottom_menu_dismissed = false;
        if self.game.running() {
            self.game.stop();
        } else {
            self.game.start(&mut self.rng, Instant::now());
        }
    }
    fn answer(&mut self, index: usize) {
        match self.game.answer(index, &mut self.rng, Instant::now()) {
            Answer::Correct => {
                self.store.save(&self.game.saved);
                self.scroll_to_latest = true;
            }
            Answer::Wrong => self.store.save(&self.game.saved),
            Answer::Ignored => {}
        }
    }
    fn keyboard(&mut self, ctx: &egui::Context) {
        for event in ctx.input(|i| i.events.clone()) {
            let Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if self.settings_blocking() {
                if key == Key::Escape {
                    ctx.input_mut(|i| {
                        i.consume_key(modifiers, key);
                    });
                    if !repeat && self.capture_key.take().is_none() {
                        self.close_settings();
                    }
                } else if let Some(index) = self.capture_key {
                    ctx.input_mut(|i| {
                        i.consume_key(modifiers, key);
                    });
                    if repeat {
                        continue;
                    }
                    self.capture_key = None;
                    let binding = Binding {
                        key,
                        ctrl: modifiers.ctrl || modifiers.command,
                        alt: modifiers.alt,
                        shift: modifiers.shift,
                    };
                    if self
                        .draft
                        .bindings
                        .iter()
                        .enumerate()
                        .any(|(i, b)| i != index && *b == binding)
                    {
                        self.settings_error = Some("这个快捷键已被占用，请选择其他按键。".into());
                    } else {
                        self.draft.bindings[index] = binding;
                        self.settings_error = None;
                    }
                } else if self.game.saved.settings.bindings[3].matches(key, modifiers) {
                    ctx.input_mut(|i| {
                        i.consume_key(modifiers, key);
                    });
                    if !repeat {
                        if self.settings_open {
                            self.close_settings();
                        } else {
                            self.open_settings(ctx);
                        }
                    }
                }
                return;
            }
            let bindings = self.game.saved.settings.bindings;
            let action = bindings.iter().position(|b| b.matches(key, modifiers));
            if action.is_some() {
                ctx.input_mut(|i|i.events.retain(|e|!matches!(e,Event::Key{key,pressed:true,modifiers,..} if bindings.iter().any(|b|b.matches(*key,*modifiers)))));
            }
            if repeat {
                continue;
            }
            match action {
                Some(0) => self.answer(0),
                Some(1) => self.answer(1),
                Some(2) => self.toggle(),
                Some(3) => self.open_settings(ctx),
                _ => continue,
            }
            break;
        }
    }
    fn receive_snapshot(&mut self, ctx: &egui::Context) {
        for event in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot {
                image, user_data, ..
            } = event
                && user_data
                    .data
                    .as_ref()
                    .and_then(|d| d.downcast_ref::<String>())
                    .is_some_and(|s| s == "acrylic-background")
                && self.settings_open
            {
                let pixels = blurred_background(&image);
                self.acrylic =
                    Some(ctx.load_texture("acrylic-scene", pixels, egui::TextureOptions::LINEAR));
                self.snapshot_pending = None;
            }
        }
        if self
            .snapshot_pending
            .is_some_and(|i| i.elapsed() > Duration::from_millis(500))
        {
            self.snapshot_pending = None;
        }
    }
    fn header(&self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter();
        let t = self.theme;
        p.rect_filled(rect, 0, t.tint(t.surface, 72));
        divider(p, rect.left_bottom(), rect.right_bottom());
        logo(
            p,
            Rect::from_min_size(Pos2::new(28.0, 23.0), Vec2::splat(42.0)),
        );
        text(
            p,
            Pos2::new(84.0, 34.0),
            Align2::LEFT_CENTER,
            "三色定则",
            23.0,
            t.text,
        );
        text(
            p,
            Pos2::new(85.0, 58.0),
            Align2::LEFT_CENTER,
            "CORNER COLOR TRAINER",
            10.0,
            t.secondary,
        );
        let badge = Rect::from_min_size(
            Pos2::new(rect.right() - 184.0, 29.0),
            Vec2::new(156.0, 31.0),
        );
        p.rect_filled(badge, 16, t.tint(t.surface, 135));
        for (i, c) in [
            Color32::from_rgb(226, 57, 67),
            Color32::from_rgb(26, 174, 112),
            Color32::from_rgb(46, 109, 233),
        ]
        .iter()
        .enumerate()
        {
            p.circle_filled(badge.min + Vec2::new(16.0 + i as f32 * 8.0, 15.5), 3.0, *c);
        }
        text(
            p,
            badge.min + Vec2::new(44.0, 15.5),
            Align2::LEFT_CENTER,
            if self.game.saved.settings.custom_palette {
                "自定义配色"
            } else {
                "国际标准配色"
            },
            12.0,
            t.secondary,
        );
    }
    fn stats(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter().clone();
        let t = self.theme;
        let x = rect.left() + 22.0;
        let surface = Rect::from_min_max(rect.min, rect.max - Vec2::new(0.0, 68.0));
        card(&p, surface, 10);
        text(
            &p,
            Pos2::new(x, rect.top() + 26.0),
            Align2::LEFT_CENTER,
            "TRAINING OVERVIEW",
            10.0,
            t.secondary,
        );
        chart_icon(&p, Pos2::new(x + 7.0, rect.top() + 56.0), t.accent);
        text(
            &p,
            Pos2::new(x + 25.0, rect.top() + 56.0),
            Align2::LEFT_CENTER,
            "训练统计",
            17.0,
            t.text,
        );
        text(
            &p,
            Pos2::new(x, rect.top() + 100.0),
            Align2::LEFT_CENTER,
            "训练次数",
            13.0,
            t.secondary,
        );
        mono(
            &p,
            Pos2::new(x, rect.top() + 139.0),
            Align2::LEFT_CENTER,
            self.game.saved.records.len(),
            38.0,
            t.text,
        );
        text(
            &p,
            Pos2::new(rect.right() - 22.0, rect.top() + 148.0),
            Align2::RIGHT_CENTER,
            "次",
            13.0,
            t.secondary,
        );
        divider(
            &p,
            Pos2::new(x, rect.top() + 173.0),
            Pos2::new(rect.right() - 22.0, rect.top() + 173.0),
        );
        let ay = rect.top() + 204.0;
        text(
            &p,
            Pos2::new(x, ay),
            Align2::LEFT_CENTER,
            "正确率",
            13.0,
            t.secondary,
        );
        mono(
            &p,
            Pos2::new(rect.right() - 22.0, ay),
            Align2::RIGHT_CENTER,
            percent(self.game.saved.accuracy()),
            20.0,
            t.text,
        );
        let bar = Rect::from_min_size(Pos2::new(x, ay + 21.0), Vec2::new(rect.width() - 44.0, 4.0));
        p.rect_filled(bar, 2, t.subtle);
        if let Some(a) = self.game.saved.accuracy() {
            p.rect_filled(
                Rect::from_min_size(bar.min, Vec2::new(bar.width() * a as f32, 4.0)),
                2,
                t.accent,
            );
        }
        ui.interact(
            Rect::from_min_max(Pos2::new(x, ay - 13.0), bar.max),
            Id::new("accuracy-tip"),
            Sense::hover(),
        )
        .on_hover_text("答对次数 ÷ 总作答次数；重复答错也计入总作答次数。");
        let recent_y = ay + 54.0;
        text(
            &p,
            Pos2::new(x, recent_y),
            Align2::LEFT_CENTER,
            "百次正确率",
            13.0,
            t.secondary,
        );
        mono(
            &p,
            Pos2::new(rect.right() - 22.0, recent_y),
            Align2::RIGHT_CENTER,
            percent(self.game.saved.accuracy_100()),
            18.0,
            t.text,
        );
        ui.interact(
            Rect::from_min_size(
                Pos2::new(x, recent_y - 14.0),
                Vec2::new(rect.width() - 44.0, 28.0),
            ),
            Id::new("accuracy-100-tip"),
            Sense::hover(),
        )
        .on_hover_text("最近 100 道已完成题中，首次作答即答对的比例。不足 100 题时显示 —。");
        let values = [
            ("最快用时", self.game.saved.best()),
            ("平均用时", self.game.saved.mean()),
            ("mo3", self.game.saved.mo3()),
            ("ao5", self.game.saved.average(5)),
            ("ao12", self.game.saved.average(12)),
            ("ao100", self.game.saved.average(100)),
        ];
        for (i, (name, value)) in values.iter().enumerate() {
            let y = recent_y + 44.0 + i as f32 * 32.0;
            text(
                &p,
                Pos2::new(x, y),
                Align2::LEFT_CENTER,
                name,
                13.0,
                t.secondary,
            );
            mono(
                &p,
                Pos2::new(rect.right() - 37.0, y),
                Align2::RIGHT_CENTER,
                time_text(*value),
                16.0,
                if i == 0 && value.is_some() {
                    t.accent
                } else {
                    t.text
                },
            );
            if value.is_some() {
                text(
                    &p,
                    Pos2::new(rect.right() - 22.0, y),
                    Align2::RIGHT_CENTER,
                    "s",
                    11.0,
                    t.secondary,
                );
            }
            if i >= 2 {
                ui.interact(
                    Rect::from_min_size(
                        Pos2::new(x, y - 14.0),
                        Vec2::new(rect.width() - 44.0, 28.0),
                    ),
                    Id::new(("average-tip", i)),
                    Sense::hover(),
                )
                .on_hover_text(if i == 2 {
                    "最近三道已完成题的算术平均；保留全部三题用时。"
                } else {
                    "取最近 N 题，去掉最快和最慢各 5%（向上取整）后求平均。"
                });
            }
        }
        let by = surface.bottom() - 31.0;
        divider(
            &p,
            Pos2::new(x, by - 26.0),
            Pos2::new(rect.right() - 22.0, by - 26.0),
        );
        let review = self.game.saved.settings.review_mistakes;
        p.circle_filled(
            Pos2::new(x + 4.0, by),
            3.0,
            if review { t.success } else { t.secondary },
        );
        text(
            &p,
            Pos2::new(x + 15.0, by),
            Align2::LEFT_CENTER,
            "错题回顾",
            12.0,
            t.secondary,
        );
        text(
            &p,
            Pos2::new(rect.right() - 22.0, by),
            Align2::RIGHT_CENTER,
            if review { "已开启" } else { "已关闭" },
            12.0,
            if review { t.success } else { t.secondary },
        );
        let settings = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - 52.0), rect.max);
        if button(ui, "settings", settings, "设置", false).clicked() && !self.settings_blocking()
        {
            self.open_settings(ui.ctx());
        }
        gear(
            &p,
            Pos2::new(settings.left() + 23.0, settings.center().y),
            t.secondary,
        );
        keycap(
            &p,
            Pos2::new(settings.right() - 33.0, settings.center().y),
            &self.game.saved.settings.bindings[3].label(),
            false,
        );
    }
    fn center(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter().clone();
        let t = self.theme;
        let cx = rect.center().x;
        let running = self.game.running();
        let timer = Rect::from_center_size(
            Pos2::new(cx, rect.top() + 80.0),
            Vec2::new((rect.width() - 60.0).min(400.0), 72.0),
        );
        digits::paint(
            &p,
            timer,
            &format!("{:.3}", self.game.elapsed(Instant::now()).as_secs_f64()),
            t.text,
        );
        if running {
            let width = ((rect.width() - 48.0) / 2.0).min(177.0);
            let gap = 20.0;
            let q = self.game.question.clone().unwrap();
            for i in 0..2 {
                let r = Rect::from_min_size(
                    Pos2::new(
                        cx - width - gap * 0.5 + i as f32 * (width + gap),
                        rect.top() + 148.0,
                    ),
                    Vec2::new(width, 59.0),
                );
                let response = ui
                    .interact(r, Id::new(("answer", i)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(format!(
                        "快捷键：{}",
                        self.game.saved.settings.bindings[i].label()
                    ));
                card(&p, r, 6);
                if response.hovered() || response.has_focus() {
                    p.rect_stroke(r, 6, Stroke::new(1.5_f32, t.accent), StrokeKind::Inside);
                }
                let face = q.options[i];
                let rgb = self.game.saved.settings.palette()[face.index()];
                let swatch = Rect::from_min_size(r.min + Vec2::new(16.0, 16.0), Vec2::splat(27.0));
                p.rect_filled(swatch, 5, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                p.rect_stroke(swatch, 5, Stroke::new(1.0_f32, t.line), StrokeKind::Inside);
                text(
                    &p,
                    r.min + Vec2::new(55.0, 29.5),
                    Align2::LEFT_CENTER,
                    if self.game.saved.settings.custom_palette {
                        format!(
                            "色 {}",
                            Face::ALL.iter().position(|f| *f == face).unwrap() + 1
                        )
                    } else {
                        face.name().into()
                    },
                    15.0,
                    t.text,
                );
                keycap(
                    &p,
                    Pos2::new(r.right() - 34.0, r.center().y),
                    &self.game.saved.settings.bindings[i].label(),
                    false,
                );
                if self.game.wrong_marks[i] {
                    cross(
                        &p,
                        Pos2::new(r.center().x, r.top() - 13.0),
                        5.5,
                        t.error,
                        2.7,
                    );
                }
                if response.clicked() && !self.settings_blocking() {
                    self.answer(i);
                    return;
                }
            }
        }
        let control =
            Rect::from_center_size(Pos2::new(cx, rect.bottom() - 38.0), Vec2::new(228.0, 52.0));
        let cube_top = rect.top() + if running { 236.0 } else { 156.0 };
        let cube_bottom = control.top() - 28.0;
        let h = (cube_bottom - cube_top).min(350.0);
        let cube = Rect::from_center_size(
            Pos2::new(cx, (cube_top + cube_bottom) * 0.5),
            Vec2::new(rect.width(), h),
        );
        if !running && !self.settings_blocking() {
            let response = ui.interact(cube, Id::new("idle-cube"), Sense::drag());
            let delta = if response.dragged_by(egui::PointerButton::Primary) {
                response.drag_delta()
            } else {
                Vec2::ZERO
            };
            let dragging = response.dragged();
            response.on_hover_cursor(if dragging {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
            self.idle_cube.update(delta, Instant::now(), &mut self.rng);
        }
        self.renderer.paint(
            &p,
            cube,
            self.game.question.as_ref(),
            self.game.saved.settings.palette(),
            self.idle_cube.rotation(),
        );
        let response = button(
            ui,
            "game-toggle",
            control,
            if running {
                "停止游戏"
            } else {
                "开始游戏"
            },
            !running,
        );
        if response.clicked() && !self.settings_blocking() {
            self.toggle();
        }
        play(
            &p,
            Pos2::new(control.left() + 26.0, control.center().y),
            running,
            if running { t.secondary } else { t.on_accent },
        );
        keycap(
            &p,
            Pos2::new(control.right() - 36.0, control.center().y),
            &self.game.saved.settings.bindings[2].label(),
            !running,
        );
        self.bottom_colors_menu(&response);
    }
    fn bottom_colors_menu(&mut self, response: &egui::Response) {
        if self.game.running() || self.settings_blocking() {
            self.bottom_menu_rect = None;
            return;
        }
        let hovering = response.hovered()
            || self.bottom_menu_rect.is_some_and(|menu| {
                response
                    .ctx
                    .pointer_hover_pos()
                    .is_some_and(|pos| menu.union(response.rect).expand(4.0).contains(pos))
            });
        if !hovering {
            self.bottom_menu_rect = None;
            self.bottom_menu_dismissed = false;
            return;
        }
        if self.bottom_menu_dismissed {
            return;
        }
        let t = self.theme;
        let palette = *self.game.saved.settings.palette();
        let custom = self.game.saved.settings.custom_palette;
        let mut chosen = self.game.saved.settings.bottom_colors;
        let mut open = true;
        let popup = egui::Popup::from_response(response)
            .align(egui::emath::RectAlign::TOP)
            .align_alternatives(&[])
            .gap(8.0)
            .open_bool(&mut open)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .frame(
                egui::Frame::popup(&response.ctx.style())
                    .fill(t.surface)
                    .corner_radius(8)
                    .inner_margin(12)
                    .stroke(Stroke::new(1.0_f32, t.line)),
            )
            .show(|ui| {
                ui.set_width(204.0);
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("固定底色").size(14.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("全选").clicked() {
                            chosen = [true; 6];
                        }
                    });
                });
                ui.label(
                    egui::RichText::new("从勾选的底面颜色中抽题")
                        .size(11.0)
                        .color(t.secondary),
                );
                egui::Grid::new("bottom-color-options")
                    .num_columns(2)
                    .spacing(Vec2::new(8.0, 8.0))
                    .show(ui, |ui| {
                        for (i, face) in Face::ALL.into_iter().enumerate() {
                            let label = if custom {
                                format!("色 {}", i + 1)
                            } else {
                                face.name().into()
                            };
                            bottom_color_option(
                                ui,
                                face,
                                &label,
                                palette[face.index()],
                                &mut chosen,
                            );
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                let count = chosen.iter().filter(|selected| **selected).count();
                ui.label(
                    egui::RichText::new(format!("已选 {count} 色 · 至少保留 1 色"))
                        .size(11.0)
                        .color(t.secondary),
                );
            });
        self.bottom_menu_rect = if open {
            popup.map(|p| p.response.rect)
        } else {
            None
        };
        self.bottom_menu_dismissed = !open;
        if chosen != self.game.saved.settings.bottom_colors {
            self.game.saved.settings.bottom_colors = chosen;
            self.store.save(&self.game.saved);
        }
    }
    fn history(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter().clone();
        let t = self.theme;
        let x = rect.left() + 22.0;
        card(&p, rect, 10);
        text(
            &p,
            Pos2::new(x, rect.top() + 26.0),
            Align2::LEFT_CENTER,
            "SOLVE HISTORY",
            10.0,
            t.secondary,
        );
        timer_icon(&p, Pos2::new(x + 7.0, rect.top() + 56.0), t.accent);
        text(
            &p,
            Pos2::new(x + 25.0, rect.top() + 56.0),
            Align2::LEFT_CENTER,
            "用时记录",
            17.0,
            t.text,
        );
        let count = self.game.saved.records.len();
        let badge = Rect::from_center_size(
            Pos2::new(rect.right() - 34.0, rect.top() + 56.0),
            Vec2::new(31.0, 25.0),
        );
        p.rect_filled(badge, 6, t.subtle);
        mono(
            &p,
            badge.center(),
            Align2::CENTER_CENTER,
            count,
            11.0,
            t.secondary,
        );
        text(
            &p,
            Pos2::new(x, rect.top() + 96.0),
            Align2::LEFT_CENTER,
            "题目",
            11.0,
            t.secondary,
        );
        text(
            &p,
            Pos2::new(rect.right() - 22.0, rect.top() + 96.0),
            Align2::RIGHT_CENTER,
            "用时 / 秒",
            11.0,
            t.secondary,
        );
        divider(
            &p,
            Pos2::new(x, rect.top() + 117.0),
            Pos2::new(rect.right() - 22.0, rect.top() + 117.0),
        );
        let scroll = Rect::from_min_max(
            Pos2::new(rect.left() + 12.0, rect.top() + 129.0),
            rect.max - Vec2::new(12.0, 18.0),
        );
        if count == 0 {
            let center = scroll.center() - Vec2::new(0.0, 24.0);
            p.circle_filled(center, 28.0, t.subtle);
            timer_icon(&p, center, t.secondary);
            text(
                &p,
                center + Vec2::new(0.0, 55.0),
                Align2::CENTER_CENTER,
                "暂无成绩",
                13.0,
                t.secondary,
            );
        } else {
            let best = self
                .game
                .saved
                .records
                .iter()
                .map(|r| r.micros)
                .min()
                .unwrap();
            ui.scope_builder(UiBuilder::new().max_rect(scroll), |ui| {
                let mut area = egui::ScrollArea::vertical()
                    .id_salt("history-scroll")
                    .auto_shrink([false, false])
                    .max_height(scroll.height());
                if self.scroll_to_latest {
                    area = area.vertical_scroll_offset(0.0);
                }
                area.show_rows(ui, 48.0, count, |ui, range| {
                    for row in range {
                        let index = count - 1 - row;
                        let record = &self.game.saved.records[index];
                        let (_, r) = ui.allocate_space(Vec2::new(ui.available_width(), 48.0));
                        let p = ui.painter();
                        if row == 0 {
                            p.rect_filled(
                                r.shrink2(Vec2::new(0.0, 3.0)),
                                6,
                                t.tint(t.accent, if t.dark { 26 } else { 15 }),
                            );
                        }
                        mono(
                            p,
                            Pos2::new(r.left() + 11.0, r.center().y),
                            Align2::LEFT_CENTER,
                            format!("{:02}", index + 1),
                            13.0,
                            if row == 0 { t.accent } else { t.secondary },
                        );
                        if record.micros == best {
                            text(
                                p,
                                Pos2::new(r.left() + 53.0, r.center().y),
                                Align2::LEFT_CENTER,
                                "BEST",
                                8.0,
                                t.accent,
                            );
                        }
                        mono(
                            p,
                            Pos2::new(r.right() - 9.0, r.center().y),
                            Align2::RIGHT_CENTER,
                            format!("{:.3}", record.micros as f64 / 1_000_000.0),
                            17.0,
                            if row == 0 { t.accent } else { t.text },
                        );
                    }
                });
            });
            self.scroll_to_latest = false;
        }
    }
    fn settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            egui::Popup::close_all(ctx);
        }
        if self.snapshot_pending.is_some() {
            return;
        }
        self.settings_progress = ctx.animate_bool_with_time(
            Id::new("settings-transition"),
            self.settings_open,
            SETTINGS_ANIMATION_TIME,
        );
        if self.settings_progress == 0.0 {
            self.acrylic = None;
            return;
        }
        let opacity =
            self.settings_progress * self.settings_progress * (3.0 - 2.0 * self.settings_progress);
        let t = self.theme;
        if let Some(texture) = &self.acrylic {
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                Id::new("acrylic-scene"),
            ))
            .image(
                texture.id(),
                ctx.content_rect(),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE.gamma_multiply(opacity),
            );
        }
        let modal = egui::Modal::new(Id::new("settings-modal"))
            .area(
                egui::Modal::default_area(Id::new("settings-modal"))
                    .fade_in(false)
                    .default_size(Vec2::new(556.0, ctx.content_rect().height() - 32.0))
                    .anchor(
                        Align2::CENTER_CENTER,
                        Vec2::new(0.0, 12.0 * (1.0 - opacity)),
                    ),
            )
            .backdrop_color(
                Color32::from_black_alpha(if t.dark { 74 } else { 36 }).gamma_multiply(opacity),
            )
            .frame(
                egui::Frame::new()
                    .fill(t.tint(t.surface, 228).gamma_multiply(opacity))
                    .corner_radius(12)
                    .inner_margin(25)
                    .stroke(Stroke::new(
                        1.0_f32,
                        t.tint(t.line, 180).gamma_multiply(opacity),
                    )),
            );
        let mut close = false;
        modal.show(ctx, |ui| {
            if !self.settings_open {
                ui.disable();
            }
            ui.set_opacity(opacity);
            ui.set_width(506.0);
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
            ui.spacing_mut().button_padding = Vec2::new(14.0, 7.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("设置").size(23.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("×").clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(5.0);
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("settings-body")
                .max_height((ctx.content_rect().height() - 185.0).max(400.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    settings_choice_row(ui, "外观", |ui| {
                        for (mode, label) in [
                            (ThemeMode::System, "跟随系统"),
                            (ThemeMode::Light, "浅色"),
                            (ThemeMode::Dark, "深色"),
                        ] {
                            if selection(ui, label, self.draft.theme == mode, t) {
                                self.draft.theme = mode;
                            }
                        }
                    });
                    ui.add_space(3.0);
                    ui.separator();
                    ui.label(egui::RichText::new("键盘快捷键").size(15.0));
                    ui.label(
                        egui::RichText::new("点击键位后按下新快捷键；支持 Ctrl、Alt、Shift 组合。")
                            .size(11.0)
                            .color(t.secondary),
                    );
                    egui::Grid::new("keybindings-grid")
                        .num_columns(2)
                        .spacing(Vec2::new(185.0, 7.0))
                        .show(ui, |ui| {
                            for (index, name) in [
                                "选择左侧颜色",
                                "选择右侧颜色",
                                "开始 / 停止游戏",
                                "打开 / 关闭设置",
                            ]
                            .iter()
                            .enumerate()
                            {
                                ui.label(*name);
                                let capturing = self.capture_key == Some(index);
                                let label = if capturing {
                                    "请按下快捷键…".into()
                                } else {
                                    self.draft.bindings[index].label()
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(label)
                                                .size(12.0)
                                                .color(if capturing { t.accent } else { t.text }),
                                        )
                                        .min_size(Vec2::new(164.0, 29.0)),
                                    )
                                    .clicked()
                                {
                                    self.capture_key = Some(index);
                                    self.settings_error = None;
                                }
                                ui.end_row();
                            }
                        });
                    ui.add_space(3.0);
                    ui.separator();
                    settings_choice_row(ui, "魔方配色", |ui| {
                        for (custom, label) in [(false, "国际标准"), (true, "自定义六色")]
                        {
                            if selection(ui, label, self.draft.custom_palette == custom, t) {
                                self.draft.custom_palette = custom;
                            }
                        }
                    });
                    ui.label(
                        egui::RichText::new("相对面：黄 / 白 · 绿 / 蓝 · 橙 / 红")
                            .size(11.0)
                            .color(t.secondary),
                    );
                    ui.horizontal(|ui| {
                        for (i, face) in Face::ALL.into_iter().enumerate() {
                            ui.vertical(|ui| {
                                ui.set_width(68.0);
                                let mut color = self.draft.palette()[face.index()];
                                palette_swatch(ui, &mut color, self.draft.custom_palette);
                                if self.draft.custom_palette {
                                    self.draft.colors[face.index()] = color;
                                }
                                ui.label(
                                    egui::RichText::new(if self.draft.custom_palette {
                                        format!("色 {}", i + 1)
                                    } else {
                                        face.name().into()
                                    })
                                    .size(11.0)
                                    .color(t.secondary),
                                );
                            });
                            if i % 2 == 1 && i + 1 < Face::ALL.len() {
                                ui.add_space(8.0);
                            }
                        }
                    });
                    ui.add_space(3.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("错题回顾").size(15.0));
                            ui.label(
                                egui::RichText::new(
                                    "错选或慢于 80% 历史成绩的角块，随机旋转回顾。",
                                )
                                .size(11.0)
                                .color(t.secondary),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            switch(ui, &mut self.draft.review_mistakes)
                        });
                    });
                    ui.add_space(3.0);
                    ui.separator();
                    ui.label(egui::RichText::new("重置训练数据").size(15.0));
                    ui.horizontal(|ui| {
                        if ui.button("仅清空成绩，保留错题库").clicked() {
                            self.reset_choice = Some(false);
                        }
                        if ui.button("清空成绩 + 清空错题库").clicked() {
                            self.reset_choice = Some(true);
                        }
                    });
                    if let Some(include) = self.reset_choice {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(if include {
                                    "确认清空成绩和错题库？"
                                } else {
                                    "确认仅清空成绩？"
                                })
                                .size(12.0)
                                .color(t.error),
                            );
                            if ui.button("确认清空").clicked() {
                                self.game.stop();
                                self.game.saved.clear(include);
                                self.store.save(&self.game.saved);
                                self.reset_choice = None;
                            }
                            if ui.button("取消").clicked() {
                                self.reset_choice = None;
                            }
                        });
                    }
                    if let Some(error) = &self.settings_error {
                        ui.label(egui::RichText::new(error).size(12.0).color(t.error));
                    }
                });
            ui.add_space(6.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("恢复默认设置").clicked() {
                    self.draft = Settings::default();
                    self.capture_key = None;
                    self.settings_error = None;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("保存设置").color(t.on_accent))
                                .fill(t.accent),
                        )
                        .clicked()
                    {
                        match self.draft.validate() {
                            Ok(()) => {
                                self.game.saved.settings = self.draft.clone();
                                self.store.save(&self.game.saved);
                                close = true;
                            }
                            Err(e) => self.settings_error = Some(e),
                        }
                    }
                    if ui.button("取消").clicked() {
                        close = true;
                    }
                });
            });
        });
        if close {
            self.close_settings();
        }
    }
}

impl eframe::App for Trainer {
    #[cfg(feature = "qa")]
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        let Some(frame) = self.qa_frame else {
            return;
        };
        input.focused = true;
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        match frame {
            901 | 971 => input
                .events
                .push(Event::PointerMoved(Pos2::new(637.0, 795.0))),
            991..=1030 => input
                .events
                .push(Event::PointerMoved(Pos2::new(545.0, 675.0))),
            651 => input
                .events
                .push(Event::PointerMoved(Pos2::new(407.0, 510.0))),
            652 => input.events.push(button(Pos2::new(407.0, 510.0), true)),
            653 => input.events.push(button(Pos2::new(407.0, 510.0), false)),
            695 => input
                .events
                .push(Event::PointerMoved(Pos2::new(637.0, 505.0))),
            696 => input.events.push(button(Pos2::new(637.0, 505.0), true)),
            697 => input
                .events
                .push(Event::PointerMoved(Pos2::new(740.0, 565.0))),
            699 => input.events.push(button(Pos2::new(740.0, 565.0), false)),
            _ => {}
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "qa")]
        self.qa(ctx);
        self.receive_snapshot(ctx);
        self.keyboard(ctx);
        self.refresh_theme(ctx);
        let fill = if self.native.active {
            self.theme.tint(self.theme.background, 78)
        } else {
            self.theme.background
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(fill))
            .show(ctx, |ui| {
                let screen = ui.max_rect();
                self.header(
                    ui,
                    Rect::from_min_max(screen.min, Pos2::new(screen.right(), screen.top() + 89.0)),
                );
                let top = screen.top() + 113.0;
                let bottom = screen.bottom() - 27.0;
                let left = Rect::from_min_max(
                    Pos2::new(screen.left() + 28.0, top),
                    Pos2::new(screen.left() + 272.0, bottom),
                );
                let right = Rect::from_min_max(
                    Pos2::new(screen.right() - 283.0, top),
                    Pos2::new(screen.right() - 28.0, bottom),
                );
                let center = Rect::from_min_max(
                    Pos2::new(left.right() + 24.0, top),
                    Pos2::new(right.left() - 24.0, bottom),
                );
                self.stats(ui, left);
                self.center(ui, center);
                self.history(ui, right);
                if let Some(warning) = &self.store.warning {
                    text(
                        ui.painter(),
                        Pos2::new(screen.center().x, screen.bottom() - 8.0),
                        Align2::CENTER_CENTER,
                        warning,
                        10.0,
                        self.theme.error,
                    );
                }
            });
        self.settings(ctx);
        ctx.request_repaint_after(Duration::from_millis(16));
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.store.save(&self.game.saved);
        #[cfg(feature = "qa")]
        for capture in self.qa_captures.drain(..) {
            capture.join().unwrap();
        }
    }
}

fn percent(value: Option<f64>) -> String {
    value
        .map(|v| format!("{:.1}%", v * 100.0))
        .unwrap_or_else(|| "—".into())
}
fn bottom_color_option(ui: &mut Ui, face: Face, label: &str, rgb: [u8; 3], chosen: &mut [bool; 6]) {
    let t = Theme::get(ui.ctx());
    let selected = chosen[face.index()];
    let last = selected && chosen.iter().filter(|selected| **selected).count() == 1;
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(98.0, 36.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, !last, selected, label)
    });
    if last {
        response = response.on_hover_text("请至少保留一种底面颜色");
    } else {
        response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    if response.clicked() && !last {
        chosen[face.index()] = !selected;
        response.mark_changed();
    }
    let selected = chosen[face.index()];
    let p = ui.painter();
    p.rect_filled(
        rect,
        5,
        if selected {
            t.tint(t.accent, if t.dark { 30 } else { 16 })
        } else if response.hovered() {
            t.hover
        } else {
            t.subtle
        },
    );
    p.rect_stroke(
        rect,
        5,
        Stroke::new(
            1.0_f32,
            if selected || response.has_focus() {
                t.accent
            } else {
                t.line
            },
        ),
        StrokeKind::Inside,
    );
    let checkbox = Rect::from_center_size(
        Pos2::new(rect.left() + 14.0, rect.center().y),
        Vec2::splat(12.0),
    );
    p.rect_filled(checkbox, 2, if selected { t.accent } else { t.surface });
    if selected {
        p.line_segment(
            [
                checkbox.min + Vec2::new(2.5, 6.0),
                checkbox.min + Vec2::new(5.0, 8.5),
            ],
            Stroke::new(1.5_f32, t.on_accent),
        );
        p.line_segment(
            [
                checkbox.min + Vec2::new(5.0, 8.5),
                checkbox.min + Vec2::new(9.5, 3.5),
            ],
            Stroke::new(1.5_f32, t.on_accent),
        );
    } else {
        p.rect_stroke(
            checkbox,
            2,
            Stroke::new(1.0_f32, t.secondary),
            StrokeKind::Inside,
        );
    }
    let swatch = Rect::from_center_size(
        Pos2::new(rect.left() + 37.0, rect.center().y),
        Vec2::splat(18.0),
    );
    p.rect_filled(swatch, 3, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
    p.rect_stroke(swatch, 3, Stroke::new(1.0_f32, t.line), StrokeKind::Inside);
    text(
        p,
        Pos2::new(rect.left() + 52.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        12.0,
        t.text,
    );
}
fn selection(ui: &mut Ui, label: &str, selected: bool, t: Theme) -> bool {
    let response = ui.add(
        egui::Button::new(egui::RichText::new(label).color(if selected {
            t.on_accent
        } else {
            t.text
        }))
        .fill(if selected { t.accent } else { t.subtle })
        .min_size(Vec2::new(0.0, 32.0)),
    );
    response.clicked()
}
fn settings_choice_row(ui: &mut Ui, title: &str, choices: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 32.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(92.0, 32.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_size(Vec2::new(92.0, 32.0));
                    ui.label(egui::RichText::new(title).size(15.0));
                },
            );
            choices(ui);
        },
    );
}
fn palette_swatch(ui: &mut Ui, color: &mut [u8; 3], editable: bool) {
    let t = Theme::get(ui.ctx());
    let (rect, mut response) = ui.allocate_exact_size(
        Vec2::new(39.0, 25.0),
        if editable {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::ColorButton));
    if editable {
        egui::Popup::menu(&response).show(|ui| {
            ui.spacing_mut().slider_width = 275.0;
            let mut chosen = Color32::from_rgb(color[0], color[1], color[2]);
            if egui::color_picker::color_picker_color32(
                ui,
                &mut chosen,
                egui::color_picker::Alpha::Opaque,
            ) {
                *color = [chosen.r(), chosen.g(), chosen.b()];
                response.mark_changed();
            }
        });
    }
    ui.painter()
        .rect_filled(rect, 5, Color32::from_rgb(color[0], color[1], color[2]));
    ui.painter().rect_stroke(
        rect,
        5,
        Stroke::new(
            1.0_f32,
            if editable && (response.hovered() || response.has_focus()) {
                t.accent
            } else {
                t.line
            },
        ),
        StrokeKind::Inside,
    );
    if editable {
        response.on_hover_text("点击修改颜色");
    }
}
fn blurred_background(image: &egui::ColorImage) -> egui::ColorImage {
    // Filter premultiplied RGB and alpha together so the native backdrop remains visible.
    let bytes: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|color| color.to_array())
        .collect();
    let buffer =
        image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, bytes).unwrap();
    let w = (image.size[0] as u32).min(400);
    let h = (image.size[1] as u32 * w / image.size[0] as u32).max(1);
    let small = image::imageops::resize(&buffer, w, h, image::imageops::FilterType::Triangle);
    let blurred = image::imageops::blur(&small, SETTINGS_BLUR_SIGMA);
    egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], blurred.as_raw())
}
#[cfg(feature = "qa")]
fn flatten(image: &egui::ColorImage, background: Color32) -> image::RgbImage {
    let bytes: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|c| {
            let remaining = 1.0 - c.a() as f32 / 255.0;
            [
                ((c.r() as f32 + background.r() as f32 * remaining).min(255.0)) as u8,
                ((c.g() as f32 + background.g() as f32 * remaining).min(255.0)) as u8,
                ((c.b() as f32 + background.b() as f32 * remaining).min(255.0)) as u8,
            ]
        })
        .collect();
    image::RgbImage::from_raw(image.size[0] as u32, image.size[1] as u32, bytes).unwrap()
}

#[cfg(feature = "qa")]
impl Trainer {
    fn qa(&mut self, ctx: &egui::Context) {
        if ctx.current_pass_index() != 0 {
            return;
        }
        let Some(frame) = self.qa_frame else {
            return;
        };
        for e in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot {
                image, user_data, ..
            } = e
                && let Some(name) = user_data
                    .data
                    .as_ref()
                    .and_then(|d| d.downcast_ref::<String>())
                && name != "acrylic-background"
            {
                let name = name.clone();
                let background = self.theme.background;
                self.qa_captures.push(std::thread::spawn(move || {
                    std::fs::create_dir_all("qa").unwrap();
                    flatten(&image, background)
                        .save(format!("qa/{name}.png"))
                        .unwrap();
                    println!("Captured {name}: {} × {}", image.size[0], image.size[1]);
                }));
            }
        }
        let press = |key| {
            ctx.input_mut(|i| {
                i.events.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
            })
        };
        let capture = |name: &str| {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                name.to_owned(),
            )));
            ctx.data_mut(|data| {
                data.insert_temp(Id::new("qa-last-capture"), Instant::now());
            });
        };
        match frame {
            1100 => self.game.saved.settings.theme = ThemeMode::Light,
            1101 | 1115 => press(Key::T),
            1105 | 1109 | 1113 | 1118 => {
                if ctx.data(|data| {
                    data.get_temp::<Instant>(Id::new("qa-last-capture"))
                        .is_some_and(|time| time.elapsed() < Duration::from_millis(150))
                }) {
                    return;
                }
                press(Key::T);
            }
            1102 => {
                if self.settings_progress == 0.0 {
                    return;
                }
                capture("29-settings-opening-start");
            }
            1103 => {
                if self.settings_progress < 0.45 {
                    return;
                }
                capture("30-settings-opening-middle");
            }
            1104 => {
                if self.settings_progress < 1.0 {
                    return;
                }
                capture("31-settings-open");
            }
            1106 => {
                if self.settings_progress >= 0.95 {
                    return;
                }
                assert!(!self.settings_open);
                if self.settings_progress > 0.0 {
                    assert!(self.acrylic.is_some());
                }
                capture("32-settings-closing-start");
            }
            1107 => {
                if self.settings_progress > 0.5 {
                    return;
                }
                capture("33-settings-closing-middle");
            }
            1108 => {
                if self.settings_progress > 0.0 {
                    return;
                }
                assert!(self.acrylic.is_none());
                capture("34-settings-closed");
            }
            1110 => {
                if self.settings_progress < 0.35 {
                    return;
                }
                press(Key::T);
            }
            1111 => {
                if self.settings_progress > 0.2 {
                    return;
                }
                press(Key::T);
            }
            1112 => {
                if self.settings_progress < 1.0 {
                    return;
                }
                assert!(self.settings_open && self.snapshot_pending.is_none());
                capture("35-settings-reopened");
            }
            1114 => {
                if self.settings_progress > 0.0 {
                    return;
                }
                self.game.saved.settings.theme = ThemeMode::Dark;
            }
            1116 => {
                if self.settings_progress < 0.5 {
                    return;
                }
                capture("36-settings-dark-opening");
            }
            1117 => {
                if self.settings_progress < 1.0 {
                    return;
                }
                capture("37-settings-dark-open");
            }
            1119 => {
                if self.settings_progress > 0.5 {
                    return;
                }
                capture("38-settings-dark-closing");
            }
            1120 => {
                if self.settings_progress > 0.0 {
                    return;
                }
                assert!(!self.settings_blocking() && self.acrylic.is_none());
                capture("39-settings-dark-closed");
            }
            1124 => {
                println!(
                    "PASS: T toggles settings, light/dark opening and closing transitions, rapid reversal and backdrop cleanup"
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            900 => self.game.saved.settings.theme = ThemeMode::Light,
            915 => {
                assert!(self.bottom_menu_rect.is_some());
                capture("23-bottom-colors-all");
            }
            920 => {
                self.game.saved.settings.bottom_colors = [false; 6];
                self.game.saved.settings.bottom_colors[Face::White.index()] = true;
            }
            935 => capture("24-bottom-colors-white"),
            940 => press(Key::Space),
            942 => {
                assert!(self.bottom_menu_rect.is_none());
                assert_eq!(self.game.question.as_ref().unwrap().bottom(), Face::White);
                assert_eq!(self.game.question.as_ref().unwrap().faces[1], Face::Yellow);
                assert_ne!(self.game.question.as_ref().unwrap().hidden, 1);
            }
            960 => capture("25-white-bottom-question"),
            965 => press(Key::Space),
            970 => {
                self.game.saved.settings.theme = ThemeMode::Dark;
                self.game.saved.settings.bottom_colors = [false; 6];
                for face in [Face::Green, Face::Blue] {
                    self.game.saved.settings.bottom_colors[face.index()] = true;
                }
            }
            985 => {
                assert!(self.bottom_menu_rect.is_some());
                capture("26-bottom-colors-dark");
            }
            990 => {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1100.0, 740.0)))
            }
            1005 => {
                assert!(self.bottom_menu_rect.is_some());
                capture("27-bottom-colors-minimum");
            }
            1010 => self.game.saved.settings.custom_palette = true,
            1020 => capture("28-bottom-colors-custom"),
            1030 => {
                println!(
                    "PASS: hover bottom-color menu, physical white bottom/yellow top, multi-selection, dark theme, custom palette and minimum layout"
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            1 => self.game.saved.settings.theme = ThemeMode::Light,
            25 => capture("01-idle"),
            60 => capture("02-rotating"),
            80 => press(Key::Space),
            82 => assert!(self.game.running()),
            100 => capture("03-active"),
            115 => {
                let q = self.game.question.as_ref().unwrap();
                let wrong = q.options.iter().position(|f| *f != q.correct()).unwrap();
                press(self.game.saved.settings.bindings[wrong].key);
            }
            117 => {
                assert!(self.game.wrong_marks.iter().any(|b| *b));
                assert!(self.game.saved.records.is_empty());
            }
            130 => capture("04-wrong"),
            145 => {
                let q = self.game.question.as_ref().unwrap();
                let right = q.options.iter().position(|f| *f == q.correct()).unwrap();
                press(self.game.saved.settings.bindings[right].key);
            }
            147 => {
                assert_eq!(self.game.saved.records.len(), 1);
                assert_eq!(self.game.wrong_marks, [false; 2]);
            }
            160 => capture("05-next-question"),
            175 => {
                for (i, s) in [
                    2.346, 1.825, 3.102, 1.472, 2.018, 1.693, 2.571, 1.358, 1.944, 2.207, 1.618,
                ]
                .into_iter()
                .enumerate()
                {
                    self.game.started_at = Some(Instant::now() - Duration::from_secs_f64(s));
                    let q = self.game.question.as_ref().unwrap();
                    let right = q.options.iter().position(|f| *f == q.correct()).unwrap();
                    if i == 3 {
                        self.game.answer(1 - right, &mut self.rng, Instant::now());
                    }
                    self.game.answer(right, &mut self.rng, Instant::now());
                }
                self.scroll_to_latest = true;
            }
            190 => capture("06-history"),
            205 => press(Key::T),
            207 => assert!(self.settings_open),
            230 => {
                assert!(self.acrylic.is_some());
                capture("07-settings");
            }
            245 => {
                self.capture_key = Some(0);
                press(Key::Q);
            }
            247 => assert_eq!(self.draft.bindings[0].key, Key::Q),
            260 => {
                self.draft.custom_palette = true;
                self.draft.colors = [
                    [220, 64, 114],
                    [252, 158, 64],
                    [234, 242, 253],
                    [245, 219, 71],
                    [78, 187, 158],
                    [97, 106, 239],
                ];
                self.draft.review_mistakes = false;
            }
            275 => capture("08-custom-settings"),
            290 => {
                assert!(self.draft.validate().is_ok());
                self.game.saved.settings = self.draft.clone();
                self.store.save(&self.game.saved);
                self.close_settings();
            }
            305 => capture("09-custom-palette"),
            320 => {
                self.game.saved.settings = Settings::default();
                self.game.saved.settings.theme = ThemeMode::Dark;
            }
            340 => capture("10-dark-active"),
            355 => press(Key::T),
            380 => capture("11-dark-settings"),
            395 => {
                self.close_settings();
                self.game.saved.settings = Settings::default();
                self.game.saved.settings.theme = ThemeMode::Light;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1100.0, 740.0)));
            }
            420 => capture("12-minimum-active"),
            435 => press(Key::T),
            460 => capture("13-minimum-settings"),
            475 => {
                self.close_settings();
                self.game.saved.settings.theme = ThemeMode::Dark;
            }
            495 => capture("14-minimum-dark"),
            510 => {
                self.game.saved.records = (0..105)
                    .map(|i| crate::model::Record {
                        micros: 500_000 + (i % 9) * 127_000,
                        had_error: i % 5 == 0,
                    })
                    .collect();
                self.game.saved.wrong_clicks = self
                    .game
                    .saved
                    .records
                    .iter()
                    .filter(|r| r.had_error)
                    .count() as u64;
                self.scroll_to_latest = true;
                assert_eq!(self.game.saved.accuracy_100(), Some(0.8));
                assert!(self.game.saved.mo3().is_some());
            }
            530 => capture("15-hundred-accuracy"),
            545 => press(Key::Space),
            547 => assert!(!self.game.running()),
            565 => {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1280.0, 860.0)));
                self.game.saved.settings.theme = ThemeMode::Light;
                self.open_settings(ctx);
            }
            585 => capture("16-standard-aligned"),
            600 => self.draft.theme = ThemeMode::Dark,
            620 => {
                assert!(self.theme.dark && self.acrylic.is_some());
                assert_eq!(self.game.saved.settings.theme, ThemeMode::Light);
                capture("17-theme-preview-dark");
            }
            630 => self.draft.custom_palette = true,
            650 => capture("18-custom-aligned-dark"),
            660 => {
                assert!(egui::Popup::is_any_open(ctx));
                capture("18b-color-picker");
            }
            665 => self.close_settings(),
            680 => {
                assert!(!self.theme.dark && !self.settings_open);
                capture("19-theme-cancelled");
            }
            705 => capture("20-idle-dragged"),
            840 => capture("21-idle-auto-resumed"),
            845 => {
                self.game.saved.settings.theme = ThemeMode::Dark;
                self.idle_cube.front_view();
            }
            855 => capture("22-thin-shell-front"),
            860 => {
                println!(
                    "PASS: stickerless rendering, digital timer, input, timing, mistake/slow review, Mica/Acrylic, theme preview and cancel, aligned settings, idle mouse drag, mo3, 100-question accuracy, persistence and minimum layout"
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {}
        }
        self.qa_frame = Some(frame + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Modifiers, PointerButton, RawInput};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Rig {
        ctx: egui::Context,
        app: Trainer,
        directory: std::path::PathBuf,
    }
    impl Rig {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let directory = std::env::temp_dir().join(format!(
                "corner-trainer-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let saved = crate::model::Saved::default();
            let ctx = egui::Context::default();
            Theme::default().apply(&ctx);
            Self {
                ctx,
                app: Trainer {
                    draft: saved.settings.clone(),
                    game: Game::new(saved),
                    store: Store {
                        path: directory.join("data.json"),
                        warning: None,
                    },
                    rng: rand::rng(),
                    renderer: CubeRenderer::default(),
                    idle_cube: IdleCube::default(),
                    bottom_menu_rect: None,
                    bottom_menu_dismissed: false,
                    settings_open: false,
                    settings_progress: 0.0,
                    capture_key: None,
                    settings_error: None,
                    reset_choice: None,
                    scroll_to_latest: false,
                    theme: Theme::default(),
                    system_dark: false,
                    theme_checked: Instant::now(),
                    native: NativeMaterial::disabled(),
                    acrylic: None,
                    snapshot_pending: None,
                    #[cfg(feature = "qa")]
                    qa_frame: None,
                    #[cfg(feature = "qa")]
                    qa_captures: Vec::new(),
                },
                directory,
            }
        }
        fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
            self.ctx.run(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 860.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    self.app.keyboard(ctx);
                    egui::CentralPanel::default().show(ctx, |ui| {
                        self.app.center(
                            ui,
                            Rect::from_min_max(Pos2::new(300.0, 113.0), Pos2::new(975.0, 833.0)),
                        )
                    });
                },
            )
        }
        fn menu_click(&mut self, label: &str) {
            let output = self.frame(vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Bottom color menu control missing: {label}"));
            self.click(pos);
            self.frame(vec![]);
        }
        fn key(&mut self, key: Key) {
            self.frame(vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]);
            self.frame(vec![Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]);
        }
        fn click(&mut self, pos: Pos2) {
            self.frame(vec![Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                self.frame(vec![Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                }]);
            }
        }
        fn modal_frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
            self.ctx.run(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 860.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    self.app.keyboard(ctx);
                    self.app.refresh_theme(ctx);
                    // The native screenshot is supplied asynchronously in the desktop app.
                    self.app.snapshot_pending = None;
                    egui::CentralPanel::default().show(ctx, |_| {});
                    self.app.settings(ctx);
                },
            )
        }
        fn modal_click(&mut self, label: &str) {
            for _ in 0..12 {
                self.modal_frame(vec![]);
            }
            let output = self.modal_frame(vec![]);
            fn text_center(shape: &egui::Shape, label: &str) -> Option<Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    }
                    egui::Shape::Vec(shapes) => {
                        shapes.iter().find_map(|shape| text_center(shape, label))
                    }
                    _ => None,
                }
            }
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| text_center(&shape.shape, label))
                .unwrap_or_else(|| panic!("Settings control missing: {label}"));
            self.modal_frame(vec![Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                self.modal_frame(vec![Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                }]);
            }
            self.modal_frame(vec![]);
        }
    }
    impl Drop for Rig {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(self.directory.join("data.json"));
            let _ = std::fs::remove_file(self.directory.join("data.tmp"));
            let _ = std::fs::remove_dir(&self.directory);
        }
    }
    #[test]
    fn mouse_start_space_stop_and_repeat_is_consumed() {
        let mut rig = Rig::new();
        rig.frame(vec![]);
        rig.click(Pos2::new(637.5, 795.0));
        assert!(rig.app.game.running());
        for repeat in [false, true] {
            rig.frame(vec![Event::Key {
                key: Key::Space,
                physical_key: None,
                pressed: true,
                repeat,
                modifiers: Modifiers::NONE,
            }]);
            assert!(!rig.app.game.running());
        }
        assert!(rig.app.game.saved.records.is_empty());
    }
    #[test]
    fn hover_menu_stays_above_start_and_saves_single_and_multiple_bottom_colors() {
        let mut rig = Rig::new();
        rig.frame(vec![]);
        rig.frame(vec![Event::PointerMoved(Pos2::new(637.5, 795.0))]);
        rig.frame(vec![]);
        let menu = rig.app.bottom_menu_rect.unwrap();
        assert!(menu.bottom() < 769.0);
        assert_eq!(rig.app.game.saved.settings.bottom_colors, [true; 6]);
        rig.frame(vec![Event::PointerMoved(Pos2::new(
            menu.center().x,
            (menu.bottom() + 769.0) * 0.5,
        ))]);
        assert!(rig.app.bottom_menu_rect.is_some());
        for label in ["黄色", "红色", "橙色", "绿色", "蓝色"] {
            rig.menu_click(label);
            assert!(rig.app.bottom_menu_rect.is_some());
            assert!(!rig.app.game.running());
        }
        let mut white = [false; 6];
        white[Face::White.index()] = true;
        assert_eq!(rig.app.game.saved.settings.bottom_colors, white);
        rig.menu_click("白色");
        assert_eq!(rig.app.game.saved.settings.bottom_colors, white);
        let persisted: crate::model::Saved =
            serde_json::from_slice(&std::fs::read(&rig.app.store.path).unwrap()).unwrap();
        assert_eq!(persisted.settings.bottom_colors, white);
        rig.key(Key::Space);
        assert!(rig.app.bottom_menu_rect.is_none());
        assert_eq!(
            rig.app.game.question.as_ref().unwrap().bottom(),
            Face::White
        );
        assert_eq!(
            rig.app.game.question.as_ref().unwrap().faces[1],
            Face::Yellow
        );
        assert_ne!(rig.app.game.question.as_ref().unwrap().hidden, 1);
        rig.key(Key::Space);
        rig.frame(vec![Event::PointerMoved(Pos2::new(637.5, 795.0))]);
        rig.frame(vec![]);
        rig.menu_click("绿色");
        rig.menu_click("蓝色");
        rig.menu_click("白色");
        assert!(rig.app.game.saved.settings.bottom_colors[Face::Green.index()]);
        assert!(rig.app.game.saved.settings.bottom_colors[Face::Blue.index()]);
        assert_eq!(
            rig.app
                .game
                .saved
                .settings
                .bottom_colors
                .iter()
                .filter(|v| **v)
                .count(),
            2
        );
        rig.menu_click("全选");
        assert_eq!(rig.app.game.saved.settings.bottom_colors, [true; 6]);
        rig.frame(vec![Event::PointerMoved(Pos2::new(310.0, 300.0))]);
        assert!(rig.app.bottom_menu_rect.is_none());
    }
    #[test]
    fn paired_colors_share_the_same_order_in_menu_and_palette_settings() {
        fn label_position(output: &egui::FullOutput, label: &str) -> Pos2 {
            fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
                    _ => None,
                }
            }
            output
                .shapes
                .iter()
                .find_map(|shape| find(&shape.shape, label))
                .unwrap_or_else(|| panic!("Color label missing: {label}"))
        }
        let pairs = [["黄色", "白色"], ["绿色", "蓝色"], ["橙色", "红色"]];
        let mut rig = Rig::new();
        rig.frame(vec![]);
        rig.frame(vec![Event::PointerMoved(Pos2::new(637.5, 795.0))]);
        rig.frame(vec![]);
        let output = rig.frame(vec![]);
        let mut previous_y = f32::NEG_INFINITY;
        for [left, right] in pairs {
            let left = label_position(&output, left);
            let right = label_position(&output, right);
            assert!(left.x < right.x);
            assert!((left.y - right.y).abs() < 1.0);
            assert!(left.y > previous_y);
            previous_y = left.y;
        }
        rig.app.open_settings(&rig.ctx);
        for custom in [false, true] {
            rig.app.draft.custom_palette = custom;
            let colors = rig.app.draft.colors;
            for _ in 0..12 {
                rig.modal_frame(vec![]);
            }
            let output = rig.modal_frame(vec![]);
            let mut previous_x = f32::NEG_INFINITY;
            for (i, name) in pairs.into_iter().flatten().enumerate() {
                let label = if custom {
                    format!("色 {}", i + 1)
                } else {
                    name.into()
                };
                let pos = label_position(&output, &label);
                assert!(pos.x > previous_x);
                previous_x = pos.x;
            }
            assert_eq!(rig.app.draft.colors, colors);
        }
    }
    #[test]
    fn custom_palette_menu_uses_face_identities_and_escape_closes_it() {
        let mut rig = Rig::new();
        rig.app.game.saved.settings.custom_palette = true;
        rig.app.game.saved.settings.colors[Face::Yellow.index()] = [200, 80, 150];
        rig.frame(vec![]);
        rig.frame(vec![Event::PointerMoved(Pos2::new(637.5, 795.0))]);
        rig.frame(vec![]);
        rig.menu_click("色 1");
        assert!(!rig.app.game.saved.settings.bottom_colors[Face::Yellow.index()]);
        rig.key(Key::Escape);
        assert!(rig.app.bottom_menu_rect.is_none());
        rig.frame(vec![Event::PointerMoved(Pos2::new(310.0, 300.0))]);
        rig.frame(vec![Event::PointerMoved(Pos2::new(637.5, 795.0))]);
        rig.frame(vec![]);
        assert!(rig.app.bottom_menu_rect.is_some());
        rig.click(Pos2::new(637.5, 795.0));
        assert!(rig.app.game.running());
        assert!(rig.app.bottom_menu_rect.is_none());
    }
    #[test]
    fn mouse_drag_rotates_idle_cube_and_training_keeps_its_question_fixed() {
        let mut rig = Rig::new();
        rig.frame(vec![]);
        let initial = rig.app.idle_cube.rotation();
        let start = Pos2::new(635.0, 505.0);
        rig.frame(vec![Event::PointerMoved(start)]);
        rig.frame(vec![Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        rig.frame(vec![Event::PointerMoved(start + Vec2::new(100.0, 60.0))]);
        let dragged = rig.app.idle_cube.rotation();
        assert_ne!(dragged, initial);
        rig.frame(vec![Event::PointerButton {
            pos: start + Vec2::new(100.0, 60.0),
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        rig.key(Key::Space);
        let question = rig.app.game.question.clone();
        rig.frame(vec![Event::PointerMoved(start)]);
        rig.frame(vec![Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        rig.frame(vec![Event::PointerMoved(start + Vec2::new(-80.0, 50.0))]);
        assert_eq!(rig.app.idle_cube.rotation(), dragged);
        assert_eq!(rig.app.game.question, question);
        assert!(rig.app.game.saved.records.is_empty());
    }
    #[test]
    fn mouse_wrong_then_correct_and_default_key_answers() {
        let mut rig = Rig::new();
        rig.frame(vec![]);
        rig.key(Key::Space);
        let q = rig.app.game.question.clone().unwrap();
        let clock = rig.app.game.started_at;
        let right = q.options.iter().position(|f| *f == q.correct()).unwrap();
        rig.click(Pos2::new(637.5 - 98.5 + (1 - right) as f32 * 197.0, 290.5));
        assert_eq!(rig.app.game.question.as_ref(), Some(&q));
        assert_eq!(rig.app.game.started_at, clock);
        assert!(rig.app.game.wrong_marks[1 - right]);
        assert!(rig.app.game.saved.records.is_empty());
        rig.key(rig.app.game.saved.settings.bindings[right].key);
        assert_eq!(rig.app.game.saved.records.len(), 1);
        assert_eq!(rig.app.game.wrong_marks, [false; 2]);
    }
    #[test]
    fn blurred_background_preserves_alpha_and_premultiplied_colors() {
        for color in [
            Color32::TRANSPARENT,
            Color32::from_rgba_unmultiplied(120, 80, 60, 90),
            Color32::from_rgb(32, 32, 32),
        ] {
            let source = egui::ColorImage::filled([800, 80], color);
            let blurred = blurred_background(&source);
            assert_eq!(blurred.size, [400, 40]);
            for pixel in blurred.pixels {
                for (actual, expected) in pixel.to_array().into_iter().zip(color.to_array()) {
                    assert!(actual.abs_diff(expected) <= 1);
                }
            }
        }
        let mut source = egui::ColorImage::filled([80, 8], Color32::TRANSPARENT);
        for row in source.pixels.chunks_mut(80) {
            row[40..].fill(Color32::from_rgba_unmultiplied(120, 80, 60, 90));
        }
        let blurred = blurred_background(&source);
        assert_eq!(blurred.pixels[0], Color32::TRANSPARENT);
        let edge = blurred.pixels[40];
        assert!(edge.a() > 0 && edge.a() < 90);
        assert!(
            edge.to_array()[..3]
                .iter()
                .all(|channel| *channel <= edge.a())
        );
    }
    #[test]
    fn settings_shortcut_toggles_custom_chords_and_key_capture_has_priority() {
        let mut rig = Rig::new();
        rig.frame(vec![]);
        rig.key(Key::T);
        assert!(rig.app.settings_open);
        rig.key(Key::T);
        assert!(!rig.app.settings_open);
        rig.key(Key::T);
        rig.app.capture_key = Some(3);
        rig.key(Key::G);
        assert!(rig.app.settings_open);
        assert_eq!(rig.app.draft.bindings[3].key, Key::G);
        rig.modal_click("保存设置");
        assert!(!rig.app.settings_open);
        rig.key(Key::G);
        assert!(rig.app.settings_open);
        rig.key(Key::G);
        assert!(!rig.app.settings_open);
        rig.app.game.saved.settings.bindings[3] = Binding {
            key: Key::K,
            ctrl: true,
            alt: false,
            shift: true,
        };
        let press = |repeat| Event::Key {
            key: Key::K,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers: Modifiers {
                ctrl: true,
                shift: true,
                ..Modifiers::NONE
            },
        };
        rig.frame(vec![press(false)]);
        assert!(rig.app.settings_open);
        rig.frame(vec![press(true)]);
        assert!(rig.app.settings_open);
        rig.frame(vec![Event::Key {
            key: Key::K,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers {
                ctrl: true,
                shift: true,
                ..Modifiers::NONE
            },
        }]);
        rig.frame(vec![press(false)]);
        assert!(!rig.app.settings_open);
    }
    #[test]
    fn settings_fade_retains_backdrop_blocks_answers_and_reverses_smoothly() {
        let mut rig = Rig::new();
        rig.key(Key::Space);
        let question = rig.app.game.question.clone().unwrap();
        let clock = rig.app.game.started_at;
        let right = question
            .options
            .iter()
            .position(|f| *f == question.correct())
            .unwrap();
        rig.app.open_settings(&rig.ctx);
        let texture = rig.ctx.load_texture(
            "test-settings-backdrop",
            egui::ColorImage::filled([2, 2], Color32::WHITE),
            egui::TextureOptions::LINEAR,
        );
        let texture_id = texture.id();
        rig.app.acrylic = Some(texture);
        rig.modal_frame(vec![]);
        assert!(rig.app.settings_progress > 0.0 && rig.app.settings_progress < 1.0);
        for _ in 0..16 {
            rig.modal_frame(vec![]);
        }
        assert_eq!(rig.app.settings_progress, 1.0);
        rig.modal_frame(vec![Event::Key {
            key: Key::T,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        assert!(!rig.app.settings_open);
        let closing = rig.app.settings_progress;
        assert!(closing > 0.0 && closing < 1.0);
        assert!(rig.app.settings_blocking());
        assert_eq!(rig.app.acrylic.as_ref().unwrap().id(), texture_id);
        rig.modal_frame(vec![
            Event::Key {
                key: Key::T,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
            Event::Key {
                key: rig.app.game.saved.settings.bindings[right].key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ]);
        assert_eq!(rig.app.game.question.as_ref(), Some(&question));
        assert_eq!(rig.app.game.started_at, clock);
        assert!(rig.app.game.saved.records.is_empty());
        let reversing = rig.app.settings_progress;
        rig.modal_frame(vec![Event::Key {
            key: Key::T,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        assert!(rig.app.settings_open);
        assert!(rig.app.settings_progress > reversing && rig.app.settings_progress <= closing);
        assert!(rig.app.snapshot_pending.is_none());
        assert_eq!(rig.app.acrylic.as_ref().unwrap().id(), texture_id);
        for _ in 0..16 {
            rig.modal_frame(vec![]);
        }
        rig.app.close_settings();
        for _ in 0..16 {
            rig.modal_frame(vec![]);
        }
        assert_eq!(rig.app.settings_progress, 0.0);
        assert!(!rig.app.settings_blocking());
        assert!(rig.app.acrylic.is_none());
        rig.frame(vec![Event::Key {
            key: rig.app.game.saved.settings.bindings[right].key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        rig.key(rig.app.game.saved.settings.bindings[right].key);
        assert_eq!(rig.app.game.saved.records.len(), 1);
    }
    #[test]
    fn settings_capture_rejects_duplicate_and_cancel_preserves_bindings() {
        let mut rig = Rig::new();
        rig.frame(vec![]);
        rig.key(Key::Space);
        rig.key(Key::T);
        assert!(rig.app.settings_open);
        rig.key(Key::Space);
        assert!(rig.app.game.running());
        rig.app.capture_key = Some(0);
        rig.key(Key::F);
        assert_eq!(rig.app.draft.bindings[0].key, Key::A);
        assert!(rig.app.settings_error.is_some());
        rig.app.capture_key = Some(0);
        rig.key(Key::Q);
        assert_eq!(rig.app.draft.bindings[0].key, Key::Q);
        rig.key(Key::Escape);
        assert!(!rig.app.settings_open);
        assert_eq!(rig.app.game.saved.settings.bindings[0].key, Key::A);
    }
    #[test]
    fn theme_buttons_preview_and_all_dismissals_restore_saved_theme() {
        for dismissal in ["取消", "×", "Esc", "T"] {
            let mut rig = Rig::new();
            rig.app.game.saved.settings.theme = ThemeMode::Light;
            rig.app.store.save(&rig.app.game.saved);
            rig.app.open_settings(&rig.ctx);
            rig.modal_click("深色");
            assert!(rig.app.theme.dark);
            assert_eq!(rig.app.game.saved.settings.theme, ThemeMode::Light);
            // Answers can be saved while the preview is active.
            rig.app.store.save(&rig.app.game.saved);
            let persisted: crate::model::Saved =
                serde_json::from_slice(&std::fs::read(&rig.app.store.path).unwrap()).unwrap();
            assert_eq!(persisted.settings.theme, ThemeMode::Light);
            if dismissal == "Esc" || dismissal == "T" {
                rig.modal_frame(vec![Event::Key {
                    key: if dismissal == "Esc" {
                        Key::Escape
                    } else {
                        Key::T
                    },
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }]);
            } else {
                rig.modal_click(dismissal);
            }
            assert!(!rig.app.settings_open);
            assert!(!rig.app.theme.dark);
            assert_eq!(rig.app.game.saved.settings.theme, ThemeMode::Light);
        }
    }
    #[test]
    fn saving_preview_persists_theme_and_default_preview_can_be_cancelled() {
        let mut rig = Rig::new();
        rig.app.game.saved.settings.theme = ThemeMode::Light;
        rig.app.open_settings(&rig.ctx);
        rig.modal_click("深色");
        rig.modal_click("保存设置");
        assert!(!rig.app.settings_open);
        assert!(rig.app.theme.dark);
        let persisted: crate::model::Saved =
            serde_json::from_slice(&std::fs::read(&rig.app.store.path).unwrap()).unwrap();
        assert_eq!(persisted.settings.theme, ThemeMode::Dark);
        rig.app.open_settings(&rig.ctx);
        assert_eq!(rig.app.draft.theme, ThemeMode::Dark);
        rig.modal_click("恢复默认设置");
        assert_eq!(rig.app.draft.theme, ThemeMode::System);
        assert!(!rig.app.theme.dark);
        rig.app.system_dark = true;
        rig.modal_frame(vec![]);
        assert!(rig.app.theme.dark);
        rig.app.system_dark = false;
        rig.modal_frame(vec![]);
        assert!(!rig.app.theme.dark);
        rig.modal_click("取消");
        assert!(rig.app.theme.dark);
        assert_eq!(rig.app.game.saved.settings.theme, ThemeMode::Dark);
    }
}
