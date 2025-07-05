use std::collections::{HashMap, HashSet};

type Point = (usize, usize);
type Line = (Point, Point);

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            return i;
        }
        self.parent[i] = self.find(self.parent[i]);
        self.parent[i]
    }

    fn union(&mut self, i: usize, j: usize) -> bool {
        let root_i = self.find(i);
        let root_j = self.find(j);

        if root_i != root_j {
            if self.rank[root_i] < self.rank[root_j] {
                self.parent[root_i] = root_j;
            } else if self.rank[root_i] > self.rank[root_j] {
                self.parent[root_j] = root_i;
            } else {
                self.parent[root_j] = root_i;
                self.rank[root_i] += 1;
            }
            true
        } else {
            false
        }
    }
}

fn group(grid: &[f64], width: usize, height: usize) -> HashMap<usize, Vec<usize>> {
    if width == 0 || height == 0 {
        return HashMap::new();
    }

    let n_cells = width * height;
    let mut uf = UnionFind::new(n_cells);

    for x in 0..width {
        for y in 0..height {
            let current_idx = x + y * width;
            if grid[current_idx] > 0.0 {
                // right
                if x + 1 < width {
                    let right_idx = current_idx + 1;
                    if grid[right_idx] > 0.0 {
                        uf.union(current_idx, right_idx);
                    }
                }

                // bottom
                if y + 1 < height {
                    let bottom_idx = current_idx + width;
                    if grid[bottom_idx] > 0.0 {
                        uf.union(current_idx, bottom_idx);
                    }
                }
            }
        }
    }

    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n_cells {
        if grid[i] > 0.0 {
            let root = uf.find(i);
            groups.entry(root).or_default().push(i);
        }
    }

    groups
}

pub fn get_edges(grid: &[f64], width: usize, height: usize) -> HashMap<usize, Vec<Line>> {
    let group_map = group(&grid, width, height);
    let mut group_boundaries: HashMap<usize, Vec<Line>> = HashMap::new();

    for (root_id, indices) in group_map.iter() {
        let mut lines: Vec<Line> = Vec::new();

        for &idx in indices {
            let x = idx % width;
            let y = idx / width;

            // top
            if y == 0 || grid[idx - width] == 0.0 {
                lines.push(((x, y), (x + 1, y)));
            }
            // bottom
            if y == height - 1 || grid[idx + width] == 0.0 {
                lines.push(((x, y + 1), (x + 1, y + 1)));
            }
            // left
            if x == 0 || grid[idx - 1] == 0.0 {
                lines.push(((x, y), (x, y + 1)));
            }
            // right
            if x == width - 1 || grid[idx + 1] == 0.0 {
                lines.push(((x + 1, y), (x + 1, y + 1)));
            }
        }

        // remove duplicate boundary segments
        let mut unique_boundaries = HashSet::new();
        for line in lines {
            let normalized_line = if line.0 <= line.1 {
                line
            } else {
                (line.1, line.0)
            };
            unique_boundaries.insert(normalized_line);
        }

        group_boundaries.insert(*root_id, unique_boundaries.into_iter().collect());
    }

    let mut root_to_id: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 1;

    let mut result: HashMap<usize, Vec<Line>> = HashMap::new();

    for (root, boundaries) in group_boundaries {
        let entry = root_to_id.entry(root).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
        result.insert(*entry, boundaries);
    }

    result
}

pub fn edges_to_paths(edges: &Vec<Line>) -> Vec<Vec<Point>> {
    let mut point_to_edges: HashMap<Point, Vec<Point>> = HashMap::new();
    let mut edge_set: HashSet<Line> = HashSet::new();

    for &(a, b) in edges {
        point_to_edges.entry(a).or_default().push(b);
        point_to_edges.entry(b).or_default().push(a);
        edge_set.insert(if a <= b { (a, b) } else { (b, a) });
    }

    let mut paths = Vec::new();
    let mut used = HashSet::new();

    for &(start, end) in edges {
        let key = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        if used.contains(&key) {
            continue;
        }
        let mut path = Vec::new();
        let mut curr = end;
        path.push(start);
        used.insert(key);
        while curr != start {
            path.push(curr);
            let neighbors = &point_to_edges[&curr];
            let mut found = false;
            for &next in neighbors {
                let k = if curr <= next {
                    (curr, next)
                } else {
                    (next, curr)
                };
                if !used.contains(&k) && edge_set.contains(&k) {
                    used.insert(k);
                    curr = next;
                    found = true;
                    break;
                }
            }
            if !found {
                break; // not closed
            }
        }

        // separate inner loop
        let mut visited_point = HashMap::new();
        let mut l = path.len();
        let mut i = 0;
        while i < l - 1 {
            let insert_ret = visited_point.insert(path[i], i);
            if let Some(p) = insert_ret {
                paths.push(path.drain(p..i).rev().collect());
                l = path.len();
            }
            i += 1;
        }

        // only closed paths
        if path.len() > 2 && path[0] == *path.last().unwrap() {
            paths.push(path);
        } else if path.len() > 2 {
            path.push(path[0]);
            paths.push(path);
        }
    }

    let n = paths.len();
    for i in 0..n {
        let mut inside_count = 0;
        for j in 0..n {
            if i == j || paths[j].is_empty() {
                continue;
            }
            if point_in_polygon(paths[i][0], &paths[j]) {
                inside_count += 1;
            }
        }

        let area = signed_area(&paths[i]);
        if inside_count % 2 == 1 {
            if area < 0.0 {
                paths[i].reverse();
            }
        } else {
            if area > 0.0 {
                paths[i].reverse();
            }
        }
    }
    paths
}

fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    let (x, y) = (point.0 as isize, point.1 as isize);
    let mut inside = false;
    let n = polygon.len();
    for i in 0..n {
        let (x0, y0) = (polygon[i].0 as isize, polygon[i].1 as isize);
        let (x1, y1) = (
            polygon[(i + 1) % n].0 as isize,
            polygon[(i + 1) % n].1 as isize,
        );
        if (y0 > y) != (y1 > y) {
            let denom = y1 - y0;
            if denom == 0 {
                continue;
            }
            let intersect_x = (x1 - x0) * (y - y0) / denom + x0;
            if x < intersect_x {
                inside = !inside;
            }
        }
    }
    inside
}

fn signed_area(path: &[Point]) -> f64 {
    let n = path.len();
    let mut area = 0.0;
    for i in 0..n {
        let (x0, y0) = (path[i].0 as f64, path[i].1 as f64);
        let (x1, y1) = (path[(i + 1) % n].0 as f64, path[(i + 1) % n].1 as f64);
        area += (x0 * y1) - (x1 * y0);
    }
    area * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox() {
        let src = "
--##--
-##---
##----
-##---
--##--
"
        .trim()
        .replace("\n", "");

        let grid = src
            .as_bytes()
            .into_iter()
            .map(|x| if *x == b'#' { 1.0f64 } else { 0.0 });

        println!("Group:");
        println!("{:?}", group(&Vec::from_iter(grid.clone()), 6, 5));

        let boundaries = get_edges(&Vec::from_iter(grid), 5, 6);
        let paths = edges_to_paths(&Vec::from_iter(boundaries.into_values().flatten()));

        println!("Path:");
        println!("{:?}", paths);

        assert_eq!(paths.len(), 2);

        let areas: Vec<_> = paths.iter().map(|p| signed_area(p)).collect();
        assert_eq!(areas.iter().filter(|&a| *a < 0.0).count(), 1);
        assert_eq!(areas.iter().filter(|&a| *a > 0.0).count(), 1);
    }
}
