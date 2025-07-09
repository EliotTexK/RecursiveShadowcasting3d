use std::{f32::INFINITY, time::Instant};

use godot::{obj::WithBaseField, prelude::*};
use ndarray::Array3;

use crate::debug_line_3d::DebugLine3D;

pub struct Rect {
    sx: f32,
    sy: f32,
    // represent end, not length
    ex: f32,
    ey: f32,
}

#[derive(GodotConvert, Var, Export, Debug)]
#[godot(via = GString)]
pub enum CoordinatePlane3D {
    XY,
    YZ,
    XZ,
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct Display {
    base: Base<Node3D>,
    #[export]
    debug_line_scene: OnEditor<Gd<PackedScene>>,
    pub occluded: Array3<bool>,
}

#[godot_api]
impl INode3D for Display {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            debug_line_scene: OnEditor::default(),
            occluded: Array3::from_elem((100, 100, 100), false),
        }
    }

    fn ready(&mut self) {
        // Profile it
        let now = Instant::now();
        let initial_slope_rect = Rect {
            sx: INFINITY,
            sy: INFINITY,
            ex: 1.0,
            ey: 1.0,
        };
        cast_light(self, &initial_slope_rect, 1, false);
        let elapsed_time = now.elapsed();
        println!(
            "Running cast_light() took {} microseconds.",
            elapsed_time.as_micros()
        );

        // Visualize it
        cast_light(self, &initial_slope_rect, 1, true);
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
        self.base_mut().add_child(&line);
    }

    pub fn draw_debug_rect(
        &mut self,
        plane: CoordinatePlane3D,
        depth: f32,
        rect: &Rect,
        color: Color,
    ) {
        match plane {
            CoordinatePlane3D::XY => {
                self.draw_debug_line(rect.sx, rect.sy, depth, rect.ex, rect.sy, depth, color);
                self.draw_debug_line(rect.sx, rect.sy, depth, rect.sx, rect.ey, depth, color);
                self.draw_debug_line(rect.ex, rect.sy, depth, rect.ex, rect.ey, depth, color);
                self.draw_debug_line(rect.sx, rect.ey, depth, rect.ex, rect.ey, depth, color);
            }
            CoordinatePlane3D::YZ => {
                self.draw_debug_line(depth, rect.sx, rect.sy, depth, rect.ex, rect.sy, color);
                self.draw_debug_line(depth, rect.sx, rect.sy, depth, rect.sx, rect.ey, color);
                self.draw_debug_line(depth, rect.ex, rect.sy, depth, rect.ex, rect.ey, color);
                self.draw_debug_line(depth, rect.sx, rect.ey, depth, rect.ex, rect.ey, color);
            }
            CoordinatePlane3D::XZ => {
                self.draw_debug_line(rect.sx, depth, rect.sy, rect.ex, depth, rect.sy, color);
                self.draw_debug_line(rect.sx, depth, rect.sy, rect.sx, depth, rect.ey, color);
                self.draw_debug_line(rect.ex, depth, rect.sy, rect.ex, depth, rect.ey, color);
                self.draw_debug_line(rect.sx, depth, rect.ey, rect.ex, depth, rect.ey, color);
            }
        }
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
}

const MAX_DEPTH: usize = 8;

fn cast_light(display: &mut Display, slope_rect: &Rect, depth: usize, draw_debug: bool) {
    if depth > MAX_DEPTH {
        return;
    }

    // Calculate start and end slopes of the visible rectangle at this depth
    let view_rect = Rect {
        sx: (depth as f32 - 0.5) / slope_rect.sx,
        sy: (depth as f32 - 0.5) / slope_rect.sy,
        ex: (depth as f32 - 0.5) / slope_rect.ex,
        ey: (depth as f32 - 0.5) / slope_rect.ey,
    };

    // Visualize view rectangle at this depth
    if draw_debug {
        display.draw_debug_rect(
            CoordinatePlane3D::XY,
            depth as f32 - 0.5,
            &view_rect,
            Color::CYAN,
        );
    }

    // Find start and end xy indices which could possibly occlude the view at this depth
    let mut s_ix = view_rect.sx.floor() as usize;
    let mut s_iy = view_rect.sy.floor() as usize;

    if s_ix > 0 {
        s_ix = s_ix - 1;
    }
    if s_iy > 0 {
        s_iy = s_iy - 1;
    }

    let mut e_ix = view_rect.ex.ceil() as usize + 1;
    let mut e_iy = view_rect.ey.ceil() as usize + 1;

    // Find occluded indices, convert them to rectangles
    let mut occluding_rectangles: Vec<Rect> = Vec::new();
    for x in s_ix..e_ix {
        for y in s_iy..e_iy {
            if display
                .occluded
                .get((x, y, depth))
                .is_some_and(|occluded| *occluded)
            {
                let xf = x as f32;
                let yf = y as f32;
                let rect_occluded = Rect {
                    // TODO: sx and sy need to be smaller
                    sx: xf - 0.5 - ((xf - 0.5) / (depth as f32 + 0.5)),
                    sy: yf - 0.5 - ((yf - 0.5) / (depth as f32 + 0.5)),
                    ex: xf + 0.5,
                    ey: yf + 0.5,
                };
                if draw_debug {
                    display.draw_debug_rect(
                        CoordinatePlane3D::XY,
                        depth as f32 - 0.5,
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
            sx: (depth as f32 - 0.5) / rect.sx,
            sy: (depth as f32 - 0.5) / rect.sy,
            ex: (depth as f32 - 0.5) / rect.ex,
            ey: (depth as f32 - 0.5) / rect.ey,
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
