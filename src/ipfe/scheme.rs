#![allow(dead_code)]

extern crate rug;
use rug::Integer;
use rug::rand::RandState;

use crate::util::group::Group;
use crate::util::group::discrete_logarithm;
use crate::util::matrix::*;
use crate::util::vector::*;
use super::keys::IpfeSk;

pub fn ipfe_setup(group: &Group, dim: usize, q: usize, rng: &mut RandState<'_>) -> IpfeSk {
    IpfeSk::new(dim, q, group, rng)
}


pub fn ipfe_keygen(sk: &IpfeSk, y: &Vec<Integer>, grp: &Group) -> Vec<Integer> {
    let mut val;
    let mod_val = grp.delta.clone();
    
    // U_t_y = (-1 * sk->U_t * y) || y
    let mut u_t_y_left = Vec::with_capacity(sk.u_t.rows);
    let mut u_t_y = Vec::with_capacity(u_t_y_left.len() + y.len());
    for i in 0..sk.u_t.rows {
        val = Integer::from(0);
        for j in 0..sk.u_t.cols {
            val -= sk.u_t.get(i, j).clone() * y[j].clone();
            val = int_mod(&val, &mod_val);
        }
        u_t_y_left.push(val);
    }

    // u_t_y <- u_t_y || y
    u_t_y.extend_from_slice(&u_t_y_left);
    u_t_y.extend_from_slice(y);

    // sk_f_mat = u_t_y * D
    // sk_f_mat = (sk->D_inv_left + sk->D_inv_right * sk->U) * u_t_y
    let mut left = Matrix::new(1, u_t_y.len());
    for i in 0..u_t_y.len() {
        let val = &u_t_y[i];
        left.set(0, i, val.clone());
    }

    let mut sk_f_mat = left * &sk.d;
    sk_f_mat.mod_inplace(&mod_val);
    let sk_f = vec_exp_with_base(&grp.g, &sk_f_mat.get_row(0), &grp.n_sq);
    
    // size of sk_f = dim + 2 + q
    sk_f
}

pub fn ipfe_enc(
    sk: &IpfeSk,
    x: &Vec<Integer>,
    grp: &Group,
    mult_mu: bool,
    rng: &mut RandState<'_>,
) -> Vec<Integer> {
    let mod_val = grp.delta.clone();
    let r_pr = mod_val.clone().random_below(rng);
    // r = 2 * N * r'
    let r: Integer = &grp.n.clone() * Integer::from(2) * r_pr.clone();

    // d_perp_rand = D_perp * rand
    let rand: Vec<Integer> = gen_random_vector(sk.d_perp.cols, &mod_val, rng);
    
    let mut d_perp_rand = sk.d_perp.mul_vec(&rand);
    vec_mod(&mut d_perp_rand, &mod_val);

    let mut x_mu = if mult_mu {
        vec_mul_scalar(x, &grp.mu)
    } else {
        x.to_vec()
    };
    vec_mod(&mut x_mu, &mod_val);

    let right_upper = vec_mul_scalar(&sk.a, &r);
    let mut right_lower = sk.u.mul_vec(&sk.a);
    right_lower = vec_mul_scalar(&right_lower, &r);
    right_lower = vec_add(&right_lower, &x_mu);

    let mut right_joined = Vec::new();
    right_joined.extend(right_upper);
    right_joined.extend(right_lower);

    let mut ctxt = sk.d_inv.mul_vec(&right_joined);
    ctxt = vec_add(&ctxt, &d_perp_rand);
    vec_mod(&mut ctxt, &mod_val);

    ctxt
}

pub fn ipfe_dec(sk_f: &Vec<Integer>, ctxt: &Vec<Integer>, grp: &Group, solve_dl: bool) -> Integer {
    let mut out = vec_inner_pow(&sk_f, &ctxt, &grp);
    if solve_dl {
        out = discrete_logarithm(out.clone(), &grp);
    }
    int_mod(&mut out, &grp.delta);
    out
}


