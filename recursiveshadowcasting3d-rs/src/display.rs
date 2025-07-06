use std::{cmp::min, f32::INFINITY};

use godot::{classes::Camera3D, obj::WithBaseField, prelude::*};
use itertools::Itertools;
use ndarray::{Array2, Array3};

use crate::debug_line_3d::DebugLine3D;

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
    main_camera: OnEditor<Gd<Camera3D>>,
    #[export]
    debug_line_scene: OnEditor<Gd<PackedScene>>,
    pub occluded: Array3<bool>,
}

#[godot_api]
impl INode3D for Display {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            main_camera: OnEditor::default(),
            debug_line_scene: OnEditor::default(),
            occluded: Array3::from_elem((100, 100, 100), false),
        }
    }

    fn ready(&mut self) {
        let initial_slope_rect = SlopeRect {
            sx: INFINITY,
            sy: INFINITY,
            ex: 1.0,
            ey: 1.0,
        };
        let mut visited_buffer_2d = Array2::from_elem((MAX_DEPTH, MAX_DEPTH), false);
        cast_light(self, &initial_slope_rect, 1, &mut visited_buffer_2d);
    }
}

#[godot_api]
impl Display {
    #[func]
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

    #[func]
    pub fn draw_debug_rect(
        &mut self,
        plane: CoordinatePlane3D,
        depth: f32,
        sx: f32,
        sy: f32,
        ex: f32,
        ey: f32,
        color: Color,
    ) {
        match plane {
            CoordinatePlane3D::XY => {
                self.draw_debug_line(sx, sy, depth, ex, sy, depth, color);
                self.draw_debug_line(sx, sy, depth, sx, ey, depth, color);
                self.draw_debug_line(ex, sy, depth, ex, ey, depth, color);
                self.draw_debug_line(sx, ey, depth, ex, ey, depth, color);
            }
            CoordinatePlane3D::YZ => {
                self.draw_debug_line(depth, sx, sy, depth, ex, sy, color);
                self.draw_debug_line(depth, sx, sy, depth, sx, ey, color);
                self.draw_debug_line(depth, ex, sy, depth, ex, ey, color);
                self.draw_debug_line(depth, sx, ey, depth, ex, ey, color);
            }
            CoordinatePlane3D::XZ => {
                self.draw_debug_line(sx, depth, sy, ex, depth, sy, color);
                self.draw_debug_line(sx, depth, sy, sx, depth, ey, color);
                self.draw_debug_line(ex, depth, sy, ex, depth, ey, color);
                self.draw_debug_line(sx, depth, ey, ex, depth, ey, color);
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

const MAX_DEPTH: usize = 5;

struct SlopeRect {
    sx: f32,
    sy: f32,
    ex: f32,
    ey: f32,
}

fn cast_light(
    display: &mut Display,
    slope_rect: &SlopeRect,
    depth: usize,
    // Buffer for storing visited cells in the rectangle difference subroutine,
    // reused to prevent reallocation
    visited_buffer_2d: &mut Array2<bool>,
) {
    if depth > MAX_DEPTH {
        return;
    }

    // Calculate start and end slopes at this depth
    let sx_at_depth = (depth as f32) / slope_rect.sx;
    let sy_at_depth = (depth as f32) / slope_rect.sy;
    let ex_at_depth = (depth as f32) / slope_rect.ex;
    let ey_at_depth = (depth as f32) / slope_rect.ey;

    display.draw_debug_rect(
        CoordinatePlane3D::XY,
        depth as f32,
        sx_at_depth,
        sy_at_depth,
        ex_at_depth,
        ey_at_depth,
        Color::CYAN,
    );

    // Convert slopes to indices
    let s_ix = sx_at_depth.floor() as usize;
    let e_ix = ex_at_depth.ceil() as usize;
    let s_iy = sy_at_depth.floor() as usize;
    let e_iy = ey_at_depth.ceil() as usize;

    // Draw rectangle representing start and end ranges checked
    display.draw_debug_rect(
        CoordinatePlane3D::XY,
        depth as f32 + 0.01,
        s_ix as f32,
        s_iy as f32,
        e_ix as f32,
        e_iy as f32,
        Color::RED,
    );

    // clear the part of the visited buffer being used
    for index in (s_ix..=e_ix).cartesian_product(s_iy..=e_iy) {
        *visited_buffer_2d.get_mut(index).expect("Out of bounds") = false;
    }

    // Find the difference between the rectangular region and all occluded cells within it,
    // decomposed into a "significantly" (optimal is NP hard) small set of rectangular regions.
    // let mut rectangles: Vec<SlopeRect> = Vec::new();

    // loop exhaustively
    for x in s_ix..=e_ix {
        for y in s_iy..=e_iy {
            // skip visited and occluded cells
            if is_visited_or_occluded(&visited_buffer_2d, &display.occluded, x, y, depth) {
                continue;
            }

            // Now, extend a rectangle into non-blocked and non-visited cells
            let (rect_end_x, rect_end_y) = grow_rectangle_and_visit(
                visited_buffer_2d,
                &display.occluded,
                x,
                y,
                e_ix,
                e_iy,
                depth,
            );

            // Stop checking more start points if we ended at the end corner of our rectangle bounds
            // TODO

            // Convert indices to slopes

            // Bounded by obstruction: more permissive
            // Bounded by start of rect: less permissive, keep old

            let mut new_sx = slope_rect.sx;
            if x != s_ix {
                new_sx = (depth as f32 - 0.5) / (x as f32 - 0.5);
            }

            let mut new_sy = slope_rect.sy;
            if y != s_iy {
                new_sy = (depth as f32 - 0.5) / (y as f32 - 0.5);
            }

            // Bounded by obstruction: less permissive
            // Bounded by end of rect: more permissive
            let mut new_ex = (depth as f32 + 0.5) / (rect_end_x as f32 + 0.5);
            if rect_end_x == e_ix {
                new_ex = (depth as f32 - 0.5) / (rect_end_x as f32 - 0.5);
            }

            let mut new_ey = (depth as f32 + 0.5) / (rect_end_y as f32 + 0.5);
            if rect_end_y == e_iy {
                new_ey = (depth as f32 - 0.5) / (rect_end_y as f32 - 0.5);
            }

            let new_sx_at_depth = (depth as f32 + 0.5) / new_sx;
            let new_sy_at_depth = (depth as f32 + 0.5) / new_sy;
            let new_ex_at_depth = (depth as f32 + 0.5) / new_ex;
            let new_ey_at_depth = (depth as f32 + 0.5) / new_ey;

            // Draw rectangle representing start and end slopes
            display.draw_debug_rect(
                CoordinatePlane3D::XY,
                depth as f32 + 0.5,
                new_sx_at_depth,
                new_sy_at_depth,
                new_ex_at_depth,
                new_ey_at_depth,
                Color::GREEN,
            );
            display.draw_debug_line(
                0.0,
                0.0,
                0.0,
                new_sx_at_depth,
                new_sy_at_depth,
                depth as f32 + 0.5,
                Color::GREEN,
            );
            display.draw_debug_line(
                0.0,
                0.0,
                0.0,
                new_sx_at_depth,
                new_ey_at_depth,
                depth as f32 + 0.5,
                Color::GREEN,
            );
            display.draw_debug_line(
                0.0,
                0.0,
                0.0,
                new_ex_at_depth,
                new_sy_at_depth,
                depth as f32 + 0.5,
                Color::GREEN,
            );
            display.draw_debug_line(
                0.0,
                0.0,
                0.0,
                new_ex_at_depth,
                new_ey_at_depth,
                depth as f32 + 0.5,
                Color::GREEN,
            );

            cast_light(
                display,
                &SlopeRect {
                    sx: new_sx,
                    sy: new_sy,
                    ex: new_ex,
                    ey: new_ey,
                },
                depth + 1,
                visited_buffer_2d,
            );
        }
    }
}

// Precondition: the cell at (x, y, depth) is not occluded or visited
// Return: endpoints of the grown rectangle
fn grow_rectangle_and_visit(
    visited: &mut Array2<bool>,
    occluded: &Array3<bool>,
    start_x: usize,
    start_y: usize,
    max_x: usize,
    max_y: usize,
    depth: usize,
) -> (usize, usize) {
    #[derive(PartialEq, Eq)]
    enum ObstructionResult {
        NoObstruction,
        ObstructedX,
        ObstructedY,
    }

    let mut obstruction_result = ObstructionResult::NoObstruction;
    let mut edge_x = start_x;
    let mut edge_y = start_y;

    // Alternate increasing x and y until some boundary is reached
    for offset in 1..=min(max_x - start_x, max_y - start_y) {
        let end_x = start_x + offset;
        let end_y = start_y + offset;

        // Check if we can extend in the y direction
        for x in start_x..=end_x {
            if is_visited_or_occluded(visited, occluded, x, end_y, depth) {
                obstruction_result = ObstructionResult::ObstructedY;
                break;
            }
        }

        // If possible, extend into the y direction
        if obstruction_result == ObstructionResult::NoObstruction {
            for x in start_x..=end_x {
                *visited.get_mut((x, end_y)).expect("Out of bounds") = true;
            }
            edge_y = end_y;
        }

        // Small optimization: the corner cell has already been checked
        let end_y = end_y - 1;

        // Check if we can extend in the x direction
        for y in start_y..=end_y {
            if is_visited_or_occluded(visited, occluded, end_x, y, depth) {
                obstruction_result = ObstructionResult::ObstructedX;
                break;
            }
        }

        // If possible, extend into the x direction
        if obstruction_result == ObstructionResult::NoObstruction {
            for y in start_y..=end_y {
                *visited.get_mut((end_x, y)).expect("Out of bounds") = true;
            }
            edge_x = end_x;
        }
    }

    // Handle the case of reaching the bounds of the total rectangular region
    obstruction_result = match (edge_x == max_x, edge_y == max_y) {
        (true, true) => ObstructionResult::NoObstruction,
        (true, false) => ObstructionResult::ObstructedX,
        (false, true) => ObstructionResult::ObstructedY,
        (false, false) => obstruction_result,
    };

    // No obstructions encountered: return endpoints
    if obstruction_result == ObstructionResult::NoObstruction {
        return (edge_x, edge_y);
    }
    // Obstruction in the x direction: grow in y direction
    else if obstruction_result == ObstructionResult::ObstructedX {
        let end_x = edge_x;

        // Grow in the y direction until an obstruction is hit, or we reach bounds
        for y in edge_y.clone() + 1..=max_y {
            // Check if we can extend in the y direction
            for x in start_x..=end_x {
                if is_visited_or_occluded(visited, occluded, x, y, depth) {
                    return (edge_x, edge_y);
                }
            }

            // If possible, extend into the y direction
            for x in start_x..=end_x {
                *visited.get_mut((x, y)).expect("Out of bounds") = true;
            }
            edge_y = y;
        }

        // Grew maximally in the y direction
        return (edge_x, edge_y);
    }
    // Obstruction in the y direction: grow in x direction
    else if obstruction_result == ObstructionResult::ObstructedY {
        let end_y = edge_y;

        // Grow in the x direction until an obstruction is hit, or we reach bounds
        for x in edge_x.clone() + 1..=max_x {
            // Check if we can extend in the x direction
            for y in start_y..=end_y {
                if is_visited_or_occluded(visited, occluded, x, y, depth) {
                    return (edge_x, edge_y);
                }
            }

            // If possible, extend into the x direction
            for y in start_y..=end_y {
                *visited.get_mut((x, y)).expect("Out of bounds") = true;
            }
            edge_x = x;
        }

        // Grew maximally in the x direction
        return (edge_x, edge_y);
    }
    // I didn't want to use a match statement, because it indents too much :)
    else {
        panic!("Good luck getting here");
    }
}

fn is_visited_or_occluded(
    visited: &Array2<bool>,
    occluded: &Array3<bool>,
    x: usize,
    y: usize,
    depth: usize,
) -> bool {
    // check visited first
    if *visited.get((x, y)).expect("Out of bounds") {
        return true;
    }

    // TODO: coordinate transformation here
    // let index_3d = transformed(coords, transformation)

    // skip occluded
    if *occluded.get((x, y, depth)).expect("Out of bounds") {
        return true;
    }

    false
}
