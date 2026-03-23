use std::cell::RefCell;
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Sub, SubAssign};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    window, CanvasRenderingContext2d, Event, HtmlCanvasElement, KeyboardEvent,
};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let win = window().ok_or_else(|| JsValue::from_str("window not available"))?;
    let doc = win
        .document()
        .ok_or_else(|| JsValue::from_str("document not available"))?;

    let canvas = doc
        .get_element_by_id("sim-canvas")
        .ok_or_else(|| JsValue::from_str("canvas #sim-canvas not found"))?
        .dyn_into::<HtmlCanvasElement>()?;

    let ctx = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("2d context not available"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    let app = Rc::new(RefCell::new(App::new(win, canvas, ctx)));

    setup_resize_listener(app.clone())?;
    setup_keyboard_listeners(app.clone())?;

    app.borrow_mut().resize()?;
    start_animation_loop(app)?;

    Ok(())
}

fn setup_resize_listener(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    let win = window().ok_or_else(|| JsValue::from_str("window not available"))?;
    let closure = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_event: Event| {
        let _ = app.borrow_mut().resize();
    }));

    win.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

fn setup_keyboard_listeners(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    let win = window().ok_or_else(|| JsValue::from_str("window not available"))?;

    let on_key_down = {
        let app = app.clone();
        Closure::<dyn FnMut(KeyboardEvent)>::wrap(Box::new(move |event: KeyboardEvent| {
            let code = event.code();
            let handled = is_control_key(&code);
            if handled {
                event.prevent_default();
            }

            let mut app = app.borrow_mut();
            match code.as_str() {
                "KeyW" => app.input.forward = true,
                "KeyS" => app.input.back = true,
                "KeyA" => app.input.left = true,
                "KeyD" => app.input.right = true,
                "KeyQ" => app.input.yaw_left = true,
                "KeyE" => app.input.yaw_right = true,
                "KeyR" => app.input.throttle_up = true,
                "KeyF" => app.input.throttle_down = true,
                "Space" if !event.repeat() => app.drone.reset(),
                "KeyP" if !event.repeat() => app.paused = !app.paused,
                _ => {}
            }
        }))
    };

    let on_key_up = Closure::<dyn FnMut(KeyboardEvent)>::wrap(Box::new(move |event: KeyboardEvent| {
        let code = event.code();
        if is_control_key(&code) {
            event.prevent_default();
        }

        let mut app = app.borrow_mut();
        match code.as_str() {
            "KeyW" => app.input.forward = false,
            "KeyS" => app.input.back = false,
            "KeyA" => app.input.left = false,
            "KeyD" => app.input.right = false,
            "KeyQ" => app.input.yaw_left = false,
            "KeyE" => app.input.yaw_right = false,
            "KeyR" => app.input.throttle_up = false,
            "KeyF" => app.input.throttle_down = false,
            _ => {}
        }
    }));

    win.add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref())?;
    win.add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref())?;
    on_key_down.forget();
    on_key_up.forget();
    Ok(())
}

fn start_animation_loop(app: Rc<RefCell<App>>) -> Result<(), JsValue> {
    let callback_holder: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let loop_ref = callback_holder.clone();

    *loop_ref.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp_ms: f64| {
        app.borrow_mut().tick(timestamp_ms);

        if let Some(callback) = callback_holder.borrow().as_ref() {
            let _ = request_animation_frame(callback);
        }
    }) as Box<dyn FnMut(f64)>));

    if let Some(callback) = loop_ref.borrow().as_ref() {
        request_animation_frame(callback)?;
    }

    Ok(())
}

fn request_animation_frame(callback: &Closure<dyn FnMut(f64)>) -> Result<i32, JsValue> {
    window()
        .ok_or_else(|| JsValue::from_str("window not available"))?
        .request_animation_frame(callback.as_ref().unchecked_ref())
}

fn is_control_key(code: &str) -> bool {
    matches!(
        code,
        "KeyW"
            | "KeyS"
            | "KeyA"
            | "KeyD"
            | "KeyQ"
            | "KeyE"
            | "KeyR"
            | "KeyF"
            | "KeyP"
            | "Space"
    )
}

struct App {
    window: web_sys::Window,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
    last_timestamp_ms: f64,
    paused: bool,
    input: InputState,
    drone: DroneState,
    camera_pos: Vec3,
    camera_target: Vec3,
}

impl App {
    fn new(window: web_sys::Window, canvas: HtmlCanvasElement, ctx: CanvasRenderingContext2d) -> Self {
        let drone = DroneState::default();
        let camera_target = drone.position + Vec3::new(0.0, 0.9, 0.0);
        let camera_pos = drone.position + Vec3::new(-8.0, 4.4, -8.5);

        Self {
            window,
            canvas,
            ctx,
            width: 1280.0,
            height: 720.0,
            last_timestamp_ms: 0.0,
            paused: false,
            input: InputState::default(),
            drone,
            camera_pos,
            camera_target,
        }
    }

    fn resize(&mut self) -> Result<(), JsValue> {
        let width = self
            .window
            .inner_width()?
            .as_f64()
            .ok_or_else(|| JsValue::from_str("failed to read innerWidth"))?;
        let height = self
            .window
            .inner_height()?
            .as_f64()
            .ok_or_else(|| JsValue::from_str("failed to read innerHeight"))?;

        self.width = width.max(640.0);
        self.height = height.max(480.0);
        self.canvas.set_width(self.width as u32);
        self.canvas.set_height(self.height as u32);
        Ok(())
    }

    fn tick(&mut self, timestamp_ms: f64) {
        if self.last_timestamp_ms == 0.0 {
            self.last_timestamp_ms = timestamp_ms;
        }

        let dt = ((timestamp_ms - self.last_timestamp_ms) / 1000.0).clamp(0.0, 0.05);
        self.last_timestamp_ms = timestamp_ms;

        if !self.paused {
            self.update(dt);
        }
        self.render();
    }

    fn update(&mut self, dt: f64) {
        if self.input.throttle_up {
            self.drone.throttle += 0.42 * dt;
        }
        if self.input.throttle_down {
            self.drone.throttle -= 0.42 * dt;
        }
        self.drone.throttle = self.drone.throttle.clamp(0.38, 0.95);

        let desired_roll = signed_axis(self.input.left, self.input.right) * 0.48;
        let desired_pitch = signed_axis(self.input.forward, self.input.back) * 0.38;
        let desired_yaw_rate = signed_axis(self.input.yaw_right, self.input.yaw_left) * 1.6;

        let roll_acc = 8.0 * (desired_roll - self.drone.roll) - 3.1 * self.drone.roll_rate;
        let pitch_acc = 8.0 * (desired_pitch - self.drone.pitch) - 3.1 * self.drone.pitch_rate;
        let yaw_acc = 5.0 * (desired_yaw_rate - self.drone.yaw_rate) - 1.8 * self.drone.yaw_rate;

        self.drone.roll_rate += roll_acc * dt;
        self.drone.pitch_rate += pitch_acc * dt;
        self.drone.yaw_rate += yaw_acc * dt;

        self.drone.roll += self.drone.roll_rate * dt;
        self.drone.pitch += self.drone.pitch_rate * dt;
        self.drone.yaw = wrap_angle(self.drone.yaw + self.drone.yaw_rate * dt);

        let roll_mix = (roll_acc * 0.04).clamp(-0.2, 0.2);
        let pitch_mix = (pitch_acc * 0.04).clamp(-0.2, 0.2);
        let yaw_mix = (desired_yaw_rate * 0.045).clamp(-0.08, 0.08);

        self.drone.motors = [
            (self.drone.throttle + pitch_mix + roll_mix - yaw_mix).clamp(0.0, 1.0),
            (self.drone.throttle + pitch_mix - roll_mix + yaw_mix).clamp(0.0, 1.0),
            (self.drone.throttle - pitch_mix + roll_mix + yaw_mix).clamp(0.0, 1.0),
            (self.drone.throttle - pitch_mix - roll_mix - yaw_mix).clamp(0.0, 1.0),
        ];

        let up_dir = rotate_vec(Vec3::Y, self.drone.roll, self.drone.pitch, self.drone.yaw);
        let total_thrust = self.drone.motors.iter().sum::<f64>() * self.drone.max_motor_force;
        let gravity = Vec3::new(0.0, -9.81 * self.drone.mass, 0.0);
        let linear_drag = self.drone.velocity * -0.55;

        let force = up_dir * total_thrust + gravity + linear_drag;
        let acceleration = force / self.drone.mass;

        self.drone.velocity += acceleration * dt;
        self.drone.position += self.drone.velocity * dt;

        if self.drone.position.y < 0.22 {
            self.drone.position.y = 0.22;
            if self.drone.velocity.y < 0.0 {
                self.drone.velocity.y *= -0.12;
            }
            self.drone.velocity.x *= 0.92;
            self.drone.velocity.z *= 0.92;
            self.drone.roll_rate *= 0.8;
            self.drone.pitch_rate *= 0.8;
        }

        let world_limit = 70.0;
        if self.drone.position.x.abs() > world_limit {
            self.drone.position.x = self.drone.position.x.clamp(-world_limit, world_limit);
            self.drone.velocity.x *= -0.25;
        }
        if self.drone.position.z.abs() > world_limit {
            self.drone.position.z = self.drone.position.z.clamp(-world_limit, world_limit);
            self.drone.velocity.z *= -0.25;
        }

        let camera_offset = rotate_vec(Vec3::new(0.0, 3.6, -11.5), 0.0, 0.0, self.drone.yaw);
        let desired_camera_pos = self.drone.position + camera_offset;
        let desired_camera_target = self.drone.position + Vec3::new(0.0, 0.9, 0.0) + self.drone.velocity * 0.18;

        self.camera_pos = self.camera_pos.lerp(desired_camera_pos, 1.0 - (-3.4 * dt).exp());
        self.camera_target = self
            .camera_target
            .lerp(desired_camera_target, 1.0 - (-5.5 * dt).exp());
    }

    fn render(&self) {
        self.ctx.set_fill_style(&JsValue::from_str("#07131d"));
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);

        let horizon_y = self.height * 0.56;
        self.ctx.set_fill_style(&JsValue::from_str("#111a18"));
        self.ctx
            .fill_rect(0.0, horizon_y, self.width, self.height - horizon_y);

        let camera = Camera::new(self.camera_pos, self.camera_target, self.width, self.height);
        self.draw_ground(&camera);
        self.draw_shadow(&camera);
        self.draw_velocity_vector(&camera);
        self.draw_drone(&camera);
        self.draw_hud();

        if self.paused {
            self.ctx.save();
            self.ctx.set_fill_style(&JsValue::from_str("rgba(5, 10, 18, 0.62)"));
            self.ctx.fill_rect(0.0, 0.0, self.width, self.height);
            self.ctx.set_fill_style(&JsValue::from_str("#ffffff"));
            self.ctx
                .set_font("700 30px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace");
            let _ = self
                .ctx
                .fill_text("ПАУЗА — нажми P, чтобы продолжить", self.width * 0.5 - 250.0, self.height * 0.5);
            self.ctx.restore();
        }
    }

    fn draw_ground(&self, camera: &Camera) {
        let step = 5.0;
        let extent = 60.0;
        let center_x = (self.drone.position.x / step).round() * step;
        let center_z = (self.drone.position.z / step).round() * step;

        self.ctx.save();
        self.ctx.set_line_width(1.0);

        for i in -12..=12 {
            let x = center_x + i as f64 * step;
            let z1 = center_z - extent;
            let z2 = center_z + extent;
            let color = if i == 0 { "rgba(0, 255, 200, 0.35)" } else { "rgba(140, 180, 170, 0.18)" };
            self.ctx.set_stroke_style(&JsValue::from_str(color));
            self.draw_world_line(camera, Vec3::new(x, 0.0, z1), Vec3::new(x, 0.0, z2));
        }

        for i in -12..=12 {
            let z = center_z + i as f64 * step;
            let x1 = center_x - extent;
            let x2 = center_x + extent;
            let color = if i == 0 { "rgba(255, 170, 100, 0.32)" } else { "rgba(140, 180, 170, 0.18)" };
            self.ctx.set_stroke_style(&JsValue::from_str(color));
            self.draw_world_line(camera, Vec3::new(x1, 0.0, z), Vec3::new(x2, 0.0, z));
        }

        self.ctx.restore();
    }

    fn draw_shadow(&self, camera: &Camera) {
        let ground_pos = Vec3::new(self.drone.position.x, 0.02, self.drone.position.z);
        if let Some(center) = camera.project(ground_pos) {
            let radius = (120.0 / center.depth).clamp(8.0, 42.0) * (1.0 + self.drone.position.y * 0.03);
            self.ctx.save();
            self.ctx.set_fill_style(&JsValue::from_str("rgba(0, 0, 0, 0.22)"));
            self.ctx.begin_path();
            let _ = self.ctx.arc(center.x, center.y, radius, 0.0, std::f64::consts::PI * 2.0);
            self.ctx.fill();
            self.ctx.restore();
        }
    }

    fn draw_velocity_vector(&self, camera: &Camera) {
        let speed = self.drone.velocity.length();
        if speed < 0.2 {
            return;
        }

        let start = self.drone.position + Vec3::new(0.0, 0.15, 0.0);
        let end = start + self.drone.velocity * 0.45;

        self.ctx.save();
        self.ctx.set_line_width(2.0);
        self.ctx.set_stroke_style(&JsValue::from_str("rgba(255, 214, 92, 0.9)"));
        self.draw_world_line(camera, start, end);
        self.ctx.restore();
    }

    fn draw_drone(&self, camera: &Camera) {
        let body_half = Vec3::new(0.34, 0.08, 0.24);
        let body_points = [
            Vec3::new(-body_half.x, -body_half.y, -body_half.z),
            Vec3::new(body_half.x, -body_half.y, -body_half.z),
            Vec3::new(body_half.x, body_half.y, -body_half.z),
            Vec3::new(-body_half.x, body_half.y, -body_half.z),
            Vec3::new(-body_half.x, -body_half.y, body_half.z),
            Vec3::new(body_half.x, -body_half.y, body_half.z),
            Vec3::new(body_half.x, body_half.y, body_half.z),
            Vec3::new(-body_half.x, body_half.y, body_half.z),
        ];

        let body_world: Vec<Vec3> = body_points
            .iter()
            .map(|p| self.drone.position + rotate_vec(*p, self.drone.roll, self.drone.pitch, self.drone.yaw))
            .collect();

        let motor_local = [
            Vec3::new(-0.78, 0.0, 0.78),
            Vec3::new(0.78, 0.0, 0.78),
            Vec3::new(-0.78, 0.0, -0.78),
            Vec3::new(0.78, 0.0, -0.78),
        ];

        let motors_world: Vec<Vec3> = motor_local
            .iter()
            .map(|p| self.drone.position + rotate_vec(*p, self.drone.roll, self.drone.pitch, self.drone.yaw))
            .collect();

        let center = self.drone.position;
        let nose = self.drone.position + rotate_vec(Vec3::new(0.0, 0.0, 1.15), self.drone.roll, self.drone.pitch, self.drone.yaw);

        self.ctx.save();
        self.ctx.set_line_width(2.0);

        self.ctx.set_stroke_style(&JsValue::from_str("rgba(178, 247, 255, 0.95)"));
        let body_edges = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        for (a, b) in body_edges {
            self.draw_world_line(camera, body_world[a], body_world[b]);
        }

        self.ctx.set_stroke_style(&JsValue::from_str("rgba(232, 238, 242, 0.92)"));
        for motor in &motors_world {
            self.draw_world_line(camera, center, *motor);
        }

        self.ctx.set_stroke_style(&JsValue::from_str("rgba(255, 153, 92, 0.96)"));
        self.draw_world_line(camera, center, nose);
        self.ctx.restore();

        for (idx, motor) in motors_world.iter().enumerate() {
            if let Some(screen) = camera.project(*motor) {
                let radius = (160.0 / screen.depth).clamp(4.0, 15.0);
                let ring_color = match idx {
                    0 => "rgba(255, 146, 120, 0.95)",
                    1 => "rgba(120, 238, 255, 0.95)",
                    2 => "rgba(255, 146, 120, 0.95)",
                    _ => "rgba(120, 238, 255, 0.95)",
                };
                self.ctx.save();
                self.ctx.set_stroke_style(&JsValue::from_str(ring_color));
                self.ctx.set_line_width(2.0);
                self.ctx.begin_path();
                let _ = self.ctx.arc(screen.x, screen.y, radius, 0.0, std::f64::consts::PI * 2.0);
                self.ctx.stroke();
                self.ctx.restore();
            }
        }

        if let Some(center_screen) = camera.project(center) {
            self.ctx.save();
            self.ctx.set_fill_style(&JsValue::from_str("#ffffff"));
            self.ctx.begin_path();
            let _ = self
                .ctx
                .arc(center_screen.x, center_screen.y, 2.5, 0.0, std::f64::consts::PI * 2.0);
            self.ctx.fill();
            self.ctx.restore();
        }
    }

    fn draw_world_line(&self, camera: &Camera, a: Vec3, b: Vec3) {
        if let (Some(pa), Some(pb)) = (camera.project(a), camera.project(b)) {
            self.ctx.begin_path();
            self.ctx.move_to(pa.x, pa.y);
            self.ctx.line_to(pb.x, pb.y);
            self.ctx.stroke();
        }
    }

    fn draw_hud(&self) {
        let hud_x = 18.0;
        let hud_y = 18.0;
        let hud_w = 420.0;
        let hud_h = 132.0;

        self.ctx.save();
        self.ctx.set_fill_style(&JsValue::from_str("rgba(6, 11, 19, 0.8)"));
        self.ctx.fill_rect(hud_x, hud_y, hud_w, hud_h);
        self.ctx.set_stroke_style(&JsValue::from_str("rgba(125, 226, 240, 0.32)"));
        self.ctx.set_line_width(1.0);
        self.ctx.stroke_rect(hud_x, hud_y, hud_w, hud_h);

        self.ctx.set_fill_style(&JsValue::from_str("#d9f4ff"));
        self.ctx
            .set_font("700 18px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace");
        let _ = self.ctx.fill_text("Rust Web Quadcopter Sim", hud_x + 14.0, hud_y + 26.0);

        self.ctx.set_fill_style(&JsValue::from_str("#b7ced9"));
        self.ctx
            .set_font("13px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace");

        let altitude = (self.drone.position.y - 0.22).max(0.0);
        let speed = self.drone.velocity.length();
        let _ = self.ctx.fill_text(
            &format!(
                "Высота: {:>5.1} м   Скорость: {:>5.1} м/с   Тяга: {:>3.0}%",
                altitude,
                speed,
                self.drone.throttle * 100.0
            ),
            hud_x + 14.0,
            hud_y + 50.0,
        );
        let _ = self.ctx.fill_text(
            &format!(
                "Крен: {:>6.1}°   Тангаж: {:>6.1}°   Рысканье: {:>6.1}°",
                self.drone.roll.to_degrees(),
                self.drone.pitch.to_degrees(),
                self.drone.yaw.to_degrees()
            ),
            hud_x + 14.0,
            hud_y + 70.0,
        );
        let _ = self.ctx.fill_text(
            "W/S тангаж  A/D крен  Q/E рысканье  R/F тяга  P пауза  Space сброс",
            hud_x + 14.0,
            hud_y + 92.0,
        );
        let _ = self.ctx.fill_text(
            "Автостабилизация включена. GitHub Pages-ready.",
            hud_x + 14.0,
            hud_y + 112.0,
        );

        let bar_x = self.width - 155.0;
        let base_y = 24.0;
        let bar_height = 82.0;
        let bar_width = 22.0;
        let labels = ["M1", "M2", "M3", "M4"];

        self.ctx.set_fill_style(&JsValue::from_str("rgba(6, 11, 19, 0.8)"));
        self.ctx.fill_rect(self.width - 174.0, 18.0, 156.0, 116.0);
        self.ctx.set_stroke_style(&JsValue::from_str("rgba(125, 226, 240, 0.32)"));
        self.ctx.stroke_rect(self.width - 174.0, 18.0, 156.0, 116.0);

        for (i, motor) in self.drone.motors.iter().enumerate() {
            let x = bar_x + i as f64 * 31.0;
            let filled = bar_height * motor.clamp(0.0, 1.0);

            self.ctx.set_fill_style(&JsValue::from_str("rgba(255,255,255,0.08)"));
            self.ctx.fill_rect(x, base_y, bar_width, bar_height);

            let fill_color = if i % 2 == 0 {
                "rgba(255, 146, 120, 0.95)"
            } else {
                "rgba(120, 238, 255, 0.95)"
            };
            self.ctx.set_fill_style(&JsValue::from_str(fill_color));
            self.ctx
                .fill_rect(x, base_y + (bar_height - filled), bar_width, filled);

            self.ctx.set_stroke_style(&JsValue::from_str("rgba(255,255,255,0.18)"));
            self.ctx.stroke_rect(x, base_y, bar_width, bar_height);
            self.ctx.set_fill_style(&JsValue::from_str("#d9f4ff"));
            let _ = self.ctx.fill_text(labels[i], x - 1.0, base_y + bar_height + 16.0);
        }

        self.ctx.restore();
    }
}

#[derive(Default)]
struct InputState {
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
    yaw_left: bool,
    yaw_right: bool,
    throttle_up: bool,
    throttle_down: bool,
}

struct DroneState {
    mass: f64,
    max_motor_force: f64,
    position: Vec3,
    velocity: Vec3,
    roll: f64,
    pitch: f64,
    yaw: f64,
    roll_rate: f64,
    pitch_rate: f64,
    yaw_rate: f64,
    throttle: f64,
    motors: [f64; 4],
}

impl Default for DroneState {
    fn default() -> Self {
        let mut drone = Self {
            mass: 1.18,
            max_motor_force: 4.6,
            position: Vec3::new(0.0, 2.6, 0.0),
            velocity: Vec3::ZERO,
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            roll_rate: 0.0,
            pitch_rate: 0.0,
            yaw_rate: 0.0,
            throttle: 0.64,
            motors: [0.64; 4],
        };
        drone.reset();
        drone
    }
}

impl DroneState {
    fn reset(&mut self) {
        self.position = Vec3::new(0.0, 2.6, 0.0);
        self.velocity = Vec3::ZERO;
        self.roll = 0.0;
        self.pitch = 0.0;
        self.yaw = 0.0;
        self.roll_rate = 0.0;
        self.pitch_rate = 0.0;
        self.yaw_rate = 0.0;
        self.throttle = 0.64;
        self.motors = [0.64; 4];
    }
}

struct Camera {
    pos: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    focal: f64,
    center_x: f64,
    center_y: f64,
}

impl Camera {
    fn new(pos: Vec3, target: Vec3, width: f64, height: f64) -> Self {
        let forward = (target - pos).normalized();
        let world_up = Vec3::Y;
        let mut right = world_up.cross(forward).normalized();
        if right.length() < 0.001 {
            right = Vec3::new(1.0, 0.0, 0.0);
        }
        let up = forward.cross(right).normalized();

        Self {
            pos,
            right,
            up,
            forward,
            focal: width.min(height) * 0.92,
            center_x: width * 0.5,
            center_y: height * 0.56,
        }
    }

    fn project(&self, world: Vec3) -> Option<ScreenPoint> {
        let relative = world - self.pos;
        let x = relative.dot(self.right);
        let y = relative.dot(self.up);
        let z = relative.dot(self.forward);

        if z <= 0.12 {
            return None;
        }

        let scale = self.focal / z;
        Some(ScreenPoint {
            x: self.center_x + x * scale,
            y: self.center_y - y * scale,
            depth: z,
        })
    }
}

#[derive(Clone, Copy)]
struct ScreenPoint {
    x: f64,
    y: f64,
    depth: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0 };

    const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let len = self.length();
        if len <= 1e-9 {
            Self::ZERO
        } else {
            self / len
        }
    }

    fn lerp(self, other: Self, t: f64) -> Self {
        self * (1.0 - t) + other * t
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

impl Div<f64> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

fn rotate_vec(v: Vec3, roll: f64, pitch: f64, yaw: f64) -> Vec3 {
    let (sr, cr) = roll.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (sy, cy) = yaw.sin_cos();

    let rolled = Vec3::new(v.x * cr - v.y * sr, v.x * sr + v.y * cr, v.z);
    let pitched = Vec3::new(
        rolled.x,
        rolled.y * cp - rolled.z * sp,
        rolled.y * sp + rolled.z * cp,
    );
    Vec3::new(
        pitched.x * cy + pitched.z * sy,
        pitched.y,
        -pitched.x * sy + pitched.z * cy,
    )
}

fn signed_axis(positive: bool, negative: bool) -> f64 {
    (positive as i8 - negative as i8) as f64
}

fn wrap_angle(angle: f64) -> f64 {
    let mut wrapped = angle;
    while wrapped > std::f64::consts::PI {
        wrapped -= std::f64::consts::PI * 2.0;
    }
    while wrapped < -std::f64::consts::PI {
        wrapped += std::f64::consts::PI * 2.0;
    }
    wrapped
}
