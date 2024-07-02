extern crate rug;
use rug::Integer;
use rug::rand::RandState;

use crate::qfe;
use crate::util::group::Group;
use crate::util::matrix::{Matrix, remove_diag_one};
use crate::util::vector;
use crate::util::vector::{gen_random_vector, vec_add, vec_mod, vec_mul_scalar};
use crate::util::decomp::Decomp;
use crate::dcr::scheme::{dcr_setup, dcr_enc, dcr_keygen, dcr_dec};
use crate::qfe::keys::QfeSk;
use crate::qfe::scheme::{divide_vector_for_functional_key, get_funcional_key_len, qe_dec, qe_enc_matrix_same_xy, qe_keygen, qe_setup};
use crate::ipfe::scheme::{ipe_enc, ipe_keygen};
use std::time::SystemTime;

pub fn protocol_setup(
    dim_vec: Vec<usize>,
    f_num: usize,
    k: usize,
    sk_bound: &Integer,
    grp: &Group,
    rng: &mut RandState<'_>,
) -> ((Vec<Integer>, Vec<Integer>), QfeSk, Vec<QfeSk>, QfeSk) {
    let dim = dim_vec[0];
    let (dcr_sk, dcr_pk) = dcr_setup(2 * dim + 1, sk_bound, grp, rng);
    let qe_sk_init = qe_setup(grp, dim + k, 2 * (dim + k) + 1, rng);
    let mut qe_sk_fcn = Vec::with_capacity(f_num);
    for i in 1..dim_vec.len()-1 {
        let dim = dim_vec[i];
        qe_sk_fcn.push(qe_setup(grp, dim + k, 2 * (dim + k) + 1, rng));
    }
    let dim = dim_vec[dim_vec.len()-1];
    let qe_sk_end = qe_setup(grp, dim + k, 2 * (dim + k) + 1, rng);

    ((dcr_sk, dcr_pk), qe_sk_init, qe_sk_fcn, qe_sk_end)
}

pub fn protocol_enc_init(
    dcr_pk: &Vec<Integer>,
    gamma_right: &Matrix,
    x: &Vec<Integer>,
    grp: &Group,
    rng: &mut RandState<'_>,
) -> Vec<Integer> {
    let mut x1 = x.clone();
    x1.push(Integer::from(1));

    let mut gamma_right_x = gamma_right.mul_vec(&x1);
    vec_mod(&mut gamma_right_x, &grp.delta);
    dcr_enc(dcr_pk, &gamma_right_x, &grp, rng)
}

pub fn protocol_keygen_switch(
    qe_sk: &QfeSk,
    dcr_sk: &Vec<Integer>,
    h_right: &Matrix,
    gamma_left: &Matrix,
    dim: usize,
    k: usize,
    decomp: &Decomp,
    grp: &Group,
    rng: &mut RandState<'_>,
) -> (
    (Matrix, Matrix, Matrix),
    (Vec<Integer>, Vec<Integer>, Vec<Integer>),
) {
    let modulo = grp.delta.clone();
    let (qe_enc_mat_x, qe_enc_mat_y, qe_enc_mat_h)
        = qe_enc_matrix_same_xy(&qe_sk, dim + k, &grp, rng);
    
    fn gen_switch_key_for_enc_and_dcr(
        dcr_sk: &Vec<Integer>,
        qe_enc_mat: &Matrix,
        h_right: &Matrix,
        gamma_left: &Matrix,
        decomp: &Decomp,
        modulo: &Integer,
    ) -> (Matrix, Vec<Integer>) {
        let mut xh = qe_enc_mat * h_right;
        xh.mod_inplace(modulo);
        let xh_decomp = decomp.matrix_col(&xh);
        let mut switch_key_x = xh_decomp * gamma_left;

        switch_key_x.mod_inplace(modulo);

        let mut switch_key_dcr_x = vec![Integer::from(0); switch_key_x.rows];
        for i in 0..switch_key_x.rows {
            let row = switch_key_x.get_row(i);
            switch_key_dcr_x[i] = dcr_keygen(&dcr_sk, &row);
        }

        (switch_key_x, switch_key_dcr_x)                
    }
    
    let (switch_key_x, switch_key_dcr_x) = gen_switch_key_for_enc_and_dcr(
        &dcr_sk,
        &qe_enc_mat_x,
        &h_right,
        &gamma_left,
        &decomp,
        &modulo,
    );

    let (switch_key_y, switch_key_dcr_y) = gen_switch_key_for_enc_and_dcr(
        &dcr_sk,
        &qe_enc_mat_y,
        &h_right,
        &gamma_left,
        &decomp,
        &modulo,
    );

    let (switch_key_h, switch_key_dcr_h) = gen_switch_key_for_enc_and_dcr(
        &dcr_sk,
        &qe_enc_mat_h,
        &h_right,
        &gamma_left,
        &decomp,
        &modulo,
    );

    (
        (switch_key_x, switch_key_y, switch_key_h),
        (switch_key_dcr_x, switch_key_dcr_y, switch_key_dcr_h),
    )
}

pub fn protocol_keyswitch(
    ct_in: &Vec<Integer>,
    (switch_key_x, switch_key_y, switch_key_h): (&Matrix, &Matrix, &Matrix),
    (switch_key_dcr_x, switch_key_dcr_y, switch_key_dcr_h): (&Vec<Integer>, &Vec<Integer>, &Vec<Integer>),
    decomp: &Decomp,
    grp: &Group,
) -> (Vec<Integer>, Vec<Integer>, Vec<Integer>) {
    
    pub fn dcr_dec_multi(
        ct_in: &Vec<Integer>,
        switch_key: &Matrix,
        switch_key_dcr: &Vec<Integer>,
        decomp: &Decomp,
        grp: &Group,
    ) -> Vec<Integer> {
        assert_eq!(switch_key.rows, switch_key_dcr.len(), "error in dcr_dec_multi inputs");
        assert_eq!(ct_in.len(), switch_key.cols + 1, "error in dcr_dec_multi inputs");
        let mut ct_out = vec![Integer::from(0); switch_key.rows];
        for i in 0..switch_key.rows {
            let row = switch_key.get_row(i);
            ct_out[i] = dcr_dec(&ct_in, &row, &switch_key_dcr[i], grp);
        }
        vec_mod(&mut ct_out, &grp.n);
        decomp.vector_inv(&ct_out)
    }

    println!("do dcr_dec_multi for {} x {} times", switch_key_x.rows, switch_key_x.cols);
    println!("do dcr_dec_multi for {} y {} times", switch_key_y.rows, switch_key_y.cols);
    println!("do dcr_dec_multi for {} h {} times", switch_key_h.rows, switch_key_h.cols);
    let ct_out_x = dcr_dec_multi(&ct_in, &switch_key_x, &switch_key_dcr_x, &decomp, &grp);
    let ct_out_y = dcr_dec_multi(&ct_in, &switch_key_y, &switch_key_dcr_y, &decomp, &grp);
    let ct_out_h = dcr_dec_multi(&ct_in, &switch_key_h, &switch_key_dcr_h, &decomp, &grp);
    (ct_out_x, ct_out_y, ct_out_h)
}

pub fn protocol_keygen_i(
    qe_sk_enc: &QfeSk,
    qe_sk_keygen: &QfeSk,
    h_right: &Matrix,
    hm_left: &Matrix,
    dim: usize,
    k: usize,
    f: &Matrix,
    decomp: &Decomp,
    grp: &Group,
    rng: &mut RandState<'_>,
) -> (
    (Vec<Integer>, Vec<Integer>, Vec<Integer>), 
    (Matrix, Matrix, Matrix), 
    (Matrix, Matrix, Matrix),
) {
    let (qe_enc_mat_x, qe_enc_mat_y, qe_enc_mat_h) = qe_enc_matrix_same_xy(&qe_sk_enc, dim + k, &grp, rng);
    
    // divide enc_mats into M * b
    pub fn divide_mat_into_m_b(
        mat: &Matrix,
    ) -> (Matrix, Vec<Integer>) {
        let mut mat_left = Matrix::new(mat.rows, mat.cols - 1);
        let mut vec_right = vec![Integer::from(0); mat.rows];

        for i in 0..mat.rows {
            for j in 0..mat.cols - 1 {
                let val = mat.get(i, j);
                mat_left.set(i, j, val);
            }
            let val = mat.get(i, mat.cols - 1);
            vec_right[i] = val;
        }
        (mat_left, vec_right)
    }
    let (qe_enc_mat_x, qe_b_x) = divide_mat_into_m_b(&qe_enc_mat_x);
    let (qe_enc_mat_y, qe_b_y) = divide_mat_into_m_b(&qe_enc_mat_y);
    let (qe_enc_mat_h, qe_b_h) = divide_mat_into_m_b(&qe_enc_mat_h);

    let qe_b_x = decomp.vector(&qe_b_x);
    let qe_b_y = decomp.vector(&qe_b_y);
    let qe_b_h = decomp.vector(&qe_b_h);

    let h_right_origin = remove_diag_one(&h_right);
    let hm_left_origin = remove_diag_one(&hm_left);
    let hmhm = Matrix::tensor_product(&hm_left_origin, &hm_left_origin, &grp.delta);

    fn mat_mul_4(
        a: &Matrix,
        b: &Matrix,
        c: &Matrix,
        d: &Matrix,
        decomp: &Decomp,
        grp: &Group,
    ) -> Matrix {
        // A = Enc, B = H, C = F, D = (H' tensor H')
        // output decomp(A * B) * (C * D)
        let modulo = grp.delta.clone();
        let mut ab = a * b;
        ab.mod_inplace(&modulo);
        ab = decomp.matrix_col(&ab);
        let mut cd = c * d;
        cd.mod_inplace(&modulo);
        let mut out = ab * cd;
        out.mod_inplace(&modulo);
        out
    }

    let total_mat_x = mat_mul_4(
        &qe_enc_mat_x, &h_right_origin, &f, &hmhm, &decomp, &grp);
    let total_mat_y = mat_mul_4(
        &qe_enc_mat_y, &h_right_origin, &f, &hmhm, &decomp, &grp);
    let total_mat_h = mat_mul_4(
        &qe_enc_mat_h, &h_right_origin, &f, &hmhm, &decomp, &grp);
    
    fn gen_f_and_red(
        qe_sk: &QfeSk,
        total_mat: Matrix,
        grp: &Group,
    ) -> (Matrix, Matrix) {
        let mut sk_f_mat = Matrix::new(1, 1);
        let mut sk_red_mat = Matrix::new(1, 1);
        println!("do qe_keygen of dim {} for {} times", total_mat.cols, total_mat.rows);
        let start = SystemTime::now();
        for i in 0..total_mat.rows {
            let row = total_mat.get_row(i);
            let fk = qe_keygen(&qe_sk, &row, grp);
            let (sk_f, sk_red) = divide_vector_for_functional_key(&fk, qe_sk.dim, qe_sk.q);
            if i == 0 {
                sk_f_mat = Matrix::new(total_mat.rows, sk_f.len());
                sk_red_mat = Matrix::new(total_mat.rows, sk_red.len());
            }
            sk_f_mat.set_row(i, &sk_f);
            sk_red_mat.set_row(i, &sk_red);
        }
        let end = SystemTime::now();
        let elapsed = end.duration_since(start).unwrap();
        println!("qe_keygen for {} times, time: {:?}", total_mat.rows, elapsed);
        (sk_f_mat, sk_red_mat)
    }

    let (sk_f_mat_x, sk_red_mat_x) = gen_f_and_red(&qe_sk_keygen, total_mat_x, &grp);
    let (sk_f_mat_y, sk_red_mat_y) = gen_f_and_red(&qe_sk_keygen, total_mat_y, &grp);
    let (sk_f_mat_h, sk_red_mat_h) = gen_f_and_red(&qe_sk_keygen, total_mat_h, &grp);
    (
        (qe_b_x, qe_b_y, qe_b_h), 
        (sk_f_mat_x, sk_f_mat_y, sk_f_mat_h), 
        (sk_red_mat_x, sk_red_mat_y, sk_red_mat_h),
    )
}

pub fn protocol_dec_i(
    ctxt_triple: (&Vec<Integer>, &Vec<Integer>, &Vec<Integer>),
    (qe_b_x, qe_b_y, qe_b_h): (&Vec<Integer>, &Vec<Integer>, &Vec<Integer>),
    (sk_f_mat_x, sk_f_mat_y, sk_f_mat_h): (&Matrix, &Matrix, &Matrix),
    (sk_red_mat_x, sk_red_mat_y, sk_red_mat_h): (&Matrix, &Matrix, &Matrix),
    dim: usize,
    q: usize,
    decomp: &Decomp,
    grp: &Group,
) -> (Vec<Integer>, Vec<Integer>, Vec<Integer>) {
    fn compute_f_red_out(
        sk_f_mat: &Matrix,
        sk_red_mat: &Matrix,
        ctxt_triple: (&Vec<Integer>, &Vec<Integer>, &Vec<Integer>),
        qe_b: &Vec<Integer>,
        dim: usize,
        q: usize,
        grp: &Group,
    ) -> Vec<Integer> {
        let mut ct_out = vec![Integer::from(0); sk_f_mat.rows];
        // println!("run qe_dec of dim {} for {} times", sk_f_mat.cols, sk_f_mat.rows);
        for i in 0..sk_f_mat.rows {
            let sk_f = sk_f_mat.get_row(i);
            // let sk_f = decomp.vector_pow_exp(&sk_f);
            let sk_red = sk_red_mat.get_row(i);
            // fk = sk_f || sk_red (concatenation)
            let mut fk = Vec::with_capacity(sk_f.len() + sk_red.len());
            fk.extend_from_slice(&sk_f);
            fk.extend_from_slice(&sk_red);
            // println!("fk.len: {}", fk.len());
            // println!(" = sk_f.len: {} + sk_red.len: {}", sk_f.len(), sk_red.len());

            let enc_x = ctxt_triple.0.clone();
            let enc_y = ctxt_triple.1.clone();
            let enc_h = ctxt_triple.2.clone();
            // ctxt = (enc_x, enc_y, enc_h)
            let mut ctxt = Vec::with_capacity(enc_x.len() + enc_y.len() + enc_h.len());
            ctxt.extend_from_slice(&enc_x);
            ctxt.extend_from_slice(&enc_y);
            ctxt.extend_from_slice(&enc_h);
            ct_out[i] =qe_dec(
                &fk, &ctxt, dim + 1, 2 * (dim + 1) + 1, grp,
            );
        }
        ct_out = vec_add(&ct_out, &qe_b);
        vec_mod(&mut ct_out, &grp.n);
        ct_out
    }

    let ct_out_x = compute_f_red_out(
        &sk_f_mat_x, &sk_red_mat_x,
        ctxt_triple,
        qe_b_x,
        dim,
        q,
        grp,
    );
    let ct_out_y = compute_f_red_out(
        &sk_f_mat_y, &sk_red_mat_y,
        ctxt_triple,
        qe_b_y,
        dim,
        q,
        grp,
    );
    let ct_out_h = compute_f_red_out(
        &sk_f_mat_h, &sk_red_mat_h,
        ctxt_triple,
        qe_b_h,
        dim,
        q,
        grp,
    );
    (ct_out_x, ct_out_y, ct_out_h)
}

pub fn protocol_keygen_end(
    qe_sk: &QfeSk,
    hm_left: &Matrix,
    f: &Matrix,
    grp: &Group,
) -> (Matrix, Matrix) {
    let modulo = grp.delta.clone();

    let hm_origin = remove_diag_one(&hm_left);
    let hmhm = Matrix::tensor_product(&hm_origin, &hm_origin, &grp.delta);
    
    let mut fhmhm = f * &hmhm;
    fhmhm.mod_inplace(&modulo);

    let mut sk_f_mat = Matrix::new(1, 1);
    let mut sk_red_mat = Matrix::new(1, 1);
    println!("do qe_keygen of dim {} for {} times", fhmhm.cols, fhmhm.rows);
    for i in 0..fhmhm.rows {
        let row = fhmhm.get_row(i);
        let fk = qe_keygen(&qe_sk, &row, grp);
        let (sk_f, sk_red) = divide_vector_for_functional_key(&fk, qe_sk.dim, qe_sk.q);
        if i == 0 {
            sk_f_mat = Matrix::new(fhmhm.rows, sk_f.len());
            sk_red_mat = Matrix::new(fhmhm.rows, sk_red.len());
        }
        sk_f_mat.set_row(i, &sk_f);
        sk_red_mat.set_row(i, &sk_red);
    }
    (sk_f_mat, sk_red_mat)
}

pub fn protocol_dec_end(
    ctxt_triple: (&Vec<Integer>, &Vec<Integer>, &Vec<Integer>),
    (sk_f_mat, sk_red_mat): (&Matrix, &Matrix),
    decomp: &Decomp,
    dim: usize,
    q: usize,
    grp: &Group,
) -> Vec<Integer> {
    let mut ct_out = vec![Integer::from(0); sk_f_mat.rows];
    println!("run qe_dec of dim {} for {} times", sk_f_mat.cols, sk_f_mat.rows);
    for i in 0..sk_f_mat.rows {
        let sk_f = sk_f_mat.get_row(i);
        // let sk_f = decomp.vector_pow_exp(&sk_f);
        let sk_red = sk_red_mat.get_row(i);

        let enc_x = ctxt_triple.0.clone();
        let enc_y = ctxt_triple.1.clone();
        let enc_h = ctxt_triple.2.clone();
        let enc_x = decomp.vector_inv(&enc_x);
        let enc_y = decomp.vector_inv(&enc_y);
        let enc_h = decomp.vector_inv(&enc_h);

        let mut fk = Vec::with_capacity(sk_f.len() + sk_red.len());
        fk.extend_from_slice(&sk_f);
        fk.extend_from_slice(&sk_red);

        let mut ctxt = Vec::with_capacity(enc_x.len() + enc_y.len() + enc_h.len());
        ctxt.extend_from_slice(&enc_x);
        ctxt.extend_from_slice(&enc_y);
        ctxt.extend_from_slice(&enc_h);
        println!("ctxt size = {}", ctxt.len());
        println!(" = enc_x.len: {} + enc_y.len: {} + enc_h.len: {}", enc_x.len(), enc_y.len(), enc_h.len());
        ct_out[i] = qe_dec(
            &fk, &ctxt, dim + 1, 2 * (dim + 1) + 1, &grp,
        );
    }
    ct_out
}

pub fn composite_enc_and_f(
    qe_sk: &QfeSk,
    f: &Matrix,
    dim: usize,
    q: usize,
    grp: &Group,
    dcp: &Decomp,
    rng: &mut RandState<'_>,
) -> Matrix {
    println!("start composite_enc_and_f");
    let modulo = grp.delta.clone();

    let mut mat_ctxts = Matrix::new((dim + 1) * (dim + 1), 6*(dim-1)+3*q+2);
    // f: (m + 1) x (dim + 1)^2 matrix
    for i in 0..f.cols {
        println!("check i = {}", i);
        let f_col = f.get_col(i);
        let mu_f_col = vec_mul_scalar(&f_col, &grp.mu);
        let rand_x = gen_random_vector(q, &modulo, rng);
        let rand_y = gen_random_vector(q, &modulo, rng);
        let r_x_pr = gen_random_vector(2, &modulo, rng);
        let r_y_pr = gen_random_vector(2, &modulo, rng);
        let r_x = vec_mul_scalar(&r_x_pr, &(Integer::from(2) * &grp.n));
        let r_y = vec_mul_scalar(&r_y_pr, &(Integer::from(2) * &grp.n));
        
        // ct0 = d_x_null * r_x + d_x_inv * mu * f_col
        // ct1 = d_y_null * r_y + d_y_inv * f_col
        // if i = f.cols:
        //  ct0_c = d_x_null * rand_x + d_x_inv * (mu * f_col + sk->V * r_x)
        //  ct1_c = d_y_null * rand_y + d_y_inv * (f_col + sk->W * r_y)
        
        let ct0_left = qe_sk.d_x_null.mul_vec(&rand_x);
        let mut ct0_right = qe_sk.d_x_inv.mul_vec(&mu_f_col);
        if i == f.cols {
            ct0_right = vec_add(&ct0_right, &qe_sk.v.mul_vec(&r_x));
        }
        let mut ct0 = vec_add(&ct0_left, &ct0_right);
        vec_mod(&mut ct0, &modulo);

        let ct1_left = qe_sk.d_y_null.mul_vec(&rand_y);
        let mut ct1_right = qe_sk.d_y_inv.mul_vec(&f_col);
        if i == f.cols {
            ct1_right = vec_add(&ct1_right, &qe_sk.w.mul_vec(&r_y));
        }
        let mut ct1 = vec_add(&ct1_left, &ct1_right);
        vec_mod(&mut ct1, &modulo);

        // h = (r_x tensor f_col) || (mu_f_col tensor r_y)
        // if i == f.cols:
        // h = (rand_x tensor f_col) || ((mu_f_col + sk->v * r_x) tensor r_y)
        let h_left = vector::tensor_product_vecs(&r_x, &f_col, &modulo);
        let tmp = if i == f.cols {
            vec_add(&mu_f_col, &qe_sk.v.mul_vec(&r_x))
        } else {
            mu_f_col.clone()
        };
        let h_right = vector::tensor_product_vecs(&tmp, &r_y, &modulo);
        let mut h = Vec::with_capacity(h_left.len() + h_right.len());
        h.extend_from_slice(&h_left);
        h.extend_from_slice(&h_right);
        println!("check i = {}", i);
        println!("qe.sk.ipe_sk.dim = {}", qe_sk.ipe_sk.dim);
        println!("h size = {}", h.len());
        let ctxt_ipe = ipe_enc(&qe_sk.ipe_sk, &h, grp, false, rng);
        println!("check i = {}", i);

        // ith row of mat_ctxts = (ct0, ct1, ctxt_ipe)
        let mut row = Vec::with_capacity(ct0.len() + ct1.len() + ctxt_ipe.len());
        row.extend_from_slice(&ct0);
        row.extend_from_slice(&ct1);
        row.extend_from_slice(&ctxt_ipe);
        println!("mat_ctxts size = {} x {}", mat_ctxts.rows, mat_ctxts.cols);
        println!("dim, q = {}, {}", dim, q);
        println!("row size = {}", row.len());
        println!("ct0.size = {}", ct0.len());
        println!("ct1.size = {}", ct1.len());
        println!("ctxt_ipe.size = {}", ctxt_ipe.len());
        mat_ctxts.set_row(i, &row);
    }

    let mat_ctxts = dcp.matrix_row(&mat_ctxts);

    println!("end composite_enc_and_f");
    // output size = L * (6 * dim + 3 * q + 2) x ((dim + 1)^2 + 1)
    mat_ctxts.transpose()
}

pub fn protocol_keygen_dcr_to_qe(
    dcr_sk: &Vec<Integer>,
    qe_sk: &QfeSk,
    h_right: &Matrix,
    gamma_left: &Matrix,
    dim: usize,
    q: usize,
    decomp: &Decomp,
    grp: &Group,
    rng: &mut RandState<'_>,
) -> (Matrix, Vec<Integer>) {
    let total_mat = h_right * gamma_left;

    println!("total_mat size = {} x {}", total_mat.rows, total_mat.cols);
    // composite enc and f
    let fk_mat = composite_enc_and_f(
        qe_sk, &total_mat, dim+1, 2*dim+1, grp, decomp, rng);
    
    println!("fk_mat size = {} x {}", fk_mat.rows, fk_mat.cols);

    // keygen_dcr for each row of mat_ctxts
    let mut fk = vec![Integer::from(0); fk_mat.rows];
    for i in 0..fk_mat.rows {
        let row = fk_mat.get_row(i);
        fk[i] = dcr_keygen(dcr_sk, &row);
    }

    (fk_mat, fk)
}

pub fn protocol_keygen_qe_to_qe(
    qe_sk_enc: &QfeSk,
    qe_sk_keygen: &QfeSk,
    h_right: &Matrix,
    hm_left: &Matrix,
    dim: usize,
    q: usize,
    k: usize,
    f: &Matrix,
    decomp: &Decomp,
    grp: &Group,
    rng: &mut RandState<'_>,
) -> Matrix {
    // function: h_right * f * (hm_left tensor hm-left)
    let modulo = grp.delta.clone();
    let hm_origin = remove_diag_one(&hm_left);
    let hmhm = Matrix::tensor_product(&hm_origin, &hm_origin, &grp.delta);
    let total_mat = h_right * f * &hmhm;

    // composite enc and f
    let mat_ctxts = composite_enc_and_f(
        qe_sk_enc, &total_mat, dim, q, grp, decomp, rng);   
        
    // keygen_qe for each row of mat_ctxts
    let dim_qe_input = (dim + 1) * (dim + 1) + 1;
    let q_qe_input =  1;
    let mut fk_mat = Matrix::new(
        mat_ctxts.rows,
        get_funcional_key_len(dim_qe_input, q_qe_input)
    );
    for i in 0..mat_ctxts.rows {
        let row = mat_ctxts.get_row(i);
        let fk = qe_keygen(qe_sk_keygen, &row, grp);
        fk_mat.set_row(i, &fk);
    }
    
    fk_mat
}

pub fn protocol_keygen_qe_to_plain(
    qe_sk: &QfeSk,
    hm_left: &Matrix,
    f: &Matrix,
    grp: &Group,
) -> Matrix {
    let hm_origin = remove_diag_one(&hm_left);
    let hmhm = Matrix::tensor_product(&hm_origin, &hm_origin, &grp.delta);
    let mut total_mat = f * &hmhm;
    total_mat.mod_inplace(&grp.delta);

    let mut fk_mat = Matrix::new(1, 1);
    for i in 0..total_mat.rows {
        let row = total_mat.get_row(i);
        let fk = qe_keygen(qe_sk, &row, grp);
        if i == 0 {
            fk_mat = Matrix::new(total_mat.rows, fk.len());
        }
        fk_mat.set_row(i, &fk);
    }
    fk_mat
}


pub fn protocol_dec_dcr_to_qe(
    ctxt: &Vec<Integer>,
    fk_mat: &Matrix,
    fk_vec: &Vec<Integer>,
    decomp: &Decomp,
    grp: &Group,
) -> Vec<Integer> {
    let mut ct_out = vec![Integer::from(0); fk_mat.rows];
    for i in 0..fk_mat.rows {
        ct_out[i] = dcr_dec(ctxt, &fk_mat.get_row(i), &fk_vec[i], grp);
    }
    vec_mod(&mut ct_out, &grp.n);
    decomp.vector_inv(&ct_out)
}

pub fn protocol_dec_qe_to_qe(
    ctxt: &Vec<Integer>,
    fk_mat: &Matrix,
    dim: usize,
    q: usize,
    decomp: &Decomp,
    grp: &Group,
) -> Vec<Integer> {
    let mut ct_out = vec![Integer::from(0); fk_mat.rows];
    for i in 0..fk_mat.rows {
        let fk = fk_mat.get_row(i);
        let fk = decomp.vector_pow_exp(&fk);
        let enc = ctxt.clone();
        ct_out[i] = qe_dec(&fk, &enc, dim, q, grp);
    }
    vec_mod(&mut ct_out, &grp.n);
    decomp.vector_inv(&ct_out)
}

pub fn protocol_dec_qe_to_plain(
    ctxt: &Vec<Integer>,
    fk_mat: &Matrix,
    dim: usize,
    q: usize,
    grp: &Group,
) -> Vec<Integer> {
    let mut res = vec![Integer::from(0); fk_mat.rows];
    for i in 0..fk_mat.rows {
        let fk = fk_mat.get_row(i);
        let enc = ctxt.clone();
        res[i] = qe_dec(&fk, &enc, dim + 1, 2 * (dim + 1) + 1, grp);
    }
    vec_mod(&mut res, &grp.n);
    res
}