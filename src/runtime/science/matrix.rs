//! Matrix operations for `import "science"`.

use super::helpers::{
    float_out, int_out, matrix_at, matrix_cols, matrix_out, matrix_rows, num_at, require_mul_shape,
    require_same_shape, require_square, vector_at, vector_out,
};
use crate::value::{Environment, Value};

fn mat(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let rows = num_at(args, 0, "mat")? as usize;
    let cols = num_at(args, 1, "mat")? as usize;
    let fill = if args.len() > 2 {
        num_at(args, 2, "mat")?
    } else {
        0.0
    };
    Ok(matrix_out(
        &(0..rows)
            .map(|_| vec![fill; cols])
            .collect::<Vec<_>>(),
    ))
}

fn mat_identity(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "mat_identity")? as usize;
    let mut m = vec![vec![0.0; n]; n];
    for i in 0..n {
        m[i][i] = 1.0;
    }
    Ok(matrix_out(&m))
}

fn mat_rows(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_rows")?;
    Ok(int_out(matrix_rows(&m) as i64))
}

fn mat_cols(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_cols")?;
    Ok(int_out(matrix_cols(&m)? as i64))
}

fn mat_transpose(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_transpose")?;
    let rows = matrix_rows(&m);
    let cols = matrix_cols(&m)?;
    let mut t = vec![vec![0.0; rows]; cols];
    for i in 0..rows {
        for j in 0..cols {
            t[j][i] = m[i][j];
        }
    }
    Ok(matrix_out(&t))
}

fn mat_add(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = matrix_at(args, 0, "mat_add")?;
    let b = matrix_at(args, 1, "mat_add")?;
    require_same_shape(&a, &b, "mat_add")?;
    let out: Vec<Vec<f64>> = a
        .iter()
        .zip(b.iter())
        .map(|(ra, rb)| ra.iter().zip(rb.iter()).map(|(x, y)| x + y).collect())
        .collect();
    Ok(matrix_out(&out))
}

fn mat_sub(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = matrix_at(args, 0, "mat_sub")?;
    let b = matrix_at(args, 1, "mat_sub")?;
    require_same_shape(&a, &b, "mat_sub")?;
    let out: Vec<Vec<f64>> = a
        .iter()
        .zip(b.iter())
        .map(|(ra, rb)| ra.iter().zip(rb.iter()).map(|(x, y)| x - y).collect())
        .collect();
    Ok(matrix_out(&out))
}

fn mat_scale(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_scale")?;
    let s = num_at(args, 1, "mat_scale")?;
    let out: Vec<Vec<f64>> = m
        .iter()
        .map(|row| row.iter().map(|x| x * s).collect())
        .collect();
    Ok(matrix_out(&out))
}

fn mat_mul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = matrix_at(args, 0, "mat_mul")?;
    let b = matrix_at(args, 1, "mat_mul")?;
    require_mul_shape(&a, &b, "mat_mul")?;
    let rows = matrix_rows(&a);
    let cols = matrix_cols(&b)?;
    let inner = matrix_cols(&a)?;
    let mut out = vec![vec![0.0; cols]; rows];
    for i in 0..rows {
        for j in 0..cols {
            for k in 0..inner {
                out[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    Ok(matrix_out(&out))
}

fn mat_dot(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "mat_dot")?;
    let b = vector_at(args, 1, "mat_dot")?;
    if a.len() != b.len() {
        return Err("mat_dot: vectors must have equal length".into());
    }
    Ok(float_out(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()))
}

fn mat_norm(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = vector_at(args, 0, "mat_norm")?;
    Ok(float_out(v.iter().map(|x| x * x).sum::<f64>().sqrt()))
}

fn det_recursive(m: &[Vec<f64>]) -> f64 {
    let n = m.len();
    if n == 1 {
        return m[0][0];
    }
    if n == 2 {
        return m[0][0] * m[1][1] - m[0][1] * m[1][0];
    }
    let mut det = 0.0;
    for col in 0..n {
        let minor: Vec<Vec<f64>> = (1..n)
            .map(|r| {
                (0..n)
                    .filter(|&c| c != col)
                    .map(|c| m[r][c])
                    .collect()
            })
            .collect();
        let sign = if col % 2 == 0 { 1.0 } else { -1.0 };
        det += sign * m[0][col] * det_recursive(&minor);
    }
    det
}

fn mat_det(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_det")?;
    require_square(&m, "mat_det")?;
    Ok(float_out(det_recursive(&m)))
}

fn gauss_jordan_inverse(m: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let n = m.len();
    let mut aug = vec![vec![0.0; 2 * n]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = m[i][j];
        }
        aug[i][n + i] = 1.0;
    }
    for col in 0..n {
        let mut pivot = col;
        for row in col..n {
            if aug[row][col].abs() > aug[pivot][col].abs() {
                pivot = row;
            }
        }
        if aug[pivot][col].abs() < 1e-12 {
            return Err("mat_inv: matrix is singular".into());
        }
        if pivot != col {
            aug.swap(pivot, col);
        }
        let div = aug[col][col];
        for j in 0..2 * n {
            aug[col][j] /= div;
        }
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = aug[row][col];
            for j in 0..2 * n {
                aug[row][j] -= factor * aug[col][j];
            }
        }
    }
    Ok((0..n)
        .map(|i| (n..2 * n).map(|j| aug[i][j]).collect())
        .collect())
}

fn mat_inv(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_inv")?;
    require_square(&m, "mat_inv")?;
    Ok(matrix_out(&gauss_jordan_inverse(&m)?))
}

fn mat_vec_mul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_vec_mul")?;
    let v = vector_at(args, 1, "mat_vec_mul")?;
    let cols = matrix_cols(&m)?;
    if v.len() != cols {
        return Err("mat_vec_mul: vector length must match matrix columns".into());
    }
    let out: Vec<f64> = m
        .iter()
        .map(|row| row.iter().zip(v.iter()).map(|(a, b)| a * b).sum())
        .collect();
    Ok(vector_out(&out))
}

fn mat_eigen2(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = matrix_at(args, 0, "mat_eigen2")?;
    require_square(&m, "mat_eigen2")?;
    if m.len() != 2 {
        return Err("mat_eigen2: matrix must be 2x2".into());
    }
    let a = m[0][0];
    let b = m[0][1];
    let c = m[1][0];
    let d = m[1][1];
    let tr = a + d;
    let det = a * d - b * c;
    let disc = tr * tr - 4.0 * det;
    if disc < 0.0 {
        return Err("mat_eigen2: complex eigenvalues not supported".into());
    }
    let s = disc.sqrt();
    Ok(vector_out(&[(tr + s) / 2.0, (tr - s) / 2.0]))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_mat", "mat"], mat);
    bind(&["science_mat_identity", "mat_identity"], mat_identity);
    bind(&["science_mat_rows", "mat_rows"], mat_rows);
    bind(&["science_mat_cols", "mat_cols"], mat_cols);
    bind(&["science_mat_transpose", "mat_transpose"], mat_transpose);
    bind(&["science_mat_add", "mat_add"], mat_add);
    bind(&["science_mat_sub", "mat_sub"], mat_sub);
    bind(&["science_mat_scale", "mat_scale"], mat_scale);
    bind(&["science_mat_mul", "mat_mul"], mat_mul);
    bind(&["science_mat_dot", "mat_dot"], mat_dot);
    bind(&["science_mat_norm", "mat_norm"], mat_norm);
    bind(&["science_mat_det", "mat_det"], mat_det);
    bind(&["science_mat_inv", "mat_inv"], mat_inv);
    bind(&["science_mat_vec_mul", "mat_vec_mul"], mat_vec_mul);
    bind(&["science_mat_eigen2", "mat_eigen2"], mat_eigen2);
}
