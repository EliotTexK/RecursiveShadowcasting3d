use std::{f32::INFINITY, time::Instant};

use godot::{obj::WithBaseField, prelude::*};
use ndarray::Array3;

use crate::debug_line_3d::DebugLine3D;

enum Axis3D {
    PX,
    NX,
    PY,
    NY,
    PZ,
    NZ,
}

struct Rect {
    sx: f32,
    sy: f32,
    // represent end, not length
    ex: f32,
    ey: f32,
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct Display {
    base: Base<Node3D>,
    #[export]
    debug_line_scene: OnEditor<Gd<PackedScene>>,
    occluded: Array3<bool>,
    origin: [usize; 3],
    origin_f32: [f32; 3],
}

#[godot_api]
impl INode3D for Display {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            debug_line_scene: OnEditor::default(),
            occluded: Array3::from_elem((100, 100, 100), false),
            origin: [0; 3],
            origin_f32: [0.0; 3],
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

    // Parallel to the XY plane
    fn draw_debug_rect_xy(&mut self, depth: f32, rect: &Rect, color: Color) {
        self.draw_debug_line(rect.sx, rect.sy, depth, rect.ex, rect.sy, depth, color);
        self.draw_debug_line(rect.sx, rect.sy, depth, rect.sx, rect.ey, depth, color);
        self.draw_debug_line(rect.ex, rect.sy, depth, rect.ex, rect.ey, depth, color);
        self.draw_debug_line(rect.sx, rect.ey, depth, rect.ex, rect.ey, depth, color);
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
        self.origin = [origin.x as usize, origin.y as usize, origin.z as usize];
        self.origin_f32 = [origin.x, origin.y, origin.z];

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
            // Profile shadowcasting
            let now = Instant::now();
            cast_light(self, &initial_slope_rect, 1, false);
            let elapsed_time = now.elapsed();
            println!(
                "Running cast_light() took {} microseconds.",
                elapsed_time.as_micros()
            );

            // Visualize shadowcasting
            cast_light(self, &initial_slope_rect, 1, true);
        }
    }
}

const MAX_DEPTH: usize = 15;

fn cast_light(display: &mut Display, slope_rect: &Rect, depth: usize, draw_debug: bool) {
    if depth > MAX_DEPTH {
        return;
    }

    // Calculate offsets
    let x_off = display.origin[0];
    let y_off = display.origin[1];
    let z_off = display.origin[2];

    let x_off_32 = x_off as f32;
    let y_off_32 = y_off as f32;
    let z_off_32 = display.origin_f32[2];
    let depth_32 = depth as f32;

    // Calculate the rectangle encompassing the view at this depth, given our slopes and offset (view rect)
    let view_rect = Rect {
        sx: ((depth_32 - 0.5) / slope_rect.sx) + x_off_32,
        sy: ((depth_32 - 0.5) / slope_rect.sy) + y_off_32,
        ex: ((depth_32 - 0.5) / slope_rect.ex) + x_off_32,
        ey: ((depth_32 - 0.5) / slope_rect.ey) + y_off_32,
    };

    // Visualize view rectangle
    if draw_debug {
        display.draw_debug_rect_xy(depth_32 + z_off_32 - 0.5, &view_rect, Color::CYAN);
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
            if display
                .occluded
                .get((x, y, depth + z_off))
                .is_some_and(|occluded| *occluded)
            {
                let xf = x as f32;
                let yf = y as f32;

                // Grow rectangles to accomodate occlusion by the back sides of objects (treating them as cubes)
                let mut grow_sx = 0.0;
                if slope_rect.ex > 0.0 && slope_rect.ex.is_finite() {
                    grow_sx = (xf - 0.5 - x_off_32) / (depth_32 + 0.5);
                }

                let mut grow_sy = 0.0;
                if slope_rect.ey > 0.0 && slope_rect.ey.is_finite() {
                    grow_sy = (yf - 0.5 - y_off_32) / (depth_32 + 0.5);
                }

                let mut grow_ex = 0.0;
                if slope_rect.sx < 0.0 && slope_rect.sx.is_finite() {
                    grow_ex = (xf + 0.5 - x_off_32) / (depth_32 + 0.5);
                }

                let mut grow_ey = 0.0;
                if slope_rect.sy < 0.0 && slope_rect.sy.is_finite() {
                    grow_ey = (yf + 0.5 - y_off_32) / (depth_32 + 0.5);
                }

                let rect_occluded = Rect {
                    sx: xf - 0.5 - grow_sx,
                    sy: yf - 0.5 - grow_sy,
                    ex: xf + 0.5 - grow_ex,
                    ey: yf + 0.5 - grow_ey,
                };

                if draw_debug {
                    display.draw_debug_rect_xy(
                        depth_32 + z_off_32 - 0.5,
                        &rect_occluded,
                        Color::RED,
                    );
                }

                occluding_rectangles.push(rect_occluded);
            }
            // here's where you would put your logic for showing/hiding the object at (x,y,depth)
        }
    }

    // Find the difference between the view rect and these rectangles,
    // Decomposed into a small *enough* set of rectangles
    let unblocked = rectangle_minus_rectangles(view_rect, occluding_rectangles);

    // Convert unblocked rectangles back to slopes, then make recursive calls at next depth
    for rect in unblocked {
        let new_slope_rect = Rect {
            sx: (depth_32 - 0.5) / (rect.sx - x_off_32),
            sy: (depth_32 - 0.5) / (rect.sy - y_off_32),
            ex: (depth_32 - 0.5) / (rect.ex - x_off_32),
            ey: (depth_32 - 0.5) / (rect.ey - y_off_32),
        };
        cast_light(display, &new_slope_rect, depth + 1, draw_debug);
    }
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
}

// Boolean difference: remove all rectangles from rectangle
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
