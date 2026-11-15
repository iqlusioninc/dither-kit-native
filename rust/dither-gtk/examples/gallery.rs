use dither_gtk::{
    gtk::{self, glib, prelude::*},
    ir::*,
    *,
};
use std::{
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};

fn panel(title: &str, child: &impl IsA<gtk::Widget>) -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.add_css_class("card");
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("heading");
    panel.append(&label);
    panel.append(child);
    panel
}

/// A real allocation must separate every chart/radar axis label before a capture
/// can pass. Merely realizing GtkFixed leaves its children at their initial origin.
fn label_layouts(
    widget: &gtk::Widget,
    layouts: &mut Vec<Vec<gtk::graphene::Rect>>,
) -> Result<(), String> {
    if let Some(fixed) = widget.downcast_ref::<gtk::Fixed>() {
        let mut bounds = vec![];
        let mut child = fixed.first_child();
        while let Some(label) = child {
            if let Some(rect) = label.compute_bounds(fixed) {
                if rect.width() <= 0.0 || rect.height() <= 0.0 {
                    return Err("axis label has no allocation".into());
                }
                bounds.push(rect);
            } else {
                return Err("axis label bounds unavailable".into());
            }
            child = label.next_sibling();
        }
        if bounds.len() > 1 {
            for (i, a) in bounds.iter().enumerate() {
                for b in &bounds[i + 1..] {
                    if (a.x() - b.x()).abs() < 8.0 && (a.y() - b.y()).abs() < 8.0 {
                        return Err(format!("axis labels overlap at ({}, {})", a.x(), a.y()));
                    }
                }
            }
            layouts.push(bounds);
        }
    }
    let mut child = widget.first_child();
    while let Some(next) = child {
        label_layouts(&next, layouts)?;
        child = next.next_sibling();
    }
    Ok(())
}

fn capture_when_ready(
    app: &gtk::Application,
    window: &gtk::ApplicationWindow,
    scroll: &gtk::ScrolledWindow,
    surfaces: Vec<DitherSurface>,
    path: String,
) {
    let (app, window, scroll) = (app.clone(), window.clone(), scroll.clone());
    let start = Instant::now();
    let stable = Cell::new(0u8);
    let previous = RefCell::new(Vec::new());
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let mut failure = None;
        let mut sizes = vec![];
        for surface in &surfaces {
            let area = surface.widget();
            if let Some(error) = area.error() {
                eprintln!("GLArea failed: {error}");
                std::process::exit(1);
            }
            sizes.push((area.width(), area.height(), surface.rendered_frames()));
            if !area.is_realized()
                || area.width() <= 0
                || area.height() <= 0
                || surface.rendered_frames() < 2
            {
                failure = Some("waiting for two successfully rendered frames per surface".into());
                surface.queue_render();
            }
        }
        let mut layouts = vec![];
        if let Err(error) = label_layouts(scroll.upcast_ref(), &mut layouts) {
            failure = Some(error);
        }
        if layouts.len() != 7 {
            failure.get_or_insert_with(|| {
                format!(
                    "waiting for seven allocated axis groups, got {}",
                    layouts.len()
                )
            });
        }
        let dimensions = sizes.iter().map(|&(w, h, _)| (w, h)).collect::<Vec<_>>();
        if *previous.borrow() != dimensions {
            *previous.borrow_mut() = dimensions;
            failure.get_or_insert("surface allocations are still changing".into());
        }
        if failure.is_none() {
            stable.set(stable.get() + 1);
        } else {
            stable.set(0);
        }
        if stable.get() < 2 {
            if start.elapsed() > Duration::from_secs(60) {
                eprintln!(
                    "Gallery readiness failed: {}; surfaces {sizes:?}",
                    failure.unwrap_or_else(|| "no stable allocated frames".into())
                );
                std::process::exit(1);
            }
            return glib::ControlFlow::Continue;
        }
        // The six chart surfaces must contain real geometry, not their empty
        // entrance frame. Check GPU readback before accepting the screenshot.
        for (index, surface) in surfaces.iter().enumerate() {
            let (w, h, pixels) = surface.capture_grid().expect("read a completed grid");
            let lit = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| pixel[3] > 0)
                .count();
            let colored = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| {
                    p[3] > 0
                        && p[..3]
                            .iter()
                            .max()
                            .unwrap()
                            .abs_diff(*p[..3].iter().min().unwrap())
                            > 10
                })
                .count();
            if (2..=7).contains(&index) && colored * 100 < w * h * 2 {
                eprintln!(
                    "Gallery surface {index} has only {colored}/{} colored geometry pixels",
                    w * h
                );
                std::process::exit(1);
            }
            eprintln!(
                "Surface {index}: {w}×{h}, {lit} populated / {colored} colored pixels, {} frames",
                surface.rendered_frames()
            );
        }
        eprintln!(
            "Verified {} allocated axis groups after {:?}",
            layouts.len(),
            start.elapsed()
        );
        let snapshot = gtk::Snapshot::new();
        window.snapshot_child(&scroll, &snapshot);
        let node = snapshot.to_node().expect("gallery produced render nodes");
        let renderer = window.renderer().expect("native GSK renderer");
        let texture = renderer.render_texture(&node, None);
        texture
            .save_to_png(&path)
            .expect("write gallery screenshot");
        eprintln!("Wrote {path} ({}×{})", texture.width(), texture.height());
        app.quit();
        glib::ControlFlow::Break
    });
}

fn activate(app: &gtk::Application, screenshot: Option<String>) {
    // Cold Mesa compilation can take longer than the entrance animation. Smoke
    // captures use the same reduced-motion path as accessibility settings.
    if screenshot.is_some() {
        gtk::Settings::default()
            .expect("GTK settings")
            .set_gtk_enable_animations(false);
    }
    let css = gtk::CssProvider::new();
    css.load_from_data("window, .gallery { background: #10151d; color: #edf3ff; } .card { background: #192230; border-radius: 12px; padding: 14px; } .heading { font-weight: bold; font-size: 16px; } .tooltip { background: #080d14; color: white; padding: 6px; border-radius: 4px; } button { color: white; background: #29374b; background-image: none; } label { color: #edf3ff; }");
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Dither Kit · GTK4")
        .default_width(1120)
        .default_height(1050)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let title = gtk::Label::new(Some("DITHER KIT / Native GTK4"));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    content.append(&title);
    let subtitle = gtk::Label::new(Some(
        "Shader pixels. Native controls. Hover charts and toggle their legends.",
    ));
    subtitle.set_xalign(0.0);
    content.append(&subtitle);
    let mut surfaces = vec![];
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_homogeneous(true);
    let gradient = DitherGradient::new(Gradient {
        to: Some(Tone::Pink),
        direction: Direction::Right,
        ..Gradient::new(Tone::Blue)
    });
    gradient.widget().set_size_request(240, 90);
    surfaces.push(gradient.surface().clone());
    row.append(&panel("Gradient", gradient.widget()));
    let background = DitherBackground::new(Background::new(Field::Aurora, Tone::Green));
    background.widget().set_size_request(240, 90);
    surfaces.push(background.surface().clone());
    row.append(&panel("Animated background", background.widget()));
    content.append(&row);
    let series = vec![
        Series::new(
            "Requests",
            Tone::Green,
            vec![12., 23., 18., 37., 28., 46., 39.],
        ),
        Series::new(
            "Responses",
            Tone::Blue,
            vec![8., 12., 15., 24., 18., 30., 34.],
        )
        .fill(Fill::Hatched),
    ];
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_homogeneous(true);
    for (title, kind) in [
        ("Area", CartesianKind::Area),
        ("Line", CartesianKind::Line),
        ("Bar", CartesianKind::Bar),
    ] {
        let mut options = ChartOptions::new(kind, series.clone());
        options.x_labels = vec!["Mon".into(), "Thu".into(), "Sun".into()];
        let chart = DitherChart::new(options);
        surfaces.push(chart.surface().clone());
        row.append(&panel(title, chart.widget()));
    }
    content.append(&row);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_homogeneous(true);
    let pie = DitherPie::new(
        vec![
            Slice::new("Compute", 42., Tone::Green),
            Slice::new("Storage", 33., Tone::Blue),
            Slice::new("Network", 25., Tone::Pink),
        ],
        0.5,
    );
    surfaces.push(pie.surface().clone());
    row.append(&panel("Pie", pie.widget()));
    let radar = DitherRadar::new(
        vec![
            Series::new("Current", Tone::Purple, vec![8., 6., 9., 5., 7.]),
            Series::new("Target", Tone::Blue, vec![6., 8., 7., 9., 5.]),
        ],
        vec![
            "Speed".into(),
            "Quality".into(),
            "Reach".into(),
            "Cost".into(),
            "Trust".into(),
        ],
    );
    surfaces.push(radar.surface().clone());
    row.append(&panel("Radar", radar.widget()));
    let small = gtk::Box::new(gtk::Orientation::Vertical, 16);
    let spark = DitherSparkline::new(vec![4., 8., 5., 12., 9., 16., 14.], Tone::Green);
    spark.widget().set_size_request(220, 60);
    surfaces.push(spark.surface().clone());
    small.append(&panel("Sparkline", spark.widget()));
    let meter = DitherMeter::new(0.42, Tone::Blue, Fill::Gradient);
    surfaces.push(meter.surface().clone());
    small.append(&panel("Spring meter", meter.widget()));
    let avatars = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    for name in ["Dither", "Native", "GTK"] {
        let avatar = DitherAvatar::new(name, 48, None);
        surfaces.push(avatar.surface().clone());
        avatars.append(avatar.widget());
    }
    small.append(&panel("Avatars", &avatars));
    row.append(&small);
    content.append(&row);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    buttons.set_homogeneous(true);
    for (name, style) in [
        ("Solid", ButtonStyle::Solid),
        ("Outline", ButtonStyle::Outline),
        ("Ghost", ButtonStyle::Ghost),
    ] {
        let button = DitherButton::new(name, Tone::Green, style);
        surfaces.push(button.surface().clone());
        let meter = meter.clone();
        let next = Cell::new(false);
        button.widget().connect_clicked(move |_| {
            let high = !next.get();
            next.set(high);
            meter.set_value(if high { 0.88 } else { 0.42 });
        });
        buttons.append(button.widget());
    }
    content.append(&panel("Buttons · click to update meter", &buttons));
    let scroll = gtk::ScrolledWindow::builder()
        .child(&content)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    scroll.add_css_class("gallery");
    window.set_child(Some(&scroll));
    window.present();
    if let Some(path) = screenshot {
        capture_when_ready(app, &window, &scroll, surfaces, path);
    }
}

fn main() -> glib::ExitCode {
    let args = std::env::args().collect::<Vec<_>>();
    let screenshot = args.iter().position(|a| a == "--smoke").map(|i| {
        args.get(i + 1)
            .cloned()
            .unwrap_or_else(|| "gallery.png".into())
    });
    let app = gtk::Application::builder()
        .application_id("org.ditherkit.Gallery")
        .build();
    app.connect_activate(move |app| activate(app, screenshot.clone()));
    app.run_with_args(&["gallery"])
}
