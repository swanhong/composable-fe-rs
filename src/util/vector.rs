use rug::Integer;
use rug::rand::RandState;
use crate::util::group::Group;
use rayon::prelude::*;

pub fn gen_random_vector(dim: usize, bound: &Integer, rand: &mut RandState<'_>) -> Vec<Integer> {
    // let mut rand = RandState::new(); // Create a single RandState object
    // let d = SystemTime::now()
    // .duration_since(SystemTime::UNIX_EPOCH)
    // .expect("Duration since UNIX_EPOCH failed");
    // rand.seed(&Integer::from(d.as_secs()));
    
    let mut vec = vec![Integer::from(0); dim];
    for i in 0..dim {
        vec[i] = bound.clone().random_below(rand);
    }
    vec
}

pub fn vec_mul_scalar(vec: &Vec<Integer>, scalar: &Integer) -> Vec<Integer> {
    let mut res = vec![Integer::from(0); vec.len()];
    for i in 0..vec.len() {
        res[i] = vec[i].clone() * scalar.clone();
    }
    res
}

pub fn vec_add(vec1: &Vec<Integer>, vec2: &Vec<Integer>) -> Vec<Integer> {
    assert!(vec1.len() == vec2.len());
    let mut res = vec![Integer::from(0); vec1.len()];
    for i in 0..vec1.len() {
        res[i] = vec1[i].clone() + vec2[i].clone();
    }
    res
}

pub fn vec_mod(vec: &mut Vec<Integer>, modulus: &Integer) {
    for i in 0..vec.len() {
        vec[i] = vec[i].clone().div_rem_euc(modulus.clone()).1;
    }
}

pub fn vec_inner_pow(v_base: &Vec<Integer>, v_exp: &Vec<Integer>, grp: &Group) -> Integer {
    assert_eq!(v_base.len(), v_exp.len());
    
    let modulo = &grp.n_sq;
    let mut out = Integer::from(1);

    for (base, exp) in v_base.iter().zip(v_exp.iter()) {
        let val: Integer = base.clone().pow_mod(exp, modulo).unwrap();
        out = out * val % modulo;
    }

    out

    // for i in 0..v_base.len() {
    //     let val: Integer = v_base[i].clone().pow_mod(&v_exp[i], &modulo).unwrap();
    //     if i == 0 {
    //         out = val.clone();
    //     } else {
    //         out = out * val % &modulo;
    //     }
    // }
    // out
}

pub fn vec_exp_with_base(base: &Integer, v_exp: &Vec<Integer>, modulo: &Integer) -> Vec<Integer> {
    // let mut out = vec![Integer::from(0); v_exp.len()];
    // for i in 0..v_exp.len() {
    //     out[i] = base.clone().pow_mod(&v_exp[i], &modulo).unwrap();
    // }
    // out
    v_exp.par_iter()
        .map(|exp| base.clone().pow_mod(exp, modulo).unwrap())
        .collect()
}


pub fn tensor_product_vecs(vec1: &Vec<Integer>, vec2: &Vec<Integer>, modulo: &Integer) -> Vec<Integer> {
    let mut res = vec![Integer::from(0); vec1.len() * vec2.len()];
    for i in 0..vec1.len() {
        for j in 0..vec2.len() {
            res[i * vec2.len() + j] = vec1[i].clone() * vec2[j].clone() % modulo;
        }
    }
    res
}

pub fn eval_quadratic(
    x: &Vec<Integer>, 
    y: &Vec<Integer>, 
    f: &Vec<Integer>, 
) -> Integer {
    assert_eq!(f.len(), x.len() * y.len());
    let mut out = Integer::from(0);
    for i in 0..x.len() {
        for j in 0..y.len() {
            out += f[i * y.len() + j].clone() * x[i].clone() * y[j].clone();
        }
    }
    out
}