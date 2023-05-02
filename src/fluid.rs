use std::{
    mem,
    ops::{Index, IndexMut},
    time::Duration,
};

use glam::Vec2;
use ndarray::Array2;

#[derive(Debug, Clone, Copy, Default)]
pub struct Cell {
    pub density: f32,
    pub velocity: Vec2,
}

#[derive(Debug, Clone, Default)]
pub struct Fluid {
    pub diffusion: f32,
    pub viscosity: f32,
    pub size: usize,
    pub cells: Array2<Cell>,
    pub prev_cells: Array2<Cell>,
}

impl Fluid {
    pub fn new(diffusion: f32, viscosity: f32, size: usize) -> Self {
        Self {
            diffusion,
            viscosity,
            size,
            cells: Array2::default((size, size)),
            prev_cells: Array2::default((size, size)),
        }
    }

    pub fn step(&mut self, delta: Duration) {
        let delta = delta.as_secs_f32();
        self.diffuse(delta);
        self.project();
        self.advect(delta);
        self.project();
    }

    fn diffuse(&mut self, delta: f32) {
        mem::swap(&mut self.cells, &mut self.prev_cells);

        let a_density = delta * self.diffusion * (self.size * self.size) as f32;
        let a_velocity = delta * self.viscosity * (self.size * self.size) as f32;

        for _ in 0..20 {
            for x in 0..self.size {
                let i = x as isize;
                for y in 0..self.size {
                    let j = y as isize;

                    self.cells[[x, y]].density = (self.prev_cells[[x, y]].density
                        + a_density
                            * (get_cell(&self.cells, i - 1, j).density
                                + get_cell(&self.cells, i + 1, j).density
                                + get_cell(&self.cells, i, j - 1).density
                                + get_cell(&self.cells, i, j + 1).density))
                        / (1.0 + 4.0 * a_density);

                    self.cells[[x, y]].velocity = (self.prev_cells[[x, y]].velocity
                        + a_density
                            * (get_cell(&self.cells, i - 1, j).velocity
                                + get_cell(&self.cells, i + 1, j).velocity
                                + get_cell(&self.cells, i, j - 1).velocity
                                + get_cell(&self.cells, i, j + 1).velocity))
                        / (1.0 + 4.0 * a_velocity);
                }
            }
        }
    }

    fn project(&mut self) {
        let h = 1.0 / self.size as f32;
        for x in 0..self.size {
            let i = x as isize;
            for y in 0..self.size {
                let j = y as isize;

                self.prev_cells[[x, y]].velocity.y = -0.5
                    * h
                    * (get_cell(&self.cells, i + 1, j).velocity.x
                        - get_cell(&self.cells, i - 1, j).velocity.x
                        + get_cell(&self.cells, i, j + 1).velocity.y
                        - get_cell(&self.cells, i, j - 1).velocity.y);

                self.prev_cells[[x, y]].velocity.x = 0.0;
            }
        }

        for _ in 0..20 {
            for x in 0..self.size {
                let i = x as isize;
                for y in 0..self.size {
                    let j = y as isize;

                    self.prev_cells[[x, y]].velocity.x = 0.25
                        * (self.prev_cells[[x, y]].velocity.y
                            + get_cell(&self.prev_cells, i - 1, j).velocity.x
                            + get_cell(&self.prev_cells, i + 1, j).velocity.x
                            + get_cell(&self.prev_cells, i, j - 1).velocity.x
                            + get_cell(&self.prev_cells, i, j + 1).velocity.x);
                }
            }
        }

        for x in 0..self.size {
            let i = x as isize;
            for y in 0..self.size {
                let j = y as isize;

                self.cells[[x, y]].velocity -=
                    0.5 * Vec2::new(
                        get_cell(&self.prev_cells, i + 1, j).velocity.x
                            - get_cell(&self.prev_cells, i - 1, j).velocity.x,
                        get_cell(&self.prev_cells, i, j + 1).velocity.x
                            - get_cell(&self.prev_cells, i, j - 1).velocity.x,
                    ) / h;
            }
        }
    }

    fn advect(&mut self, delta: f32) {
        mem::swap(&mut self.cells, &mut self.prev_cells);

        let delta_size = delta * self.size as f32;

        for ((x, y), cell) in self.cells.indexed_iter_mut() {
            let source_pos =
                Vec2::new(x as f32, y as f32) - delta_size * self.prev_cells[[x, y]].velocity;

            let left_idx = source_pos.x.floor() as isize;
            let right_idx = left_idx + 1;
            let top_idx = source_pos.y.floor() as isize;
            let bottom_idx = top_idx + 1;

            let top_left = get_cell(&self.prev_cells, left_idx, top_idx);
            let top_right = get_cell(&self.prev_cells, right_idx, top_idx);
            let bottom_left = get_cell(&self.prev_cells, left_idx, bottom_idx);
            let bottom_right = get_cell(&self.prev_cells, right_idx, bottom_idx);

            let right_coefficient = source_pos.x - left_idx as f32;
            let left_coefficient = 1.0 - right_coefficient;
            let bottom_coefficient = source_pos.y - top_idx as f32;
            let top_coefficient = 1.0 - bottom_coefficient;

            cell.density = left_coefficient
                * (top_coefficient * top_left.density + bottom_coefficient * bottom_left.density)
                + right_coefficient
                    * (top_coefficient * top_right.density
                        + bottom_coefficient * bottom_right.density);

            cell.velocity = left_coefficient
                * (top_coefficient * top_left.velocity + bottom_coefficient * bottom_left.velocity)
                + right_coefficient
                    * (top_coefficient * top_right.velocity
                        + bottom_coefficient * bottom_right.velocity);
        }
    }
}

fn get_cell(cells: &Array2<Cell>, i: isize, j: isize) -> &Cell {
    let x = wrap_index(i, cells.dim().0);
    let y = wrap_index(j, cells.dim().1);
    &cells[[x, y]]
}

fn get_cell_mut(cells: &mut Array2<Cell>, i: isize, j: isize) -> &mut Cell {
    let x = wrap_index(i, cells.dim().0);
    let y = wrap_index(j, cells.dim().1);
    &mut cells[[x, y]]
}

fn wrap_index(mut index: isize, size: usize) -> usize {
    let size = size as isize;
    index %= size;
    if index < 0 {
        index += size;
    }
    index as usize
}

impl Index<(isize, isize)> for Fluid {
    type Output = Cell;
    fn index(&self, (x, y): (isize, isize)) -> &Self::Output {
        get_cell(&self.cells, x, y)
    }
}

impl IndexMut<(isize, isize)> for Fluid {
    fn index_mut(&mut self, (x, y): (isize, isize)) -> &mut Self::Output {
        get_cell_mut(&mut self.cells, x, y)
    }
}
