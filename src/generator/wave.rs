/*
* a board has a vector 2d of cells that can be in two states a collapsed one where
* it holds a usize a uncollapsed one that holds vector of usizes, the usizes are the
* index of a vector that contains all possible values that the cell could be,
* the vector is owned by the board it also holds the size of
* the units just to no make it annoying to retrive
*
* a cell can be in two states:
* - collapsed: where the cell contains the index of the unit that is like
* - uncollapsed: where the cell contains all the possible units that it can be
* a unit has 4 borders that can have n amounts of states
*
* units are like this
*
*     a b
*   |-----|
* b |     | a
* a |     | b
*   |-----|
*     b a
*
* for example
*
* lets say there is 4 types of borders of 2 bits long
*     ab
* AIR 00 where there is only air
* RIG 01 where there is only solid in the right
* LEF 10 where there is only solid in the left
* SOL 11 where there is only solid
*
* a fully air unit would be like
* n = AIR, s = RIG, w = AIR, e = AIR
*
* a surface unit would be
* n = AIR, s = SOL, w = LEF, e = RIG
*
* a border of two surface uints are
*
*     0 0       0 1
*   |-----|   |-----|
* 0 | uni | 0 | uni | 0
* 1 |  1  | 1 |  2  | 1
*   |-----|   |-----|
*     1 1       1 1
*
* for uni_1 it's east is 01
* for uni_2 its' west is 10
*
* and to evaluate that they share a border you need to
* "mirror" the value of one two make them match and then being
* able to say "yes this two match"
*
* the idea of that each border has to be 2 bits is just
* a easy representation of how they should be paired
*
*
* the border always must have an "mirror" border
*
*
*/

use crate::generator::random::Random;
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

pub trait TileConnection {
    fn can_connect(&self, x: usize, y: usize, direction: Direction, other: &Self) -> u64;
}

type Possible<T> = Vec<T>;

type Collapsed = usize;
type Uncollapsed = Vec<usize>;

#[derive(Clone, PartialEq, Eq)]
pub enum Cell {
    Collapsed(Collapsed),
    Uncollapsed(Uncollapsed),
}

macro_rules! compare_tiles {
    ($possible: tt, $pos: tt, $found: tt, $neighbor: tt, $direction: expr, $x: expr, $y: expr) => {
        $found = false;
        for v in $neighbor.uncollapse() {
            if $possible[$pos].can_connect($x, $y, $direction, &$possible[v]) > 0 {
                $found = true;
                break;
            }
        }
        if !$found {
            continue;
        }
    };
}

impl Cell {
    /*
     * when it is 1 the cell is collapsed
     * when it is different from one the cell in undetermined
     * and the value is all the possible states that it has
     */
    pub fn entropy(&self) -> usize {
        match self {
            Cell::Collapsed(_) => 1,
            /*
             * this part should really collapse the cell if the vector only holds
             * one value, but I don't think that will ever happen
             * so I don't care
             */
            Cell::Uncollapsed(v) => v.len(),
        }
    }

    pub fn collapsed(&self) -> bool {
        matches!(&self, Cell::Collapsed(_))
    }

    /*
     * this function should be only used once you
     * verified the state of the cell
     */
    pub fn collapse_val(&self) -> usize {
        match &self {
            Cell::Collapsed(ind) => ind.clone(),
            _ => 0,
        }
    }

    fn uncollapse(&self) -> Vec<usize> {
        match self {
            Self::Uncollapsed(u) => u.clone(),
            Self::Collapsed(c) => vec![c.clone()],
        }
    }

    /* this is going to have a long ass documentation lmao
     *
     * it takes as it's inputs the bordering cells,
     * if there is no bordering cell it should take a default cell
     * all the possible cells it can be
     * and returns its entropy
     */

    fn collapse<TileT: TileConnection + Copy>(
        &mut self,
        x: usize,
        y: usize,
        north: &Cell,
        south: &Cell,
        east: &Cell,
        west: &Cell,
        possible: &Possible<TileT>,
    ) -> usize {
        if self.collapsed() {
            return 1;
        }
        let current = self.uncollapse();
        let mut new_self = Uncollapsed::new();
        for pos in current {
            let mut found: bool;
            compare_tiles!(possible, pos, found, north, Direction::North, x, y);
            compare_tiles!(possible, pos, found, south, Direction::South, x, y);
            compare_tiles!(possible, pos, found, east, Direction::East, x, y);
            compare_tiles!(possible, pos, found, west, Direction::West, x, y);

            new_self.push(pos);
        }

        if new_self.len() == 1 {
            self.clone_from(&Cell::Collapsed(new_self[0]));
        } else {
            self.clone_from(&Cell::Uncollapsed(new_self));
        }

        return self.entropy();
    }

    pub fn force_collapse(&mut self, selected: usize) {
        let coll = match self {
            Cell::Collapsed(_) => return,
            Cell::Uncollapsed(u) if u.is_empty() => {
                panic!("cannot force-collapse a cell with no possible units")
            }
            Cell::Uncollapsed(_) => selected,
        };

        self.clone_from(&Cell::Collapsed(coll));
    }
}

pub struct FiniteMap<TileT: TileConnection + Copy> {
    pub width: usize,
    pub height: usize,
    pub default: Uncollapsed,
    pub defcell: Cell,
    pub possible: Possible<TileT>,
    pub map: Vec<Vec<Cell>>,
    pub seed: u64,
    propagation_queue: VecDeque<[usize; 2]>,
    step: u64,
}

pub struct LeastContainer {
    pub vec: Vec<[usize; 2]>,
    pub grade: usize,
}

impl<TileT: TileConnection + Copy> FiniteMap<TileT> {
    pub fn new(
        width: usize,
        height: usize,
        possible: Possible<TileT>,
        seed: u64,
    ) -> FiniteMap<TileT> {
        let mut map: Vec<Vec<Cell>> = Vec::<Vec<Cell>>::new();
        let mut default_vec: Vec<Cell> = Vec::<Cell>::new();
        let mut default: Vec<usize> = Vec::new();
        for i in 0..possible.len() {
            default.push(i);
        }
        default_vec.resize(height, Cell::Uncollapsed(default.clone()));
        map.resize(width, default_vec);

        FiniteMap {
            width,
            height,
            default: default.clone(),
            defcell: Cell::Uncollapsed(default),
            possible,
            map,
            seed,
            propagation_queue: VecDeque::new(),
            step: 0,
        }
    }

    pub fn collapse_cell(&mut self, i: usize, j: usize) -> bool {
        let imax = self.width - 1;
        let jmax = self.height - 1;

        let east = match i {
            p if p >= imax => &self.defcell,
            _ => &self.map[i + 1][j],
        };
        let west = match i {
            0 => &self.defcell,
            _ => &self.map[i - 1][j],
        };
        let north = match j {
            p if p >= jmax => &self.defcell,
            _ => &self.map[i][j + 1],
        };
        let south = match j {
            0 => &self.defcell,
            _ => &self.map[i][j - 1],
        };

        //checks if all the neightbors have some degree of determination
        let entropy = self.possible.len();
        if north.entropy() == entropy
            && south.entropy() == entropy
            && east.entropy() == entropy
            && west.entropy() == entropy
        {
            return false;
        } else {
            let old_cell = self.map[i][j].clone();
            let mut cell = self.map[i][j].clone();
            cell.collapse(i, j, north, south, east, west, &self.possible);
            self.map[i][j].clone_from(&cell);
            return self.map[i][j] != old_cell;
        }
    }

    pub fn print_self(&self) {
        for __j in 0..self.height {
            let j = (self.height - 1) - __j;
            print!("{}: ", j);
            for i in 0..self.width {
                match &self.map[i][j] {
                    Cell::Collapsed(c) => print!("|{}", c),
                    Cell::Uncollapsed(u) => {
                        print!("[{}", u.len());
                        //for i in u{
                        //    print!("{},", i);
                        //}
                        //print!("");
                    }
                }
            }
            println!();
        }
        println!();
    }

    fn enqueue_cell(&mut self, i: usize, j: usize) {
        if !self.propagation_queue.contains(&[i, j]) {
            self.propagation_queue.push_back([i, j]);
        }
    }

    fn enqueue_neighbors(&mut self, i: usize, j: usize) {
        if i > 0 {
            self.enqueue_cell(i - 1, j);
        }
        if i + 1 < self.width {
            self.enqueue_cell(i + 1, j);
        }
        if j > 0 {
            self.enqueue_cell(i, j - 1);
        }
        if j + 1 < self.height {
            self.enqueue_cell(i, j + 1);
        }
    }

    pub fn propagation_substep(&mut self) -> bool {
        let Some([i, j]) = self.propagation_queue.pop_front() else {
            return false;
        };

        if self.collapse_cell(i, j) {
            self.enqueue_neighbors(i, j);
        }

        true
    }

    pub fn force_collapse(&mut self, i: usize, j: usize) {
        let seed = self.seed + (i ^ j) as u64;
        let Some(selected) = self.weighted_collapse_choice(i, j, &seed) else {
            return;
        };
        self.map[i][j].force_collapse(selected);
        self.enqueue_neighbors(i, j);
    }

    fn weighted_collapse_choice(&self, i: usize, j: usize, seed: &u64) -> Option<usize> {
        let candidates = match &self.map[i][j] {
            Cell::Collapsed(_) => return None,
            Cell::Uncollapsed(u) if u.is_empty() => return None,
            Cell::Uncollapsed(u) => u,
        };

        let mut total_weight = 0u64;
        let mut weighted = Vec::<(usize, u64)>::new();

        for candidate in candidates {
            let weight = self.candidate_weight(i, j, *candidate);
            if weight == 0 {
                continue;
            }

            total_weight = total_weight.saturating_add(weight);
            weighted.push((*candidate, weight));
        }

        if weighted.is_empty() || total_weight == 0 {
            return None;
        }

        let mut roll = u64::rands_range(&0, &total_weight, seed);
        for (candidate, weight) in weighted {
            if roll < weight {
                return Some(candidate);
            }
            roll -= weight;
        }

        None
    }

    fn candidate_weight(&self, i: usize, j: usize, candidate: usize) -> u64 {
        let imax = self.width - 1;
        let jmax = self.height - 1;

        let east = match i {
            p if p >= imax => &self.defcell,
            _ => &self.map[i + 1][j],
        };
        let west = match i {
            0 => &self.defcell,
            _ => &self.map[i - 1][j],
        };
        let north = match j {
            p if p >= jmax => &self.defcell,
            _ => &self.map[i][j + 1],
        };
        let south = match j {
            0 => &self.defcell,
            _ => &self.map[i][j - 1],
        };

        let mut weight = 1u64;
        for (direction, neighbor) in [
            (Direction::North, north),
            (Direction::South, south),
            (Direction::East, east),
            (Direction::West, west),
        ] {
            let direction_weight = self.best_neighbor_weight(i, j, candidate, direction, neighbor);
            if direction_weight == 0 {
                return 0;
            }
            weight = weight.saturating_mul(direction_weight);
        }

        weight
    }

    fn best_neighbor_weight(
        &self,
        i: usize,
        j: usize,
        candidate: usize,
        direction: Direction,
        neighbor: &Cell,
    ) -> u64 {
        let mut best = 0u64;

        for neighbor_candidate in neighbor.uncollapse() {
            let connection_weight = self.possible[candidate].can_connect(
                i,
                j,
                direction,
                &self.possible[neighbor_candidate],
            );
            best = best.max(connection_weight);
        }

        best
    }

    pub fn determine(&mut self) {
        while self.step() {}
    }

    pub fn substep(&mut self) -> bool {
        self.substep_inner(false)
    }

    pub fn substep_with_random_fallback(&mut self) -> bool {
        self.substep_inner(true)
    }

    pub fn step(&mut self) -> bool {
        self.step_inner(false)
    }

    pub fn step_with_random_fallback(&mut self) -> bool {
        self.step_inner(true)
    }

    fn substep_inner(&mut self, random_fallback: bool) -> bool {
        if self.propagation_substep() {
            return true;
        }

        self.force_next_cell(random_fallback)
    }

    fn step_inner(&mut self, random_fallback: bool) -> bool {
        let mut progressed = false;

        while self.propagation_substep() {
            progressed = true;
        }

        if progressed {
            return true;
        }

        if !self.force_next_cell(random_fallback) {
            return false;
        }

        while self.propagation_substep() {}

        true
    }

    fn force_next_cell(&mut self, random_fallback: bool) -> bool {
        let cp = if self.step == 0 {
            let ci = usize::rands_range(&0, &self.width, &self.seed);
            let cj = usize::rands_range(&0, &self.height, &(self.seed + ci as u64));
            [ci, cj]
        } else {
            let v = self.least();
            if v.vec.len() == 0 {
                if random_fallback {
                    match self.random_uncollapsed() {
                        Some(pos) => pos,
                        None => return false,
                    }
                } else {
                    return false;
                }
            } else {
                let choice_seed = self.seed + self.step + v.grade as u64;
                v.vec[usize::rands_range(&0, &v.vec.len(), &choice_seed)]
            }
        };

        self.force_collapse(cp[0], cp[1]);
        self.step += 1;
        true
    }

    pub fn random_uncollapsed(&self) -> Option<[usize; 2]> {
        let mut vec = Vec::<[usize; 2]>::new();

        for i in 0..self.width {
            for j in 0..self.height {
                if !self.map[i][j].collapsed() && self.map[i][j].entropy() > 0 {
                    vec.push([i, j]);
                }
            }
        }

        if vec.len() == 0 {
            None
        } else {
            let seed = self.seed + self.step + vec.len() as u64;
            Some(vec[usize::rands_range(&0, &vec.len(), &seed)])
        }
    }

    pub fn least(&self) -> LeastContainer {
        let mut vec = Vec::<[usize; 2]>::new();

        let mut min_grade = self.possible.len();
        for i in 0..self.width {
            for j in 0..self.height {
                let grade = self.map[i][j].entropy();
                if min_grade > grade && grade > 1 {
                    min_grade = grade;
                    vec.clear();
                }
                if min_grade == grade {
                    vec.push([i, j]);
                }
            }
        }

        if min_grade == self.possible.len() {
            vec.clear();
        }
        return LeastContainer {
            vec,
            grade: min_grade,
        };
    }
}
