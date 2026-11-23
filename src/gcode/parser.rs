//! Modal state machine that turns G-code blocks into cutting geometry.

use std::collections::BTreeSet;

use crate::gcode::arc::center_from_radius;
use crate::gcode::lexer::{
    is_block_deleted, is_percent, is_subroutine_label, strip_comments,
    tokenize, Word,
};
use crate::gcode::GcodeParseOutput;
use crate::geo::geometry::Geometry;
use crate::geo::types::Point;

const INCH_IN_MM: f64 = 25.4;
const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Motion {
    Rapid,
    Linear,
    ArcClockwise,
    ArcCounterClockwise,
}

struct State {
    motion: Motion,
    absolute: bool,
    unit_scale: f64,
    plane_xy: bool,
    arc_center_absolute: bool,
    spindle_speed: Option<f64>,
    x: f64,
    y: f64,
    path_open: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            motion: Motion::Rapid,
            absolute: true,
            unit_scale: 1.0,
            plane_xy: true,
            arc_center_absolute: false,
            spindle_speed: None,
            x: 0.0,
            y: 0.0,
            path_open: false,
        }
    }
}

#[derive(Default)]
struct Axes {
    x: Option<f64>,
    y: Option<f64>,
    i: Option<f64>,
    j: Option<f64>,
    r: Option<f64>,
}

/// Parses one program, collecting cutting geometry and skipped codes.
pub(crate) struct Parser {
    state: State,
    geometry: Geometry,
    unsupported: BTreeSet<String>,
    s0_is_travel: bool,
    terminated: bool,
}

impl Parser {
    pub(crate) fn new(s0_is_travel: bool) -> Self {
        Self {
            state: State::default(),
            geometry: Geometry::new(),
            unsupported: BTreeSet::new(),
            s0_is_travel,
            terminated: false,
        }
    }

    pub(crate) fn run(&mut self, text: &str) {
        let mut percent_seen = false;
        for raw_line in text.lines() {
            if self.terminated {
                break;
            }
            if is_percent(raw_line) {
                if percent_seen {
                    break;
                }
                percent_seen = true;
                continue;
            }
            if is_block_deleted(raw_line) {
                continue;
            }
            let line = strip_comments(raw_line);
            if line.trim().is_empty() {
                continue;
            }
            if is_subroutine_label(&line) {
                self.unsupported.insert("O".to_string());
                continue;
            }
            if line.contains('#') {
                self.unsupported.insert("#".to_string());
                continue;
            }
            let words = tokenize(&line);
            if !words.is_empty() {
                self.execute(&words);
            }
        }
    }

    pub(crate) fn finish(self) -> GcodeParseOutput {
        GcodeParseOutput {
            geometry: self.geometry,
            unsupported: self.unsupported,
        }
    }

    fn execute(&mut self, words: &[Word]) {
        let mut g_codes = Vec::new();
        let mut m_codes = Vec::new();
        let mut axes = Axes::default();
        let mut spindle = None;
        let mut has_axis_word = false;
        for word in words {
            match word.letter {
                'G' => g_codes.push(word.value),
                'M' => m_codes.push(word.value),
                'X' => {
                    axes.x = Some(word.value);
                    has_axis_word = true;
                }
                'Y' => {
                    axes.y = Some(word.value);
                    has_axis_word = true;
                }
                'I' => {
                    axes.i = Some(word.value);
                    has_axis_word = true;
                }
                'J' => {
                    axes.j = Some(word.value);
                    has_axis_word = true;
                }
                'R' => {
                    axes.r = Some(word.value);
                    has_axis_word = true;
                }
                'Z' => has_axis_word = true,
                'S' => spindle = Some(word.value),
                'A' | 'B' | 'C' | 'U' | 'V' | 'W' | 'E' => {
                    self.unsupported.insert(word.letter.to_string());
                }
                _ => {}
            }
        }
        let motion_given = g_codes.iter().any(|&code| is_motion_code(code));
        let suppresses = g_codes.iter().any(|&code| suppresses_motion(code));
        if let Some(speed) = spindle {
            self.state.spindle_speed = Some(speed);
        }
        for code in g_codes {
            self.apply_g_code(code);
        }
        if !suppresses && (motion_given || has_axis_word) {
            self.apply_motion(&axes);
        }
        for code in m_codes {
            self.apply_m_code(code);
        }
    }

    fn apply_g_code(&mut self, code: f64) {
        if near(code, 0.0) {
            self.state.motion = Motion::Rapid;
        } else if near(code, 1.0) {
            self.state.motion = Motion::Linear;
        } else if near(code, 2.0) {
            self.state.motion = Motion::ArcClockwise;
        } else if near(code, 3.0) {
            self.state.motion = Motion::ArcCounterClockwise;
        } else if near(code, 17.0) {
            self.state.plane_xy = true;
        } else if near(code, 18.0) || near(code, 19.0) {
            self.state.plane_xy = false;
            self.unsupported.insert(format!("G{code}"));
        } else if near(code, 20.0) {
            self.state.unit_scale = INCH_IN_MM;
        } else if near(code, 21.0) {
            self.state.unit_scale = 1.0;
        } else if near(code, 90.0) {
            self.state.absolute = true;
        } else if near(code, 91.0) {
            self.state.absolute = false;
        } else if near(code, 90.1) {
            self.state.arc_center_absolute = true;
        } else if near(code, 91.1) {
            self.state.arc_center_absolute = false;
        } else if ignored_g_code(code) {
            // Non-geometric modes and safe preambles.
        } else {
            self.unsupported.insert(format!("G{code}"));
        }
    }

    fn apply_m_code(&mut self, code: f64) {
        if near(code, 2.0) || near(code, 30.0) {
            self.terminated = true;
        } else if !ignored_m_code(code) {
            self.unsupported.insert(format!("M{code}"));
        }
    }

    fn apply_motion(&mut self, axes: &Axes) {
        match self.state.motion {
            Motion::Rapid => self.rapid(axes),
            Motion::Linear => self.linear(axes),
            Motion::ArcClockwise => self.arc(axes, true),
            Motion::ArcCounterClockwise => self.arc(axes, false),
        }
    }

    fn rapid(&mut self, axes: &Axes) {
        if axes.x.is_none() && axes.y.is_none() {
            return;
        }
        if let Some(x) = self.target(axes.x, self.state.x) {
            self.state.x = x;
        }
        if let Some(y) = self.target(axes.y, self.state.y) {
            self.state.y = y;
        }
        self.state.path_open = false;
    }

    fn linear(&mut self, axes: &Axes) {
        if axes.x.is_none() && axes.y.is_none() {
            return;
        }
        let x = self.target(axes.x, self.state.x).unwrap_or(self.state.x);
        let y = self.target(axes.y, self.state.y).unwrap_or(self.state.y);
        if self.cutting() {
            self.start_path();
            self.geometry.line_to(x, y, 0.0);
        } else {
            self.state.path_open = false;
        }
        self.state.x = x;
        self.state.y = y;
    }

    fn arc(&mut self, axes: &Axes, clockwise: bool) {
        let start = Point::new(self.state.x, self.state.y);
        let end = self.arc_end(axes);
        if !self.state.plane_xy {
            self.state.path_open = false;
            self.state.x = end.x;
            self.state.y = end.y;
            return;
        }
        let offset = self.arc_offset(axes, start, end, clockwise);
        let Some(offset) = offset else {
            self.unsupported.insert(if clockwise {
                "G2".to_string()
            } else {
                "G3".to_string()
            });
            self.state.path_open = false;
            self.state.x = end.x;
            self.state.y = end.y;
            return;
        };
        if self.cutting() {
            self.start_path();
            self.geometry
                .arc_to(end.x, end.y, offset.x, offset.y, clockwise, 0.0);
        } else {
            self.state.path_open = false;
        }
        self.state.x = end.x;
        self.state.y = end.y;
    }

    fn arc_end(&self, axes: &Axes) -> Point {
        let x = self.target(axes.x, self.state.x).unwrap_or(self.state.x);
        let y = self.target(axes.y, self.state.y).unwrap_or(self.state.y);
        Point::new(x, y)
    }

    fn arc_offset(
        &self,
        axes: &Axes,
        start: Point,
        end: Point,
        clockwise: bool,
    ) -> Option<Point> {
        if axes.i.is_some() || axes.j.is_some() {
            let i = self.scale(axes.i.unwrap_or(0.0));
            let j = self.scale(axes.j.unwrap_or(0.0));
            let center = if self.state.arc_center_absolute {
                Point::new(i, j)
            } else {
                Point::new(start.x + i, start.y + j)
            };
            let offset = center - start;
            return (offset.length() > EPSILON).then_some(offset);
        }
        let radius = self.scale(axes.r?);
        if start.distance(end) <= EPSILON {
            return None;
        }
        center_from_radius(start, end, radius, clockwise).map(|c| c - start)
    }

    fn start_path(&mut self) {
        if !self.state.path_open {
            self.geometry.move_to(self.state.x, self.state.y, 0.0);
            self.state.path_open = true;
        }
    }

    fn cutting(&self) -> bool {
        !(self.s0_is_travel && self.state.spindle_speed == Some(0.0))
    }

    fn target(&self, word: Option<f64>, current: f64) -> Option<f64> {
        word.map(|value| {
            if self.state.absolute {
                value * self.state.unit_scale
            } else {
                current + value * self.state.unit_scale
            }
        })
    }

    fn scale(&self, value: f64) -> f64 {
        value * self.state.unit_scale
    }
}

fn near(value: f64, expected: f64) -> bool {
    (value - expected).abs() < EPSILON
}

fn is_motion_code(code: f64) -> bool {
    near(code, 0.0) || near(code, 1.0) || near(code, 2.0) || near(code, 3.0)
}

/// True for codes whose axis words describe a position we cannot interpret.
///
/// Their blocks are skipped entirely so they do not produce phantom cuts.
fn suppresses_motion(code: f64) -> bool {
    near(code, 10.0)
        || near(code, 28.0)
        || near(code, 30.0)
        || near(code, 53.0)
        || is_integer_in(code, 81.0, 89.0)
        || near(code, 92.0)
        || near(code, 92.1)
        || near(code, 92.2)
        || near(code, 92.3)
}

fn is_integer_in(value: f64, min: f64, max: f64) -> bool {
    value.fract() == 0.0 && (min..=max).contains(&value)
}

fn ignored_g_code(code: f64) -> bool {
    near(code, 4.0)
        || near(code, 40.0)
        || near(code, 43.0)
        || near(code, 49.0)
        || is_integer_in(code, 54.0, 59.0)
        || near(code, 61.0)
        || near(code, 64.0)
        || near(code, 80.0)
        || is_integer_in(code, 93.0, 95.0)
}

fn ignored_m_code(code: f64) -> bool {
    near(code, 0.0)
        || near(code, 1.0)
        || near(code, 3.0)
        || near(code, 4.0)
        || near(code, 5.0)
        || near(code, 7.0)
        || near(code, 8.0)
        || near(code, 9.0)
        || near(code, 48.0)
        || near(code, 49.0)
}
