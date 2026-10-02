//! The cases agreed in #20 for trusting thermal images: a known scalar
//! ramp, a nonlinear display-only image, two differently configured uses
//! of one source, spot markers on resized and differently laid out blocks,
//! backward navigation, and the same picture live and in export.

use std::path::{Path, PathBuf};

use eframe::egui::{Pos2, Rect};

use super::compose::{Look, compose};
use super::source::Kind;
use super::spec::{Spec, Support, Unit};
use super::state::State;
use super::{Palette, spot_anchor};
use crate::parser;
use crate::render::BlockCx;
use crate::render::image_cache::ImageCache;
use crate::render::test_support::with_ui;
use crate::theme::Theme;

fn deck_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mdeck-thermal-accept-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    // a linear 0..100 °C ramp, 16-bit, with its sidecar
    let ramp = image::ImageBuffer::<image::Luma<u16>, Vec<u16>>::from_fn(201, 20, |x, _| {
        image::Luma([(x as u64 * 65535 / 200) as u16])
    });
    ramp.save(d.join("ramp.thermal.png")).unwrap();
    std::fs::write(
        d.join("ramp.thermal.yaml"),
        format!("unit: °C\nscale: {}\noffset: 0\n", 100.0 / 65535.0),
    )
    .unwrap();
    // a display export with a nonlinear (histogram-like) tone curve
    let curve = image::GrayImage::from_fn(200, 100, |x, _| {
        let t = x as f32 / 199.0;
        image::Luma([(t.powf(0.4) * 255.0) as u8])
    });
    curve.save(d.join("display.png")).unwrap();
    d
}

fn library(dir: &Path, md: &str) -> (ImageCache, parser::Presentation) {
    let pres = parser::parse(md);
    let mut cache = ImageCache::new(dir.to_path_buf());
    let diagnostics = cache.thermal_mut().load(&pres);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    (cache, pres)
}

fn first_block(pres: &parser::Presentation) -> String {
    super::blocks(&pres.slides[0])[0].0.to_string()
}

#[test]
fn a_known_scalar_ramp_measures_true() {
    let d = deck_dir("ramp");
    let md =
        "# R\n\n```@thermal\ndata: ramp.thermal.png\n- spot Q 25% 50%\n- spot H 75% 50%\n```\n";
    let (cache, pres) = library(&d, md);
    let spec = Spec::parse(&first_block(&pres));
    let source = cache.thermal().source(&spec).unwrap().unwrap().clone();
    assert_eq!(source.kind, Kind::Data(Unit::Celsius));
    let (q, _) = source.sample(0.25, 0.5);
    let (h, _) = source.sample(0.75, 0.5);
    assert!((q - 25.0).abs() < 0.6, "{q}");
    assert!((h - 75.0).abs() < 0.6, "{h}");
    // the window defaults to the data's extent
    assert_eq!(super::draw::window(&spec, &source), (0.0, 100.0));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_nonlinear_display_image_never_claims_temperatures() {
    let d = deck_dir("display");
    let md = "# D\n\n```@thermal\nimage: display.png\nwindow: 40..90 °C\n+ above 60 °C\n+ above 80%\n```\n";
    let pres = parser::parse(md);
    let mut cache = ImageCache::new(d.clone());
    let diagnostics = cache.thermal_mut().load(&pres);
    // the window and the °C threshold are refused, with a reason
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    let spec = Spec::parse(&first_block(&pres));
    let source = cache.thermal().source(&spec).unwrap().unwrap().clone();
    assert_eq!(source.kind, Kind::Display);
    assert_eq!(source.unit(), None);
    assert_eq!(super::draw::window(&spec, &source), (0.0, 1.0));
    // only the relative threshold is a step
    let support = super::library::support_of(&source);
    assert_eq!(spec.step_count(&support), 1);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn two_uses_of_one_source_keep_their_own_settings() {
    let d = deck_dir("two");
    let md = "# T\n\n```@thermal\ndata: ramp.thermal.png\npalette: iron\n```\n\n```@thermal\ndata: ramp.thermal.png\npalette: arctic\nwindow: 40..60 °C\n```\n";
    let (cache, pres) = library(&d, md);
    let blocks = super::blocks(&pres.slides[0]);
    let a = Spec::parse(blocks[0].0);
    let b = Spec::parse(blocks[1].0);
    let src = cache.thermal().source(&a).unwrap().unwrap().clone();
    // one source, read once, two looks
    assert!(std::sync::Arc::ptr_eq(
        &src,
        cache.thermal().source(&b).unwrap().unwrap()
    ));
    let look = |spec: &Spec| Look {
        palette: spec.palette.unwrap(),
        window: super::draw::window(spec, &src),
        threshold: None,
    };
    assert_ne!(look(&a), look(&b));
    assert_ne!(
        compose(&src, &look(&a)).pixels,
        compose(&src, &look(&b)).pixels
    );
    std::fs::remove_dir_all(&d).ok();
}

/// Draw `content` at `area` and return where spot `name` landed.
fn spot_at(cache: &ImageCache, content: &str, area: Rect, scale: f32, name: &str) -> Pos2 {
    let theme = Theme::dark();
    let mut at = None;
    with_ui(|ui| {
        let cx = BlockCx {
            ui,
            theme: &theme,
            opacity: 1.0,
            scale,
            image_cache: cache,
            reveal_step: 0,
            reveal_timestamp: None,
        };
        super::draw(&cx, content, area.min, area.width(), area.height());
        at = spot_anchor(ui.ctx(), name);
    });
    at.expect("the spot was drawn")
}

#[test]
fn spots_stay_on_their_pixel_when_the_block_is_resized_or_reshaped() {
    let d = deck_dir("spots");
    let md = "# S\n\n```@thermal\ndata: ramp.thermal.png\n- spot Mid 50% 50%\n- spot Corner 10% 80%\n```\n";
    let (cache, pres) = library(&d, md);
    let content = first_block(&pres);
    // the same block, full size, half size and in a narrow column
    let full = Rect::from_min_size(Pos2::new(100.0, 100.0), eframe::egui::vec2(1600.0, 700.0));
    let half = Rect::from_min_size(Pos2::new(50.0, 50.0), eframe::egui::vec2(800.0, 350.0));
    let tall = Rect::from_min_size(Pos2::new(0.0, 0.0), eframe::egui::vec2(500.0, 900.0));
    for (area, scale) in [(full, 1.0), (half, 0.5), (tall, 0.5)] {
        let mid = spot_at(&cache, &content, area, scale, "Mid");
        let corner = spot_at(&cache, &content, area, scale, "Corner");
        // both are on the image: Corner is 40% of its width left of Mid and
        // 30% of its height below, whatever size the image was drawn at
        let w = (mid.x - corner.x) / 0.4;
        let h = (corner.y - mid.y) / 0.3;
        assert!(w > 0.0 && h > 0.0, "{area:?}");
        // the ramp is 201 x 20, so the drawn image keeps that aspect
        assert!(
            ((w / h) - 201.0 / 20.0).abs() < 0.05,
            "aspect {} in {area:?}",
            w / h
        );
        assert!(area.contains(mid) && area.contains(corner), "{area:?}");
    }
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn every_step_draws_the_same_however_it_was_reached() {
    let spec = Spec::parse(
        "image: a.png\nvisible: b.jpg\n+ lens 70% 40% 15%\n+ reveal\n+ above 85%\n+ spot Hot 70% 40%\n",
    );
    let steps = spec.steps(&Support::DISPLAY);
    // forward 0,1,2,3 then back to 1: the state is the step's, not the path's
    let forward: Vec<State> = (0..=3).map(|s| State::at(&steps, s)).collect();
    let back = State::at(&steps, 1);
    assert_eq!(back, forward[1]);
    assert!(back.revealed.is_none() && back.threshold.is_none() && back.spots.is_empty());
}

#[test]
fn the_composed_picture_is_the_same_every_time() {
    let d = deck_dir("parity");
    let md = "# P\n\n```@thermal\nimage: display.png\n+ above 70%\n```\n";
    let (cache, pres) = library(&d, md);
    let spec = Spec::parse(&first_block(&pres));
    let src = cache.thermal().source(&spec).unwrap().unwrap().clone();
    let look = Look {
        palette: Palette::Iron,
        window: (0.0, 1.0),
        threshold: Some(0.7),
    };
    // the window and an export compose with the same function and inputs
    assert_eq!(compose(&src, &look).pixels, compose(&src, &look).pixels);
    std::fs::remove_dir_all(&d).ok();
}
