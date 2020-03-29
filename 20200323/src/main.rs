use nannou::geom::range::Range;
use nannou::image;
use nannou::image::GenericImageView;
use nannou::image::Pixel;
use nannou::noise::NoiseFn;
use nannou::noise::Seedable;
use nannou::prelude::*;
use nannou::ui::prelude::*;
use rand::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;
extern crate rand;

type ImgBuf = image::ImageBuffer<image::Rgba<u8>, Vec<u8>>;

mod son;

const MAX_LINE_LENGTH2: f32 = 2500.0;
const MAX_LINES_EVER: usize = 20;
const LIFETIME: i32 = 20;
const RENDER: bool = false;

struct Ids {
    ring_width: widget::Id,
    max_lines: widget::Id,
    friction: widget::Id,
    force_strength: widget::Id,
}

struct LinePoint {
    pos: Point2,
    vel: Vector2,
    sine_i: usize,
    max_lines: usize,
    activated: bool,
    lines: Vec<Rc<RefCell<LinePoint>>>,
    lifetime: i32,
    force_strength: f32,
}

impl LinePoint {
    fn new_at(pos: Point2, sine_i: usize, lifetime: i32) -> Self {
        let mut lines = Vec::new();
        lines.reserve(MAX_LINES_EVER);
        LinePoint {
            pos,
            vel: vec2(0.0, 0.0),
            sine_i,
            max_lines: 5,
            activated: false,
            lines: lines,
            lifetime,
            force_strength: 0.0,
        }
    }

    fn update(
        &mut self,
        win_rect: &nannou::geom::rect::Rect,
        friction: f32,
        force_strength: f32,
    ) {
        self.force_strength = force_strength;
        if self.activated {
            self.lifetime -= 1;
        }
        // Remove lines if there are too many.
        if self.lines.len() > self.max_lines {
            for _ in 0..self.lines.len() - self.max_lines {
                self.lines.pop();
            }
        }
        // Remove lines to points that are too far away.
        let local_pos = self.pos;
        self.lines
            .retain(|x| x.borrow().pos.distance2(local_pos) < MAX_LINE_LENGTH2);

        // Move towards connected dots it's far away from and away from close ones.
        self.vel *= friction; // Velocity damping, 0.7 is gooood
        let dist_range = Range::new(0.0, MAX_LINE_LENGTH2);
        let vel_range = Range::new(-self.force_strength, self.force_strength);
        for lp in &self.lines {
            let dist2 = lp.borrow().pos.distance2(local_pos);
            let force = dist_range.map_value(dist2, &vel_range);
            self.vel += (lp.borrow().pos - self.pos) * force;
        }

        // self.pos += self.vel;

        let speed_range = Range::new(0.0, 10.0);
        let freq_range = Range::new(100.0, 1000.0);
        let freq: f64 =
            speed_range.map_value(self.vel.distance2(pt2(0.0, 0.0)) as f64, &freq_range);
        // audio_interface.set_sine_freq(self.sine_i, freq);

        // Wrap at the edges of the screen.
        self.pos.x = self.pos.x.max(win_rect.left()).min(win_rect.right());
        self.pos.y = self.pos.y.max(win_rect.bottom()).min(win_rect.top());
    }

    fn trigger_sound(&self, audio_interface: &mut son::AudioInterface) {
        // audio_interface.set_sine_amp(self.sine_i, 0.05 );
    }
}

impl PartialEq for LinePoint {
    fn eq(&self, other: &LinePoint) -> bool {
        self.pos == other.pos
    }
}

fn main() {
    nannou::app(model).update(update).run();
}

struct Model {
    _window: window::Id,
    // audio_interface: son::AudioInterface,
    points: Vec<Rc<RefCell<LinePoint>>>,
    isolated_points: Vec<Rc<RefCell<LinePoint>>>,
    ui: Ui,
    widget_ids: Ids,
    friction: f32,
    max_lines: usize,
    force_strength: f32,
    show_gui: bool,
    shape_angle: f32,
    ring_width: f32,
    points_removed: usize,
}

impl Model {
    fn generate_points(&mut self) {
        let points = &mut self.points;
        points.clear();
        let mut rng = rand::thread_rng();
        // Create rings of points
        // let ring_width = 40.0;
        for ring in 1..15 {
            let r = ring as f32 * self.ring_width;
            let num_points = (r * PI * 0.04);
            let num_pointsi = num_points as usize;
            let angle_offset = rng.gen::<f32>()*10.0;
            for n in 0..num_pointsi {
                let angle_offset2 = rng.gen::<f32>() * 0.3;
                let angle = (angle_offset + angle_offset2 + PI * 2.0 * n as f32) / num_points;
                let new_point = Rc::new(RefCell::new(LinePoint::new_at(
                    pt2(angle.cos() * r, angle.sin() * r),
                    0,
                    LIFETIME,
                )));
                points.push(new_point);
            }
        }
        // Shuffling changes the order the points are checked for distance -> the order they are connected
        points.shuffle(&mut rng);
    }
}

fn model(app: &App) -> Model {
    let _window = app
        .new_window()
        .size(1024, 1024)
        .view(view)
        .event(window_event)
        .build()
        .unwrap();

    // Audio setup

    // let audio_interface = son::AudioInterface::new();

    // Ui setup

    let mut ui = app.new_ui().build().unwrap();

    let widget_ids = Ids {
        ring_width: ui.generate_widget_id(),
        max_lines: ui.generate_widget_id(),
        friction: ui.generate_widget_id(),
        force_strength: ui.generate_widget_id(),
    };

    

    // let mut point_dist: f32 = MAX_LINE_LENGTH2.sqrt();
    // point_dist *= 0.6;
    // let points_per_row: u64 =
    //     ((app.window_rect().right() - app.window_rect().left()) / point_dist) as u64;

    // for ix in 1..points_per_row {
    //     for iy in 1..points_per_row {
    //         let mut x = ix as f32 * point_dist + app.window_rect().left();
    //         println!("x: {:?}", x);
    //         if iy % 2 == 1 {
    //             x += point_dist / 2.0;
    //         }
    //         let y = app.window_rect().top() - iy as f32 * point_dist;
    //         if !(iy % 2 == 0 && ix == 1) {
    //             let screen_point = pt2(x, y);
    //             let image_point = pt2(
    //                 (screen_point.x + app.window_rect().right()) * img_ratio,
    //                 (screen_point.y + app.window_rect().top()) * img_ratio);
    //             println!("image_point: {:?}", image_point);
    //             let pixel = image_rgba.get_pixel(image_point.x as u32, image_point.y as u32);
    //             let luma = pixel.to_luma().channels()[0];
    //             let rgb_vals: Vec<f32> = pixel.channels().iter().cloned().map(|x| x as f32 / 255.0).collect();
    //             let color: Rgba = rgba(rgb_vals[0], rgb_vals[1], rgb_vals[2], rgb_vals[3]);
    //             let new_point = Rc::new(RefCell::new(LinePoint::new_at(
    //                 screen_point,
    //                 0,
    //                 LIFETIME,
    //                 color,
    //             )));
    //             points.push(new_point);
    //         }
    // }
    // }

    let mut model = Model {
        _window,
        // audio_interface,
        points: vec![],
        isolated_points: vec![],
        ui,
        widget_ids,
        friction: 0.7,
        max_lines: 2,
        force_strength: 0.25,
        show_gui: false,
        shape_angle: 0.0,
        ring_width: 30.0,
        points_removed: 10,
    };
    model.generate_points();
    model
}

fn update(app: &App, model: &mut Model, _update: Update) {
    {
        // Calling `set_widgets` allows us to instantiate some widgets.
        let ui = &mut model.ui.set_widgets();

        fn slider(val: f32, min: f32, max: f32) -> widget::Slider<'static, f32> {
            widget::Slider::new(val, min, max)
                .w_h(200.0, 30.0)
                .label_font_size(15)
                .rgb(0.3, 0.3, 0.3)
                .label_rgb(1.0, 1.0, 1.0)
                .border(1.0)
        }

        for value in slider(model.friction, 0.0, 1.0)
            .top_left_with_margin(20.0)
            .label(&format!("Friction: {}", model.friction))
            .set(model.widget_ids.friction, ui)
        {
            model.friction = value;
        }

        for value in slider(model.max_lines as f32, 0.0, MAX_LINES_EVER as f32)
            .down(20.0)
            .label("Max lines")
            .set(model.widget_ids.max_lines, ui)
        {
            model.max_lines = value as usize;
        }

        for value in slider(model.force_strength, 0.0, 1.0)
            .down(20.0)
            .label("Force strength")
            .set(model.widget_ids.force_strength, ui)
        {
            model.force_strength = value;
        }

        for value in slider(model.ring_width, 5.0, 100.0)
            .down(20.0)
            .label(&format!("Ring width: {}", model.ring_width))
            .set(model.widget_ids.ring_width, ui)
        {
            model.ring_width = value;
        }
    }

    // Activate point close to the mouse
    if let Some(pushed_point) = app.mouse.buttons.left().if_down() {
        let mouse_pos = pt2(app.mouse.x, app.mouse.y);
        for p in &model.points {
            if mouse_pos.distance2(p.borrow().pos) < MAX_LINE_LENGTH2 * 0.5 {
                p.borrow_mut().activated = true;
            }
        }
    }

    // Every x frames, generate new points and activate one of them
    // if app.elapsed_frames() % 140 == 0 {
    //     model.generate_points();
    //     model.points.choose(&mut thread_rng()).unwrap().borrow_mut().activated = true;;
    // }

    // When there are no more active points, make a new grid
    let mut active_points = 0;
    for p in &model.points {
        if p.borrow().activated { active_points += 1; }
    }
    if active_points == 0 {
        if model.points.len() < 50 {
            // Generate a new set of points
            model.generate_points();
            model.isolated_points = vec![];
            // remove a number of points
            // let num_points_to_remove = (thread_rng().gen::<f32>() * model.points.len() as f32) as usize;
            let num_points_to_remove = model.points_removed;
            for _ in 0..num_points_to_remove {
                let removed = model.points.pop();
                model.isolated_points.push(removed.unwrap());
            }
            model.points_removed += 20;
            model.points.choose(&mut thread_rng()).unwrap().borrow_mut().activated = true;
        } else {
            // Activate a new point
            model.points.choose(&mut thread_rng()).unwrap().borrow_mut().activated = true;
        }
        
    }

    // Modulate force strength so that it increases for a split second at an interval.
    // model.force_strength = ((app.elapsed_frames() as f32 * 0.06).sin() * 0.5 - 0.25).max(0.0);

    // Pull the shape apart in a few seconds
    // model.force_strength = (app.elapsed_frames() as f32 * 0.004 - (PI/2.0)).sin() * 0.08 + 0.08;
    // model.force_strength = (app.elapsed_frames() as f32 * 0.04 - (PI / 2.0)).sin() * 0.08 + 0.08;

    model.force_strength = 0.5 + (app.elapsed_frames() as f32 * 0.013 - (PI / 2.0)).sin() * 0.15;
    // model.friction = 0.6 + (app.elapsed_frames() as f32 * 0.018).sin() * 0.1;

    // add lines to points without neighbours
    for p in &model.points {
        let find_new_neighbours = p.borrow().lines.len() < p.borrow().max_lines
            && p.borrow().activated;
        if find_new_neighbours {
            let min_dist = 1000000.0;
            let mut closest_neighbour: Option<Rc<RefCell<LinePoint>>> = None;
            let pos = p.borrow().pos.clone();
            for np in &model.points {
                if np.borrow().lines.len() < np.borrow().max_lines
                    && !np.borrow().lines.contains(p)
                    && !p.borrow().lines.contains(np)
                {
                    let dist = np.borrow().pos.distance2(pos);
                    if dist < min_dist && dist > 0.0 && dist < MAX_LINE_LENGTH2 {
                        closest_neighbour = Some(Rc::clone(&np));
                    }
                }
            }
            if let Some(point_rc) = closest_neighbour {
                // add as a line to both points
                point_rc.borrow_mut().activated = true;
                p.borrow_mut().lines.push(point_rc);
                // p.borrow().trigger_sound(&mut model.audio_interface);
                // break;
            }
        }
    }
    

    // Update all points.
    for p in &model.points {
        p.borrow_mut().max_lines = model.max_lines;
        p.borrow_mut().update(
            &app.window_rect(),
            model.friction,
            model.force_strength,
        ); // &mut model.audio_interface);
    }

    // Remove expired points.
    model.points.retain(|x| x.borrow().lifetime > 0);

    // model.audio_interface.update();
    // println!("fps: {}, points: {}", app.fps(), model.points.len());
}

fn window_event(_app: &App, model: &mut Model, event: WindowEvent) {
    match event {
        KeyPressed(key) => {
            match key {
                Key::R => {
                    for p in &model.points {
                        p.borrow_mut().lines.clear();
                    }
                }
                Key::Space => {
                    // let i = model.audio_interface.get_new_sine();
                    // println!("Setting sine no {}", i);
                    // model.audio_interface.set_sine_amp(i, 0.1);
                    // model.audio_interface.set_sine_freq(i, i as f64 * 50.0 + 50.0);
                }
                Key::G => {
                    model.show_gui = !model.show_gui;
                }
                Key::C => {
                    model.generate_points();
                    let num_points_to_remove = (thread_rng().gen::<f32>() * model.points.len() as f32) as usize;
                    for _ in 0..num_points_to_remove {
                        model.points.pop();
                    }
                }
                _ => (),
            }
        }
        KeyReleased(_key) => {}
        MouseMoved(_pos) => {}
        MousePressed(_button) => {}
        MouseReleased(_button) => {}
        MouseEntered => {}
        MouseExited => {}
        MouseWheel(_amount, _phase) => {}
        Moved(_pos) => {}
        Resized(_size) => {}
        Touch(_touch) => {}
        TouchPressure(_pressure) => {}
        HoveredFile(_path) => {}
        DroppedFile(_path) => {}
        HoveredFileCancelled => {}
        Focused => {}
        Unfocused => {}
        Closed => {}
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    // Prepare to draw.
    let draw = app.draw();
    // Beating of the heart.
    let angle = app.elapsed_frames() as f32 * 0.06;
    let noise = nannou::noise::Perlin::new().set_seed(42);
    let beating: f32 = noise.get([angle as f64 * 0.2, model.shape_angle as f64 * 0.1]) as f32;
    let beating2: f32 = noise.get([angle as f64 * 0.6, model.shape_angle as f64 * 0.5]) as f32;
    let scale = 1.0;
    // Clear the background to pink.
    let hue = 0.05;
    let lightness = 0.0;

    draw.background().color(hsl(hue, 0.5, 0.0 + lightness));
    for p in &model.points {
        let vel = p.borrow().vel;
        let activated = p.borrow().activated;
        let lifeforce = p.borrow().lifetime as f32 / LIFETIME as f32;
        let lifeforce = (lifeforce * PI).sin();
        let radius = if activated {
            lifeforce * 5.0
        } else {
            2.0
        };
        let pos = p.borrow().pos;
        let color = if activated {
            hsla(0.4, 0.95, 0.4, 0.5 * lifeforce)
        } else {
            hsla(0.4, 0.2, 0.4, 0.1)
        };
        // draw points
        draw.ellipse().xy(pos).radius(radius).color(color);
        // draw lines
        for np in &p.borrow().lines {
            draw.line()
                .points(pos * scale, np.borrow().pos * scale)
                .color(color)
                .weight(2.0);
        }
    }

    // for p in &model.isolated_points {
    //     let radius = 5.0;
    //     let color = hsla(0.4, 0.5, 0.2, 0.1);
    //     let pos = p.borrow().pos;
    //     draw.ellipse().xy(pos).radius(radius).color(color);
    // }

    // Draw text
    // How many points have been infected
    // How many points were "isolated" model.isolated_points.len()
    // draw.text()
    // Write to the window frame.
    draw.to_frame(app, &frame).unwrap();

    // Draw the state of the `Ui` to the frame.
    if model.show_gui {
        model.ui.draw_to_frame(app, &frame).unwrap();
    }

    // Capture the frame!
    //
    // NOTE: You can speed this up with `capture_frame_threaded`, however be aware that if the
    // image writing threads can't keep up you may quickly begin to run out of RAM!
    if RENDER {
        let file_path = captured_frame_path(app, &frame);
        app.main_window().capture_frame(file_path);
    }
}

fn captured_frame_path(app: &App, frame: &Frame) -> std::path::PathBuf {
    // Create a path that we want to save this frame to.
    app.project_path()
        .expect("failed to locate `project_path`")
        // Capture all frames to a directory called `/<path_to_nannou>/nannou/simple_capture`.
        .join(app.exe_name().unwrap())
        .join("render")
        // Name each file after the number of the frame.
        .join(frame.nth().to_string())
        // The extension will be PNG. We also support tiff, bmp, gif, jpeg, webp and some others.
        .with_extension("png")
}
