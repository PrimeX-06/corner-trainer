#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cube;
mod digits;
mod model;
mod native;
mod theme;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("三色定则 · Corner Trainer")
            .with_inner_size([1280.0, 860.0])
            .with_min_inner_size([1100.0, 740.0])
            .with_transparent(true)
            .with_icon(icon()),
        multisampling: 4,
        ..Default::default()
    };
    eframe::run_native(
        "Corner Trainer",
        options,
        Box::new(|cc| Ok(Box::new(app::Trainer::new(cc)))),
    )
}

fn icon() -> egui::IconData {
    let size = 64;
    let mut rgba = vec![0; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let i = (y * size + x) * 4;
            let diagonal = ((12..30).contains(&x) && (12..30).contains(&y))
                || ((34..52).contains(&x) && (34..52).contains(&y));
            let off_diagonal = ((34..52).contains(&x) && (12..30).contains(&y))
                || ((12..30).contains(&x) && (34..52).contains(&y));
            let c = if diagonal {
                [232, 240, 255, 255]
            } else if off_diagonal {
                [116, 164, 255, 255]
            } else {
                [43, 102, 232, 255]
            };
            rgba[i..i + 4].copy_from_slice(&c);
        }
    }
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
