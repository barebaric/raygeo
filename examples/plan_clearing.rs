//! Embedding raygeo's adaptive clearing from Rust.
//!
//! Run with `cargo run --example plan_clearing`. The same three steps work
//! for any pocket: build a `Part`, plan it, run the plan. See
//! `docs/embedding-rust.md` for the full guide — islands, rest machining,
//! multi-pocket parts and wasm32.

use raygeo::cnc::execution::run::run_plan;
use raygeo::cnc::plan::clearing::{plan_clearing, ClearingWorkplanOptions};
use raygeo::geo::types::Point;
use raygeo::ops::part::Part;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A 60x40 mm plate pocket with a 10x10 mm island.
    let boundary: Vec<Point> =
        [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)]
            .iter()
            .map(|(x, y)| Point::new(*x, *y))
            .collect();
    let island: Vec<Point> =
        [(25.0, 15.0), (35.0, 15.0), (35.0, 25.0), (25.0, 25.0)]
            .iter()
            .map(|(x, y)| Point::new(*x, *y))
            .collect();
    let part = Part::from_polygons(&boundary, &[island], (60.0, 40.0));

    // Millimetres throughout. wall_margin keeps the tool clear of the walls
    // and the island in every pass; feed_rate lands in the emitted moves.
    let options = ClearingWorkplanOptions {
        tool_radius: 3.0,
        step_over: 2.0,
        step_length: 0.6,
        target_z: -4.0,
        safe_z: 10.0,
        wall_margin: 0.2,
        safe_margin: 0.6,
        stock_to_leave: 0.0,
        plunge_pitch: 0.9,
        angular_step: 0.1,
        area_tolerance: 1.0,
        max_deflection_deg: 30.0,
        finishing: true,
        feed_rate: Some(900.0),
    };
    let plan = plan_clearing(&part, "", &options)
        .map_err(|e| format!("planning failed: {e}"))?;
    println!("plan: {} steps", plan.steps.len());

    let output =
        run_plan(&plan, &part).map_err(|e| format!("execution failed: {e}"))?;
    let seconds = output
        .time_estimate
        .map(|t| format!("{t:.1} s"))
        .unwrap_or_else(|| "n/a".into());
    println!(
        "toolpath: {} moves, estimated {}",
        output.ops.len(),
        seconds
    );

    // `output.ops` is a normal raygeo Ops: hand it to your own encoder,
    // simulator, or display. Cutting moves are LINE_TO / ARC_TO.
    Ok(())
}
