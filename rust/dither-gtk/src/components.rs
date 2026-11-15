//! Native GTK chrome around shader surfaces. Each wrapper exposes its native widget.
use crate::DitherSurface;
use dither_ir::*;
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn animations(widget: &impl IsA<gtk::Widget>) -> bool {
    widget.settings().is_gtk_enable_animations()
}
fn entrance(surface: &DitherSurface, progress: Rc<Cell<f32>>) {
    if !animations(surface.widget()) {
        progress.set(1.0);
        return;
    }
    progress.set(0.0);
    let start = Cell::new(None);
    surface.widget().add_tick_callback(move |area, clock| {
        let now = clock.frame_time();
        let first = start.get().unwrap_or(now);
        start.set(Some(first));
        let t = if animations(area) {
            ((now - first) as f32 / 900_000.0).clamp(0.0, 1.0)
        } else {
            1.0
        };
        progress.set(1.0 - (1.0 - t).powi(3));
        area.queue_render();
        if t >= 1.0 {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}
fn motion(surface: &DitherSurface, change: impl Fn(bool, f64, f64) + 'static) {
    let controller = gtk::EventControllerMotion::new();
    let change = Rc::new(change);
    let f = change.clone();
    controller.connect_motion(move |_, x, y| f(true, x, y));
    controller.connect_leave(move |_| change(false, 0.0, 0.0));
    surface.widget().add_controller(controller);
}
fn tooltip() -> gtk::Label {
    let label = gtk::Label::builder()
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .visible(false)
        .can_target(false)
        .build();
    label.add_css_class("tooltip");
    label
}
fn position_tooltip(label: &gtk::Label, surface: &DitherSurface, x: f64, y: f64) {
    let width = label.measure(gtk::Orientation::Horizontal, -1).1;
    let height = label.measure(gtk::Orientation::Vertical, width).1;
    label.set_margin_start((x as i32 + 10).clamp(0, (surface.widget().width() - width).max(0)));
    label.set_margin_top((y as i32 + 10).clamp(0, (surface.widget().height() - height).max(0)));
}

#[derive(Clone)]
pub struct DitherGradient {
    surface: DitherSurface,
}
impl DitherGradient {
    pub fn new(options: Gradient) -> Self {
        let surface = DitherSurface::new(3, move |w, h| options.encode(w, h));
        surface.set_accessible_label("Dither gradient");
        Self { surface }
    }
    pub fn widget(&self) -> &gtk::GLArea {
        self.surface.widget()
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
    pub fn set_options(&self, options: Gradient) {
        self.surface.set_encoder(move |w, h| options.encode(w, h));
    }
}
#[derive(Clone)]
pub struct DitherBackground {
    surface: DitherSurface,
    speed: Rc<Cell<f32>>,
}
impl DitherBackground {
    pub fn new(options: Background) -> Self {
        let time = Rc::new(Cell::new(0.0));
        let t = time.clone();
        let surface = DitherSurface::new(3, move |w, h| options.encode(w, h, t.get()));
        surface.set_accessible_label("Animated dither background");
        let speed = Rc::new(Cell::new(1.0));
        let s = speed.clone();
        let previous = Cell::new(None);
        surface.widget().add_tick_callback(move |area, clock| {
            let now = clock.frame_time();
            let prev = previous.replace(Some(now)).unwrap_or(now);
            if animations(area) && s.get() != 0.0 {
                time.set(time.get() + (now - prev) as f32 / 1_000_000.0 * s.get());
                area.queue_render();
            }
            glib::ControlFlow::Continue
        });
        Self { surface, speed }
    }
    pub fn set_speed(&self, speed: f32) {
        self.speed.set(if speed.is_finite() { speed } else { 0.0 });
    }
    pub fn widget(&self) -> &gtk::GLArea {
        self.surface.widget()
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}
#[derive(Clone)]
pub struct DitherAvatar {
    surface: DitherSurface,
}
impl DitherAvatar {
    pub fn new(name: &str, size: i32, hue: Option<f32>) -> Self {
        let name = name.to_owned();
        let label = format!("Avatar for {name}");
        let surface = DitherSurface::new((size / 8).max(1), move |_, _| avatar(&name, hue));
        surface.widget().set_size_request(size, size);
        surface.widget().set_hexpand(false);
        surface.widget().set_vexpand(false);
        surface.set_accessible_label(&label);
        Self { surface }
    }
    pub fn widget(&self) -> &gtk::GLArea {
        self.surface.widget()
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}
#[derive(Clone)]
pub struct DitherSwatch {
    surface: DitherSurface,
}
impl DitherSwatch {
    pub fn new(tone: Tone) -> Self {
        let surface = DitherSurface::new(3, move |_, _| swatch(tone));
        surface.widget().set_size_request(18, 18);
        surface.widget().set_hexpand(false);
        surface.widget().set_vexpand(false);
        surface.set_accessible_label("Series color swatch");
        Self { surface }
    }
    pub fn widget(&self) -> &gtk::GLArea {
        self.surface.widget()
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}

#[derive(Clone)]
pub struct DitherLegend {
    widget: gtk::Box,
}
impl DitherLegend {
    /// Active toggles denote visible series. Pointer/focus selection emphasizes a series.
    pub fn new(
        series: &[Series],
        changed: impl Fn(usize, bool) + 'static,
        selected: impl Fn(Option<usize>) + 'static,
    ) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        widget.set_halign(gtk::Align::Center);
        let changed = Rc::new(changed);
        let selected = Rc::new(selected);
        for (index, series) in series.iter().enumerate() {
            let button = gtk::ToggleButton::builder().active(!series.hidden).build();
            button.set_tooltip_text(Some("Toggle series visibility"));
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            row.append(DitherSwatch::new(series.tone).widget());
            row.append(&gtk::Label::new(Some(&series.label)));
            button.set_child(Some(&row));
            button.update_property(&[gtk::accessible::Property::Label(&series.label)]);
            let f = changed.clone();
            button.connect_toggled(move |b| f(index, b.is_active()));
            let controller = gtk::EventControllerMotion::new();
            let f = selected.clone();
            controller.connect_enter(move |_, _, _| f(Some(index)));
            let f = selected.clone();
            controller.connect_leave(move |_| f(None));
            button.add_controller(controller);
            let controller = gtk::EventControllerFocus::new();
            let f = selected.clone();
            controller.connect_enter(move |_| f(Some(index)));
            let f = selected.clone();
            controller.connect_leave(move |_| f(None));
            button.add_controller(controller);
            widget.append(&button);
        }
        Self { widget }
    }
    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}

#[derive(Clone)]
pub struct ChartOptions {
    pub kind: CartesianKind,
    pub series: Vec<Series>,
    pub stacked: bool,
    pub bloom: Bloom,
    pub y_max: Option<f64>,
    pub grid_lines: bool,
    pub tick_count: usize,
    pub x_labels: Vec<String>,
}
impl ChartOptions {
    pub fn new(kind: CartesianKind, series: Vec<Series>) -> Self {
        Self {
            kind,
            series,
            stacked: false,
            bloom: Bloom::Low,
            y_max: None,
            grid_lines: true,
            tick_count: 4,
            x_labels: vec![],
        }
    }
    fn encoder(
        &self,
        progress: f32,
        hover: Option<usize>,
        selected: Option<usize>,
    ) -> Cartesian<'_> {
        let mut c = Cartesian::new(self.kind, &self.series);
        c.stacked = self.stacked;
        c.bloom = self.bloom;
        c.y_max = self.y_max;
        c.grid_lines = self.grid_lines;
        c.tick_count = self.tick_count;
        c.progress = progress;
        c.hover = hover;
        c.selected = selected;
        c
    }
}
#[derive(Clone)]
pub struct DitherChart {
    widget: gtk::Box,
    surface: DitherSurface,
}
impl DitherChart {
    pub fn new(options: ChartOptions) -> Self {
        let options = Rc::new(RefCell::new(options));
        let progress = Rc::new(Cell::new(1.0));
        let hover = Rc::new(Cell::new(None));
        let selected = Rc::new(Cell::new(None));
        let (o, p, h, s) = (
            options.clone(),
            progress.clone(),
            hover.clone(),
            selected.clone(),
        );
        let surface = DitherSurface::new(3, move |w, hg| {
            o.borrow().encoder(p.get(), h.get(), s.get()).encode(w, hg)
        });
        surface.set_accessible_label("Interactive chart; use legend buttons to toggle series");
        surface.widget().set_size_request(240, 150);
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let yaxis = gtk::Fixed::new();
        yaxis.set_overflow(gtk::Overflow::Visible);
        let ycontainer = gtk::Overlay::new();
        ycontainer.set_size_request(64, 1);
        ycontainer.set_vexpand(true);
        ycontainer.set_overflow(gtk::Overflow::Visible);
        ycontainer.add_overlay(&yaxis);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(surface.widget()));
        let tip = tooltip();
        overlay.add_overlay(&tip);
        row.append(&ycontainer);
        row.append(&overlay);
        widget.append(&row);
        let xaxis = gtk::Fixed::new();
        xaxis.set_overflow(gtk::Overflow::Visible);
        let xcontainer = gtk::Overlay::new();
        xcontainer.set_size_request(1, 24);
        xcontainer.set_margin_start(72);
        xcontainer.set_hexpand(true);
        xcontainer.set_overflow(gtk::Overflow::Visible);
        xcontainer.add_overlay(&xaxis);
        widget.append(&xcontainer);
        let refresh: Rc<dyn Fn()> = {
            let (o, y, x, sf) = (
                options.clone(),
                yaxis.clone(),
                xaxis.clone(),
                surface.downgrade(),
            );
            Rc::new(move || {
                let Some(sf) = sf.upgrade() else {
                    return;
                };
                while let Some(child) = y.first_child() {
                    y.remove(&child);
                }
                while let Some(child) = x.first_child() {
                    x.remove(&child);
                }
                let o = o.borrow();
                let chart = o.encoder(1.0, None, None);
                let scale = chart.scale();
                let cell = sf.cell();
                let grid_w = (sf.widget().width() / cell).max(1) as usize;
                let grid_h = (sf.widget().height() / cell).max(1) as usize;
                for tick in &scale.ticks {
                    let label = gtk::Label::new(Some(&format_value(*tick)));
                    let width = label.measure(gtk::Orientation::Horizontal, -1).1;
                    let height = label.measure(gtk::Orientation::Vertical, width).1;
                    y.put(
                        &label,
                        (64 - width).max(0) as f64,
                        scale.y(*tick, grid_h) as f64 * cell as f64 - height as f64 / 2.0,
                    );
                }
                for (i, text) in o.x_labels.iter().enumerate() {
                    let label = gtk::Label::new(Some(text));
                    let width = label.measure(gtk::Orientation::Horizontal, -1).1;
                    let index = if o.x_labels.len() > 1 {
                        i * (chart.len().saturating_sub(1)) / (o.x_labels.len() - 1)
                    } else {
                        0
                    };
                    x.put(
                        &label,
                        chart.column_x(index, grid_w) as f64 * cell as f64 - width as f64 / 2.0,
                        0.0,
                    );
                }
            })
        };
        refresh();
        let r = refresh.clone();
        surface.widget().connect_resize(move |_, _, _| {
            let r = r.clone();
            glib::idle_add_local_full(glib::Priority::DEFAULT, move || {
                r();
                glib::ControlFlow::Break
            });
        });
        let (o, h, sf) = (options.clone(), hover.clone(), surface.downgrade());
        motion(&surface, move |inside, x, y| {
            let Some(sf) = sf.upgrade() else {
                return;
            };
            let o = o.borrow();
            let grid = (sf.widget().width() / sf.cell()).max(1) as usize;
            let index = if inside {
                o.encoder(1.0, None, None)
                    .index_at(x as f32 / sf.cell() as f32, grid)
            } else {
                None
            };
            h.set(index);
            if let Some(i) = index {
                let text = o
                    .series
                    .iter()
                    .filter(|s| !s.hidden)
                    .filter_map(|s| {
                        s.values
                            .get(i)
                            .map(|v| format!("{}: {}", s.label, format_value(*v)))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                tip.set_text(&text);
                position_tooltip(&tip, &sf, x, y);
                tip.set_visible(true);
            } else {
                tip.set_visible(false);
            }
            sf.queue_render();
        });
        let (o, sf, r) = (options.clone(), surface.clone(), refresh.clone());
        let sf2 = surface.clone();
        let legend = DitherLegend::new(
            &options.borrow().series,
            move |i, visible| {
                o.borrow_mut().series[i].hidden = !visible;
                r();
                sf.queue_render();
            },
            move |i| {
                selected.set(i);
                sf2.queue_render();
            },
        );
        widget.append(legend.widget());
        entrance(&surface, progress);
        Self { widget, surface }
    }
    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}

#[derive(Clone)]
pub struct DitherSparkline {
    surface: DitherSurface,
}
impl DitherSparkline {
    pub fn new(values: Vec<f64>, tone: Tone) -> Self {
        let hovered = Rc::new(Cell::new(false));
        let progress = Rc::new(Cell::new(1.0));
        let (h, p) = (hovered.clone(), progress.clone());
        let surface = DitherSurface::new(3, move |w, hg| {
            sparkline(&values, tone, w, hg, h.get(), p.get())
        });
        surface.set_accessible_label("Sparkline; hover for highlighted values");
        let sf = surface.downgrade();
        motion(&surface, move |inside, _, _| {
            let Some(sf) = sf.upgrade() else {
                return;
            };
            hovered.set(inside);
            sf.queue_render();
        });
        entrance(&surface, progress);
        Self { surface }
    }
    pub fn widget(&self) -> &gtk::GLArea {
        self.surface.widget()
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}

#[derive(Clone)]
pub struct DitherPie {
    widget: gtk::Overlay,
    surface: DitherSurface,
}
impl DitherPie {
    pub fn new(slices: Vec<Slice>, inner_radius: f32) -> Self {
        let slices = Rc::new(slices);
        let hover = Rc::new(Cell::new(None));
        let progress = Rc::new(Cell::new(1.0));
        let (s, h, p) = (slices.clone(), hover.clone(), progress.clone());
        let surface = DitherSurface::new(3, move |w, hg| {
            let mut pie = Pie::new(&s);
            pie.inner_radius = inner_radius;
            pie.hover = h.get();
            pie.progress = p.get();
            pie.bloom = Bloom::Low;
            pie.encode(w, hg)
        });
        surface.set_accessible_label("Pie chart");
        surface.widget().set_size_request(180, 180);
        let widget = gtk::Overlay::new();
        widget.set_child(Some(surface.widget()));
        let total: f64 = slices.iter().map(|s| s.value.max(0.0)).sum();
        let center = gtk::Label::new(Some(&format_value(total)));
        center.set_halign(gtk::Align::Center);
        center.set_valign(gtk::Align::Center);
        center.set_can_target(false);
        center.set_visible(inner_radius > 0.0);
        widget.add_overlay(&center);
        let tip = tooltip();
        widget.add_overlay(&tip);
        let sf = surface.downgrade();
        motion(&surface, move |inside, x, y| {
            let Some(sf) = sf.upgrade() else {
                return;
            };
            let mut pie = Pie::new(&slices);
            pie.inner_radius = inner_radius;
            let geometry = pie.geometry(
                (sf.widget().width() / sf.cell()).max(1) as usize,
                (sf.widget().height() / sf.cell()).max(1) as usize,
            );
            let index = if inside {
                geometry.index_at(x as f32 / sf.cell() as f32, y as f32 / sf.cell() as f32)
            } else {
                None
            };
            hover.set(index);
            if let Some(i) = index {
                tip.set_text(&format!(
                    "{}: {}",
                    slices[i].label,
                    format_value(slices[i].value)
                ));
                position_tooltip(&tip, &sf, x, y);
                tip.set_visible(true);
            } else {
                tip.set_visible(false);
            }
            sf.queue_render();
        });
        entrance(&surface, progress);
        Self { widget, surface }
    }
    pub fn widget(&self) -> &gtk::Overlay {
        &self.widget
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}

#[derive(Clone)]
pub struct DitherRadar {
    widget: gtk::Box,
    surface: DitherSurface,
}
impl DitherRadar {
    pub fn new(series: Vec<Series>, axes: Vec<String>) -> Self {
        let series = Rc::new(RefCell::new(series));
        let progress = Rc::new(Cell::new(1.0));
        let selected = Rc::new(Cell::new(None));
        let (s, p, sel) = (series.clone(), progress.clone(), selected.clone());
        let surface = DitherSurface::new(3, move |w, h| {
            let data = s.borrow();
            let mut radar = Radar::new(&data);
            radar.progress = p.get();
            radar.selected = sel.get();
            radar.bloom = Bloom::Low;
            radar.encode(w, h)
        });
        surface.set_accessible_label("Radar chart");
        surface.widget().set_size_request(240, 240);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(surface.widget()));
        let labels = gtk::Fixed::new();
        labels.set_can_target(false);
        overlay.add_overlay(&labels);
        let labels_vec: Vec<_> = axes
            .iter()
            .map(|text| {
                let l = gtk::Label::new(Some(text));
                labels.put(&l, 0.0, 0.0);
                l
            })
            .collect();
        let s = series.clone();
        let sf = surface.downgrade();
        let layout: Rc<dyn Fn(i32, i32)> = Rc::new(move |_, _| {
            let Some(sf) = sf.upgrade() else {
                return;
            };
            let (w, h) = (sf.widget().width(), sf.widget().height());
            let data = s.borrow();
            let geometry = Radar::new(&data).geometry(
                (w / sf.cell()).max(1) as usize,
                (h / sf.cell()).max(1) as usize,
            );
            for (label, (x, y)) in labels_vec.iter().zip(geometry.axis_tips) {
                let width = label.measure(gtk::Orientation::Horizontal, -1).1;
                let height = label.measure(gtk::Orientation::Vertical, width).1;
                labels.move_(
                    label,
                    (x as f64 * sf.cell() as f64 - width as f64 / 2.0)
                        .clamp(0.0, (w - width).max(0) as f64),
                    (y as f64 * sf.cell() as f64 - height as f64 / 2.0)
                        .clamp(0.0, (h - height).max(0) as f64),
                );
            }
        });
        surface.widget().connect_resize(move |_, w, h| {
            let layout = layout.clone();
            glib::idle_add_local_full(glib::Priority::DEFAULT, move || {
                layout(w, h);
                glib::ControlFlow::Break
            });
        });
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 8);
        widget.append(&overlay);
        let (s, sf) = (series.clone(), surface.clone());
        let sf2 = surface.clone();
        let legend = DitherLegend::new(
            &series.borrow(),
            move |i, visible| {
                s.borrow_mut()[i].hidden = !visible;
                sf.queue_render();
            },
            move |i| {
                selected.set(i);
                sf2.queue_render();
            },
        );
        widget.append(legend.widget());
        entrance(&surface, progress);
        Self { widget, surface }
    }
    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}

#[derive(Clone)]
pub struct DitherMeter {
    widget: gtk::Overlay,
    surface: DitherSurface,
    target: Rc<Cell<f32>>,
    value: Rc<Cell<f32>>,
    running: Rc<Cell<bool>>,
    label: gtk::Label,
}
impl DitherMeter {
    pub fn new(value: f32, tone: Tone, fill: Fill) -> Self {
        let value = Rc::new(Cell::new(value.clamp(0.0, 1.0)));
        let target = Rc::new(Cell::new(value.get()));
        let v = value.clone();
        let surface = DitherSurface::new(3, move |w, h| meter(w, h, v.get(), tone, fill));
        surface.set_accessible_label(&format!("Meter {:.0} percent", value.get() * 100.0));
        surface.widget().set_size_request(180, 36);
        let widget = gtk::Overlay::new();
        widget.set_child(Some(surface.widget()));
        let label = gtk::Label::new(Some(&format!("{:.0}%", value.get() * 100.0)));
        label.set_can_target(false);
        widget.add_overlay(&label);
        Self {
            widget,
            surface,
            target,
            value,
            running: Rc::new(Cell::new(false)),
            label,
        }
    }
    pub fn set_value(&self, value: f32) {
        let target = if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.target.set(target);
        self.label.set_text(&format!("{:.0}%", target * 100.0));
        self.surface
            .set_accessible_label(&format!("Meter {:.0} percent", target * 100.0));
        if !animations(self.widget()) {
            self.value.set(target);
            self.surface.queue_render();
            return;
        }
        if self.running.replace(true) {
            return;
        }
        let (target, value, running) = (
            self.target.clone(),
            self.value.clone(),
            self.running.clone(),
        );
        let velocity = Cell::new(0.0f32);
        let previous = Cell::new(None);
        self.surface.widget().add_tick_callback(move |area, clock| {
            let now = clock.frame_time();
            let prev = previous.replace(Some(now)).unwrap_or(now - 16_667);
            let dt = ((now - prev) as f32 / 1_000_000.0).clamp(0.0, 0.033);
            let delta = target.get() - value.get();
            let speed = (velocity.get() + delta * 180.0 * dt) * (-20.0 * dt).exp();
            velocity.set(speed);
            value.set((value.get() + speed * dt).clamp(0.0, 1.0));
            area.queue_render();
            if !animations(area) || (delta.abs() < 0.001 && speed.abs() < 0.005) {
                value.set(target.get());
                running.set(false);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
    pub fn widget(&self) -> &gtk::Overlay {
        &self.widget
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}

#[derive(Clone)]
pub struct DitherButton {
    widget: gtk::Button,
    surface: DitherSurface,
}
impl DitherButton {
    pub fn new(label: &str, tone: Tone, style: ButtonStyle) -> Self {
        let hovered = Rc::new(Cell::new(false));
        let pressed = Rc::new(Cell::new(false));
        let time = Rc::new(Cell::new(0.0));
        let (h, p, t) = (hovered.clone(), pressed.clone(), time.clone());
        let surface = DitherSurface::new(3, move |w, hg| {
            button(w, hg, tone, style, style.density(h.get(), p.get()), t.get())
        });
        surface.set_accessible_label(label);
        surface.widget().set_size_request(150, 42);
        surface.widget().set_can_target(false);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(surface.widget()));
        let text = gtk::Label::new(Some(label));
        text.set_can_target(false);
        overlay.add_overlay(&text);
        let widget = gtk::Button::builder().child(&overlay).build();
        widget.update_property(&[gtk::accessible::Property::Label(label)]);
        if let Some(tint) = style.surface(tone) {
            let css = gtk::CssProvider::new();
            let class = format!("dither-solid-{}-{}-{}", tint.r, tint.g, tint.b);
            widget.add_css_class(&class);
            css.load_from_data(&format!(
                "button.{class} {{ background: rgb({},{},{}); }}",
                tint.r, tint.g, tint.b
            ));
            gtk::style_context_add_provider_for_display(
                &widget.display(),
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        let running = Rc::new(Cell::new(false));
        let start: Rc<dyn Fn()> = {
            let (sf, h, p, t, r) = (
                surface.clone(),
                hovered.clone(),
                pressed.clone(),
                time.clone(),
                running.clone(),
            );
            Rc::new(move || {
                sf.queue_render();
                if style.density(h.get(), p.get()) <= 0.4
                    || !animations(sf.widget())
                    || r.replace(true)
                {
                    return;
                }
                let (h, p, t, r) = (h.clone(), p.clone(), t.clone(), r.clone());
                let first = Cell::new(None);
                sf.widget().add_tick_callback(move |area, clock| {
                    if style.density(h.get(), p.get()) <= 0.4 || !animations(area) {
                        r.set(false);
                        return glib::ControlFlow::Break;
                    }
                    let now = clock.frame_time();
                    let previous = first.replace(Some(now)).unwrap_or(now);
                    t.set(t.get() + (now - previous) as f32 / 1_000_000.0);
                    area.queue_render();
                    glib::ControlFlow::Continue
                });
            })
        };
        let controller = gtk::EventControllerMotion::new();
        let (h, f) = (hovered.clone(), start.clone());
        controller.connect_enter(move |_, _, _| {
            h.set(true);
            f();
        });
        let (h, p, f) = (hovered.clone(), pressed.clone(), start.clone());
        controller.connect_leave(move |_| {
            h.set(false);
            p.set(false);
            f();
        });
        widget.add_controller(controller);
        // GTK's ACTIVE flag includes keyboard activation and pointer gestures without stealing Button's click gesture.
        let (p, f) = (pressed.clone(), start.clone());
        widget.connect_state_flags_changed(move |button, _| {
            p.set(button.state_flags().contains(gtk::StateFlags::ACTIVE));
            f();
        });
        Self { widget, surface }
    }
    pub fn widget(&self) -> &gtk::Button {
        &self.widget
    }
    pub fn surface(&self) -> &DitherSurface {
        &self.surface
    }
}
