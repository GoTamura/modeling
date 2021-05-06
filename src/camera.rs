use winit::event::*;

use cgmath::InnerSpace;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Projection {
    pub aspect: f32,
    pub fovy: cgmath::Rad<f32>,
    pub znear: f32,
    pub zfar: f32,
}

impl Projection {
    pub fn new<F: Into<cgmath::Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> cgmath::Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
}

impl Camera {
    pub fn calc_matrix(&self) -> cgmath::Matrix4<f32> {
        cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up)
    }
}

pub struct CameraController {
    speed: f32,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_middle_pressed: bool,
    is_wheel_scrolled: bool,
    scroll: f32,
    cursor_position_before: (f64, f64),
    cursor_position_current: (f64, f64),
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_middle_pressed: false,
            is_wheel_scrolled: false,
            scroll: 0.,
            cursor_position_before: (0., 0.),
            cursor_position_current: (0., 0.),
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::Q | VirtualKeyCode::Space => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::R | VirtualKeyCode::LShift => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(horizontal, vertical) => {
                    self.scroll = *vertical;
                    self.is_wheel_scrolled = true;
                    true
                }
                winit::event::MouseScrollDelta::PixelDelta(d) => {
                    false
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position_current = (position.x, position.y);
                false
            },
            WindowEvent::MouseInput {
                state,
                button,
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match button {
                    winit::event::MouseButton::Middle => {
                        self.is_middle_pressed = is_pressed;
                        true
                    },
                    _ => false
                }
            }
            _ => false
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        if self.is_wheel_scrolled && self.scroll >= 0. && forward_mag > self.scroll {
            camera.eye += forward_norm * self.scroll;
            self.is_wheel_scrolled = false;
            self.scroll = 0.;
        }

        if self.is_wheel_scrolled && self.scroll < 0. {
            camera.eye += forward_norm * self.scroll;
            self.is_wheel_scrolled = false;
            self.scroll = 0.;
        }

        let right = forward_norm.cross(camera.up);
        let up = forward_norm.cross(right);

        // Redo radius calc in case the up/ down is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and eye so
            // that it doesn't change. The eye therefore still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }

        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }

        if self.is_middle_pressed {
            let cursor_diff = (self.cursor_position_current.0 - self.cursor_position_before.0, self.cursor_position_current.1 - self.cursor_position_before.1);
            camera.eye = camera.target - (forward + right * 0.1 * cursor_diff.0 as f32 + up * 0.1 * cursor_diff.1 as f32).normalize() * forward_mag;
        }

        if self.is_up_pressed {
            camera.eye += camera.up * self.speed;
        }

        if self.is_down_pressed {
            camera.eye -= camera.up * self.speed;
        }
        self.cursor_position_before = self.cursor_position_current;
    }
}
