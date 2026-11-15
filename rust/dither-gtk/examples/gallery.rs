use dither_gtk::{
    gtk::{self, glib, prelude::*},
    ir::*,
    *,
};

use std::{cell::RefCell, rc::Rc};

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

fn main() -> glib::ExitCode {
    let args: Vec<_> = std::env::args().collect();

    let screenshot = args.iter().position(|a| a == "--smoke").map(|i| {
        args.get(i + 1)
            .cloned()
            .unwrap_or_else(|| "gallery.png".into())
    });

    let app = gtk::Application::builder()
        .application_id("org.ditherkit.Gallery")
        .build();

    app.connect_activate(move |app| {

        let css=gtk::CssProvider::new();
 css.load_from_data("window, .gallery { background: #10151d; color: #edf3ff; } .card { background: #192230; border-radius: 12px; padding: 14px; } .heading { font-weight: bold; font-size: 16px; } .tooltip { background: #080d14; color: white; padding: 6px; border-radius: 4px; } button { color: white; background: #29374b; background-image: none; } label { color: #edf3ff; }");

        if let Some(display)=gtk::gdk::Display::default() {
 gtk::style_context_add_provider_for_display(&display,&css,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
 }

        let window=gtk::ApplicationWindow::builder().application(app).title("Dither Kit · GTK4").default_width(1120).default_height(1050).build();

        let content=gtk::Box::new(gtk::Orientation::Vertical,16);
 content.set_margin_top(20);
 content.set_margin_bottom(20);
 content.set_margin_start(20);
 content.set_margin_end(20);

        let title=gtk::Label::new(Some("DITHER KIT / Native GTK4"));
 title.set_xalign(0.0);
 title.add_css_class("title-1");
 content.append(&title);

        let subtitle=gtk::Label::new(Some("Shader pixels. Native controls. Hover charts and toggle their legends."));
 subtitle.set_xalign(0.0);
 content.append(&subtitle);

        let surfaces:Rc<RefCell<Vec<gtk::GLArea>>>=Rc::default();

        let row=gtk::Box::new(gtk::Orientation::Horizontal,12);
 row.set_homogeneous(true);

        let gradient=DitherGradient::new(Gradient {
 to:Some(Tone::Pink), direction:Direction::Right, ..Gradient::new(Tone::Blue) }
);
 gradient.widget().set_size_request(240,90);
 surfaces.borrow_mut().push(gradient.widget().clone());
 row.append(&panel("Gradient",gradient.widget()));

        let background=DitherBackground::new(Background::new(Field::Aurora,Tone::Green));
 background.widget().set_size_request(240,90);
 surfaces.borrow_mut().push(background.widget().clone());
 row.append(&panel("Animated background",background.widget()));
 content.append(&row);

        let series=vec![Series::new("Requests",Tone::Green,vec![12.,23.,18.,37.,28.,46.,39.]),Series::new("Responses",Tone::Blue,vec![8.,12.,15.,24.,18.,30.,34.]).fill(Fill::Hatched)];

        let row=gtk::Box::new(gtk::Orientation::Horizontal,12);
 row.set_homogeneous(true);

        for (title,kind) in [("Area",CartesianKind::Area),("Line",CartesianKind::Line),("Bar",CartesianKind::Bar)] {
 let mut options=ChartOptions::new(kind,series.clone());
 options.x_labels=vec!["Mon".into(),"Thu".into(),"Sun".into()];
 let chart=DitherChart::new(options);
 surfaces.borrow_mut().push(chart.surface().widget().clone());
 row.append(&panel(title,chart.widget()));
 }
 content.append(&row);

        let row=gtk::Box::new(gtk::Orientation::Horizontal,12);
 row.set_homogeneous(true);

        let pie=DitherPie::new(vec![Slice::new("Compute",42.,Tone::Green),Slice::new("Storage",33.,Tone::Blue),Slice::new("Network",25.,Tone::Pink)],0.5);
 surfaces.borrow_mut().push(pie.surface().widget().clone());
 row.append(&panel("Pie",pie.widget()));

        let radar=DitherRadar::new(vec![Series::new("Current",Tone::Purple,vec![8.,6.,9.,5.,7.]),Series::new("Target",Tone::Blue,vec![6.,8.,7.,9.,5.])],vec!["Speed".into(),"Quality".into(),"Reach".into(),"Cost".into(),"Trust".into()]);
 surfaces.borrow_mut().push(radar.surface().widget().clone());
 row.append(&panel("Radar",radar.widget()));

        let small=gtk::Box::new(gtk::Orientation::Vertical,16);
 let spark=DitherSparkline::new(vec![4.,8.,5.,12.,9.,16.,14.],Tone::Green);
 spark.widget().set_size_request(220,60);
 surfaces.borrow_mut().push(spark.widget().clone());
 small.append(&panel("Sparkline",spark.widget()));

        let meter=DitherMeter::new(0.42,Tone::Blue,Fill::Gradient);
 surfaces.borrow_mut().push(meter.surface().widget().clone());
 small.append(&panel("Spring meter",meter.widget()));

        let avatars=gtk::Box::new(gtk::Orientation::Horizontal,12);
 for name in ["Dither","Native","GTK"] {
 let avatar=DitherAvatar::new(name,48,None);
 surfaces.borrow_mut().push(avatar.widget().clone());
 avatars.append(avatar.widget());
 }
 small.append(&panel("Avatars",&avatars));
 row.append(&small);
 content.append(&row);

        let buttons=gtk::Box::new(gtk::Orientation::Horizontal,12);
 buttons.set_homogeneous(true);

        for (name,style) in [("Solid",ButtonStyle::Solid),("Outline",ButtonStyle::Outline),("Ghost",ButtonStyle::Ghost)] {
 let button=DitherButton::new(name,Tone::Green,style);
 surfaces.borrow_mut().push(button.surface().widget().clone());
 let m=meter.clone();
 let next=std::cell::Cell::new(false);
 button.widget().connect_clicked(move |_| {
 let high=!next.get();
 next.set(high);
 m.set_value(if high {
0.88}
 else {
0.42}
);
 }
);
 buttons.append(button.widget());
 }
 content.append(&panel("Buttons · click to update meter",&buttons));

        // Keep all examples visible in smoke captures;
        // Normal use may scroll smaller windows.
        let scroll=gtk::ScrolledWindow::builder().child(&content).hscrollbar_policy(gtk::PolicyType::Never).build();
 scroll.add_css_class("gallery");
 window.set_child(Some(&scroll));
 window.present();

        if let Some(path)=screenshot.clone() {

            let window=window.clone();
 let app=app.clone();

            glib::timeout_add_local_once(std::time::Duration::from_secs(3),move || {

                for area in surfaces.borrow().iter() {
 if let Some(error)=area.error() {
 eprintln!("GLArea failed: {error}");
 std::process::exit(1);
 }
 if !area.is_realized() {
 eprintln!("GLArea never realized");
 std::process::exit(1);
 }
 }

                let snapshot=gtk::Snapshot::new();
 window.snapshot_child(&scroll,&snapshot);

                let node=snapshot.to_node().expect("gallery produced render nodes");

                let renderer=window.renderer().expect("native GSK renderer");
 let texture=renderer.render_texture(&node,None);
 texture.save_to_png(&path).expect("write gallery screenshot");

                eprintln!("Wrote {path} ({}×{})",texture.width(),texture.height());
 app.quit();

            }
);

        }

    }
);

    app.run_with_args(&["gallery"])
}
