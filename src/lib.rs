use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Sub, SubAssign};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Simulator {
    input: InputState,
    drone: DroneState,
    paused: bool,
}

#[wasm_bindgen]
impl Simulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self {
            input: InputState::default(),
            drone: DroneState::default(),
            paused: false,
        }
    }

    pub fn update(&mut self, dt: f64) {
        let dt = dt.clamp(0.0, 0.05);
        if !self.paused {
            self.step(dt);
        }
    }

    pub fn set_key(&mut self, code: &str, pressed: bool) {
        match code {
            "KeyW" => self.input.forward = pressed,
            "KeyS" => self.input.back = pressed,
            "KeyA" => self.input.left = pressed,
            "KeyD" => self.input.right = pressed,
            "KeyQ" => self.input.throttle_up = pressed,
            "KeyE" => self.input.throttle_down = pressed,
            "KeyR" => self.input.yaw_left = pressed,
            "KeyF" => self.input.yaw_right = pressed,
            "KeyP" if pressed => self.paused = !self.paused,
            "Space" if pressed => self.drone.reset(),
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        self.drone.reset();
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn x(&self) -> f64 {
        self.drone.position.x
    }

    pub fn y(&self) -> f64 {
        self.drone.position.y
    }

    pub fn z(&self) -> f64 {
        self.drone.position.z
    }

    pub fn velocity_x(&self) -> f64 {
        self.drone.velocity.x
    }

    pub fn velocity_y(&self) -> f64 {
        self.drone.velocity.y
    }

    pub fn velocity_z(&self) -> f64 {
        self.drone.velocity.z
    }

    pub fn speed(&self) -> f64 {
        self.drone.velocity.length()
    }

    pub fn roll(&self) -> f64 {
        self.drone.roll
    }

    pub fn pitch(&self) -> f64 {
        self.drone.pitch
    }

    pub fn yaw(&self) -> f64 {
        self.drone.yaw
    }

    pub fn throttle(&self) -> f64 {
        self.drone.throttle
    }

    pub fn motor(&self, index: u32) -> f64 {
        self.drone
            .motors
            .get(index as usize)
            .copied()
            .unwrap_or(0.0)
    }
}

impl Simulator {
    fn step(&mut self, dt: f64) {
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

        if self.drone.position.y < 0.28 {
            self.drone.position.y = 0.28;
            if self.drone.velocity.y < 0.0 {
                self.drone.velocity.y *= -0.12;
            }
            self.drone.velocity.x *= 0.92;
            self.drone.velocity.z *= 0.92;
            self.drone.roll_rate *= 0.8;
            self.drone.pitch_rate *= 0.8;
        }

        let world_limit = 86.0;
        if self.drone.position.x.abs() > world_limit {
            self.drone.position.x = self.drone.position.x.clamp(-world_limit, world_limit);
            self.drone.velocity.x *= -0.25;
        }
        if self.drone.position.z.abs() > world_limit {
            self.drone.position.z = self.drone.position.z.clamp(-world_limit, world_limit);
            self.drone.velocity.z *= -0.25;
        }
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

    fn length(self) -> f64 {
        self.dot(self).sqrt()
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
