use std::f32::INFINITY;

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

impl Rect {
    fn new(sx: f32, sy: f32, ex: f32, ey: f32) -> Self {
        Rect { sx, sy, ex, ey }
    }

    fn is_empty(&self) -> bool {
        self.sx >= self.ex || self.sy >= self.ey
    }

    fn intersects(&self, other: &Rect) -> bool {
        self.sx < other.ex && other.sx < self.ex && self.sy < other.ey && other.sy < self.ey
    }
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
        let initial_slope_rect = Rect {
            sx: INFINITY,
            sy: INFINITY,
            ex: 1.0,
            ey: 1.0,
        };
        cast_light(self, &initial_slope_rect, 1);
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

const MAX_DEPTH: usize = 5;

fn cast_light(display: &mut Display, slope_rect: &Rect, depth: usize) {
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
    display.draw_debug_rect(
        CoordinatePlane3D::XY,
        depth as f32 - 0.5,
        &view_rect,
        Color::CYAN,
    );
    // display.draw_debug_line(
    //     0.0,
    //     0.0,
    //     0.0,
    //     view_rect.sx,
    //     view_rect.sy,
    //     depth as f32 - 0.5,
    //     Color::CYAN,
    // );
    // display.draw_debug_line(
    //     0.0,
    //     0.0,
    //     0.0,
    //     view_rect.sx,
    //     view_rect.ey,
    //     depth as f32 - 0.5,
    //     Color::CYAN,
    // );
    // display.draw_debug_line(
    //     0.0,
    //     0.0,
    //     0.0,
    //     view_rect.ex,
    //     view_rect.sy,
    //     depth as f32 - 0.5,
    //     Color::CYAN,
    // );
    // display.draw_debug_line(
    //     0.0,
    //     0.0,
    //     0.0,
    //     view_rect.ex,
    //     view_rect.ey,
    //     depth as f32 - 0.5,
    //     Color::CYAN,
    // );

    // Find start and end xy indices which could possibly occlude the view at this depth
    let mut s_ix = view_rect.sx.floor() as usize;
    let mut s_iy = view_rect.sy.floor() as usize;

    if s_ix > 0 {
        s_ix = s_ix - 1;
    }
    if s_iy > 0 {
        s_iy = s_iy - 1;
    }

    let e_ix = view_rect.ex.ceil() as usize;
    let e_iy = view_rect.ey.ceil() as usize;

    

    // Find occluded indices, convert them to rectangles
    let mut occluding_rectangles: Vec<Rect> = Vec::new();
    for x in s_ix..e_ix {
        for y in s_iy..e_iy {
            if display.occluded.get((x, y, depth)).is_some_and(|occluded| *occluded) {
                let xf = x as f32;
                let yf = y as f32;
                let rect_occluded = Rect {
                    // TODO: sx and sy need to be smaller
                    sx: xf - 0.5 - ((xf - 0.5) / (depth as f32 + 0.5)),
                    sy: yf - 0.5 - ((yf - 0.5) / (depth as f32 + 0.5)),
                    ex: xf + 0.5,
                    ey: yf + 0.5,
                };
                display.draw_debug_rect(
                    CoordinatePlane3D::XY,
                    depth as f32 - 0.5,
                    &rect_occluded,
                    Color::RED,
                );
                occluding_rectangles.push(rect_occluded);
            }
            // here's where you would put your 
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
        cast_light(display, &new_slope_rect, depth + 1);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Event {
    x: f32,
    y_start: f32,
    y_end: f32,
    event_type: EventType,
}

#[derive(Debug, Clone, PartialEq)]
enum EventType {
    Enter, // Left edge of rectangle
    Exit,  // Right edge of rectangle
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.x
            .partial_cmp(&other.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                // Process exits before enters at same x coordinate
                match (&self.event_type, &other.event_type) {
                    (EventType::Exit, EventType::Enter) => std::cmp::Ordering::Less,
                    (EventType::Enter, EventType::Exit) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            })
    }
}

impl Eq for Event {}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct Interval {
    start: f32,
    end: f32,
}

impl Interval {
    fn new(start: f32, end: f32) -> Self {
        Interval { start, end }
    }

    fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

fn subtract_intervals(base: &Interval, to_subtract: &[Interval]) -> Vec<Interval> {
    let mut result = vec![base.clone()];

    for sub in to_subtract {
        let mut new_result = Vec::new();

        for interval in result {
            if interval.end <= sub.start || interval.start >= sub.end {
                // No overlap
                new_result.push(interval);
            } else {
                // There's overlap, split the interval
                if interval.start < sub.start {
                    new_result.push(Interval::new(interval.start, sub.start));
                }
                if interval.end > sub.end {
                    new_result.push(Interval::new(sub.end, interval.end));
                }
            }
        }

        result = new_result;
    }

    result
}

fn rectangle_minus_rectangles(rectangle: Rect, rectangles: Vec<Rect>) -> Vec<Rect> {
    if rectangle.is_empty() {
        return vec![];
    }

    // Filter rectangles that actually intersect with the rectangle
    let relevant_rectangles: Vec<_> = rectangles
        .into_iter()
        .filter(|sq| rectangle.intersects(sq))
        .map(|sq| {
            Rect::new(
                sq.sx.max(rectangle.sx),
                sq.sy.max(rectangle.sy),
                sq.ex.min(rectangle.ex),
                sq.ey.min(rectangle.ey),
            )
        })
        .filter(|sq| !sq.is_empty())
        .collect();

    if relevant_rectangles.is_empty() {
        return vec![rectangle];
    }

    // Create events for sweep line
    let mut events = Vec::new();

    for rectangle in &relevant_rectangles {
        events.push(Event {
            x: rectangle.sx,
            y_start: rectangle.sy,
            y_end: rectangle.ey,
            event_type: EventType::Enter,
        });
        events.push(Event {
            x: rectangle.ex,
            y_start: rectangle.sy,
            y_end: rectangle.ey,
            event_type: EventType::Exit,
        });
    }

    events.sort();

    let mut result = Vec::new();
    let mut active_intervals: Vec<Interval> = Vec::new();
    let mut prev_x = rectangle.sx;

    // Process rectangle from start to first event
    if !events.is_empty() && events[0].x > rectangle.sx {
        result.push(Rect::new(
            rectangle.sx,
            rectangle.sy,
            events[0].x,
            rectangle.ey,
        ));
        prev_x = events[0].x;
    }

    for event in events {
        let curr_x = event.x;

        // Generate rectangles for the current active region
        if curr_x > prev_x {
            let base_interval = Interval::new(rectangle.sy, rectangle.ey);
            let free_intervals = subtract_intervals(&base_interval, &active_intervals);

            for interval in free_intervals {
                if !interval.is_empty() {
                    result.push(Rect::new(prev_x, interval.start, curr_x, interval.end));
                }
            }
        }

        // Update active intervals
        match event.event_type {
            EventType::Enter => {
                active_intervals.push(Interval::new(event.y_start, event.y_end));
            }
            EventType::Exit => {
                active_intervals.retain(|interval| {
                    !(interval.start == event.y_start && interval.end == event.y_end)
                });
            }
        }

        prev_x = curr_x;
    }

    // Process rectangle from last event to end
    if prev_x < rectangle.ex {
        let base_interval = Interval::new(rectangle.sy, rectangle.ey);
        let free_intervals = subtract_intervals(&base_interval, &active_intervals);

        for interval in free_intervals {
            if !interval.is_empty() {
                result.push(Rect::new(
                    prev_x,
                    interval.start,
                    rectangle.ex,
                    interval.end,
                ));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_rectangles() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let rectangles = vec![];
        let result = rectangle_minus_rectangles(rect, rectangles);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].sx, 0.0);
        assert_eq!(result[0].sy, 0.0);
        assert_eq!(result[0].ex, 10.0);
        assert_eq!(result[0].ey, 10.0);
    }

    #[test]
    fn test_single_rectangle_center() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let rectangles = vec![Rect::new(4.0, 4.0, 6.0, 6.0)];
        let result = rectangle_minus_rectangles(rect, rectangles);

        // Should produce multiple rectangles around the center rectangle
        assert!(result.len() > 1);

        // Check total area is correct (100 - 4 = 96)
        let total_area: f32 = result.iter().map(|r| (r.ex - r.sx) * (r.ey - r.sy)).sum();
        assert!((total_area - 96.0).abs() < 0.001);
    }

    #[test]
    fn test_non_overlapping_rectangle() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let rectangles = vec![Rect::new(20.0, 20.0, 25.0, 25.0)];
        let result = rectangle_minus_rectangles(rect, rectangles);

        // Should return original rectangle
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].sx, 0.0);
        assert_eq!(result[0].sy, 0.0);
        assert_eq!(result[0].ex, 10.0);
        assert_eq!(result[0].ey, 10.0);
    }

    #[test]
    fn test_multiple_rectangles() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let rectangles = vec![Rect::new(1.0, 1.0, 3.0, 3.0), Rect::new(7.0, 7.0, 9.0, 9.0)];
        let result = rectangle_minus_rectangles(rect, rectangles);

        // Check total area is correct (100 - 4 - 4 = 92)
        let total_area: f32 = result.iter().map(|r| (r.ex - r.sx) * (r.ey - r.sy)).sum();
        assert!((total_area - 92.0).abs() < 0.001);
    }
}
