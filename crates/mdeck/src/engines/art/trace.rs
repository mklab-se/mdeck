//! A point cloud read back as a drawing, for the art engines' fallback.
//! Clouds are dotted line drawings (importance ordered points along the
//! subject's lines), so a single pen tour through them wanders like a maze.
//! Instead the dots are joined into the lines they sit on (a minimum
//! spanning forest over near neighbours, gaps closed between line ends,
//! stray spurs dropped) and, for the tonal media, the closed shapes are
//! shaded with hatching that is heavier on the side away from the light.

use std::collections::{HashSet, VecDeque};

/// A point in the cloud's unit square.
pub type P = [f32; 2];

/// Most points traced; the forest is quadratic in them.
const MAX_TRACE: usize = 1600;

/// How far apart two dots may be, in typical dot spacings, and still sit
/// on one line. Sparser dots (the stipple inside some clouds) stay apart.
const REACH: f32 = 1.8;

/// Stubs of up to this many dots hanging off a line are dropped.
const SPUR: usize = 4;

/// How a medium draws a point cloud picture that has no artwork.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fill {
    /// One pen tour through the points.
    Tour,
    /// The cloud's own lines, traced; shaded with hatching when given.
    Lines(Option<Shading>),
}

/// Hatching inside the traced shapes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shading {
    /// Space between hatch lines, as a fraction of the picture's width.
    pub spacing: f32,
    /// Hatch the whole shape (else only its shadow side).
    pub everywhere: bool,
    /// Cross-hatch the shadow side.
    pub cross: bool,
}

/// A traced drawing in the cloud's unit square: the lines (longest first)
/// and the hatch strokes.
#[derive(Clone, Debug, Default)]
pub struct Traced {
    pub lines: Vec<Vec<P>>,
    pub hatch: Vec<Vec<P>>,
}

/// Trace `points` (a cloud of height/width `aspect`) into lines, and shade
/// them when `shading` says so.
pub fn trace(points: &[P], aspect: f32, shading: Option<Shading>) -> Traced {
    let aspect = aspect.max(1e-3);
    let pts: Vec<P> = points
        .iter()
        .take(MAX_TRACE)
        .map(|p| [p[0], p[1] * aspect])
        .collect();
    let Some(graph) = Graph::build(&pts) else {
        return Traced::default();
    };
    let lines = graph.polylines(&pts);
    let hatch = shading
        .map(|s| hatch(&lines, aspect, graph.spacing, s))
        .unwrap_or_default();
    let unit = |l: Vec<P>| l.into_iter().map(|p| [p[0], p[1] / aspect]).collect();
    Traced {
        lines: lines.into_iter().map(unit).collect(),
        hatch: hatch.into_iter().map(unit).collect(),
    }
}

fn dist(a: P, b: P) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The dots joined into lines.
struct Graph {
    adj: Vec<Vec<usize>>,
    /// The typical distance between neighbouring dots.
    spacing: f32,
}

impl Graph {
    fn build(pts: &[P]) -> Option<Graph> {
        let n = pts.len();
        if n < 3 {
            return None;
        }
        let d = |a: usize, b: usize| dist(pts[a], pts[b]);
        let mut nn: Vec<f32> = (0..n)
            .map(|i| {
                (0..n)
                    .filter(|&j| j != i)
                    .map(|j| d(i, j))
                    .fold(f32::MAX, f32::min)
            })
            .collect();
        nn.sort_by(f32::total_cmp);
        let spacing = nn[n / 2].max(1e-4);
        let reach = spacing * REACH;

        // Prim's algorithm, starting a new tree whenever the nearest dot is
        // out of reach: a forest along the dotted lines
        let mut adj = vec![Vec::new(); n];
        let mut in_tree = vec![false; n];
        let mut best = vec![f32::MAX; n];
        let mut parent = vec![usize::MAX; n];
        for _ in 0..n {
            let mut u = usize::MAX;
            let mut bd = f32::MAX;
            for i in 0..n {
                if !in_tree[i] && best[i] < bd {
                    bd = best[i];
                    u = i;
                }
            }
            if u == usize::MAX {
                u = (0..n).find(|&i| !in_tree[i])?;
            } else if bd <= reach {
                adj[u].push(parent[u]);
                adj[parent[u]].push(u);
            }
            in_tree[u] = true;
            for v in 0..n {
                if !in_tree[v] {
                    let dv = d(u, v);
                    if dv < best[v] {
                        best[v] = dv;
                        parent[v] = u;
                    }
                }
            }
        }
        let mut g = Graph { adj, spacing };
        g.close_gaps(pts, reach * 1.5);
        g.drop_spurs();
        Some(g)
    }

    /// Hops from `a` to `b`, if `b` is within `limit` hops.
    fn hops(&self, a: usize, b: usize, limit: usize) -> Option<usize> {
        let mut seen = HashSet::from([a]);
        let mut queue = VecDeque::from([(a, 0)]);
        while let Some((u, h)) = queue.pop_front() {
            if u == b {
                return Some(h);
            }
            if h == limit {
                continue;
            }
            for &v in &self.adj[u] {
                if seen.insert(v) {
                    queue.push_back((v, h + 1));
                }
            }
        }
        None
    }

    /// A tree cuts every closed outline somewhere: join each line end to the
    /// nearest dot within `reach` that is not already a few steps along the
    /// same line, so circles close and broken strokes meet.
    fn close_gaps(&mut self, pts: &[P], reach: f32) {
        let ends: Vec<usize> = (0..pts.len()).filter(|&i| self.adj[i].len() == 1).collect();
        for a in ends {
            if self.adj[a].len() != 1 {
                continue;
            }
            let candidate = (0..pts.len())
                .filter(|&b| b != a && !self.adj[a].contains(&b))
                .map(|b| (b, dist(pts[a], pts[b])))
                .filter(|(_, d)| *d <= reach)
                .filter(|(b, _)| self.hops(a, *b, 6).is_none())
                .min_by(|x, y| x.1.total_cmp(&y.1));
            if let Some((b, _)) = candidate {
                self.adj[a].push(b);
                self.adj[b].push(a);
            }
        }
    }

    /// Drop stubs of a few dots hanging off a line: noise, not detail.
    fn drop_spurs(&mut self) {
        let n = self.adj.len();
        for leaf in 0..n {
            if self.adj[leaf].len() != 1 {
                continue;
            }
            let mut path = vec![leaf];
            let (mut prev, mut cur) = (leaf, self.adj[leaf][0]);
            while self.adj[cur].len() == 2 && path.len() < SPUR {
                path.push(cur);
                let next = self.adj[cur].iter().copied().find(|&v| v != prev);
                let Some(next) = next else { break };
                (prev, cur) = (cur, next);
            }
            if self.adj[cur].len() >= 3 && path.len() < SPUR {
                // cut the stub where it joins the line
                let last = *path.last().unwrap_or(&leaf);
                self.adj[cur].retain(|&v| v != last);
                self.adj[last].retain(|&v| v != cur);
                for w in path.windows(2) {
                    self.adj[w[0]].retain(|&v| v != w[1]);
                    self.adj[w[1]].retain(|&v| v != w[0]);
                }
            }
        }
    }

    /// The graph as polylines: runs between line ends and junctions, then
    /// any closed loops, longest first. Specks shorter than a few dots go.
    fn polylines(&self, pts: &[P]) -> Vec<Vec<P>> {
        let key = |a: usize, b: usize| (a.min(b), a.max(b));
        let mut used: HashSet<(usize, usize)> = HashSet::new();
        let mut runs: Vec<Vec<usize>> = Vec::new();
        let walk = |start: usize, first: usize, used: &mut HashSet<(usize, usize)>| {
            let mut run = vec![start];
            let (mut prev, mut cur) = (start, first);
            used.insert(key(start, first));
            loop {
                run.push(cur);
                if self.adj[cur].len() != 2 {
                    break;
                }
                let next = self.adj[cur]
                    .iter()
                    .copied()
                    .find(|&v| v != prev && !used.contains(&key(cur, v)));
                let Some(next) = next else { break };
                used.insert(key(cur, next));
                (prev, cur) = (cur, next);
            }
            run
        };
        for u in 0..self.adj.len() {
            if self.adj[u].len() == 2 {
                continue;
            }
            for &v in &self.adj[u] {
                if !used.contains(&key(u, v)) {
                    runs.push(walk(u, v, &mut used));
                }
            }
        }
        // what is left are loops of two-neighbour dots
        for u in 0..self.adj.len() {
            for &v in &self.adj[u] {
                if !used.contains(&key(u, v)) {
                    runs.push(walk(u, v, &mut used));
                }
            }
        }
        let length =
            |r: &Vec<usize>| -> f32 { r.windows(2).map(|w| dist(pts[w[0]], pts[w[1]])).sum() };
        let mut runs: Vec<(f32, Vec<usize>)> = runs
            .into_iter()
            .map(|r| (length(&r), r))
            .filter(|(l, r)| {
                let loose = |i: usize| self.adj[i].len() <= 1;
                // a free-standing speck is noise; a short run between
                // junctions is part of the drawing
                !(loose(r[0]) && loose(r[r.len() - 1]) && *l < self.spacing * 3.5)
            })
            .collect();
        runs.sort_by(|a, b| b.0.total_cmp(&a.0));
        runs.into_iter()
            .map(|(_, r)| relax(r.into_iter().map(|i| pts[i]).collect()))
            .collect()
    }
}

/// Even out the jitter of the dots along a line: a few passes of
/// neighbour averaging, ends kept (a closed loop is averaged all round).
fn relax(mut line: Vec<P>) -> Vec<P> {
    let n = line.len();
    if n < 3 {
        return line;
    }
    let closed = dist(line[0], line[n - 1]) < 1e-6;
    for _ in 0..4 {
        let prev = line.clone();
        for i in 0..n {
            let (a, b) = match (i, closed) {
                (0, false) => continue,
                (i, false) if i == n - 1 => continue,
                (0, true) => (prev[n - 2], prev[1]),
                (i, true) if i == n - 1 => (prev[n - 2], prev[1]),
                (i, _) => (prev[i - 1], prev[i + 1]),
            };
            line[i] = [
                0.25 * a[0] + 0.5 * prev[i][0] + 0.25 * b[0],
                0.25 * a[1] + 0.5 * prev[i][1] + 0.25 * b[1],
            ];
        }
    }
    line
}

/// Hatch strokes inside the closed shapes the `lines` make (in real units:
/// width 1, height `aspect`): parallel diagonals, heavier on the side away
/// from a light at the top left.
fn hatch(lines: &[Vec<P>], aspect: f32, spacing: f32, s: Shading) -> Vec<Vec<P>> {
    let cell = spacing.max(0.004);
    let (cols, rows) = (
        ((1.0 / cell).ceil() as usize + 4).min(500),
        ((aspect / cell).ceil() as usize + 4).min(500),
    );
    let cell_of = |p: P| -> (usize, usize) {
        (
            ((p[0] / cell) as isize + 2).clamp(0, cols as isize - 1) as usize,
            ((p[1] / cell) as isize + 2).clamp(0, rows as isize - 1) as usize,
        )
    };
    // the lines, thickened a cell so small gaps still close a shape
    let mut wall = vec![false; cols * rows];
    for l in lines {
        for w in l.windows(2) {
            let steps = (dist(w[0], w[1]) / (cell * 0.5)).ceil().max(1.0) as usize;
            for k in 0..=steps {
                let f = k as f32 / steps as f32;
                let p = [
                    w[0][0] + (w[1][0] - w[0][0]) * f,
                    w[0][1] + (w[1][1] - w[0][1]) * f,
                ];
                let (cx, cy) = cell_of(p);
                for dy in -1..=1isize {
                    for dx in -1..=1isize {
                        let (x, y) = (cx as isize + dx, cy as isize + dy);
                        if x >= 0 && y >= 0 && (x as usize) < cols && (y as usize) < rows {
                            wall[y as usize * cols + x as usize] = true;
                        }
                    }
                }
            }
        }
    }
    // outside: everything reached from the border without crossing a line
    let mut outside = vec![false; cols * rows];
    let mut queue = VecDeque::new();
    for x in 0..cols {
        for y in [0, rows - 1] {
            queue.push_back((x, y));
        }
    }
    for y in 0..rows {
        for x in [0, cols - 1] {
            queue.push_back((x, y));
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let i = y * cols + x;
        if outside[i] || wall[i] {
            continue;
        }
        outside[i] = true;
        if x > 0 {
            queue.push_back((x - 1, y));
        }
        if x + 1 < cols {
            queue.push_back((x + 1, y));
        }
        if y > 0 {
            queue.push_back((x, y - 1));
        }
        if y + 1 < rows {
            queue.push_back((x, y + 1));
        }
    }
    let inside = |p: P| {
        let (x, y) = cell_of(p);
        !outside[y * cols + x]
    };
    let interior = (0..cols * rows)
        .filter(|&i| !outside[i] && !wall[i])
        .count();
    if interior < cols * rows / 40 {
        // an open drawing: nothing encloses enough to shade
        return Vec::new();
    }
    // where the shape is, to find its shadow side
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for y in 0..rows {
        for x in 0..cols {
            if !outside[y * cols + x] {
                let t = x as f32 + y as f32;
                lo = lo.min(t);
                hi = hi.max(t);
            }
        }
    }
    let shade = |p: P| -> f32 {
        let (x, y) = cell_of(p);
        ((x as f32 + y as f32 - lo) / (hi - lo).max(1.0)).clamp(0.0, 1.0)
    };
    let mut out = Vec::new();
    let gap = s.spacing.max(cell * 1.5);
    let mut layer = |dir: P, keep: &dyn Fn(P) -> bool| {
        // lines along `dir`, stepped across it
        let normal = [-dir[1], dir[0]];
        let span = (1.0 + aspect) * 1.5;
        let mut k = -span;
        while k <= span {
            let base = [0.5 + normal[0] * k, aspect / 2.0 + normal[1] * k];
            let step = cell * 0.5;
            let mut run: Option<P> = None;
            let mut last = base;
            let mut t = -span;
            while t <= span {
                let p = [base[0] + dir[0] * t, base[1] + dir[1] * t];
                let on = p[0] >= 0.0
                    && p[0] <= 1.0
                    && p[1] >= 0.0
                    && p[1] <= aspect
                    && inside(p)
                    && keep(p);
                match (on, run) {
                    (true, None) => run = Some(p),
                    (false, Some(a)) => {
                        if dist(a, last) > cell * 2.0 {
                            out.push(vec![a, last]);
                        }
                        run = None;
                    }
                    _ => {}
                }
                last = p;
                t += step;
            }
            if let Some(a) = run
                && dist(a, last) > cell * 2.0
            {
                out.push(vec![a, last]);
            }
            k += gap;
        }
    };
    let r = std::f32::consts::FRAC_1_SQRT_2;
    let everywhere = s.everywhere;
    layer([r, -r], &|p| everywhere || shade(p) > 0.45);
    if s.cross {
        layer([r, r], &|p| shade(p) > 0.55);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dotted circle of `n` dots and radius 0.4 round the centre.
    fn circle(n: usize) -> Vec<P> {
        (0..n)
            .map(|k| {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                [0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()]
            })
            .collect()
    }

    #[test]
    fn a_dotted_circle_is_one_closed_line_not_a_maze() {
        // shuffled, the way clouds come (importance order, not drawing order)
        let mut dots = circle(120);
        for i in 0..dots.len() {
            dots.swap(i, (i * 37 + 11) % 120);
        }
        let t = trace(&dots, 1.0, None);
        assert_eq!(t.lines.len(), 1, "{} lines", t.lines.len());
        let line = &t.lines[0];
        assert!(line.len() >= 118, "{} of 120 dots", line.len());
        // every step is a short step along the circle, never across it
        assert!(line.windows(2).all(|w| dist(w[0], w[1]) < 0.06));
        assert!(t.hatch.is_empty());
    }

    #[test]
    fn closed_shapes_are_hatched_and_open_ones_are_not() {
        let shading = Shading {
            spacing: 0.04,
            everywhere: true,
            cross: true,
        };
        let closed = trace(&circle(120), 1.0, Some(shading));
        assert!(closed.hatch.len() > 10, "{}", closed.hatch.len());
        // hatching stays inside the circle
        for h in &closed.hatch {
            for p in h {
                assert!(dist(*p, [0.5, 0.5]) < 0.45, "{p:?}");
            }
        }
        // the shadow side (bottom right) is cross-hatched, the lit side not
        let crossing = |h: &Vec<P>| (h[1][0] - h[0][0]) * (h[1][1] - h[0][1]) > 0.0;
        let crossed: Vec<&Vec<P>> = closed.hatch.iter().filter(|h| crossing(h)).collect();
        assert!(!crossed.is_empty());
        assert!(
            crossed
                .iter()
                .all(|h| h[0][0] + h[0][1] > 0.8 || h[1][0] + h[1][1] > 0.8)
        );
        let line: Vec<P> = (0..60).map(|k| [0.1 + k as f32 * 0.013, 0.5]).collect();
        let open = trace(&line, 1.0, Some(shading));
        assert_eq!(open.lines.len(), 1);
        assert!(open.hatch.is_empty());
    }

    #[test]
    fn specks_are_dropped() {
        let mut dots = circle(120);
        dots.push([0.02, 0.02]);
        dots.push([0.95, 0.03]);
        let t = trace(&dots, 1.0, None);
        assert_eq!(t.lines.len(), 1);
    }
}
