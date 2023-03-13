/*! `cayley` is a crate for generic linear algebra. It aims to do everything stack-allocated
and constantly sized (though there are workarounds possible if dynamically sized
types are needed). `cayley` is named after Arthur Cayley, a prominent mathematician who
introduced matrix multiplication.

In addition to this, it aims to assume as little as possible
about the type over which its structures are generic. For example, you can construct
an identity matrix of any type that implements `One`, `Zero` and `Copy`, and you can multiply
matrices of different types A and B, so long as there exists a type C so that A * B = C
and C + C = C. In practice, of course, all numerical types meet these conditions.

Due to the nature of generic matrices, it's necessary to use the `#[feature(generic_const_exprs)]`
feature; there is no other way to provide compile-time multiplicability or invertibility checks.

In its current state, cayley is VERY work-in-progress. Don't use this in production.
*/

#![allow(dead_code)]
#![doc(test(attr(feature(generic_const_exprs))))]
#![feature(generic_const_exprs)]
#![deny(missing_docs)]
use num_traits::{NumOps, One, Zero};
use std::fmt::{self, Display};
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Sub, SubAssign};

/// The following is some weird shit. This enum is generic over a boolean condition.
/// It then only implements the IsTrue trait for `DimensionAssertion<true>`, so that
/// an assertion can be made within a function signature or an impl block.
pub enum DimensionAssertion<const CONDITION: bool> {}
/// IsTrue is only ever implemented on `DimensionAssertion<true>`. See its documentation
/// for info on why this exists.
pub trait IsTrue {}
impl IsTrue for DimensionAssertion<true> {}

/// The base Matrix struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matrix<T, const N: usize, const M: usize>
where
    [(); N * M]:,
{
    data: [T; N * M],
    rows: usize,
    cols: usize,
}

/// Convenience stuff.
impl<T, const N: usize, const M: usize> Index<(usize, usize)> for Matrix<T, N, M>
where
    [(); N * M]:,
{
    type Output = T;
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        assert!(
            index.0 < self.rows,
            "Tried indexing into row {}, which outside of the matrix (has {} rows).",
            index.0,
            self.rows
        );
        assert!(
            index.1 < self.cols,
            "Tried indexing into column {}, which outside of the matrix (has {} column).",
            index.1,
            self.cols
        );
        &self.data[index.0 * self.cols + index.1]
    }
}

impl<T, const N: usize, const M: usize> IndexMut<(usize, usize)> for Matrix<T, N, M>
where
    [(); N * M]:,
{
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        &mut self.data[index.0 * self.cols + index.1]
    }
}

impl<T, const N: usize, const M: usize> Display for Matrix<T, N, M>
where
    T: Display,
    [(); N * M]:,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Displaying a matrix is kind of interesting: we want to have nicely
        // spaced and nicely aligned numbers, but use cases might arise where
        // the elements of a Matrix implement NumAssignOps and Display but aren't
        // numbers in and of themselves. We have to figure out the longest
        // string representation first, then do all of the printing stuff.

        let string_reps = self.data.iter().map(|e| e.to_string()).collect::<Vec<_>>();
        let longest = string_reps.iter().fold(0, |current_max, new| {
            if new.len() > current_max {
                new.len()
            } else {
                current_max
            }
        });

        let padded_string_reps = string_reps
            .iter()
            .map(|s| format!("{:0l$} ", s, l = longest))
            .collect::<Vec<String>>();

        for row in padded_string_reps.chunks_exact(self.rows) {
            writeln!(
                f,
                "{}",
                row.iter().fold(String::new(), |mut acc, val| {
                    acc.push_str(val);
                    acc
                })
            )?;
        }

        Ok(())
    }
}

impl<T, const N: usize, const M: usize> From<Vec<Vec<T>>> for Matrix<T, N, M>
where
    T: Copy,
    [(); N * M]:,
{
    fn from(value: Vec<Vec<T>>) -> Matrix<T, N, M> {
        assert!(
            value.iter().all(|row| row.len() == value[0].len()),
            "Not all rows have the same length."
        );
        let mut data = [value[0][0]; N * M];
        let mut flattened = value.iter().flatten();
        for i in 0..N * M {
            data[i] = *flattened.next().unwrap();
        }
        Self {
            data,
            rows: value.len(),
            cols: value[0].len(),
        }
    }
}

/// Constructors.
impl<T, const N: usize, const M: usize> Matrix<T, N, M>
where
    T: Zero + Copy,
    [(); N * M]:,
{
    /// Initialises a matrix filled with zeroes.
    ///
    /// This requires the type of the matrix to implement the num_traits::Zero trait.
    /// A type implementing NumAssignOps but not Zero is very rare though.
    ///
    /// ## Panics
    ///
    /// If the specified rows and columns don't create a matrix with N elements.
    pub fn zeroes(r: usize, c: usize) -> Self {
        assert_eq!(
            N, r,
            "Dimensionality of the matrix does not hold: rows do not match."
        );
        assert_eq!(
            M, c,
            "Dimensionality of the matrix does not hold: columns do not match."
        );

        Matrix {
            data: [T::zero(); N * M],
            rows: r,
            cols: c,
        }
    }

    /// Constructs a Matrix from a closure. The closure takes in two zero-indexed usizes and
    /// returns any type T. It requires that T implements Zero for allocation purposes.
    /// This should probably be changed to implementing Default, now that I think about it.
    ///
    /// ## Example
    ///
    /// ```
    /// use cayley::Matrix;
    /// let m: Matrix<usize, 2, 3> = Matrix::from_closure(2, 3, |x, y| x + y);
    /// assert_eq!(m, Matrix::from(vec![vec![0, 1, 2], vec![1, 2, 3]]));
    /// ```
    pub fn from_closure<F>(r: usize, c: usize, func: F) -> Self
    where
        F: Fn(usize, usize) -> T,
    {
        let mut result = Matrix::zeroes(r, c);
        for x in 0..N {
            for y in 0..M {
                result[(x, y)] = func(x, y);
            }
        }

        result
    }
}

impl<T, const N: usize, const M: usize> Matrix<T, N, M>
where
    T: One + Copy,
    [(); N * M]:,
{
    /// Initialises a matrix filled with ones.
    ///
    /// This requires the type of the matrix to implement the num_traits::One trait.
    /// A type implementing NumAssignOps but not One is very rare though.
    ///
    /// ## Panics
    ///
    /// If the specified rows and columns don't create a matrix with N elements.
    pub fn ones(r: usize, c: usize) -> Self {
        assert_eq!(
            N, r,
            "Dimensionality of the matrix does not hold: rows do not match."
        );
        assert_eq!(
            M, c,
            "Dimensionality of the matrix does not hold: columns do not match."
        );

        Matrix {
            data: [T::one(); N * M],
            rows: r,
            cols: c,
        }
    }
}

impl<T, const N: usize> Matrix<T, N, N>
where
    T: Zero + One + Copy,
    [(); N * N]:,
{
    /// Constructs the identity matrix. Requires the target type to implement
    /// Zero and One, for obvious reasons.
    ///
    /// ## Example
    /// ```
    /// use cayley::Matrix;
    /// let m: Matrix<i32, 3, 3> = Matrix::identity(3);
    /// assert_eq!(
    ///     m,
    ///     Matrix::from(vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]])
    /// );
    /// ```
    pub fn identity(size: usize) -> Self {
        let mut base = Matrix::zeroes(size, size);
        for i in 0..size {
            base[(i, i)] = T::one();
        }
        base
    }
}

/// Operations on matrices.
/// Note that the resulting matrix takes on the type of the left matrix.

// Addition.
impl<T, Q, const N: usize, const M: usize> Add<Matrix<Q, N, M>> for Matrix<T, N, M>
where
    T: Add<Q, Output = T> + Copy,
    Q: Copy,
    [(); N * M]:,
{
    type Output = Matrix<T, N, M>;
    fn add(self, rhs: Matrix<Q, N, M>) -> Self::Output {
        assert_eq!(
            self.rows, rhs.rows,
            "Matrices do not have the same dimension."
        );
        let mut data: [T; N * M] = self.data;
        for i in 0..N * M {
            data[i] = data[i] + rhs.data[i];
        }

        Matrix {
            data,
            rows: self.rows,
            cols: self.cols,
        }
    }
}

impl<T, Q, const N: usize, const M: usize> AddAssign<Matrix<Q, N, M>> for Matrix<T, N, M>
where
    T: AddAssign<Q>,
    Q: Copy,
    [(); N * M]:,
{
    fn add_assign(&mut self, rhs: Matrix<Q, N, M>) {
        for i in 0..N * M {
            self.data[i] += rhs.data[i];
        }
    }
}

// Subtraction.
impl<T, Q, const N: usize, const M: usize> Sub<Matrix<Q, N, M>> for Matrix<T, N, M>
where
    T: Sub<Q, Output = T> + Copy,
    Q: Copy,
    [(); N * M]:,
{
    type Output = Matrix<T, N, M>;
    fn sub(self, rhs: Matrix<Q, N, M>) -> Self::Output {
        assert_eq!(
            self.rows, rhs.rows,
            "Matrices do not have the same dimension."
        );
        let mut data: [T; N * M] = self.data;
        for i in 0..N * M {
            data[i] = data[i] - rhs.data[i];
        }

        Matrix {
            data,
            rows: self.rows,
            cols: self.cols,
        }
    }
}

impl<T, Q, const N: usize, const M: usize> SubAssign<Matrix<Q, N, M>> for Matrix<T, N, M>
where
    T: SubAssign<Q>,
    Q: Copy,
    [(); N * M]:,
{
    fn sub_assign(&mut self, rhs: Matrix<Q, N, M>) {
        for i in 0..N * M {
            self.data[i] -= rhs.data[i];
        }
    }
}

// Multiplication

impl<T, Q, R, const N: usize, const M: usize, const O: usize, const P: usize> Mul<Matrix<Q, O, P>>
    for Matrix<T, N, M>
where
    T: Copy + Mul<Q, Output = R>,
    Q: Copy,
    R: Add + Zero + Copy,
    [(); N * M]:,
    [(); O * P]:,
    [(); N * P]:,
    DimensionAssertion<{ M == O }>: IsTrue,
{
    type Output = Matrix<R, N, P>;
    /// Multiplies two matrices.
    ///
    /// ## Examples
    ///
    /// ```compile_fail
    /// let m1: Matrix<i32, 2, 3> = Matrix::from(vec![vec![1, 2, 3], vec![4, 5, 6]]);
    /// let m2: Matrix<i32, 2, 2> = Matrix::from(vec![vec![1, 2], vec![3, 4]]);
    /// let a = m1 * m2; // this does not compile!
    /// ```
    fn mul(self, rhs: Matrix<Q, O, P>) -> Self::Output {
        let mut result: Matrix<R, N, P> = Matrix::zeroes(N, P);

        for x in 0..N {
            for y in 0..P {
                let mut dot_product_terms = [R::zero(); M];
                for i in 0..M {
                    dot_product_terms[i] = self[(x, i)] * rhs[(i, y)];
                }
                result[(x, y)] = dot_product_terms
                    .iter()
                    .fold(R::zero(), |acc, val| acc + *val);
            }
        }

        result
    }
}

impl<T, const N: usize, const M: usize> Matrix<T, N, M>
where
    [(); N * M]:,
    [(); M * N]:,
    T: Copy + Zero,
{
    /// Returns the transpose of the matrix.
    ///
    /// Note that this means that (unless the Matrix is square) the return type is
    /// different from the caller type: `Matrix<u8, 2, 3>.transpose()` returns a `Matrix<u8, 3, 2>`.
    pub fn transpose(&self) -> Matrix<T, M, N> {
        let mut result = Matrix::zeroes(M, N);

        for x in 0..M {
            for y in 0..N {
                result[(x, y)] = self[(y, x)];
            }
        }

        result
    }
}

impl<T, const N: usize, const M: usize> Matrix<T, N, M>
where [(); N * M]:, [(); (N-1)*(M-1)]
{
    pub fn submatrix(&self, r: usize, c: usize) -> Matrix<T, N - 1, M - 1> {
        assert!(r < self.rows);
        assert!(c < self.cols);
    }
}

impl<T, const N: usize> Matrix<T, N, N>
where
    [(); N * N]:,
    T: Copy + NumOps + Zero,
{
    /// Calculates the determinant of a Matrix.
    /// Requires the relevant type to implement NumOps (Add, Sub, Mul, Div), as well
    /// as Copy and Zero.
    pub fn determinant(&self) -> T {
        match N {
            1 => self[(0, 0)],
            2 => self[(0, 0)] * self[(1, 1)] - self[(0, 1)] * self[(1, 0)],
            3 => self[(0, 0)] * self[(1, 1)] * self[(2, 2)] + self[(0, 1)] * self[(1, 2)] * self[(2, 0)] + self[(0, 2)] * self[(1, 0)] * self[(2, 1)] - 
                 self[(0, 2)] * self[(1, 1)] * self[(2, 0)] + self[(0, 1)] * self[(1, 0)] * self[(2, 2)] + self[(0, 0)] * self[(1, 2)] * self[(2, 1)]
            n => {
                // recursive solution: determine cofactors of top row, multiply with top row's entries, then sum together
                
            },
        }
    }
}

impl<T, const N: usize> Matrix<T, N, N>
where
    [(); N * N]:,
    T: Copy + NumOps + Zero + PartialEq,
{
    /// Attempts to calculate the inverse of the Matrix. Note that this is only
    /// implemented for `Matrix<T, N, N>`, i.e. square matrices.
    ///
    /// ## Returns
    ///
    /// An `Option<Self>`: `None` if the matrix isn't invertible and `Some(m)` with
    /// m being the inverted matrix.
    pub fn inverse(&self) -> Option<Self> {
        if self.determinant() == T::zero() {
            None
        } else {
            todo!()
        }
    }
}

mod tests;
