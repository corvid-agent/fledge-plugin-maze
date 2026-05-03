use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::Duration;
use std::{env, process, thread};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::prelude::*;
use rand::rngs::StdRng;

struct Args {
    width: usize,
    height: usize,
    seed: Option<u64>,
    solve: bool,
    animate: bool,
    ascii: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();
    let mut a = Args {
        width: 20,
        height: 10,
        seed: None,
        solve: false,
        animate: false,
        ascii: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--width" | "-w" => {
                i += 1;
                a.width = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(20);
            }
            "--height" | "-h" if i + 1 < args.len() => {
                i += 1;
                a.height = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(10);
            }
            "--seed" | "-s" => {
                i += 1;
                a.seed = args.get(i).and_then(|s| s.parse().ok());
            }
            "--solve" => a.solve = true,
            "--animate" | "-a" => {
                a.animate = true;
                a.solve = true;
            }
            "--ascii" => a.ascii = true,
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_usage();
                process::exit(1);
            }
        }
        i += 1;
    }

    if a.width < 2 || a.height < 2 {
        eprintln!("Error: width and height must be at least 2");
        process::exit(1);
    }
    if a.width > 100 || a.height > 50 {
        eprintln!("Error: maximum size is 100x50");
        process::exit(1);
    }

    a
}

fn print_usage() {
    println!(
        r#"fledge maze — terminal maze generator and solver (Rust TUI)

USAGE:
    fledge maze [options]

OPTIONS:
    --width, -w <n>     Maze width in cells (default: 20, max: 100)
    --height, -h <n>    Maze height in cells (default: 10, max: 50)
    --seed, -s <n>      Random seed for reproducible mazes
    --solve             Show solution path
    --animate, -a       Animated generation and solve (implies --solve)
    --ascii             Use ASCII instead of Unicode box-drawing
    --help              Show this help

EXAMPLES:
    fledge maze
    fledge maze --width 30 --height 15
    fledge maze --seed 42 --solve
    fledge maze --animate
    fledge maze --ascii"#
    );
}

struct Maze {
    width: usize,
    height: usize,
    h_walls: Vec<bool>,
    v_walls: Vec<bool>,
}

impl Maze {
    fn new(width: usize, height: usize) -> Self {
        Maze {
            width,
            height,
            h_walls: vec![true; width * (height + 1)],
            v_walls: vec![true; (width + 1) * height],
        }
    }

    fn h_wall(&self, col: usize, row: usize) -> bool {
        self.h_walls[row * self.width + col]
    }

    fn v_wall(&self, col: usize, row: usize) -> bool {
        self.v_walls[row * (self.width + 1) + col]
    }

    fn set_h_wall(&mut self, col: usize, row: usize, val: bool) {
        self.h_walls[row * self.width + col] = val;
    }

    fn set_v_wall(&mut self, col: usize, row: usize, val: bool) {
        self.v_walls[row * (self.width + 1) + col] = val;
    }

    fn generate(&mut self, rng: &mut (impl Rng + ?Sized)) -> Vec<CarvingStep> {
        let w = self.width;
        let h = self.height;
        let total = w * h;
        let mut visited = vec![false; total];
        let mut stack: Vec<(usize, usize)> = Vec::with_capacity(total);
        let mut steps = Vec::new();

        visited[0] = true;
        stack.push((0, 0));

        while let Some(&(cx, cy)) = stack.last() {
            let mut neighbors = Vec::new();
            if cy > 0 && !visited[(cy - 1) * w + cx] {
                neighbors.push((cx, cy - 1, WallKind::H, cx, cy));
            }
            if cy + 1 < h && !visited[(cy + 1) * w + cx] {
                neighbors.push((cx, cy + 1, WallKind::H, cx, cy + 1));
            }
            if cx > 0 && !visited[cy * w + cx - 1] {
                neighbors.push((cx - 1, cy, WallKind::V, cx, cy));
            }
            if cx + 1 < w && !visited[cy * w + cx + 1] {
                neighbors.push((cx + 1, cy, WallKind::V, cx + 1, cy));
            }

            if neighbors.is_empty() {
                stack.pop();
            } else {
                let &(nx, ny, kind, wx, wy) = &neighbors[rng.random_range(0..neighbors.len())];
                match kind {
                    WallKind::H => self.set_h_wall(wx, wy, false),
                    WallKind::V => self.set_v_wall(wx, wy, false),
                }
                steps.push(CarvingStep {
                    cell: (nx, ny),
                    wall_kind: kind,
                    wall_pos: (wx, wy),
                });
                visited[ny * w + nx] = true;
                stack.push((nx, ny));
            }
        }
        steps
    }

    fn solve(&self) -> (Vec<bool>, Vec<(usize, usize)>) {
        let w = self.width;
        let h = self.height;
        let total = w * h;
        let mut visited = vec![false; total];
        let mut parent = vec![usize::MAX; total];
        let mut queue = VecDeque::new();
        let mut explore_order = Vec::new();
        let target = total - 1;

        visited[0] = true;
        queue.push_back((0usize, 0usize));

        while let Some((cx, cy)) = queue.pop_front() {
            let cidx = cy * w + cx;
            explore_order.push((cx, cy));
            if cidx == target {
                break;
            }

            // Up
            if cy > 0 && !self.h_wall(cx, cy) {
                let nidx = (cy - 1) * w + cx;
                if !visited[nidx] {
                    visited[nidx] = true;
                    parent[nidx] = cidx;
                    queue.push_back((cx, cy - 1));
                }
            }
            // Down
            if cy + 1 < h && !self.h_wall(cx, cy + 1) {
                let nidx = (cy + 1) * w + cx;
                if !visited[nidx] {
                    visited[nidx] = true;
                    parent[nidx] = cidx;
                    queue.push_back((cx, cy + 1));
                }
            }
            // Left
            if cx > 0 && !self.v_wall(cx, cy) {
                let nidx = cy * w + cx - 1;
                if !visited[nidx] {
                    visited[nidx] = true;
                    parent[nidx] = cidx;
                    queue.push_back((cx - 1, cy));
                }
            }
            // Right
            if cx + 1 < w && !self.v_wall(cx + 1, cy) {
                let nidx = cy * w + cx + 1;
                if !visited[nidx] {
                    visited[nidx] = true;
                    parent[nidx] = cidx;
                    queue.push_back((cx + 1, cy));
                }
            }
        }

        let mut on_path = vec![false; total];
        let mut idx = target;
        let mut path_len = 0;
        while idx != usize::MAX {
            on_path[idx] = true;
            path_len += 1;
            idx = parent[idx];
        }
        let _ = path_len;
        (on_path, explore_order)
    }

    fn intersection_char(&self, col: usize, row: usize, ascii: bool) -> char {
        if ascii {
            return '+';
        }
        let up = row > 0 && self.v_wall(col, row - 1);
        let down = row < self.height && self.v_wall(col, row);
        let left = col > 0 && self.h_wall(col - 1, row);
        let right = col < self.width && self.h_wall(col, row);

        match (up, down, left, right) {
            (false, false, false, false) => ' ',
            (false, false, false, true) => '╶',
            (false, true, false, false) => '╷',
            (false, true, false, true) => '┌',
            (false, false, true, false) => '╴',
            (false, false, true, true) => '─',
            (false, true, true, false) => '┐',
            (false, true, true, true) => '┬',
            (true, false, false, false) => '╵',
            (true, false, false, true) => '└',
            (true, true, false, false) => '│',
            (true, true, false, true) => '├',
            (true, false, true, false) => '┘',
            (true, false, true, true) => '┴',
            (true, true, true, false) => '┤',
            (true, true, true, true) => '┼',
        }
    }
}

#[derive(Clone, Copy)]
enum WallKind {
    H,
    V,
}

struct CarvingStep {
    cell: (usize, usize),
    wall_kind: WallKind,
    wall_pos: (usize, usize),
}

fn render_static(maze: &Maze, solution: Option<&[bool]>, ascii: bool) {
    let out = io::stdout();
    let mut w = io::BufWriter::new(out.lock());
    let mw = maze.width;
    let mh = maze.height;

    let wall_color = if ascii { Color::White } else { Color::Cyan };
    let path_color = Color::Green;
    let start_color = Color::Green;
    let end_color = Color::Red;

    // Header
    queue!(
        w,
        SetForegroundColor(Color::DarkCyan),
        SetAttribute(Attribute::Bold),
        Print(format!("fledge maze — {}×{}", mw, mh)),
        SetAttribute(Attribute::Reset),
        ResetColor,
        Print("\n\n")
    )
    .ok();

    let h_wall_str = if ascii { "---" } else { "───" };
    let h_space = "   ";
    let v_wall_ch = if ascii { '|' } else { '│' };

    for row in 0..=mh {
        // Intersection + horizontal wall row
        for col in 0..=mw {
            let ch = maze.intersection_char(col, row, ascii);
            queue!(w, SetForegroundColor(wall_color), Print(ch)).ok();
            if col < mw {
                if maze.h_wall(col, row) {
                    queue!(w, Print(h_wall_str)).ok();
                } else {
                    queue!(w, ResetColor, Print(h_space)).ok();
                }
            }
        }
        queue!(w, ResetColor, Print('\n')).ok();

        // Cell + vertical wall row
        if row < mh {
            for col in 0..=mw {
                if maze.v_wall(col, row) {
                    queue!(w, SetForegroundColor(wall_color), Print(v_wall_ch)).ok();
                } else {
                    queue!(w, ResetColor, Print(' ')).ok();
                }
                if col < mw {
                    let cidx = row * mw + col;
                    let is_start = col == 0 && row == 0;
                    let is_end = col == mw - 1 && row == mh - 1;
                    let on_path = solution.map_or(false, |s| s[cidx]);

                    if is_start {
                        queue!(
                            w,
                            SetForegroundColor(start_color),
                            SetAttribute(Attribute::Bold),
                            Print(" S "),
                            SetAttribute(Attribute::Reset)
                        )
                        .ok();
                    } else if is_end {
                        queue!(
                            w,
                            SetForegroundColor(end_color),
                            SetAttribute(Attribute::Bold),
                            Print(" E "),
                            SetAttribute(Attribute::Reset)
                        )
                        .ok();
                    } else if on_path {
                        queue!(
                            w,
                            SetForegroundColor(path_color),
                            Print(" · "),
                        )
                        .ok();
                    } else {
                        queue!(w, ResetColor, Print("   ")).ok();
                    }
                }
            }
            queue!(w, ResetColor, Print('\n')).ok();
        }
    }

    if solution.is_some() {
        let path_count = solution.unwrap().iter().filter(|&&x| x).count();
        queue!(
            w,
            Print('\n'),
            SetForegroundColor(Color::DarkGreen),
            Print(format!(
                "Path: top-left → bottom-right ({} steps)\n",
                path_count
            )),
            ResetColor,
        )
        .ok();
    }

    w.flush().ok();
}

fn render_animated(maze: &Maze, steps: &[CarvingStep], solution: &[bool], explore_order: &[(usize, usize)]) {
    let mut stdout = io::stdout();
    let mw = maze.width;
    let mh = maze.height;

    execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All)).ok();
    terminal::enable_raw_mode().ok();

    let wall_color = Color::DarkGrey;
    let carved_color = Color::Cyan;
    let active_color = Color::Yellow;
    let explore_color = Color::DarkBlue;
    let path_color = Color::Green;

    // Draw initial grid (all walls)
    draw_full_grid(&mut stdout, mw, mh, wall_color);

    // Header
    queue!(
        stdout,
        MoveTo(0, 0),
        SetForegroundColor(Color::DarkCyan),
        SetAttribute(Attribute::Bold),
        Print(format!(" fledge maze — {}×{} — generating...", mw, mh)),
        SetAttribute(Attribute::Reset),
        ResetColor,
    )
    .ok();
    stdout.flush().ok();
    thread::sleep(Duration::from_millis(500));

    // Rebuild maze walls step by step
    let mut anim_maze = Maze::new(mw, mh);

    let delay = if mw * mh > 200 {
        Duration::from_millis(8)
    } else if mw * mh > 100 {
        Duration::from_millis(15)
    } else {
        Duration::from_millis(25)
    };

    for step in steps {
        let (cx, cy) = step.cell;

        // Highlight active cell
        let term_col = (cx * 4 + 1) as u16;
        let term_row = (cy * 2 + 2) as u16;
        queue!(
            stdout,
            MoveTo(term_col, term_row),
            SetForegroundColor(active_color),
            SetAttribute(Attribute::Bold),
            Print(" ◆ "),
            SetAttribute(Attribute::Reset),
        )
        .ok();

        // Remove the wall
        let (wx, wy) = step.wall_pos;
        match step.wall_kind {
            WallKind::H => {
                anim_maze.set_h_wall(wx, wy, false);
                let tc = (wx * 4 + 1) as u16;
                let tr = (wy * 2 + 1) as u16;
                queue!(
                    stdout,
                    MoveTo(tc, tr),
                    SetForegroundColor(carved_color),
                    Print("   "),
                )
                .ok();
                // Update adjacent intersections
                redraw_intersection(&mut stdout, &anim_maze, wx, wy, carved_color);
                redraw_intersection(&mut stdout, &anim_maze, wx + 1, wy, carved_color);
            }
            WallKind::V => {
                anim_maze.set_v_wall(wx, wy, false);
                let tc = (wx * 4) as u16;
                let tr = (wy * 2 + 2) as u16;
                queue!(
                    stdout,
                    MoveTo(tc, tr),
                    SetForegroundColor(carved_color),
                    Print(' '),
                )
                .ok();
                redraw_intersection(&mut stdout, &anim_maze, wx, wy, carved_color);
                redraw_intersection(&mut stdout, &anim_maze, wx, wy + 1, carved_color);
            }
        }

        stdout.flush().ok();
        thread::sleep(delay);

        // Clear active cell marker
        queue!(
            stdout,
            MoveTo(term_col, term_row),
            ResetColor,
            Print("   "),
        )
        .ok();
    }

    // Update header
    queue!(
        stdout,
        MoveTo(0, 0),
        SetForegroundColor(Color::DarkCyan),
        SetAttribute(Attribute::Bold),
        Print(format!(
            " fledge maze — {}×{} — solving...       ",
            mw, mh
        )),
        SetAttribute(Attribute::Reset),
        ResetColor,
    )
    .ok();
    stdout.flush().ok();
    thread::sleep(Duration::from_millis(300));

    // Animate BFS exploration
    let explore_delay = if explore_order.len() > 200 {
        Duration::from_millis(5)
    } else {
        Duration::from_millis(15)
    };

    for &(ex, ey) in explore_order {
        let cidx = ey * mw + ex;
        if !solution[cidx] {
            let tc = (ex * 4 + 1) as u16;
            let tr = (ey * 2 + 2) as u16;
            queue!(
                stdout,
                MoveTo(tc, tr),
                SetForegroundColor(explore_color),
                Print(" · "),
            )
            .ok();
            stdout.flush().ok();
            thread::sleep(explore_delay);
        }
    }

    thread::sleep(Duration::from_millis(200));

    // Draw solution path
    for row in 0..mh {
        for col in 0..mw {
            let cidx = row * mw + col;
            if solution[cidx] {
                let tc = (col * 4 + 1) as u16;
                let tr = (row * 2 + 2) as u16;
                let is_start = col == 0 && row == 0;
                let is_end = col == mw - 1 && row == mh - 1;

                if is_start {
                    queue!(
                        stdout,
                        MoveTo(tc, tr),
                        SetForegroundColor(Color::Green),
                        SetAttribute(Attribute::Bold),
                        Print(" S "),
                        SetAttribute(Attribute::Reset),
                    )
                    .ok();
                } else if is_end {
                    queue!(
                        stdout,
                        MoveTo(tc, tr),
                        SetForegroundColor(Color::Red),
                        SetAttribute(Attribute::Bold),
                        Print(" E "),
                        SetAttribute(Attribute::Reset),
                    )
                    .ok();
                } else {
                    queue!(
                        stdout,
                        MoveTo(tc, tr),
                        SetForegroundColor(path_color),
                        SetAttribute(Attribute::Bold),
                        Print(" ● "),
                        SetAttribute(Attribute::Reset),
                    )
                    .ok();
                }
            }
        }
    }

    // Final header
    let path_count = solution.iter().filter(|&&x| x).count();
    queue!(
        stdout,
        MoveTo(0, 0),
        SetForegroundColor(Color::Green),
        SetAttribute(Attribute::Bold),
        Print(format!(
            " fledge maze — {}×{} — solved! ({} steps)   ",
            mw, mh, path_count
        )),
        SetAttribute(Attribute::Reset),
        ResetColor,
    )
    .ok();

    // Footer hint
    let footer_row = (mh * 2 + 3) as u16;
    queue!(
        stdout,
        MoveTo(0, footer_row),
        SetForegroundColor(Color::DarkGrey),
        Print("Press any key to exit..."),
        ResetColor,
    )
    .ok();
    stdout.flush().ok();

    // Wait for keypress
    loop {
        if crossterm::event::poll(Duration::from_secs(30)).unwrap_or(false) {
            if let Ok(crossterm::event::Event::Key(_)) = crossterm::event::read() {
                break;
            }
        } else {
            break;
        }
    }

    terminal::disable_raw_mode().ok();
    execute!(stdout, Show, LeaveAlternateScreen).ok();

    // Print final summary to normal terminal
    println!(
        "Maze {}×{} — solved in {} steps",
        mw, mh, path_count
    );
}

fn draw_full_grid(stdout: &mut io::Stdout, width: usize, height: usize, color: Color) {
    // Draw a fully-walled grid starting at row 1 (row 0 is header)
    for row in 0..=height {
        let tr = (row * 2 + 1) as u16;
        for col in 0..=width {
            let tc = (col * 4) as u16;
            queue!(
                stdout,
                MoveTo(tc, tr),
                SetForegroundColor(color),
                Print('┼'),
            )
            .ok();
            if col < width {
                queue!(stdout, Print("───")).ok();
            }
        }
        if row < height {
            let tr2 = (row * 2 + 2) as u16;
            for col in 0..=width {
                let tc = (col * 4) as u16;
                queue!(
                    stdout,
                    MoveTo(tc, tr2),
                    SetForegroundColor(color),
                    Print('│'),
                )
                .ok();
                if col < width {
                    queue!(stdout, ResetColor, Print("   ")).ok();
                }
            }
        }
    }
    stdout.flush().ok();
}

fn redraw_intersection(stdout: &mut io::Stdout, maze: &Maze, col: usize, row: usize, color: Color) {
    let tc = (col * 4) as u16;
    let tr = (row * 2 + 1) as u16;
    let ch = maze.intersection_char(col, row, false);
    queue!(
        stdout,
        MoveTo(tc, tr),
        SetForegroundColor(color),
        Print(ch),
    )
    .ok();
}

fn main() {
    let args = parse_args();

    let mut rng: Box<dyn RngCore> = match args.seed {
        Some(seed) => Box::new(StdRng::seed_from_u64(seed)),
        None => Box::new(StdRng::from_os_rng()),
    };

    let mut maze = Maze::new(args.width, args.height);
    let steps = maze.generate(&mut *rng);

    if args.animate {
        let (solution, explore_order) = maze.solve();
        render_animated(&maze, &steps, &solution, &explore_order);
    } else if args.solve {
        let (solution, _) = maze.solve();
        render_static(&maze, Some(&solution), args.ascii);
    } else {
        render_static(&maze, None, args.ascii);
    }
}
