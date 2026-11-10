use crate::theme::Theme;
use egui::{
    Align2, Color32, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2,
};

pub fn text(
    p: &Painter,
    pos: Pos2,
    align: Align2,
    value: impl ToString,
    size: f32,
    color: Color32,
) {
    p.text(
        pos,
        align,
        value.to_string(),
        FontId::proportional(size),
        color,
    );
}
pub fn mono(
    p: &Painter,
    pos: Pos2,
    align: Align2,
    value: impl ToString,
    size: f32,
    color: Color32,
) {
    p.text(
        pos,
        align,
        value.to_string(),
        FontId::monospace(size),
        color,
    );
}
pub fn card(p: &Painter, rect: Rect, radius: u8) {
    let t = Theme::get(p.ctx());
    p.rect_filled(
        rect.translate(Vec2::new(0.0, 2.0)),
        radius,
        Color32::from_black_alpha(if t.dark { 18 } else { 7 }),
    );
    p.rect_filled(rect, radius, t.tint(t.surface, 205));
    p.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0_f32, t.tint(t.line, 190)),
        StrokeKind::Inside,
    );
    p.line_segment(
        [
            rect.left_top() + Vec2::new(radius as f32, 0.5),
            rect.right_top() + Vec2::new(-(radius as f32), 0.5),
        ],
        Stroke::new(
            1.0_f32,
            Color32::from_white_alpha(if t.dark { 12 } else { 120 }),
        ),
    );
}
pub fn divider(p: &Painter, a: Pos2, b: Pos2) {
    p.line_segment([a, b], Stroke::new(1.0_f32, Theme::get(p.ctx()).line));
}
pub fn button(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    rect: Rect,
    title: &str,
    primary: bool,
) -> Response {
    let t = Theme::get(ui.ctx());
    let r = ui
        .interact(rect, Id::new(id), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let fill = if primary {
        if r.is_pointer_button_down_on() {
            t.accent.linear_multiply(0.80)
        } else if r.hovered() {
            t.accent.linear_multiply(0.90)
        } else {
            t.accent
        }
    } else if r.hovered() {
        t.hover
    } else {
        t.tint(t.surface, 230)
    };
    ui.painter().rect_filled(rect, 6, fill);
    if !primary {
        ui.painter()
            .rect_stroke(rect, 6, Stroke::new(1.0_f32, t.line), StrokeKind::Inside);
    }
    if r.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            8,
            Stroke::new(1.5_f32, t.accent),
            StrokeKind::Inside,
        );
    }
    text(
        ui.painter(),
        rect.center(),
        Align2::CENTER_CENTER,
        title,
        15.0,
        if primary { t.on_accent } else { t.text },
    );
    r
}
pub fn keycap(p: &Painter, center: Pos2, label: &str, primary: bool) {
    let t = Theme::get(p.ctx());
    let color = if primary { t.on_accent } else { t.secondary };
    let mut galley = p.layout_no_wrap(label.into(), FontId::monospace(11.0), color);
    if galley.size().x > 48.0 {
        let mut compact: String = label.chars().filter(|c| !c.is_whitespace()).collect();
        while !compact.is_empty() {
            compact.pop();
            galley = p.layout_no_wrap(format!("{compact}…"), FontId::monospace(11.0), color);
            if galley.size().x <= 48.0 {
                break;
            }
        }
    }
    let rect = Rect::from_center_size(center, Vec2::new((galley.size().x + 14.0).max(25.0), 24.0));
    p.rect_filled(
        rect,
        4,
        if primary {
            t.tint(t.on_accent, 18)
        } else {
            t.subtle
        },
    );
    p.rect_stroke(
        rect,
        4,
        Stroke::new(
            1.0_f32,
            if primary {
                t.tint(t.on_accent, 40)
            } else {
                t.line
            },
        ),
        StrokeKind::Inside,
    );
    p.galley(rect.center() - galley.size() * 0.5, galley, color);
}
pub fn cross(p: &Painter, center: Pos2, radius: f32, color: Color32, width: f32) {
    p.line_segment(
        [center - Vec2::splat(radius), center + Vec2::splat(radius)],
        Stroke::new(width, color),
    );
    p.line_segment(
        [
            center + Vec2::new(-radius, radius),
            center + Vec2::new(radius, -radius),
        ],
        Stroke::new(width, color),
    );
}
pub fn switch(ui: &mut Ui, value: &mut bool) {
    let t = Theme::get(ui.ctx());
    let (rect, r) = ui.allocate_exact_size(Vec2::new(44.0, 24.0), Sense::click());
    if r.clicked() {
        *value = !*value;
    }
    ui.painter()
        .rect_filled(rect, 12, if *value { t.accent } else { t.secondary });
    ui.painter().circle_filled(
        Pos2::new(
            if *value {
                rect.right() - 12.0
            } else {
                rect.left() + 12.0
            },
            rect.center().y,
        ),
        8.5,
        if *value { t.on_accent } else { t.surface },
    );
    if r.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            14,
            Stroke::new(1.5_f32, t.accent),
            StrokeKind::Inside,
        );
    }
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(if *value {
            "关闭错题回顾"
        } else {
            "开启错题回顾"
        });
}
pub fn logo(p: &Painter, rect: Rect) {
    let t = Theme::get(p.ctx());
    p.rect_filled(rect, 9, t.accent);
    let cell = rect.width() * 0.24;
    let start = rect.min + Vec2::splat(rect.width() * 0.2);
    for y in 0..2 {
        for x in 0..2 {
            p.rect_filled(
                Rect::from_min_size(
                    start + Vec2::new(x as f32 * (cell + 4.0), y as f32 * (cell + 4.0)),
                    Vec2::splat(cell),
                ),
                2,
                if x == y {
                    t.on_accent
                } else {
                    t.tint(t.on_accent, 130)
                },
            );
        }
    }
}
pub fn play(p: &Painter, center: Pos2, running: bool, color: Color32) {
    if running {
        p.rect_filled(Rect::from_center_size(center, Vec2::splat(10.0)), 2, color);
    } else {
        p.add(Shape::convex_polygon(
            vec![
                center + Vec2::new(-4.0, -6.0),
                center + Vec2::new(-4.0, 6.0),
                center + Vec2::new(6.0, 0.0),
            ],
            color,
            Stroke::NONE,
        ));
    }
}
pub fn gear(p: &Painter, center: Pos2, color: Color32) {
    p.circle_stroke(center, 6.0, Stroke::new(1.7_f32, color));
    p.circle_stroke(center, 2.2, Stroke::new(1.5_f32, color));
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.0;
        let d = Vec2::new(a.cos(), a.sin());
        p.line_segment(
            [center + d * 6.0, center + d * 9.0],
            Stroke::new(2.2_f32, color),
        );
    }
}
pub fn timer_icon(p: &Painter, center: Pos2, color: Color32) {
    p.circle_stroke(center, 7.0, Stroke::new(1.6_f32, color));
    p.line_segment(
        [center + Vec2::new(0.0, -4.0), center],
        Stroke::new(1.5_f32, color),
    );
    p.line_segment(
        [center, center + Vec2::new(3.0, 1.0)],
        Stroke::new(1.5_f32, color),
    );
    p.line_segment(
        [
            center + Vec2::new(-3.0, -10.0),
            center + Vec2::new(3.0, -10.0),
        ],
        Stroke::new(1.8_f32, color),
    );
}
pub fn chart_icon(p: &Painter, center: Pos2, color: Color32) {
    for (x, h) in [(-6.0, 6.0), (0.0, 13.0), (6.0, 9.0)] {
        p.rect_filled(
            Rect::from_min_size(center + Vec2::new(x - 1.5, 7.0 - h), Vec2::new(3.0, h)),
            1,
            color,
        );
    }
}
