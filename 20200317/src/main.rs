use nannou::prelude::*;
use nannou::ui::prelude::*;
use nannou::geom::range::Range;

use std::cell::{RefCell};
use std::rc::Rc;
extern crate rand;
use rand::seq::SliceRandom;

mod son;

const MAX_LINE_LENGTH2: f32 = 4000.0;
const MAX_LINES_EVER: usize = 20;
const LIFETIME: i32 = 60 * 8;

struct Ids {
    max_lines: widget::Id,
    friction: widget::Id,
    force_strength: widget::Id,
}

struct LinePoint {
    pos: Point2,
    vel: Vector2,
    sine_i: usize,
    max_lines: usize,
    lines: Vec<Rc<RefCell<LinePoint>>>,
    lifetime: i32,
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
            lines: lines,
            lifetime,
        }
    }

    fn update(&mut self, win_rect: &nannou::geom::rect::Rect, friction: f32, force_strength: f32) {
        self.lifetime -= 1;
        // Remove lines if there are too many.
        if self.lines.len() > self.max_lines {
            for _ in 0..self.lines.len() - self.max_lines {
                self.lines.pop();
            }
        }
        // Remove lines to points that are too far away.
        let local_pos = self.pos;
        self.lines.retain(|x| 
            x.borrow().pos.distance2(local_pos) < MAX_LINE_LENGTH2);
        
        // Move towards connected dots it's far away from and away from close ones.
        self.vel *= friction; // Velocity damping, 0.7 is gooood
        let dist_range = Range::new(0.0, MAX_LINE_LENGTH2);
        let vel_range = Range::new(-force_strength, force_strength);
        for lp in &self.lines {
            let dist2 = lp.borrow().pos.distance2(local_pos);
            let force = dist_range.map_value(dist2, &vel_range);
            self.vel += (lp.borrow().pos - self.pos) * force;
        }

        self.pos += self.vel;

        let speed_range = Range::new(0.0, 10.0);
        let freq_range = Range::new(100.0, 1000.0);
        let freq: f64 = speed_range.map_value(self.vel.distance2(pt2(0.0, 0.0)) as f64, &freq_range);
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
    ui: Ui,
    widget_ids: Ids,
    friction: f32,
    max_lines: usize,
    force_strength: f32,
    show_gui: bool,
}

impl Model {
    
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
        max_lines: ui.generate_widget_id(),
        friction: ui.generate_widget_id(),
        force_strength: ui.generate_widget_id(),
    };

    Model { 
        _window, 
        // audio_interface, 
        points: vec![], 
        ui, widget_ids, 
        friction: 0.7, 
        max_lines: 10, 
        force_strength: 0.0, 
        show_gui: false
    }
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
            .label("Friction")
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
    }

    // Create new point on a heart curve.
    if app.elapsed_frames() % 1 == 0 {
        let frame_num = app.elapsed_frames() as f32;
        let angle = frame_num * 0.06; 
        // Heart curve equation from here: https://mathworld.wolfram.com/HeartCurve.html
        let scale = (frame_num * 0.01).cos() * 82.0 + 100.0;
        let r = 2.0 - 2.0*angle.sin() + (angle.sin() * ( angle.cos().abs().sqrt() / (angle.sin() + 1.4)));
        let new_pos = 
            pt2(angle.cos() * r * scale, (angle.sin() * r + 1.0) * scale);
        // let sine_i = model.audio_interface.get_new_sine();
        let new_point = Rc::new(RefCell::new(LinePoint::new_at(new_pos, 0, LIFETIME)));
        model.points.push(new_point);
    }

    // Modulate force strength so that it increases for a split second at an interval.
    model.force_strength = ((app.elapsed_frames() as f32 * 0.06).sin() * 0.5 - 0.25).max(0.0);

    // add lines to points without neighbours
    for p in &model.points {
        let find_new_neighbours = p.borrow().lines.len() < p.borrow().max_lines;
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
                point_rc.borrow_mut().lines.push(Rc::clone(p));
                p.borrow_mut().lines.push(point_rc);
                // p.borrow().trigger_sound(&mut model.audio_interface);
            }
        }
    }
    

    // Update all points.
    for p in &model.points {
        p.borrow_mut().max_lines = model.max_lines;
        p.borrow_mut().update(&app.window_rect(), model.friction, model.force_strength);// &mut model.audio_interface);
    }

    // Remove expired points.
    model.points.retain(|x| 
        x.borrow().lifetime > 0);
    
    // model.audio_interface.update();
    println!("fps: {}, points: {}", app.fps(), model.points.len());
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
                _ => ()
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
    let beating = (angle.sin() > 0.0) as i32 as f32 * ((angle*2.0).sin().abs() + (angle*2.0).sin() * 0.25);
    let scale = beating * 0.1 + 1.0;
    // Clear the background to pink.
    let hue = (app.elapsed_frames() as f32 * 0.001 + beating * 0.1).sin()* 0.02 + 0.98;
    let lightness = beating * 0.1 + 0.1;
    
    draw.background().color(hsl(hue, 0.95, 0.15 + lightness));
    for p in &model.points {
        let lifeforce = p.borrow().lifetime as f32 / LIFETIME as f32;
        let pos = p.borrow().pos;
        draw.ellipse()
            .xy(pos * scale)
            .radius(3.0)
            .color(hsla(hue, 1.0, 0.2, 0.5 - lifeforce));
        // draw lines
        for np in &p.borrow().lines {
            draw.line()
                .points(pos * scale, np.borrow().pos * scale)
                .color(hsla(hue, 0.95, 0.1 + (1.0 - lifeforce)*0.7, 0.6 * lifeforce - model.force_strength*0.6))
                .weight(3.0);
        }
    }
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
    let file_path = captured_frame_path(app, &frame);
    app.main_window().capture_frame(file_path);
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
