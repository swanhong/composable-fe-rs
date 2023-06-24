use rug::Integer;
use rug::rand::RandState;
use crate::util::matrix::{Matrix, concatenate_diag_one, concatenate_col};

pub fn echelon_form(
    m: &mut Matrix,
    mod_val: &Integer,
) -> (Vec<usize>, Vec<usize>, i32) {
    let n_rows = m.rows();
    let n_cols = m.cols();

    let mut pivot_cols: Vec<usize> = Vec::new();
    let mut free_vars: Vec<usize> = Vec::new();
    let mut rank = -1;
    let mut pivot_row = 0;
    let mut pivot_col = 0;

    while pivot_row < n_rows && pivot_col < n_cols {
        let mut pivot = m.get(pivot_row, pivot_col);
        
        while pivot == 0 {
            // if pivot_col < n_cols - 1 {
            //     free_vars.push(pivot_col);
            //     pivot_col += 1;
            // } else {
            //     free_vars.push(pivot_col);
            //     pivot_row += 1;
            //     pivot_col = 0;
            // }
            pivot_row += 1;

            if pivot_row >= n_rows {
                return (pivot_cols, free_vars, -1);
            }

            pivot = m.get(pivot_row, pivot_col);
        }

        rank += 1;
        let pivot = m.get(pivot_row, pivot_col);
        let pivot_inv = match pivot.clone().invert(&mod_val) {
            Ok(x) => x,
            Err(_e) => Integer::from(-1)
        };

        if pivot_inv != Integer::from(-1) {
            for j in pivot_col..n_cols {
                let val = m.get(pivot_row, j) * &pivot_inv % mod_val;
                m.set(pivot_row, j, val);
            }
        } else {
            return (pivot_cols, free_vars, -1);
        }

        for i in 0..n_rows {
            let pivot = m.get(pivot_row, pivot_col);
            let mut ratio = Integer::from(0);
            let pivot_inv = match pivot.clone().invert(&mod_val) {
                Ok(x) => x,
                Err(_e) => Integer::from(-1)
            };
            if pivot_inv != Integer::from(-1) {
                let val1 = m.get(i, pivot_col);
                ratio = val1 * pivot_inv % mod_val;
            } else {
                return (pivot_cols, free_vars, -1);
            }

            for j in 0..n_cols {
                if i != pivot_row {
                    let val = m.get(pivot_row, j) * ratio.clone();
                    let mut entry = m.get(i, j);
                    entry = (entry - val) % mod_val;
                    m.set(i, j, entry);
                }
            }
        }

        pivot_cols.push(pivot_col);
        pivot_row += 1;
        pivot_col += 1;
    }
    if pivot_col < n_cols {
        for j in pivot_col..n_cols {
            free_vars.push(j);
        }
    }  
    m.mod_inplace(mod_val);

    (pivot_cols, free_vars, rank)
}

pub fn matrix_inverse(
    m: &mut Matrix,
    mod_val: &Integer,
) -> Result<Matrix, i32> {
    assert_eq!(m.rows, m.cols);
    let n = m.rows;
    let mut m_inv = Matrix::get_identity(n);
    let mut m_aug = concatenate_col(m, &m_inv);
    let (pivot_cols, free_vars, r) = echelon_form(&mut m_aug, mod_val);
    
    if r != -1 {
        for i in 0..n {
            for j in 0..n {
                m_inv.set(i, j, m_aug.get(i, j + n));
            }
        }
    }
    
    match r {
        -1 => Err(-1),
        _ => Ok(m_inv)
    }
}

pub fn sample_h(dim: usize, k: usize, modulo: &Integer, rng: &mut RandState<'_>) -> (Matrix, Matrix) {
    // sample two matrices h_left_1 and h_right_1
    
    // h_left is dim * (dim + k ) ternary matrix
    // h_right is (dim + k) * dim matrix satisfying h_left * h_right = identity_dim

    // h_right = h^t * (h * h^t)^-1 
    // h_left_1 = (h_left, 1)
    // h_right_1 = (h_right, 1)

    let mut h_0: Matrix = Matrix::new(dim, dim + k);
    let mut h_t: Matrix = Matrix::new(dim + k, dim);
    let mut h_0_inv: Matrix = Matrix::new(dim, dim);
    while true {
        h_0 = Matrix::random(dim, dim + k, &Integer::from(3), rng);
        h_0.add_int_inplace(&Integer::from(-1));
        h_0.mod_inplace(modulo);

        h_t = h_0.transpose();
        let mut tmp = h_0.clone() * h_t.clone();
        tmp.mod_inplace(modulo);
        match matrix_inverse(&mut tmp, modulo) {
            Ok(m_inv) => {
                h_0_inv = m_inv.clone();
                break;
            }
            Err(rank) => {
                continue;
            }
        }

    }
    
    let h_pr_0 = h_t * h_0_inv;

    let h = concatenate_diag_one(&h_0);
    let h_pr = concatenate_diag_one(&h_pr_0);

    (h, h_pr)
}

pub fn sample_gamma(
    dim: usize,
    k: usize,
    modulo: &Integer,
    rng: &mut RandState<'_>,
) -> (Matrix, Matrix) {
    // sample two matrices gamma_left_1 and gamma_right_1
    
    // gamma_left is dim * (dim + k ) binary matrix
    // gamma_right is (dim + k) * dim matrix satisfying gamma_left * gamma_right = identity_dim

    // gamma_right = gamma^t * (gamma * gamma^t)^-1 
    // gamma_left_1 = (gamma_left, 1)
    // gamma_right_1 = (gamma_right, 1)
    let mut gamma_0 = Matrix::new(1, 1);
    let mut gamma_0_t = Matrix::new(1, 1);
    let mut gamma_0_inv = Matrix::new(1, 1);

    while true {
        gamma_0 = Matrix::random( dim, dim + k, &Integer::from(2), rng);
        gamma_0_t = gamma_0.transpose();
        let mut tmp = gamma_0.clone() * gamma_0_t.clone();
        tmp.mod_inplace(modulo);
        match matrix_inverse(&mut tmp, modulo) {
            Ok(m_inv) => {
                gamma_0_inv = m_inv.clone();
                break;
            }
            Err(rank) => {
                continue;
            }
        }
    }
    

    let gamma_pr_0 = gamma_0_t * gamma_0_inv;

    let gamma_1 = concatenate_diag_one(&gamma_0);
    let gamma_pr_1 = concatenate_diag_one(&gamma_pr_0);

    (gamma_1, gamma_pr_1)
}