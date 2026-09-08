use wasm_bindgen::prelude::*;
use std::cell::RefCell;

const EMPTY: u8 = 0xFF;
const GROW_CHUNK: usize = 1 << 20; // 1M cells per resize step

struct Player {
    offsets: Vec<(i32, i32)>,
    rgba: [u8; 4],
    frontier: u64,
    // Bitmask of which other player indices this piece checks attacks
    // against. Lets a piece ignore specific colors instead of hardcoding
    // "every other color is an enemy".
    enemy_mask: u32,
}

struct State {
    players: Vec<Player>,
    staging: Vec<Player>,
    // Spiral indices are dense integers 0..N, so occupancy is a flat byte
    // array indexed directly by index (EMPTY = unoccupied, else color idx).
    // This uses ~1 byte/cell instead of the ~16-40 bytes/entry a hash map
    // needs, which is what let placement counts reach the tens of millions
    // without approaching WASM32's 4GB linear-memory ceiling.
    // Note: on wasm32, a single Vec is capped at ~2GB (isize::MAX bytes) by
    // Rust itself regardless of available memory, so a byte array caps out
    // around 2.1 billion cells (radius ~23,000) -- far past anything
    // reachable in practice, but a real ceiling if pushed hard enough.
    occupied: Vec<u8>,
    turn_pointer: usize,
    placed_count: u64,
    tex_size: u32,
    pixels: Vec<u8>,
    view_cx: f64,
    view_cy: f64,
    view_radius: f64,
}

impl State {
    fn new() -> Self {
        State {
            players: Vec::new(),
            staging: Vec::new(),
            occupied: Vec::new(),
            turn_pointer: 0,
            placed_count: 0,
            tex_size: 0,
            pixels: Vec::new(),
            view_cx: 0.0,
            view_cy: 0.0,
            view_radius: 1.0,
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new());
}

fn ensure_capacity(st: &mut State, idx: u64) {
    let idx = idx as usize;
    if idx >= st.occupied.len() {
        let new_len = (idx / GROW_CHUNK + 1) * GROW_CHUNK;
        st.occupied.resize(new_len, EMPTY);
    }
}

// Inverse of index_to_coord: given a coordinate, find its unique spiral index.
fn coord_to_index(x: i32, y: i32) -> u64 {
    if x == 0 && y == 0 {
        return 0;
    }
    let r = (x.unsigned_abs()).max(y.unsigned_abs()) as i64;
    let xi = x as i64;
    let yi = y as i64;
    let p: i64 = if xi == r && yi != -r {
        yi + r - 1
    } else if yi == r && xi < r {
        3 * r - 1 - xi
    } else if xi == -r && yi < r {
        5 * r - 1 - yi
    } else {
        7 * r - 1 + xi
    };
    let start = (2 * r - 1) * (2 * r - 1);
    (start + p) as u64
}

fn index_to_coord(index: u64) -> (i32, i32) {
    if index == 0 {
        return (0, 0);
    }
    let idx = index as i64;
    let mut r = (((index as f64).sqrt() + 1.0) / 2.0).ceil() as i64;
    while (2 * r - 1) * (2 * r - 1) > idx {
        r -= 1;
    }
    while (2 * (r + 1) - 1) * (2 * (r + 1) - 1) <= idx {
        r += 1;
    }
    let start = (2 * r - 1) * (2 * r - 1);
    let mut p = idx - start;
    let seg_n = 2 * r - 1;
    let seg = 2 * r;
    if p == 0 {
        return (r as i32, (-(r - 1) + p) as i32);
    }
    if p <= seg_n {
        return (r as i32, (-(r - 1) + p) as i32);
    }
    p -= seg_n;
    if p <= seg {
        return ((r - p) as i32, r as i32);
    }
    p -= seg;
    if p <= seg {
        return (-r as i32, (r - p) as i32);
    }
    p -= seg;
    ((-r + p) as i32, -r as i32)
}

fn radius_to_index(rad: f64) -> u64 {
    let r = rad.ceil() as i64 + 1;
    ((2 * r + 1) * (2 * r + 1)) as u64
}

// A leaper's offset set is symmetric, so "does a piece at S attack C" is the
// same test as "does a piece at C attack S". That means checking whether C is
// attacked no longer needs a precomputed per-cell attack map (which held one
// entry per reachable square and dominated memory use) — for each other
// color's piece type, just probe the handful of squares C-offset directly.
fn is_attacked_by_others(st: &mut State, coord: (i32, i32), ci: usize) -> bool {
    let mask = st.players[ci].enemy_mask;
    for pj in 0..st.players.len() {
        if pj == ci || (mask & (1u32 << pj)) == 0 {
            continue;
        }
        let n = st.players[pj].offsets.len();
        for k in 0..n {
            let (dx, dy) = st.players[pj].offsets[k];
            let src_idx = coord_to_index(coord.0 - dx, coord.1 - dy);
            ensure_capacity(st, src_idx);
            if st.occupied[src_idx as usize] as usize == pj {
                return true;
            }
        }
    }
    false
}

fn plot_cell(st: &mut State, coord: (i32, i32), rgba: [u8; 4]) {
    let scale = st.tex_size as f64 / (2.0 * st.view_radius);
    let half = 0.5 * scale;
    let cx = (coord.0 as f64 - st.view_cx) * scale + st.tex_size as f64 / 2.0;
    let cy = st.tex_size as f64 / 2.0 - (coord.1 as f64 - st.view_cy) * scale;
    let x0 = (cx - half).round() as i64;
    let x1 = ((cx + half).round() as i64).max(x0 + 1);
    let y0 = (cy - half).round() as i64;
    let y1 = ((cy + half).round() as i64).max(y0 + 1);
    let tsz = st.tex_size as i64;
    let x_start = x0.max(0);
    let x_end = x1.min(tsz);
    let y_start = y0.max(0);
    let y_end = y1.min(tsz);
    for py in y_start..y_end {
        let row = (py as usize) * st.tex_size as usize;
        for px in x_start..x_end {
            let o = (row + px as usize) * 4;
            st.pixels[o] = rgba[0];
            st.pixels[o + 1] = rgba[1];
            st.pixels[o + 2] = rgba[2];
            st.pixels[o + 3] = 255;
        }
    }
}

// Returns (placed, steps_taken). If the scan budget runs out before a free
// cell is found, `frontier` is still advanced to the resume point (safe,
// since skipped cells are permanently resolved as occupied/blocked) but
// `turn_pointer` is left unchanged so the same color resumes next call
// instead of the caller blocking indefinitely on one pathological scan.
fn do_turn_bounded(st: &mut State, budget: u32) -> (bool, u32) {
    let ci = st.turn_pointer;
    let mut idx = st.players[ci].frontier;
    let mut steps = 0u32;
    loop {
        if steps >= budget {
            st.players[ci].frontier = idx;
            return (false, steps);
        }
        steps += 1;
        ensure_capacity(st, idx);
        if st.occupied[idx as usize] != EMPTY {
            idx += 1;
            continue;
        }
        let coord = index_to_coord(idx);
        if is_attacked_by_others(st, coord, ci) {
            idx += 1;
            continue;
        }
        st.occupied[idx as usize] = ci as u8;
        let rgba = st.players[ci].rgba;
        plot_cell(st, coord, rgba);
        st.players[ci].frontier = idx + 1;
        st.placed_count += 1;
        st.turn_pointer = (st.turn_pointer + 1) % st.players.len();
        return (true, steps);
    }
}

#[wasm_bindgen]
pub fn config_reset() {
    STATE.with(|s| {
        s.borrow_mut().staging.clear();
    });
}

#[wasm_bindgen]
pub fn config_add_piece(r: u8, g: u8, b: u8, offsets: &[i32], enemy_mask: u32) {
    let mut offs = Vec::with_capacity(offsets.len() / 2);
    let mut i = 0;
    while i + 1 < offsets.len() {
        offs.push((offsets[i], offsets[i + 1]));
        i += 2;
    }
    STATE.with(|s| {
        s.borrow_mut().staging.push(Player {
            offsets: offs,
            rgba: [r, g, b, 255],
            frontier: 1,
            enemy_mask,
        });
    });
}

#[wasm_bindgen]
pub fn start(tex_size: u32) {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let players = std::mem::take(&mut st.staging);
        st.players = players;
        st.occupied = Vec::new();
        st.turn_pointer = if st.players.len() > 1 { 1 } else { 0 };
        st.placed_count = 0;
        st.tex_size = tex_size;
        st.pixels = vec![255u8; (tex_size as usize) * (tex_size as usize) * 4];
        st.view_cx = 0.0;
        st.view_cy = 0.0;
        st.view_radius = tex_size as f64 / 2.0;

        if !st.players.is_empty() {
            st.players[0].frontier = 1;
            ensure_capacity(&mut st, 0);
            st.occupied[0] = 0;
            let rgba = st.players[0].rgba;
            plot_cell(&mut st, (0, 0), rgba);
            st.placed_count = 1;
        }
    });
}

const SCAN_BUDGET: u32 = 300_000;

#[wasm_bindgen]
pub fn simulate_batch(target_radius: f64, max_turns: u32) -> u32 {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.players.is_empty() {
            return 0;
        }
        let target_index = radius_to_index(target_radius);
        let mut done = 0u32;
        let mut scan_budget = SCAN_BUDGET;
        while done < max_turns && scan_budget > 0 {
            let min_frontier = st.players.iter().map(|p| p.frontier).min().unwrap_or(0);
            if min_frontier >= target_index {
                break;
            }
            let (placed, steps) = do_turn_bounded(&mut st, scan_budget);
            scan_budget = scan_budget.saturating_sub(steps);
            if placed {
                done += 1;
            } else {
                break;
            }
        }
        done
    })
}

#[wasm_bindgen]
pub fn target_reached(target_radius: f64) -> bool {
    STATE.with(|s| {
        let st = s.borrow();
        if st.players.is_empty() {
            return true;
        }
        let min_frontier = st.players.iter().map(|p| p.frontier).min().unwrap_or(0);
        min_frontier >= radius_to_index(target_radius)
    })
}

#[wasm_bindgen]
pub fn set_view(cx: f64, cy: f64, radius: f64) {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.view_cx = cx;
        st.view_cy = cy;
        st.view_radius = radius;
    });
}

#[wasm_bindgen]
pub fn redraw() {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        for p in st.pixels.iter_mut() {
            *p = 255;
        }
        let n = st.occupied.len();
        for idx in 0..n {
            let color_idx = st.occupied[idx];
            if color_idx == EMPTY {
                continue;
            }
            let coord = index_to_coord(idx as u64);
            let rgba = st.players[color_idx as usize].rgba;
            plot_cell(&mut st, coord, rgba);
        }
    });
}

#[wasm_bindgen]
pub fn pixel_ptr() -> *const u8 {
    STATE.with(|s| s.borrow().pixels.as_ptr())
}

#[wasm_bindgen]
pub fn placed_count() -> f64 {
    STATE.with(|s| s.borrow().placed_count as f64)
}
