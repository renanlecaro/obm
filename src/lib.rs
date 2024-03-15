extern crate console_error_panic_hook;

use std::cmp::{max, min};
use std::collections::HashSet;
use std::ops::Deref;

use rand::prelude::*;
use random;
use wasm_bindgen::prelude::*;

use Direction::{Down, Left, Right, Up};
use Mutation::*;

const GRID_WIDTH: i32 = 6;

const GRID_HEIGHT: i32 = 4;

pub struct RulesParams {
    individual: Individual,
    descendants_ids: Vec<Vec<usize>>,
    envelopes: Vec<Rectangle>,
    envelope_borders: Vec<[Rectangle; 4]>,
    link_parts: Vec<(usize, Link, Vec<Rectangle>)>,
    nodes_scores: Vec<i32>,
}

fn get_rules() -> Vec<(&'static str, i32, Box<dyn Fn(&mut RulesParams) -> i32>)> {
    vec![
        (
            "avoid name/name overlap",
            10,
            Box::new(|rp| {
                let mut t = 0;
                for a in rp.individual.nodes.iter() {
                    for b in &rp.individual.nodes[a.id + 1..] {
                        let delta = &a.position.overlap_with(&b.position);
                        rp.nodes_scores[a.id] += delta;
                        rp.nodes_scores[b.id] += delta;

                        t += delta;
                    }
                }
                t
            }),
        ),
        (
            "order nodes left to right",
            1,
            Box::new(|rp| {
                let mut t = 0;
                for a in rp.individual.nodes.iter() {
                    for b in &rp.individual.nodes[a.id + 1..] {
                        if b.position.x < a.position.x {
                            let delta = a.position.x - b.position.x;

                            rp.nodes_scores[b.id] += delta;
                            t += delta;
                        }
                        if b.position.y < a.position.y {
                            let delta = a.position.y - b.position.y;

                            rp.nodes_scores[b.id] += delta;
                            t += delta;
                        }
                    }
                }
                t
            }),
        ),
        (
            "avoid name/envelope border overlap",
            10,
            Box::new(|rp| {
                let mut t = 0;
                for a in rp.individual.nodes.iter() {
                    for (id, borders) in rp.envelope_borders.iter().enumerate() {
                        if id != a.id {
                            for border in borders {
                                let delta = &a.position.overlap_with(&border);
                                rp.nodes_scores[a.id] += delta;
                                rp.nodes_scores[id] += delta;
                                t += delta;
                            }
                        }
                    }
                }
                t
            }),
        ),
        (
            "avoid envelope/envelope overlap for non parents",
            100,
            Box::new(|rp| {
                let mut t = 0;
                for a in rp.individual.nodes.iter() {
                    for b in &rp.individual.nodes[a.id + 1..] {
                        if !(rp.descendants_ids[a.id].contains(&b.id)
                            || rp.descendants_ids[b.id].contains(&a.id))
                        {
                            let delta = &rp.envelopes[a.id].overlap_with(&rp.envelopes[b.id]);
                            // let delta = overlap_padded(&rp.envelopes[a.id], &rp.envelopes[b.id], 2);
                            rp.nodes_scores[a.id] += delta;
                            rp.nodes_scores[b.id] += delta;
                            t += delta;
                        }
                    }
                }
                t
            }),
        ),
        (
            "envelopes should stay close to the center",
            1,
            Box::new(|rp| {
                let mut t = 0;
                let center = Point {
                    x: rp.individual.width / 2,
                    y: rp.individual.height / 2,
                };
                for a in rp.individual.nodes.iter() {
                    let delta = max(
                        rp.envelopes[a.id].top_left().d2(&center),
                        rp.envelopes[a.id].bottom_right().d2(&center),
                    ) / 20;
                    rp.nodes_scores[a.id] += delta;
                    t += delta;
                }
                t
            }),
        ),
        (
            "enforce graph bounds",
            100,
            Box::new(|rp| {
                let mut t = 0;
                let graph_area = Rectangle {
                    x: 0,
                    y: 0,
                    w: rp.individual.width,
                    h: rp.individual.height,
                };

                for a in rp.individual.nodes.iter() {
                    let envelope = &rp.envelopes[a.id];
                    let delta =
                        envelope.overlap_with(envelope) - envelope.overlap_with(&graph_area);
                    rp.nodes_scores[a.id] += delta;
                    t += delta;
                }

                for (_, link, parts) in rp.link_parts.iter() {
                    for rect in parts {
                        let delta = rect.overlap_with(rect) - rect.overlap_with(&graph_area);
                        rp.nodes_scores[link.from] += delta;
                        rp.nodes_scores[link.to] += delta;
                        t += delta;
                    }
                }
                t
            }),
        ),
        (
            // When integrating a graph in a md doc, we don't want it to get too tall
            "minize graph height",
            50,
            Box::new(|rp| {
                let mut ymin = i32::MAX;
                let mut ymax = i32::MIN;

                for e in rp.envelopes.iter() {
                    ymin = min(ymin, e.y);
                    ymax = max(ymax, e.y + e.h);
                }

                for (_, _link, parts) in rp.link_parts.iter() {
                    for e in parts {
                        ymin = min(ymin, e.y);
                        ymax = max(ymax, e.y + e.h);
                    }
                }
                ymax - ymin
            }),
        ),
        (
            "envelopes shouldn't get too big",
            1,
            Box::new(|rp| {
                let mut t = 0;
                for a in rp.individual.nodes.iter() {
                    let delta = (rp.envelopes[a.id].w + rp.envelopes[a.id].h) / 4;
                    rp.nodes_scores[a.id] += delta;
                    t += delta;
                }
                t
            }),
        ),
        (
            "names should be close to top left of envelopes",
            10,
            Box::new(|rp| {
                let mut t = 0;
                for node in rp.individual.nodes.iter() {
                    let delta = node
                        .position
                        .top_left()
                        .d2(&rp.envelopes[node.id].top_left());

                    rp.nodes_scores[node.id] += delta;

                    t += delta;
                }
                t
            }),
        ),
        (
            "avoid link/link overlap",
            100,
            Box::new(|rp| {
                let mut t = 0;
                for (link_a_index, link_a, rects_a) in rp.link_parts.iter() {
                    for (link_b_index, link_b, rects_b) in rp.link_parts.iter() {
                        if link_b_index > link_a_index {
                            for link_a_part in rects_a {
                                for link_b_part in rects_b {
                                    let delta = &link_a_part.overlap_with(&link_b_part);

                                    rp.nodes_scores[link_a.from] += delta;
                                    rp.nodes_scores[link_a.to] += delta;
                                    rp.nodes_scores[link_b.from] += delta;
                                    rp.nodes_scores[link_b.to] += delta;

                                    t += delta;
                                }
                            }
                        }
                    }
                }
                t
            }),
        ),
        (
            "avoid link/name",
            100,
            Box::new(|rp| {
                let mut t = 0;
                for (_, link_a, rects_a) in rp.link_parts.iter() {
                    for b in rp.individual.nodes.iter() {
                        for rect in rects_a {
                            let delta = rect.overlap_with(&b.position);

                            rp.nodes_scores[link_a.from] += delta;
                            rp.nodes_scores[link_a.to] += delta;
                            rp.nodes_scores[b.id] += delta;

                            t += delta;
                        }
                    }
                }
                t
            }),
        ),
        (
            "avoid link/non child envelope overlap",
            100,
            Box::new(|rp| {
                let mut t = 0;
                for (_, link_a, rects_a) in rp.link_parts.iter() {
                    for b in rp.individual.nodes.iter() {
                        // Ignore a link traversing the parent of its target
                        if !(rp.descendants_ids[b.id].contains(&link_a.from)
                            || rp.descendants_ids[b.id].contains(&link_a.to))
                        {
                            for rect in rects_a {
                                let delta = rect.overlap_with(&b.position);

                                rp.nodes_scores[link_a.from] += delta;
                                rp.nodes_scores[link_a.to] += delta;
                                rp.nodes_scores[b.id] += delta;

                                t += delta;
                            }
                        }
                    }
                }
                t
            }),
        ),
        (
            "avoid link/border overlap",
            10,
            Box::new(|rp| {
                let mut t = 0;
                for (_, link_a, rects_a) in rp.link_parts.iter() {
                    for b in rp.individual.nodes.iter() {
                        for rect in rects_a {
                            for border in &rp.envelope_borders[b.id] {
                                let delta = rect.overlap_with(border);

                                rp.nodes_scores[link_a.from] += delta;
                                rp.nodes_scores[link_a.to] += delta;
                                rp.nodes_scores[b.id] += delta;

                                t += delta;
                            }
                        }
                    }
                }
                t
            }),
        ),
        (
            "avoid angles in links",
            1,
            Box::new(|rp| {
                let mut t = 0;
                for (_, link_a, rects_a) in rp.link_parts.iter() {
                    let delta = rects_a.len() as i32;

                    rp.nodes_scores[link_a.from] += delta;
                    rp.nodes_scores[link_a.to] += delta;
                    t += delta;
                }
                t
            }),
        ),
        (
            "links should be short if possible",
            1,
            Box::new(|rp| {
                let mut t = 0;
                for (_, link, rects_a) in rp.link_parts.iter() {
                    for rect in rects_a {
                        let delta = rect.w + rect.h * 2;

                        rp.nodes_scores[link.from] += delta;
                        rp.nodes_scores[link.to] += delta;
                        t += delta;
                    }
                }
                t
            }),
        ),
    ]
}

#[derive(PartialEq, Debug, Clone, Copy)]

pub enum Direction {
    Left,
    Up,
    Down,
    Right,
}

impl Direction {
    fn flip(&self) -> Direction {
        match self {
            Up => Down,
            Down => Up,
            Left => Right,
            Right => Left,
        }
    }
    fn a_to_b(a: &Point, b: &Point) -> Direction {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        if dx.abs() < dy.abs() {
            if a.y < b.y {
                Down
            } else {
                Up
            }
        } else {
            if a.x < b.x {
                Right
            } else {
                Left
            }
        }
    }
}

#[test]
fn directions() {
    assert_eq!(Up.flip(), Down);
    assert_eq!(Down.flip(), Up);
    assert_eq!(Left.flip(), Right);
    assert_eq!(Right.flip(), Left);
    assert_eq!(
        Direction::a_to_b(&Point { x: 0, y: 0 }, &Point { x: 10, y: 0 }),
        Right
    );
    assert_eq!(
        Direction::a_to_b(&Point { x: 0, y: 0 }, &Point { x: 0, y: 10 }),
        Down
    );
    assert_eq!(
        Direction::a_to_b(&Point { x: 10, y: 0 }, &Point { x: 0, y: 0 }),
        Left
    );
    assert_eq!(
        Direction::a_to_b(&Point { x: 0, y: 10 }, &Point { x: 0, y: 0 }),
        Up
    );
}

fn angle_character<'a>(from: &Direction, to: &Direction, font: &'a [&str; 6]) -> &'a str {
    let [v, h, a, b, c, d] = font;

    match (from, to) {
        (Up, Right) => a,
        (Left, Down) => a,
        (Right, Down) => b,
        (Up, Left) => b,
        (Down, Left) => c,
        (Right, Up) => c,
        (Down, Right) => d,
        (Left, Up) => d,
        (Left, _) => h,
        (Right, _) => h,
        (Up, _) => v,
        (Down, _) => v,
    }
}
fn transition(a: &Point, b: &Point, mode: bool) -> Point {
    if mode {
        Point { x: a.x, y: b.y }
    } else {
        Point { x: b.x, y: a.y }
    }
}

fn walk_rectangle_perimeter(rect: &Rectangle, walk: i32) -> (Direction, Point) {
    let circumference = rect.w * 2 + rect.h * 2;
    let walk = (walk % circumference + circumference) % circumference;
    assert!(walk >= 0);
    assert!(walk < circumference);
    // Top
    if walk < rect.w {
        return (
            Up,
            Point {
                x: rect.x + walk,
                y: rect.y - 1,
            },
        );
    }
    // right
    let walk = walk - rect.w;
    if walk < rect.h {
        return (
            Right,
            Point {
                x: rect.x + rect.w,
                y: rect.y + walk,
            },
        );
    }
    // bottom
    let walk = walk - rect.h;
    if walk < rect.w {
        return (
            Down,
            Point {
                x: rect.x + rect.w - 1 - walk,
                y: rect.y + rect.h,
            },
        );
    }
    let walk = walk - rect.w;

    return (
        Left,
        Point {
            x: rect.x - 1,
            y: rect.y + rect.h - 1 - walk,
        },
    );
}

#[test]
fn linking_point_test() {
    let rect_3x3 = Rectangle {
        x: 0,
        y: 0,
        w: 3,
        h: 3,
    };

    [
        (Up, Point { x: 0, y: -1 }),
        (Up, Point { x: 1, y: -1 }),
        (Up, Point { x: 2, y: -1 }),
        (Right, Point { x: 3, y: 0 }),
        (Right, Point { x: 3, y: 1 }),
        (Right, Point { x: 3, y: 2 }),
        (Down, Point { x: 2, y: 3 }),
        (Down, Point { x: 1, y: 3 }),
        (Down, Point { x: 0, y: 3 }),
        (Left, Point { x: -1, y: 2 }),
        (Left, Point { x: -1, y: 1 }),
        (Left, Point { x: -1, y: 0 }),
        (Up, Point { x: 0, y: -1 }),
    ]
    .into_iter()
    .enumerate()
    .for_each(|(walk, expected)| {
        assert_eq!(walk_rectangle_perimeter(&rect_3x3, walk as i32), expected)
    })
}

fn stops_of_link(
    from: &Rectangle,
    to: &Rectangle,
    link: &Link,
) -> (Direction, Vec<Point>, Direction) {
    let (start_dir, start) = walk_rectangle_perimeter(&from, link.start);
    let (end_dir, end) = walk_rectangle_perimeter(&to, link.end);
    let center = transition(&start, &end, link.mode);
    let mut stops = vec![start, center, end];
    stops.dedup();
    return (start_dir, stops, end_dir.flip());
}

fn overlap_1d(x1: i32, w1: i32, x2: i32, w2: i32) -> i32 {
    let (x1, w1, x2, w2) = if x1 > x2 {
        (x2, w2, x1, w1)
    } else {
        (x1, w1, x2, w2)
    };

    if x1 + w1 <= x2 {
        return 0;
    }
    if x2 >= x1 && x2 <= x1 + w1 && x2 + w2 >= x1 + w1 {
        return x1 + w1 - x2;
    }
    return w2;
}

#[test]
fn overlap_1d_test() {
    assert_eq!(overlap_1d(0, 10, 0, 10), 10);
    assert_eq!(overlap_1d(0, 10, 0, 5), 5);
    assert_eq!(overlap_1d(0, 10, 5, 10), 5);
    assert_eq!(overlap_1d(0, 10, 10, 10), 0);
    assert_eq!(overlap_1d(0, 10, 20, 10), 0);
}

fn overlap(r1: &Rectangle, r2: &Rectangle) -> i32 {
    return overlap_1d(r1.x, r1.w, r2.x, r2.w) * overlap_1d(r1.y, r1.h, r2.y, r2.h);
}

#[test]
fn overlap_test() {
    assert_eq!(
        overlap(
            &Rectangle {
                x: 0,
                y: 0,
                w: 1,
                h: 1
            },
            &Rectangle {
                x: 0,
                y: 0,
                w: 1,
                h: 1
            }
        ),
        1
    );
    assert_eq!(
        overlap(
            &Rectangle {
                x: 0,
                y: 0,
                w: 1,
                h: 1
            },
            &Rectangle {
                x: 2,
                y: 0,
                w: 1,
                h: 1
            }
        ),
        0
    );
    assert_eq!(
        overlap(
            &Rectangle {
                x: 0,
                y: 0,
                w: 10,
                h: 10
            },
            &Rectangle {
                x: 5,
                y: 5,
                w: 10,
                h: 10
            }
        ),
        5 * 5
    );
    assert_eq!(
        overlap(
            &Rectangle {
                x: 0,
                y: 0,
                w: 10,
                h: 10
            },
            &Rectangle {
                x: 0,
                y: 5,
                w: 10,
                h: 10
            }
        ),
        5 * 10
    );
}

fn overlap_padded(r1: &Rectangle, r2: &Rectangle, padding: i32) -> i32 {
    assert!(padding >= 0);
    overlap_1d(
        r1.x - padding,
        r1.w + padding * 2,
        r2.x - padding,
        r2.w + padding * 2,
    ) * overlap_1d(
        r1.y - padding,
        r1.h + padding * 2,
        r2.y - padding,
        r2.h + padding * 2,
    )
}
#[test]
fn overlap_padded_test() {
    assert_eq!(
        overlap(
            &Rectangle {
                x: 10,
                y: 10,
                w: 1,
                h: 1
            },
            &Rectangle {
                x: 10,
                y: 10,
                w: 1,
                h: 1
            }
        ),
        1
    );
    assert_eq!(
        overlap_padded(
            &Rectangle {
                x: 10,
                y: 10,
                w: 1,
                h: 1
            },
            &Rectangle {
                x: 10,
                y: 10,
                w: 1,
                h: 1
            },
            1
        ),
        3 * 3
    );
    assert_eq!(overlap_1d(0, 1, 0, 1), 1);
    assert_eq!(overlap_1d(0, 5, 0, 5), 5);
    assert_eq!(overlap_1d(-1, 1, -1, 1), 1);
    assert_eq!(overlap_1d(-1, 2, -1, 2), 2);
}

#[derive(PartialEq, Debug, Clone)]

pub struct Rectangle {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[test]
fn rectangle_gobble_up_test() {
    assert_eq!(
        Rectangle {
            x: 0,
            y: 0,
            w: 3,
            h: 3
        }
        .gobble_up(&Rectangle {
            x: 0,
            y: 0,
            w: 3,
            h: 3
        }),
        Rectangle {
            x: -1,
            y: -1,
            w: 5,
            h: 5
        }
    )
}
impl Rectangle {
    fn overlap_with(&self, target: &Rectangle) -> i32 {
        overlap(self, target)
    }
    fn overlaps(&self, target: &Rectangle) -> bool {
        self.overlap_with(target) > 0
    }
    fn gobble_up(&self, target: &Rectangle) -> Rectangle {
        //  Follow grid here ?
        let x = [
            self.x,
            self.x + self.w - 1,
            target.x - 1,
            target.x + target.w,
        ];
        let y = [
            self.y,
            self.y + self.h - 1,
            target.y - 1,
            target.y + target.h,
        ];
        Rectangle::from_points(
            &Point {
                x: *x.iter().min().unwrap(),
                y: *y.iter().min().unwrap(),
            },
            &Point {
                x: *x.iter().max().unwrap(),
                y: *y.iter().max().unwrap(),
            },
        )
    }
    fn from_points(p1: &Point, p2: &Point) -> Self {
        let (x, w) = if p1.x < p2.x {
            (p1.x, p2.x - p1.x + 1)
        } else {
            (p2.x, p1.x - p2.x + 1)
        };
        let (y, h) = if p1.y < p2.y {
            (p1.y, p2.y - p1.y + 1)
        } else {
            (p2.y, p1.y - p2.y + 1)
        };
        Rectangle { x, y, w, h }
    }
    fn center(&self) -> Point {
        Point {
            x: self.x + self.w / 2,
            y: self.y + self.h / 2,
        }
    }
    fn shift_by(&self, p: &Point) -> Rectangle {
        Rectangle {
            x: self.x + p.x,
            y: self.y + p.y,
            ..*self
        }
    }
    fn top_left(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }
    fn bottom_right(&self) -> Point {
        Point {
            x: self.x + self.w - 1,
            y: self.y + self.h - 1,
        }
    }
    fn left(&self) -> Point {
        Point {
            x: self.x - 1,
            y: self.y + self.h / 2,
        }
    }
    fn top(&self) -> Point {
        Point {
            x: self.x + self.w / 2,
            y: self.y - 1,
        }
    }
    fn right(&self) -> Point {
        Point {
            x: self.x + self.w,
            y: self.y + self.h / 2,
        }
    }
    fn bottom(&self) -> Point {
        Point {
            x: self.x + self.w / 2,
            y: self.y + self.h,
        }
    }

    fn size(&self) -> i32 {
        self.h * 2 * self.w
    }

    fn borders(&self) -> [Rectangle; 4] {
        [
            Rectangle {
                x: self.x,
                y: self.y,
                w: self.w,
                h: 1,
            },
            Rectangle {
                x: self.x,
                y: self.y + self.h - 1,
                w: self.w,
                h: 1,
            },
            Rectangle {
                x: self.x,
                y: self.y + 1,
                w: 1,
                h: self.h - 2,
            },
            Rectangle {
                x: self.x + self.w - 1,
                y: self.y + 1,
                w: 1,
                h: self.h - 2,
            },
        ]
    }
}

#[test]
fn rectangle_borders() {
    assert_eq!(
        Rectangle {
            x: 0,
            y: 0,
            w: 3,
            h: 3
        }
        .borders(),
        [
            Rectangle {
                x: 0,
                y: 0,
                w: 3,
                h: 1
            },
            Rectangle {
                x: 0,
                y: 2,
                w: 3,
                h: 1
            },
            Rectangle {
                x: 0,
                y: 1,
                w: 1,
                h: 1
            },
            Rectangle {
                x: 2,
                y: 1,
                w: 1,
                h: 1
            }
        ]
    )
}

#[derive(PartialEq, Debug, Clone)]

pub struct Point {
    x: i32,
    y: i32,
}

fn fit_on_x_grid(position: i32) -> i32 {
    (position / GRID_WIDTH) * GRID_WIDTH
}

fn fit_on_y_grid(position: i32) -> i32 {
    (position / GRID_HEIGHT) * GRID_HEIGHT
}

impl Point {
    fn d2(&self, b: &Point) -> i32 {
        (self.x - b.x).pow(2) + ((self.y - b.y) * 2).pow(2)
    }
    fn dabs(&self, b: &Point) -> i32 {
        (self.x - b.x).abs() + (self.y - b.y).abs() * 2
    }

    fn moved_in_direction_of(&self, b: &Point) -> Point {
        if self == b {
            return self.clone();
        }
        match Direction::a_to_b(self, b) {
            Up => Point {
                x: self.x,
                y: self.y - 1,
            },
            Down => Point {
                x: self.x,
                y: self.y + 1,
            },
            Left => Point {
                x: self.x - 1,
                y: self.y,
            },
            Right => Point {
                x: self.x + 1,
                y: self.y,
            },
        }
    }
}

#[derive(PartialEq, Debug, Clone)]

pub struct Individual {
    nodes: Vec<Node>,
    links: Vec<Link>,
    width: i32,
    height: i32,
    descendants_ids: Option<Vec<Vec<usize>>>,
}

pub struct StoryStep {
    visible_nodes_ids: HashSet<usize>,
    visible_link_ids: HashSet<usize>,

    highlighted_nodes_ids: HashSet<usize>,
    highlighted_link_ids: HashSet<usize>,

    pub md: String,
}

#[derive(PartialEq, Debug, Clone)]

pub struct Node {
    id: usize,
    parent: Option<usize>,
    depth: usize,
    position: Rectangle,
    name: String,
    fixed: bool,
}
impl Individual {
    fn sample_node_id(&self) -> usize {
        let mut rng = thread_rng();
        self.nodes.choose(&mut rng).unwrap().id
    }
    fn upsert_node(&mut self, name: &str, parent: Option<usize>) -> usize {
        if let Some(found) = self
            .nodes
            .iter()
            .find(|n| n.name == name && (parent == n.parent))
        {
            return found.id;
        }

        // let mut rng = thread_rng();
        let name = format!("{}", name);
        let w = name.chars().count() as i32 + 4;
        let id = self.nodes.len();

        let depth = if let Some(parent_id) = parent {
            self.nodes.iter().find(|n| n.id == parent_id).unwrap().depth + 1
        } else {
            0
        };
        let new_node = Node {
            id,
            parent,
            depth,
            name,
            fixed: false,
            position: self.random_position(w),
        };
        self.nodes.push(new_node);
        let mut cursor = parent;
        loop {
            if let Some(id) = cursor {
                cursor = self.nodes[id].parent;
            } else {
                break;
            }
        }

        id
    }

    fn upsert_link(&mut self, from: usize, to: usize) -> usize {
        if let Some(link) = self.links.iter().find(|link| {
            (link.from == from && link.to == to) || (link.from == to && link.to == from)
        }) {
            link.id
        } else {
            let id = self.links.len();
            self.links.push(Link {
                id,
                from,
                to,
                start: 0,
                end: 0,
                fixed: false,
                mode: random(),
            });
            id
        }
    }

    pub fn from_string(str: &str, width: i32, height: i32) -> (Individual, Vec<StoryStep>) {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        assert!(width > 10);
        assert!(height > 10);
        let mut individual = Individual {
            nodes: vec![],
            links: vec![],
            width,
            height,
            descendants_ids: None,
        };
        let mut story = vec![StoryStep {
            visible_nodes_ids: HashSet::new(),
            visible_link_ids: HashSet::new(),
            highlighted_nodes_ids: HashSet::new(),
            highlighted_link_ids: HashSet::new(),
            md: String::new(),
        }];

        str.lines().for_each(|l| {
            if l.contains("->") && !l.starts_with("//") && !l.contains("<") {
                let current_story = story.last().unwrap();
                if !current_story.md.is_empty() {
                    story.push(StoryStep {
                        visible_nodes_ids: current_story.visible_nodes_ids.clone(),
                        visible_link_ids: current_story.visible_link_ids.clone(),
                        highlighted_nodes_ids: HashSet::new(),
                        highlighted_link_ids: HashSet::new(),
                        md: String::new(),
                    })
                }
                let mut previous = None;
                for id in l.split("->") {
                    let id = id.trim();
                    let mut parent = None;
                    let mut new_node_id = None;
                    id.split(":").for_each(|path| {
                        if path.trim() != "" {
                            let id = individual.upsert_node(path.trim(), parent);
                            story.last_mut().unwrap().visible_nodes_ids.insert(id);
                            story.last_mut().unwrap().highlighted_nodes_ids.insert(id);
                            new_node_id = Some(id);
                            parent = Some(id);
                        }
                    });

                    if let Some(to) = new_node_id {
                        if let Some(from) = previous {
                            let id = individual.upsert_link(from, to);
                            story.last_mut().unwrap().visible_link_ids.insert(id);
                            story.last_mut().unwrap().highlighted_link_ids.insert(id);
                        }

                        previous = Some(to);
                    }
                }
            } else {
                story.last_mut().unwrap().md.push_str(l);
                story.last_mut().unwrap().md.push_str("\n");
            }
        });
        individual.recompute_descendants_ids();
        // We don't sort nodes by depth here because their position is their id
        (individual, story)
    }

    fn make_node(&mut self, id: &str) -> Option<usize> {
        let mut parent = None;
        let mut new_node_id = None;
        id.split(":").for_each(|path| {
            if path.trim() != "" {
                let id = self.upsert_node(path.trim(), parent);
                new_node_id = Some(id);
                parent = Some(id);
            }
        });
        new_node_id
    }

    fn envelopes(&self) -> Vec<Rectangle> {
        let mut result: Vec<Rectangle> = self.nodes.iter().map(|n| n.position.clone()).collect();
        // Reversing the list ensures that we grow the children first, then the parents
        self.nodes.iter().rev().for_each(|n| {
            let parent_id = n.parent;
            if let Some(parent_id) = parent_id {
                result[parent_id] = result[parent_id].gobble_up(&result[n.id]);
            }
        });
        result
    }

    fn recompute_descendants_ids(&mut self) {
        let mut descendants_ids: Vec<Vec<usize>> = vec![];
        for i in 0..self.nodes.len() {
            let mut list = vec![i];
            self.nodes[i + 1..].iter().for_each(|n| {
                if let Some(parent_id) = n.parent {
                    if list.contains(&parent_id) {
                        list.push(n.id);
                    }
                }
            });
            descendants_ids.push(list)
        }
        self.descendants_ids = Some(descendants_ids);
    }

    pub fn to_string(&self, step: &StoryStep) -> String {
        let bold = ["┃", "━", "┏", "┓", "┛", "┗"];
        let normal = ["│", "─", "┌", "┐", "┘", "└"];

        // Initialize empty canvas
        let mut lines = vec![];
        for _ in 0..self.height {
            let mut line = vec![];
            for _ in 0..self.width {
                line.push(' ');
            }
            lines.push(line)
        }

        let mut xmin = self.width as usize;
        let mut xmax = 0;
        let mut ymin = self.height as usize;
        let mut ymax = 0;

        //  draw envelopes
        let mut draw_char = |x: i32, y: i32, str: &str| {
            let x = x as usize;
            let y = y as usize;
            xmin = min(xmin, x);
            xmax = max(xmax, x);
            ymin = min(ymin, y);
            ymax = max(ymax, y);
            str.chars().enumerate().for_each(|(dx, char)| {
                if x + dx > 0 && x + dx < self.width as usize {
                    if y > 0 && y < self.height as usize {
                        lines[y][x + dx] = char;
                    }
                }
            })
        };
        let envelopes = self.envelopes();

        for node in self.nodes.iter() {
            if !step.visible_nodes_ids.contains(&node.id) {
                continue;
            }
            let font = if step.highlighted_nodes_ids.contains(&node.id) {
                bold
            } else {
                normal
            };
            let Rectangle { x, y, w, h } = envelopes[node.id];
            draw_char(x, y, font[2]);
            draw_char(x + w - 1, y, font[3]);
            draw_char(x + w - 1, y + h - 1, font[4]);
            draw_char(x, y + h - 1, font[5]);

            for xi in x + 1..x + w - 1 {
                draw_char(xi, y, font[1]);
                draw_char(xi, y + h - 1, font[1]);
            }
            for yi in y + 1..y + h - 1 {
                draw_char(x, yi, font[0]);
                draw_char(x + w - 1, yi, font[0]);
            }
        }

        for link in self.links.iter() {
            if !step.highlighted_link_ids.contains(&link.id) {
                continue;
            }
            let font = bold;

            let from = &envelopes[link.from];
            let to = &envelopes[link.to];

            let (mut last_direction, stops, to_dir) = stops_of_link(from, to, link);

            let mut iter = stops.iter();
            let mut last_point = iter.next().unwrap().clone();
            for point in iter {
                let point = point.clone();
                while last_point != point {
                    let current_direction = Direction::a_to_b(&last_point, &point);

                    let angle_character =
                        angle_character(&last_direction, &current_direction, &font);
                    last_direction = current_direction;
                    draw_char(last_point.x, last_point.y, angle_character);
                    last_point = last_point.moved_in_direction_of(&point);
                }
            }
            draw_char(
                last_point.x,
                last_point.y,
                angle_character(&last_direction, &to_dir, &font),
            )
        }

        for node in self.nodes.iter() {
            if !step.visible_nodes_ids.contains(&node.id) {
                continue;
            }
            draw_char(node.position.x + 2, node.position.y + 1, &node.name);
        }

        if ymin > ymax {
            return String::new();
        }
        lines[ymin..=ymax]
            .into_iter()
            .map(|line| format!("    {}", line[xmin..=xmax].into_iter().collect::<String>()))
            .collect::<Vec<String>>()
            .join("\n")
            + "\n"
    }

    fn random_position(&self, width: i32) -> Rectangle {
        let mut rng = thread_rng();
        return Rectangle {
            x: fit_on_x_grid(rng.gen_range(1..self.width - width)),
            y: fit_on_y_grid(rng.gen_range(1..self.height - 3)),
            w: width,
            h: 3,
        };
    }
    fn score_params(&self) -> RulesParams {
        let descendants_ids = self.descendants_ids.clone().unwrap();
        let envelopes = self.envelopes();
        let link_parts: Vec<(usize, Link, Vec<Rectangle>)> = self
            .links
            .iter()
            .enumerate()
            .map(|(link_index, link)| {
                let rects = stops_to_rects(
                    stops_of_link(&envelopes[link.from], &envelopes[link.to], &link).1,
                );
                (link_index, link.clone(), rects)
            })
            .collect();

        let envelope_borders: Vec<[Rectangle; 4]> = envelopes.iter().map(|e| e.borders()).collect();
        let mut nodes_scores = vec![];
        for _ in &self.nodes {
            nodes_scores.push(0);
        }
        RulesParams {
            individual: self.clone(),
            descendants_ids,
            envelopes,
            envelope_borders,
            link_parts,
            nodes_scores,
        }
    }
    pub fn score(&self) -> (i32, Vec<i32>) {
        let mut rules_params = self.score_params();
        let mut total = 0;
        get_rules()
            .iter()
            .enumerate()
            .for_each(|(index, (_, factor, rule))| {
                if *factor > 0 {
                    let score = rule.deref()(&mut rules_params) * factor;
                    total += score;
                }
            });

        return (total, rules_params.nodes_scores);
    }

    pub fn improve(&mut self) {
        let mut score = self.score().0;
        let _node_count = self.nodes.len();
        let link_count = self.links.len();
        let descendants_ids = self.descendants_ids.clone().unwrap();
        loop {
            let score_at_start = score;
            // First, center the whole graph
            let mut try_to_move_all = |x: i32, y: i32| loop {
                for node in self.nodes.iter_mut() {
                    if !node.fixed {
                        node.position.x += x;
                        node.position.y += y;
                    }
                }

                let new_score = self.score().0;
                if new_score < score {
                    score = new_score
                } else {
                    for node in self.nodes.iter_mut() {
                        if !node.fixed {
                            // We ignore fixed for this specific case
                            node.position.x -= x;
                            node.position.y -= y;
                        }
                    }
                    break;
                }
            };
            //
            for amount in [-10, 10, -1, 1] {
                try_to_move_all(amount * GRID_WIDTH, 0);
                try_to_move_all(0, amount * GRID_HEIGHT);
            }

            let mut scored = self
                .score()
                .1
                .into_iter()
                .enumerate()
                .collect::<Vec<(usize, i32)>>();

            scored.sort_by_key(|t| -t.1);

            let hottest_node_ids = scored.into_iter().map(|t| t.0).collect::<Vec<usize>>();

            let mut try_to_move = |x: i32, y: i32, id: usize, with_descendants: bool| loop {
                let ids = vec![id];
                let ids = if with_descendants {
                    &descendants_ids[id]
                } else {
                    &ids
                };
                self.nodes.iter_mut().for_each(|node| {
                    if ids.contains(&node.id) && !node.fixed {
                        node.position.x += x;
                        node.position.y += y;
                    }
                });

                let new_score = self.score().0;
                if new_score < score {
                    score = new_score
                } else {
                    self.nodes.iter_mut().for_each(|node| {
                        if ids.contains(&node.id) && !node.fixed {
                            node.position.x -= x;
                            node.position.y -= y;
                        }
                    });
                    break;
                }
                break;
            };

            for id in hottest_node_ids {
                for with_descendants in [true, false] {
                    for amount in [-10, 10, -1, 1] {
                        try_to_move(amount * GRID_WIDTH, 0, id, with_descendants);
                        try_to_move(0, amount * GRID_HEIGHT, id, with_descendants);
                    }
                }
            }

            let mut try_move_start_end = |id: usize, start: i32, end: i32| {
                if !self.links[id].fixed {
                    self.links[id].start += start;
                    self.links[id].end += end;
                    let new_score = self.score().0;
                    if new_score < score {
                        score = new_score
                    } else {
                        self.links[id].start -= start;
                        self.links[id].end -= end;
                    }
                }
            };

            for link_index in 0..link_count {
                for amount in [-10, 10, 5, -5, -1, 1] {
                    try_move_start_end(link_index, amount, 0);
                    try_move_start_end(link_index, 0, amount);
                    try_move_start_end(link_index, amount, amount);
                    try_move_start_end(link_index, amount, -amount);
                }
            }

            let mut try_flip_mode = |id: usize| {
                self.links[id].mode = !self.links[id].mode;
                let new_score = self.score().0;
                if new_score < score {
                    score = new_score
                } else {
                    self.links[id].mode = !self.links[id].mode;
                }
            };

            for link_index in 0..link_count {
                try_flip_mode(link_index);
            }

            if score == score_at_start {
                break;
            }
        }
    }

    pub fn mutate(&mut self) -> Mutation {
        let mut rng = thread_rng();

        let mutation = [
            FDG,
            MoveOne,
            MoveHalf,
            SwapTwo,
            MoveHottest,
            FullRandom,
            InsertRow,
            InsertColumn,
            Transpose,
        ]
        .choose(&mut rng)
        .unwrap();

        match mutation {
            FDG => fdg(self),
            MoveOne => {
                let id = self.sample_node_id();
                let pos = self.random_position(self.nodes[id].position.w);
                self.nodes[id].position = pos;
            }
            MoveHalf => {
                let dx = rng.gen_range(-10..10) * GRID_WIDTH;
                let dy = rng.gen_range(-10..10) * GRID_HEIGHT;
                for node in self.nodes.iter_mut() {
                    if random() {
                        node.position.x += dx;
                        node.position.y += dy;
                    }
                }
            }

            InsertRow => {
                for node in self.nodes.iter_mut() {
                    if node.position.y >= self.height / 2 {
                        node.position.y += GRID_HEIGHT;
                    }
                }
            }
            InsertColumn => {
                for node in self.nodes.iter_mut() {
                    if node.position.x >= self.width / 2 {
                        node.position.x += GRID_WIDTH;
                    }
                }
            }
            Transpose => {
                for node in self.nodes.iter_mut() {
                    node.position.y += fit_on_y_grid(node.position.x);
                    node.position.x += fit_on_x_grid(node.position.y);
                }
            }
            SwapTwo => {
                let a = self.sample_node_id();
                let b = self.sample_node_id();

                let tmp = self.nodes[a].position.clone();

                self.nodes[b].position.x = tmp.x;
                self.nodes[b].position.y = tmp.y;

                self.nodes[a].position.x = self.nodes[b].position.x;
                self.nodes[a].position.y = self.nodes[b].position.y;
            }
            MoveHottest => {
                let id: usize = self
                    .score()
                    .1
                    .into_iter()
                    .enumerate()
                    .max_by_key(|(_node_id, score)| *score)
                    .unwrap()
                    .0;

                let pos = self.random_position(self.nodes[id].position.w);
                self.nodes[id].position = pos;
            }
            // Full random
            FullRandom => {
                for index in 0..self.nodes.len() {
                    self.nodes[index].position = self.random_position(self.nodes[index].position.w);
                }

                for link in self.links.iter_mut() {
                    link.start = rng.gen_range(0..1000);
                    link.end = rng.gen_range(0..1000);
                    link.mode = random()
                }
            }
        };

        self.improve();

        *mutation
    }
}

fn stops_to_rects(points: Vec<Point>) -> Vec<Rectangle> {
    if points.len() == 0 {
        return vec![];
    }
    if points.len() == 1 {
        return vec![Rectangle {
            x: points[0].x,
            y: points[0].y,
            w: 1,
            h: 1,
        }];
    }

    let mut rectangles = vec![];
    let mut iter = points.iter();
    let mut last_point = iter.next().unwrap();

    for pt in iter {
        // if pt != last_point {
        let corner_1 = if rectangles.len() > 0 {
            last_point.moved_in_direction_of(&pt)
        } else {
            last_point.clone()
        };

        rectangles.push(Rectangle::from_points(&corner_1, &pt));
        last_point = pt;
        // }
    }

    rectangles
}

#[test]
fn test_stops_to_rects() {
    assert_eq!(
        stops_to_rects(vec![
            Point { x: 0, y: 0 },
            Point { x: 3, y: 0 },
            Point { x: 3, y: 3 }
        ]),
        vec![
            Rectangle {
                x: 0,
                y: 0,
                w: 4,
                h: 1
            },
            Rectangle {
                x: 3,
                y: 1,
                w: 1,
                h: 3
            },
        ]
    )
}

#[derive(PartialEq, Debug, Clone)]

pub struct Link {
    id: usize,
    from: usize,
    to: usize,
    start: i32,
    end: i32,
    fixed: bool,
    mode: bool,
}

fn fdg(source: &mut Individual) {
    let _starting_score = source.score().0;
    let center = (source.width as f32 / 2.0, source.height as f32 / 2.0);
    // FDG, using floats
    let mut nodes: Vec<(f32, f32, f32, f32)> = source
        .nodes
        .iter()
        .map(|n| {
            (
                n.position.center().x as f32,
                n.position.center().y as f32,
                0.,
                0.,
            )
        })
        .collect();

    let nodes_count = nodes.len();
    let mut links: Vec<(usize, usize)> = vec![];
    for node in source.nodes.iter() {
        if let Some(parent) = node.parent {
            links.push((node.id, parent))
        }
    }
    for link in source.links.iter() {
        links.push((link.from, link.to))
    }

    for i in 0..1000 {
        // Apply speed to position
        for a_index in 0..nodes_count {
            nodes[a_index].2 *= 0.90;
            nodes[a_index].3 *= 0.90;

            nodes[a_index].0 += nodes[a_index].2;
            nodes[a_index].1 += nodes[a_index].3;
        }
        // Bounce on edges

        for a_index in 0..nodes_count {
            nodes[a_index].2 += (center.0 - nodes[a_index].0) / 1000.0;
            nodes[a_index].3 += (center.1 - nodes[a_index].1) / 500.0;
        }

        for a_index in 0..nodes_count {
            for b_index in 0..nodes_count {
                if a_index != b_index {
                    let mut delta_speed_a = (0., 0.);

                    // Nodes avoid being too close
                    let mut b_to_a = (
                        nodes[a_index].0 - nodes[b_index].0,
                        nodes[a_index].1 - nodes[b_index].1,
                    );
                    if b_to_a == (0., 0.) {
                        // avoid dividing by zero when colliding
                        b_to_a = (1., 1.);
                    }

                    let d2 = b_to_a.0.powi(2) + b_to_a.1.powi(2);
                    let d = d2.sqrt();
                    let unit = (b_to_a.0 / d, b_to_a.1 / d);
                    delta_speed_a.0 += unit.0 / d2;
                    delta_speed_a.1 += unit.1 / d2;
                    // linked nodes avoid beeing too far
                    if d > 10. && links.contains(&(a_index, b_index))
                        || links.contains(&(b_index, a_index))
                    {
                        delta_speed_a.0 -= unit.0 * 0.02;
                        delta_speed_a.1 -= unit.1 * 0.02;
                    }

                    nodes[a_index].2 += delta_speed_a.0;
                    nodes[a_index].3 += delta_speed_a.1;
                }
            }
        }

        let mut avg_speed = 0.;
        for (_, _, dx, dy) in nodes.iter() {
            avg_speed += dx.abs() + dy.abs()
        }
        let avg_speed = avg_speed / nodes_count as f32;
        if avg_speed < 0.05 && i > 10 {
            break;
        }
    }
    for id in 0..nodes_count {
        source.nodes[id].position.x = nodes[id].0 as i32;
        source.nodes[id].position.y = nodes[id].1 as i32;
    }
}
const MUTATIONS_TOTAL: i32 = 7;

#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]

pub enum Mutation {
    FDG,
    MoveOne,
    MoveHalf,
    SwapTwo,
    MoveHottest,
    FullRandom,
    InsertRow,
    InsertColumn,
    Transpose,
}

#[wasm_bindgen]
pub fn md_to_md(source: String, width: i32, height: i32) -> String {
    let (mut best_world, story) = Individual::from_string(&source, width, height);

    // Mostly for the first run
    best_world.improve();

    let max_stalled_runs = 20;

    let mut best_score = best_world.score().0;
    let mut runs_with_no_improvement = 0;
    while runs_with_no_improvement < max_stalled_runs {
        let mut clone = best_world.clone();
        clone.mutate();
        let score = clone.score().0;
        if score < best_score {
            best_score = score;
            runs_with_no_improvement = 0;
            best_world = clone;
        } else {
            runs_with_no_improvement += 1
        }
    }

    let mut out = String::new();

    for step in story {
        out.push_str(&best_world.to_string(&step));
        out.push_str(&step.md);
    }

    out
}
