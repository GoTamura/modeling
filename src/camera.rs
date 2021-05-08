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
    
    pub fn default() -> Self {
        Self {
            eye: (3.0, 4.0, -6.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
        }
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
    is_shift_pressed: bool,
    is_camera_front_pressed: bool,
    is_camera_right_pressed: bool,
    is_camera_top_pressed: bool,
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
            is_shift_pressed: false,
            is_camera_front_pressed: false,
            is_camera_right_pressed: false,
            is_camera_top_pressed: false,
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
                    VirtualKeyCode::Q | VirtualKeyCode::Space | VirtualKeyCode::Numpad8 => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::R | VirtualKeyCode::Numpad2 => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left | VirtualKeyCode::Numpad4 => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right | VirtualKeyCode::Numpad6 => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::LShift => {
                        self.is_shift_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Numpad1 => {
                        self.is_camera_front_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Numpad3 => {
                        self.is_camera_right_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::Numpad7 => {
                        self.is_camera_top_pressed = is_pressed;
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
                winit::event::MouseScrollDelta::PixelDelta(d) => false,
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position_current = (position.x, position.y);
                false
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let is_pressed = *state == ElementState::Pressed;
                match button {
                    winit::event::MouseButton::Middle => {
                        self.is_middle_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        //if self.is_wheel_scrolled && self.scroll >= 0. && forward_mag > self.scroll {
        if self.is_wheel_scrolled && self.scroll >= 0. {
            camera.eye += forward / 10. * self.scroll;
            self.is_wheel_scrolled = false;
            self.scroll = 0.;
        }

        if self.is_wheel_scrolled && self.scroll < 0. {
            //camera.eye += forward_norm * self.scroll;
            camera.eye += forward / 10. * self.scroll;
            self.is_wheel_scrolled = false;
            self.scroll = 0.;
        }

        if self.is_right_pressed {
            let forward = camera.target - camera.eye;
            let rotate =
                quartanion_matrix(rotate_quartanion(-0.05, cgmath::Vector3::new(0., 1., 0.)));
            camera.eye = camera.target - rotate * forward;
            camera.up = rotate * camera.up;
            camera.up = camera.up.normalize();
        }

        if self.is_left_pressed {
            let forward = camera.target - camera.eye;
            let rotate =
                quartanion_matrix(rotate_quartanion(0.05, cgmath::Vector3::new(0., 1., 0.)));
            camera.eye = camera.target - rotate * forward;
            camera.up = rotate * camera.up;
            camera.up = camera.up.normalize();
        }

        if self.is_up_pressed {
            const SENSITIVITY: f32 = 0.05;
            let forward = camera.target - camera.eye;
            let right = forward.normalize().cross(camera.up);
            let v = rotate_quartanion(SENSITIVITY as f32, right);
            let rotate = quartanion_matrix(v);
            camera.eye = camera.target - rotate * forward;
            camera.up = rotate * camera.up;
            camera.up = camera.up.normalize();
        }

        if self.is_down_pressed {
            const SENSITIVITY: f32 = 0.05;
            let forward = camera.target - camera.eye;
            let right = forward.normalize().cross(camera.up);
            let v = rotate_quartanion(-SENSITIVITY as f32, right);
            let rotate = quartanion_matrix(v);
            camera.eye = camera.target - rotate * forward;
            camera.up = rotate * camera.up;
            camera.up = camera.up.normalize();
        }

        if self.is_camera_front_pressed {
            let forward = camera.target - camera.eye;
            let forward_mag = forward.magnitude();
            camera.eye = cgmath::Point3::new(0., 0., -forward_mag);
            camera.target = cgmath::Point3::new(0., 0., 0.);
            camera.up = cgmath::Vector3::new(0., 1., 0.);
        }

        if self.is_camera_right_pressed {
            let forward = camera.target - camera.eye;
            let forward_mag = forward.magnitude();
            camera.eye = cgmath::Point3::new(-forward_mag, 0., 0.);
            camera.target = cgmath::Point3::new(0., 0., 0.);
            camera.up = cgmath::Vector3::new(0., 2., 0.);
        }

        if self.is_camera_top_pressed {
            let forward = camera.target - camera.eye;
            let forward_mag = forward.magnitude();
            camera.eye = cgmath::Point3::new(0., forward_mag, 0.);
            camera.target = cgmath::Point3::new(0., 0., 0.);
            camera.up = cgmath::Vector3::new(0., 0., 1.);
        }

        if self.is_middle_pressed {
            let cursor_diff = (
                self.cursor_position_current.0 - self.cursor_position_before.0,
                self.cursor_position_current.1 - self.cursor_position_before.1,
            );
            const SENSITIVITY: f32 = 0.007;
            if self.is_shift_pressed {
                let right = forward.normalize().cross(camera.up);
                camera.eye += -right * SENSITIVITY * cursor_diff.0 as f32;
                camera.eye += camera.up * SENSITIVITY * cursor_diff.1 as f32;
                camera.target += -right * SENSITIVITY * cursor_diff.0 as f32;
                camera.target += camera.up * SENSITIVITY * cursor_diff.1 as f32;
            } else {
                let forward = camera.target - camera.eye;
                let right = forward.normalize().cross(camera.up);
                let a = rotate_quartanion(-SENSITIVITY * cursor_diff.1 as f32, right);
                let b = rotate_quartanion(
                    SENSITIVITY * cursor_diff.0 as f32,
                    cgmath::Vector3::new(0., 1., 0.),
                );
                let v = mult_quartanion(a, b);
                let rotate = quartanion_matrix(v);
                camera.eye = camera.target - rotate * forward;
                camera.up = rotate * camera.up;
                camera.up = camera.up.normalize();
            }
        }

        self.cursor_position_before = self.cursor_position_current;
    }
}

pub fn quartanion_matrix(v: cgmath::Vector4<f32>) -> cgmath::Matrix3<f32> {
    let w = v.w;
    let ww = w * w;
    let x = v.x;
    let xx = x * x;
    let y = v.y;
    let yy = y * y;
    let z = v.z;
    let zz = z * z;
    let xy = x * y;
    let xz = x * z;
    let xw = x * w;
    let yz = y * z;
    let yw = y * w;
    let zw = z * w;

    cgmath::Matrix3::new(
        ww + xx - yy - zz,
        2. * (xy - zw),
        2. * (xz + yw),
        2. * (xy + zw),
        ww - xx + yy - zz,
        2. * (yz - xw),
        2. * (xz - yw),
        2. * (yz + xw),
        ww - xx - yy + zz,
    )
}

pub fn rotate_quartanion(t: f32, n: cgmath::Vector3<f32>) -> cgmath::Vector4<f32> {
    let s = f32::sin(t / 2.) * n;
    let c = f32::cos(t / 2.);
    cgmath::Vector4::new(s.x, s.y, s.z, c)
}

pub fn mult_quartanion(a: cgmath::Vector4<f32>, b: cgmath::Vector4<f32>) -> cgmath::Vector4<f32> {
    cgmath::Matrix4::new(
        a.w, -a.z, a.y, a.x, a.z, a.w, -a.x, a.y, -a.y, a.x, a.w, a.z, -a.x, -a.y, -a.z, a.w,
    ) * b
}
