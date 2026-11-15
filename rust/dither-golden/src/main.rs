//! Golden fixture generator.
//!
//! For every scene in `scenes()`:
//! 1. render it with upstream `dither-core` (the source of truth),
//! 2. encode it with `dither-ir` and run the reference interpreter,
//! 3. compare the two pixel grids,
//! 4. write `fixtures/scenes.json` (scene parameters + encoded program) and
//!    `fixtures/png/<name>.png` (the dither-core pixels).
//!
//! The Swift and C# ports read `scenes.json` to check their encoders (program
//! floats) and their shaders (pixels against the PNGs).
//!
//! Usage: `cargo run -p dither-golden -- <fixtures-dir> [--check]`

use std::fs;
use std::path::{Path, PathBuf};

use dither_core as core;
use dither_ir as ir;
use serde_json::{Value, json};

fn tone_json(t: &Value) -> ir::Tone {
    match t {
        Value::Number(n) => ir::Tone::Hue(n.as_f64().unwrap() as f32),
        Value::String(s) => ir::Tone::parse(s).expect("tone"),
        _ => panic!("tone"),
    }
}

fn ctone(t: ir::Tone) -> core::Tone {
    match t {
        ir::Tone::Green => core::Tone::Green,
        ir::Tone::Blue => core::Tone::Blue,
        ir::Tone::Purple => core::Tone::Purple,
        ir::Tone::Pink => core::Tone::Pink,
        ir::Tone::Orange => core::Tone::Orange,
        ir::Tone::Red => core::Tone::Red,
        ir::Tone::Grey => core::Tone::Grey,
        ir::Tone::Hue(h) => core::Tone::Hue(h),
    }
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or_else(|| panic!("missing {k}"))
}
fn f(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap_or_else(|| panic!("missing {k}"))
}
fn opt_usize(v: &Value, k: &str) -> Option<usize> {
    v[k].as_u64().map(|x| x as usize)
}

fn matrix(name: &str) -> (ir::Matrix, core::Matrix) {
    match name {
        "bayer2" => (ir::Matrix::Bayer2, core::Matrix::Bayer2),
        "bayer4" => (ir::Matrix::Bayer4, core::Matrix::Bayer4),
        "bayer8" => (ir::Matrix::Bayer8, core::Matrix::Bayer8),
        "halftone" => (ir::Matrix::Halftone, core::Matrix::Halftone),
        "noise" => (ir::Matrix::Noise, core::Matrix::Noise),
        _ => panic!("matrix {name}"),
    }
}

fn fill(name: &str) -> (ir::Fill, core::Fill) {
    match name {
        "gradient" => (ir::Fill::Gradient, core::Fill::Gradient),
        "dotted" => (ir::Fill::Dotted, core::Fill::Dotted),
        "hatched" => (ir::Fill::Hatched, core::Fill::Hatched),
        "solid" => (ir::Fill::Solid, core::Fill::Solid),
        _ => panic!("fill {name}"),
    }
}

fn bloom(name: &str) -> (ir::Bloom, core::Bloom) {
    match name {
        "off" => (ir::Bloom::Off, core::Bloom::Off),
        "low" => (ir::Bloom::Low, core::Bloom::Low),
        "high" => (ir::Bloom::High, core::Bloom::High),
        "aura" => (ir::Bloom::Aura, core::Bloom::Aura),
        _ => panic!("bloom {name}"),
    }
}

fn series(v: &Value) -> (Vec<ir::Series>, Vec<core::Series>) {
    let mut a = vec![];
    let mut b = vec![];
    for sv in v["series"].as_array().unwrap() {
        let t = tone_json(&sv["tone"]);
        let values: Vec<f64> = sv["values"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        let (fi, fc) = fill(s(sv, "fill"));
        let hidden = sv["hidden"].as_bool().unwrap_or(false);
        a.push(ir::Series::new(s(sv, "label"), t, values.clone()).fill(fi).hidden(hidden));
        b.push(core::Series::new(s(sv, "label"), ctone(t), values).fill(fc).hidden(hidden));
    }
    (a, b)
}

/// Renders a scene both ways. Returns (program, ir pixels, core pixels).
fn render(scene: &Value) -> (Vec<f32>, Vec<Option<ir::Rgba>>, Vec<Option<core::Rgba>>) {
    let (w, h) = (f(scene, "w") as usize, f(scene, "h") as usize);
    let (prog, grid): (ir::Program, core::PixelGrid) = match s(scene, "kind") {
        "gradient" => {
            let from = tone_json(&scene["from"]);
            let to = (!scene["to"].is_null()).then(|| tone_json(&scene["to"]));
            let dir = match s(scene, "direction") {
                "up" => (ir::Direction::Up, core::Direction::Up),
                "down" => (ir::Direction::Down, core::Direction::Down),
                "left" => (ir::Direction::Left, core::Direction::Left),
                "right" => (ir::Direction::Right, core::Direction::Right),
                "radial" => (ir::Direction::Radial, core::Direction::Radial),
                d => panic!("direction {d}"),
            };
            let (mi, mc) = matrix(s(scene, "matrix"));
            let density = f(scene, "density") as f32;
            let mut gi = ir::Gradient::new(from);
            gi.to = to;
            gi.direction = dir.0;
            gi.matrix = mi;
            gi.density = density;
            let mut gc = core::Gradient::new(ctone(from)).direction(dir.1).matrix(mc).density(density);
            if let Some(t) = to {
                gc = gc.to(ctone(t));
            }
            (gi.encode(w, h), gc.render(w, h))
        }
        "background" => {
            let t = tone_json(&scene["tone"]);
            let acc = (!scene["accent"].is_null()).then(|| tone_json(&scene["accent"]));
            let field = match s(scene, "field") {
                "plasma" => (ir::Field::Plasma, core::Field::Plasma),
                "waves" => (ir::Field::Waves, core::Field::Waves),
                "ripple" => (ir::Field::Ripple, core::Field::Ripple),
                "noise" => (ir::Field::Noise, core::Field::Noise),
                "aurora" => (ir::Field::Aurora, core::Field::Aurora),
                x => panic!("field {x}"),
            };
            let (mi, mc) = matrix(s(scene, "matrix"));
            let density = f(scene, "density") as f32;
            let time = f(scene, "time") as f32;
            let mut bi = ir::Background::new(field.0, t);
            bi.accent = acc;
            bi.matrix = mi;
            bi.density = density;
            let mut bc = core::Background::new(field.1, ctone(t)).matrix(mc).density(density);
            if let Some(a) = acc {
                bc = bc.accent(ctone(a));
            }
            (bi.encode(w, h, time), bc.render(w, h, time))
        }
        "button" => {
            let t = tone_json(&scene["tone"]);
            let style = match s(scene, "style") {
                "solid" => (ir::ButtonStyle::Solid, core::ButtonStyle::Solid),
                "outline" => (ir::ButtonStyle::Outline, core::ButtonStyle::Outline),
                "ghost" => (ir::ButtonStyle::Ghost, core::ButtonStyle::Ghost),
                x => panic!("style {x}"),
            };
            let (d, time) = (f(scene, "density") as f32, f(scene, "time") as f32);
            (ir::button(w, h, t, style.0, d, time), core::button_texture(w, h, ctone(t), style.1, d, time))
        }
        "meter" => {
            let t = tone_json(&scene["tone"]);
            let (fi, fc) = fill(s(scene, "fill"));
            let v = f(scene, "value") as f32;
            (ir::meter(w, h, v, t, fi), core::meter(w, h, v, ctone(t), fc))
        }
        "avatar" => {
            let hue = scene["hue"].as_f64().map(|x| x as f32);
            (ir::avatar(s(scene, "text"), hue), core::avatar(s(scene, "text"), hue))
        }
        "swatch" => {
            let t = tone_json(&scene["tone"]);
            let mut g = core::PixelGrid::new(w, h);
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    g.set(x, y, ctone(t).shade(if (x + y) % 2 == 0 { 0.72 } else { 0.48 }));
                }
            }
            (ir::swatch(t), g)
        }
        "sparkline" => {
            let t = tone_json(&scene["tone"]);
            let values: Vec<f64> = scene["values"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
            let hovered = scene["hovered"].as_bool().unwrap();
            let p = f(scene, "progress") as f32;
            (ir::sparkline(&values, t, w, h, hovered, p), core::sparkline(&values, ctone(t), w, h, hovered, p))
        }
        "cartesian" => {
            let (si, sc) = series(scene);
            let kind = match s(scene, "chart") {
                "area" => (ir::CartesianKind::Area, core::CartesianKind::Area),
                "line" => (ir::CartesianKind::Line, core::CartesianKind::Line),
                "bar" => (ir::CartesianKind::Bar, core::CartesianKind::Bar),
                x => panic!("chart {x}"),
            };
            let (bi, bc) = bloom(s(scene, "bloom"));
            let mut ci = ir::Cartesian::new(kind.0, &si);
            let mut cc = core::Cartesian::new(kind.1, &sc);
            ci.stacked = scene["stacked"].as_bool().unwrap();
            cc.stacked = ci.stacked;
            ci.progress = f(scene, "progress") as f32;
            cc.progress = ci.progress;
            ci.hover = opt_usize(scene, "hover");
            cc.hover = ci.hover;
            ci.selected = opt_usize(scene, "selected");
            cc.selected = ci.selected;
            ci.bloom = bi;
            cc.bloom = bc;
            (ci.encode(w, h), cc.render(w, h))
        }
        "pie" => {
            let mut a = vec![];
            let mut b = vec![];
            for sv in scene["slices"].as_array().unwrap() {
                let t = tone_json(&sv["tone"]);
                let (fi, fc) = fill(s(sv, "fill"));
                a.push(ir::Slice::new(s(sv, "label"), f(sv, "value"), t).fill(fi));
                b.push(core::Slice::new(s(sv, "label"), f(sv, "value"), ctone(t)).fill(fc));
            }
            let (bi, bc) = bloom(s(scene, "bloom"));
            let mut pi = ir::Pie::new(&a);
            let mut pc = core::Pie::new(&b);
            pi.inner_radius = f(scene, "inner") as f32;
            pc.inner_radius = pi.inner_radius;
            pi.progress = f(scene, "progress") as f32;
            pc.progress = pi.progress;
            pi.hover = opt_usize(scene, "hover");
            pc.hover = pi.hover;
            pi.bloom = bi;
            pc.bloom = bc;
            (pi.encode(w, h), pc.render(w, h))
        }
        "radar" => {
            let (si, sc) = series(scene);
            let (bi, bc) = bloom(s(scene, "bloom"));
            let mut ri = ir::Radar::new(&si);
            let mut rc = core::Radar::new(&sc);
            ri.max = scene["max"].as_f64();
            rc.max = ri.max;
            ri.progress = f(scene, "progress") as f32;
            rc.progress = ri.progress;
            ri.selected = opt_usize(scene, "selected");
            rc.selected = ri.selected;
            ri.rings = f(scene, "rings") as usize;
            rc.rings = ri.rings;
            ri.bloom = bi;
            rc.bloom = bc;
            (ri.encode(w, h), rc.render(w, h))
        }
        k => panic!("kind {k}"),
    };
    let program = prog.to_vec();
    let ours = ir::vm::render(&program, w, h);
    let theirs = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| grid.get(x, y)).collect();
    (program, ours, theirs)
}

fn scenes() -> Vec<Value> {
    let visitors = json!([
        {"label": "Desktop", "tone": "blue", "fill": "gradient", "values": [186, 305, 237, 73, 209, 214, 250, 190]},
        {"label": "Mobile", "tone": "pink", "fill": "hatched", "values": [80, 200, 120, 190, 130, 140, 160, 110]},
        {"label": "Tablet", "tone": "green", "fill": "dotted", "values": [40, 60, 90, 70, 50, 80, 75, 60]}
    ]);
    let skills = json!([
        {"label": "Alpha", "tone": "purple", "fill": "gradient", "values": [5, 3, 4, 2, 5, 4]},
        {"label": "Beta", "tone": "orange", "fill": "dotted", "values": [3, 5, 2, 4, 3, 5]}
    ]);
    let mut v = vec![];
    for (i, (from, to, dir, m)) in [
        ("green", Value::Null, "up", "bayer8"),
        ("blue", json!("pink"), "right", "bayer4"),
        ("purple", Value::Null, "radial", "halftone"),
        ("orange", json!("red"), "down", "noise"),
        ("200", json!("grey"), "left", "bayer2"),
    ]
    .iter()
    .enumerate()
    {
        let from = if from.parse::<f64>().is_ok() { json!(200) } else { json!(from) };
        v.push(json!({"name": format!("gradient_{i}"), "kind": "gradient", "w": 64, "h": 24, "from": from,
            "to": to, "direction": dir, "matrix": m, "density": if i == 3 { 0.7 } else { 1.0 }}));
    }
    for (i, field) in ["plasma", "waves", "ripple", "noise", "aurora"].iter().enumerate() {
        v.push(json!({"name": format!("background_{field}"), "kind": "background", "w": 80, "h": 32,
            "field": field, "tone": (["green", "blue", "purple", "orange", "pink"][i]),
            "accent": if i % 2 == 0 { json!("pink") } else { Value::Null },
            "matrix": if i == 3 { "noise" } else { "bayer8" }, "density": 0.6 + 0.08 * i as f64, "time": 1.25 * i as f64}));
    }
    for (i, (style, d)) in [("solid", 0.32), ("solid", 0.85), ("outline", 0.3), ("ghost", 0.5), ("outline", 0.6)].iter().enumerate() {
        v.push(json!({"name": format!("button_{i}_{style}"), "kind": "button", "w": 40, "h": 12,
            "tone": (["green", "blue", "pink", "orange", "purple"][i]), "style": style, "density": d, "time": 0.7 + i as f64}));
    }
    for (i, fill) in ["gradient", "dotted", "hatched", "solid"].iter().enumerate() {
        v.push(json!({"name": format!("meter_{fill}"), "kind": "meter", "w": 60, "h": 6,
            "value": ([0.62, 0.35, 0.8, 0.5][i]), "tone": ([json!("green"), json!("blue"), json!("red"), json!(75)][i].clone()), "fill": fill}));
    }
    for name in ["Ada Lovelace", "Grace Hopper", "Zaki", "satoshi"] {
        v.push(json!({"name": format!("avatar_{}", name.replace(' ', "_").to_lowercase()), "kind": "avatar",
            "w": 8, "h": 8, "text": name, "hue": Value::Null}));
    }
    v.push(json!({"name": "swatch_blue", "kind": "swatch", "w": 5, "h": 5, "tone": "blue"}));
    for (i, hov) in [false, true].iter().enumerate() {
        v.push(json!({"name": format!("sparkline_{i}"), "kind": "sparkline", "w": 48, "h": 14,
            "values": [3, 7, 4, 9, 6, 11, 8, 12, 10], "tone": (["green", "pink"][i]), "hovered": hov, "progress": 1.0}));
    }
    let chart_cases = [
        ("area", false, None, None, "off", 1.0),
        ("area", true, Some(3), None, "low", 1.0),
        ("area", false, None, Some(1), "high", 0.6),
        ("line", false, Some(5), None, "off", 1.0),
        ("line", true, None, None, "aura", 1.0),
        ("bar", false, Some(2), None, "off", 1.0),
        ("bar", true, None, Some(0), "low", 0.7),
    ];
    for (i, (chart, stacked, hover, selected, bl, prog)) in chart_cases.iter().enumerate() {
        v.push(json!({"name": format!("{chart}_{i}"), "kind": "cartesian", "w": 120, "h": 44, "chart": chart,
            "stacked": stacked, "hover": hover, "selected": selected, "bloom": bl, "progress": prog,
            "series": visitors}));
    }
    let slices = json!([
        {"label": "Chrome", "value": 275, "tone": "blue", "fill": "gradient"},
        {"label": "Safari", "value": 200, "tone": "green", "fill": "dotted"},
        {"label": "Firefox", "value": 187, "tone": "orange", "fill": "hatched"},
        {"label": "Edge", "value": 90, "tone": "purple", "fill": "solid"}
    ]);
    for (i, (inner, hover, bl, prog)) in [(0.0, None, "off", 1.0), (0.55, Some(1), "low", 1.0), (0.4, None, "aura", 0.7)]
        .iter()
        .enumerate()
    {
        v.push(json!({"name": format!("pie_{i}"), "kind": "pie", "w": 48, "h": 48, "slices": slices,
            "inner": inner, "hover": hover, "bloom": bl, "progress": prog}));
    }
    for (i, (sel, bl, prog)) in [(None, "off", 1.0), (Some(0), "high", 1.0), (None, "low", 0.5)].iter().enumerate() {
        v.push(json!({"name": format!("radar_{i}"), "kind": "radar", "w": 56, "h": 56, "series": skills,
            "max": Value::Null, "selected": sel, "bloom": bl, "progress": prog, "rings": 4}));
    }
    v
}

fn write_png(path: &Path, w: usize, h: usize, px: &[Option<core::Rgba>]) {
    let file = fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().unwrap();
    let data: Vec<u8> = px.iter().flat_map(|c| c.map(|c| [c.r, c.g, c.b, c.a]).unwrap_or([0; 4])).collect();
    wr.write_image_data(&data).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("../fixtures"));
    let check_only = args.iter().any(|a| a == "--check");
    if !check_only {
        fs::create_dir_all(dir.join("png")).unwrap();
    }
    let mut out = vec![];
    let mut failed = 0;
    for mut scene in scenes() {
        let name = s(&scene, "name").to_string();
        let (program, ours, theirs) = render(&scene);
        let (w, h) = (f(&scene, "w") as usize, f(&scene, "h") as usize);
        let mut diff = 0;
        for (a, b) in ours.iter().zip(&theirs) {
            let a = a.map(|c| (c.r, c.g, c.b, c.a));
            let b = b.map(|c| (c.r, c.g, c.b, c.a));
            if a != b {
                diff += 1;
            }
        }
        let lit = theirs.iter().filter(|c| c.is_some()).count();
        println!("{name:<24} {w:>3}x{h:<3} lit {lit:>5}  mismatched {diff}");
        if diff > 0 {
            failed += 1;
        }
        if !check_only {
            write_png(&dir.join("png").join(format!("{name}.png")), w, h, &theirs);
        } else if let Err(error) = check_png(&dir.join("png").join(format!("{name}.png")), w, h, &theirs) {
            eprintln!("{name}: {error}");
            failed += 1;
        }
        scene["program"] = json!(program);
        out.push(scene);
    }
    let expected = json!({"version": 1, "scenes": out});
    if !check_only {
        fs::write(
            dir.join("scenes.json"),
            serde_json::to_string_pretty(&expected).unwrap(),
        )
        .unwrap();
    } else {
        let stored = fs::read(dir.join("scenes.json"))
            .map_err(|error| error.to_string())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(|error| error.to_string()));
        match stored {
            Ok(value) if value == expected => {}
            Ok(_) => {
                eprintln!("scenes.json differs from the generated scene parameters or programs");
                failed += 1;
            }
            Err(error) => {
                eprintln!("scenes.json: {error}");
                failed += 1;
            }
        }
    }
    if failed > 0 {
        eprintln!("{failed} parity or fixture check(s) failed");
        std::process::exit(1);
    }
}

/// Compare decoded pixels rather than compression bytes so PNG encoders may vary.
fn check_png(path: &Path, w: usize, h: usize, pixels: &[Option<core::Rgba>]) -> Result<(), String> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info().map_err(|error| error.to_string())?;
    let mut bytes = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut bytes).map_err(|error| error.to_string())?;
    if info.width as usize != w || info.height as usize != h
        || info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err("expected grid-size 8-bit RGBA PNG".into());
    }
    let expected: Vec<u8> = pixels.iter()
        .flat_map(|color| color.map(|c| [c.r, c.g, c.b, c.a]).unwrap_or([0; 4])).collect();
    if bytes[..info.buffer_size()] != expected {
        return Err("stored pixels differ from dither-core".into());
    }
    Ok(())
}
