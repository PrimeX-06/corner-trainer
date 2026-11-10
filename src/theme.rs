use egui::{Color32, Id};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub dark: bool,
    pub background: Color32,
    pub surface: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub subtle: Color32,
    pub hover: Color32,
    pub line: Color32,
    pub error: Color32,
    pub success: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(false)
    }
}
impl Theme {
    pub fn new(dark: bool) -> Self {
        let rgb = Color32::from_rgb;
        if dark {
            Self {
                dark,
                background: rgb(32, 32, 32),
                surface: rgb(44, 44, 44),
                text: rgb(244, 244, 244),
                secondary: rgb(170, 170, 170),
                accent: rgb(96, 205, 255),
                on_accent: rgb(0, 40, 56),
                subtle: rgb(53, 53, 53),
                hover: rgb(66, 66, 66),
                line: rgb(68, 68, 68),
                error: rgb(255, 153, 164),
                success: rgb(108, 203, 95),
            }
        } else {
            Self {
                dark,
                background: rgb(243, 243, 243),
                surface: rgb(255, 255, 255),
                text: rgb(26, 26, 26),
                secondary: rgb(108, 108, 108),
                accent: rgb(0, 103, 192),
                on_accent: Color32::WHITE,
                subtle: rgb(248, 248, 248),
                hover: rgb(235, 239, 244),
                line: rgb(228, 228, 228),
                error: rgb(196, 43, 28),
                success: rgb(15, 123, 15),
            }
        }
    }
    pub fn resolve(mode: ThemeMode, system_dark: bool) -> Self {
        Self::new(match mode {
            ThemeMode::System => system_dark,
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
        })
    }
    pub fn apply(self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(Id::new("fluent-theme"), self));
        let mut style = (*ctx.style()).clone();
        style.visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        style.visuals.panel_fill = self.background;
        style.visuals.window_fill = self.surface;
        style.visuals.window_corner_radius = egui::CornerRadius::same(12);
        style.visuals.override_text_color = Some(self.text);
        style.visuals.selection.bg_fill = self.accent;
        style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, self.on_accent);
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
            widget.bg_stroke = egui::Stroke::new(1.0_f32, self.line);
            widget.fg_stroke = egui::Stroke::new(1.0_f32, self.text);
        }
        style.visuals.widgets.inactive.bg_fill = self.subtle;
        style.visuals.widgets.inactive.weak_bg_fill = self.subtle;
        style.visuals.widgets.hovered.bg_fill = self.hover;
        style.visuals.widgets.hovered.weak_bg_fill = self.hover;
        style.visuals.widgets.active.bg_fill = self.hover;
        style.visuals.widgets.active.weak_bg_fill = self.hover;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, self.line);
        style.spacing.item_spacing = egui::vec2(12.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        ctx.set_style(style);
    }
    pub fn get(ctx: &egui::Context) -> Self {
        ctx.data(|d| d.get_temp(Id::new("fluent-theme")))
            .unwrap_or_default()
    }
    pub fn tint(self, color: Color32, alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_mode_resolves_both_modes_and_overrides_remain_stable() {
        assert!(!Theme::resolve(ThemeMode::System, false).dark);
        assert!(Theme::resolve(ThemeMode::System, true).dark);
        assert!(!Theme::resolve(ThemeMode::Light, true).dark);
        assert!(Theme::resolve(ThemeMode::Dark, false).dark);
        assert_ne!(Theme::new(false).text, Theme::new(true).text);
    }
}
