use std::{
    f32::INFINITY,
    ops::{Add, Sub},
    time::Instant,
};

use godot::{obj::WithBaseField, prelude::*};
use ndarray::Array3;

use crate::debug_line_3d::DebugLine3D;

const MAX_DEPTH: usize = 15;

enum UnitPlane3d {
    XY,
    ZY,
    ZX,
}

struct Rect {
    sx: f32,
    sy: f32,
    // represent end, not length
    ex: f32,
    ey: f32,
}

impl Rect {
    fn is_valid(&self) -> bool {
        self.sx < self.ex && self.sy < self.ey
    }

    fn intersects(&self, other: &Rect) -> bool {
        self.sx < other.ex && self.ex > other.sx && self.sy < other.ey && self.ey > other.sy
    }

    fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let result = Rect {
            sx: self.sx.max(other.sx),
            sy: self.sy.max(other.sy),
            ex: self.ex.min(other.ex),
            ey: self.ey.min(other.ey),
        };

        if result.is_valid() {
            Some(result)
        } else {
            None
        }
    }

    fn swap_start_and_end(&self) -> Rect {
        Rect {
            sx: self.ex,
            sy: self.ey,
            ex: self.sx,
            ey: self.sy,
        }
    }

    pub const ZERO: Rect = Rect {
        sx: 0.0,
        sy: 0.0,
        ex: 0.0,
        ey: 0.0,
    };
}

impl Add for Rect {
    type Output = Rect;

    fn add(self, rhs: Self) -> Self::Output {
        Rect {
            sx: self.sx + rhs.sx,
            sy: self.sy + rhs.sy,
            ex: self.ex + rhs.ex,
            ey: self.ey + rhs.ey,
        }
    }
}

impl Sub for Rect {
    type Output = Rect;

    fn sub(self, rhs: Self) -> Self::Output {
        Rect {
            sx: self.sx - rhs.sx,
            sy: self.sy - rhs.sy,
            ex: self.ex - rhs.ex,
            ey: self.ey - rhs.ey,
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct Display {
    base: Base<Node3D>,
    #[export]
    debug_line_scene: OnEditor<Gd<PackedScene>>,
    occluded: Array3<bool>,
    origin: Vector3i,
    origin_float: Vector3,
}

#[godot_api]
impl INode3D for Display {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            debug_line_scene: OnEditor::default(),
            occluded: Array3::from_elem((100, 100, 100), false),
            origin: Vector3i::ZERO,
            origin_float: Vector3::ZERO,
        }
    }
}

#[godot_api]
impl Display {
    pub fn draw_debug_line(
        &mut self,
        sx: f32,
        sy: f32,
        sz: f32,
        ex: f32,
        ey: f32,
        ez: f32,
        color: Color,
    ) {
        let start = Vector3 {
            x: sx,
            y: sy,
            z: sz,
        };
        let end = Vector3 {
            x: ex,
            y: ey,
            z: ez,
        };
        let line = DebugLine3D::new(start, end, &self.debug_line_scene, color);
        self.base_mut()
            .call_deferred("add_child", &[line.to_variant()]);
        // self.base_mut().add_child(&line);
    }

    fn draw_debug_rect_xy(&mut self, depth: f32, rect: &Rect, color: Color) {
        self.draw_debug_line(rect.sx, rect.sy, depth, rect.ex, rect.sy, depth, color);
        self.draw_debug_line(rect.sx, rect.sy, depth, rect.sx, rect.ey, depth, color);
        self.draw_debug_line(rect.ex, rect.sy, depth, rect.ex, rect.ey, depth, color);
        self.draw_debug_line(rect.sx, rect.ey, depth, rect.ex, rect.ey, depth, color);
    }

    fn draw_debug_rect_zy(&mut self, depth: f32, rect: &Rect, color: Color) {
        self.draw_debug_line(depth, rect.sy, rect.sx, depth, rect.sy, rect.ex, color);
        self.draw_debug_line(depth, rect.sy, rect.sx, depth, rect.ey, rect.sx, color);
        self.draw_debug_line(depth, rect.sy, rect.ex, depth, rect.ey, rect.ex, color);
        self.draw_debug_line(depth, rect.ey, rect.sx, depth, rect.ey, rect.ex, color);
    }

    fn draw_debug_rect_zx(&mut self, depth: f32, rect: &Rect, color: Color) {
        self.draw_debug_line(rect.sy, depth, rect.sx, rect.sy, depth, rect.ex, color);
        self.draw_debug_line(rect.sy, depth, rect.sx, rect.ey, depth, rect.sx, color);
        self.draw_debug_line(rect.sy, depth, rect.ex, rect.ey, depth, rect.ex, color);
        self.draw_debug_line(rect.ey, depth, rect.sx, rect.ey, depth, rect.ex, color);
    }

    #[func]
    pub fn set_occluded(&mut self, pos: Vector3i) {
        let index = (pos.x as usize, pos.y as usize, pos.z as usize);
        if let Some(val) = self.occluded.get_mut(index) {
            *val = true;
        } else {
            godot_script_error!("Out of bounds at position {}", pos)
        }
    }

    #[func]
    pub fn set_origin_and_recompute(&mut self, origin: Vector3) {
        // Set origin
        self.origin = origin.cast_int();
        self.origin_float = origin;

        for initial_slope_rect in [
            Rect {
                sx: INFINITY,
                sy: INFINITY,
                ex: 1.0,
                ey: 1.0,
            },
            Rect {
                sx: -1.0,
                sy: INFINITY,
                ex: INFINITY,
                ey: 1.0,
            },
            Rect {
                sx: INFINITY,
                sy: -1.0,
                ex: 1.0,
                ey: INFINITY,
            },
            Rect {
                sx: -1.0,
                sy: -1.0,
                ex: INFINITY,
                ey: INFINITY,
            },
        ] {
            for reverse_z in [false, true] {
                for plane in [UnitPlane3d::XY, UnitPlane3d::ZX, UnitPlane3d::ZY] {
                    // Profile shadowcasting
                    let now = Instant::now();
                    cast_light(self, &initial_slope_rect, 1, false, reverse_z, &plane);
                    let elapsed_time = now.elapsed();
                    println!(
                        "Running cast_light() took {} microseconds.",
                        elapsed_time.as_micros()
                    );

                    // Visualize shadowcasting
                    cast_light(self, &initial_slope_rect, 1, true, reverse_z, &plane);
                }
            }
        }
    }
}

fn cast_light(
    display: &mut Display,
    slope_rect: &Rect,
    depth: usize,
    draw_debug: bool,
    reverse_z: bool,
    plane: &UnitPlane3d,
) {
    if depth > MAX_DEPTH {
        return;
    }

    let origin = match plane {
        UnitPlane3d::XY => display.origin,
        UnitPlane3d::ZY => Vector3i {
            x: display.origin.z,
            y: display.origin.y,
            z: display.origin.x,
        },
        UnitPlane3d::ZX => Vector3i {
            x: display.origin.z,
            y: display.origin.x,
            z: display.origin.y,
        },
    };

    let origin_float = origin.cast_float();

    let z = match reverse_z {
        true => -(depth as i32),
        false => depth as i32,
    };
    let z_f32 = z as f32;

    // Calculate the rectangle encompassing the view at this depth, given our slopes and offset (view rect)
    let z_half_offset = match reverse_z {
        true => 0.5,
        false => -0.5,
    };

    let view_rect = match reverse_z {
        true => Rect {
            ex: ((z_f32 + z_half_offset) / slope_rect.sx) + origin_float.x,
            ey: ((z_f32 + z_half_offset) / slope_rect.sy) + origin_float.y,
            sx: ((z_f32 + z_half_offset) / slope_rect.ex) + origin_float.x,
            sy: ((z_f32 + z_half_offset) / slope_rect.ey) + origin_float.y,
        },
        false => Rect {
            sx: ((z_f32 + z_half_offset) / slope_rect.sx) + origin_float.x,
            sy: ((z_f32 + z_half_offset) / slope_rect.sy) + origin_float.y,
            ex: ((z_f32 + z_half_offset) / slope_rect.ex) + origin_float.x,
            ey: ((z_f32 + z_half_offset) / slope_rect.ey) + origin_float.y,
        },
    };

    // Visualize view rectangle
    if draw_debug {
        match plane {
            UnitPlane3d::XY => display.draw_debug_rect_xy(
                z_f32 + origin_float.z + z_half_offset,
                &view_rect,
                Color::CYAN,
            ),
            UnitPlane3d::ZY => display.draw_debug_rect_zy(
                z_f32 + origin_float.z + z_half_offset,
                &view_rect,
                Color::CYAN,
            ),
            UnitPlane3d::ZX => display.draw_debug_rect_zx(
                z_f32 + origin_float.z + z_half_offset,
                &view_rect,
                Color::CYAN,
            ),
        }
    }

    // Find start and end xy indices which could possibly occlude the view
    let mut s_ix = view_rect.sx.floor() as usize;
    let mut s_iy = view_rect.sy.floor() as usize;

    if s_ix > 0 {
        s_ix = s_ix - 1;
    }
    if s_iy > 0 {
        s_iy = s_iy - 1;
    }

    let e_ix = view_rect.ex.ceil() as usize + 1;
    let e_iy = view_rect.ey.ceil() as usize + 1;

    // Find occluded indices, convert them to rectangles
    let mut occluding_rectangles: Vec<Rect> = Vec::new();
    for x in s_ix..e_ix {
        for y in s_iy..e_iy {
            let mut x_check = 0;
            let mut y_check = 0;
            let mut z_check = 0;
            match plane {
                UnitPlane3d::XY => {
                    x_check = x;
                    y_check = y;
                    z_check = (z + origin.z) as usize;
                },
                UnitPlane3d::ZY => {
                    x_check = (z + origin.z) as usize;
                    y_check = y;
                    z_check = x;
                },
                UnitPlane3d::ZX => {
                    x_check = y;
                    y_check = (z + origin.z) as usize;
                    z_check = x;
                },
            };
            if display
                .occluded
                .get((x_check, y_check, z_check))
                .is_some_and(|occluded| *occluded)
            {
                let rect_occluded = get_cube_occlusion(
                    x as f32,
                    y as f32,
                    z_f32,
                    origin_float,
                    slope_rect,
                    reverse_z,
                );

                if draw_debug {
                    match plane {
                        UnitPlane3d::XY => display.draw_debug_rect_xy(
                            z_f32 + origin_float.z + z_half_offset,
                            &rect_occluded,
                            Color::RED,
                        ),
                        UnitPlane3d::ZY => display.draw_debug_rect_zy(
                            z_f32 + origin_float.z + z_half_offset,
                            &rect_occluded,
                            Color::RED,
                        ),
                        UnitPlane3d::ZX => display.draw_debug_rect_zx(
                            z_f32 + origin_float.z + z_half_offset,
                            &rect_occluded,
                            Color::RED,
                        ),
                    }
                }

                occluding_rectangles.push(rect_occluded);
            }
            // here's where you would put your logic for showing/hiding the object at (x,y,depth)
        }
    }

    // Find the difference between the view rect and these rectangles,
    let unblocked = rectangle_minus_rectangles(view_rect, occluding_rectangles);

    // Convert unblocked rectangles back to slopes, then make recursive calls at next depth
    for rect in unblocked {
        let new_slope_rect = match reverse_z {
            true => Rect {
                ex: (z_f32 + z_half_offset) / (rect.sx - origin_float.x),
                ey: (z_f32 + z_half_offset) / (rect.sy - origin_float.y),
                sx: (z_f32 + z_half_offset) / (rect.ex - origin_float.x),
                sy: (z_f32 + z_half_offset) / (rect.ey - origin_float.y),
            },
            false => Rect {
                sx: (z_f32 + z_half_offset) / (rect.sx - origin_float.x),
                sy: (z_f32 + z_half_offset) / (rect.sy - origin_float.y),
                ex: (z_f32 + z_half_offset) / (rect.ex - origin_float.x),
                ey: (z_f32 + z_half_offset) / (rect.ey - origin_float.y),
            },
        };
        cast_light(
            display,
            &new_slope_rect,
            depth + 1,
            draw_debug,
            reverse_z,
            plane,
        );
    }
}

/// Get occlusion from the front and side sides of a cube from some origin point, at some depth
fn get_cube_occlusion(
    x: f32,
    y: f32,
    z: f32,
    origin: Vector3,
    slope_rect: &Rect,
    reverse_z: bool,
) -> Rect {
    let z_half_offset = match reverse_z {
        true => 0.5,
        false => -0.5,
    };

    let base_occluded = Rect {
        sx: x - 0.5,
        sy: y - 0.5,
        ex: x + 0.5,
        ey: y + 0.5,
    };

    // grow rectangle based on occlusion of cube side-faces
    let mut extra = Rect::ZERO;

    if slope_rect.ex > 0.0 && slope_rect.ex.is_finite() {
        extra.sx = (x + z_half_offset - origin.x) / (z - z_half_offset);
    }
    if slope_rect.ey > 0.0 && slope_rect.ey.is_finite() {
        extra.sy = (y + z_half_offset - origin.y) / (z - z_half_offset);
    }
    if slope_rect.sx < 0.0 && slope_rect.sx.is_finite() {
        extra.ex = (x - z_half_offset - origin.x) / (z - z_half_offset);
    }
    if slope_rect.sy < 0.0 && slope_rect.sy.is_finite() {
        extra.ey = (y - z_half_offset - origin.y) / (z - z_half_offset);
    }

    match reverse_z {
        true => base_occluded + extra.swap_start_and_end(),
        false => base_occluded - extra,
    }
}

/// Boolean difference: remove all rectangles from rectangle
/// The result is decomposed into a *reasonably small* set of rectangles, (since optimally small is NP-hard)
fn rectangle_minus_rectangles(rectangle: Rect, rectangles: Vec<Rect>) -> Vec<Rect> {
    let mut result = vec![rectangle];

    for subtract_rect in rectangles {
        let mut new_result = Vec::new();

        for rect in result {
            if let Some(intersection) = rect.intersection(&subtract_rect) {
                // Split the rectangle around the intersection
                let mut splits = Vec::new();

                // Left part
                if rect.sx < intersection.sx {
                    splits.push(Rect {
                        sx: rect.sx,
                        sy: rect.sy,
                        ex: intersection.sx,
                        ey: rect.ey,
                    });
                }

                // Right part
                if intersection.ex < rect.ex {
                    splits.push(Rect {
                        sx: intersection.ex,
                        sy: rect.sy,
                        ex: rect.ex,
                        ey: rect.ey,
                    });
                }

                // Top part (only the middle section to avoid overlap)
                if rect.sy < intersection.sy {
                    splits.push(Rect {
                        sx: intersection.sx,
                        sy: rect.sy,
                        ex: intersection.ex,
                        ey: intersection.sy,
                    });
                }

                // Bottom part (only the middle section to avoid overlap)
                if intersection.ey < rect.ey {
                    splits.push(Rect {
                        sx: intersection.sx,
                        sy: intersection.ey,
                        ex: intersection.ex,
                        ey: rect.ey,
                    });
                }

                // Add all valid splits
                for split in splits {
                    if split.is_valid() {
                        new_result.push(split);
                    }
                }
            } else {
                // No intersection, keep the rectangle as is
                new_result.push(rect);
            }
        }

        result = new_result;
    }

    result
}
