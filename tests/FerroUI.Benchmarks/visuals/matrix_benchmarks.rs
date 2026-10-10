//! Decomposing a matrix into translation, rotation, scale and skew.

use crate::harness::Registry;
use ferroui_base::Matrix;

static S_DATA: Matrix = Matrix::IDENTITY;

pub struct MatrixBenchmarks;

impl MatrixBenchmarks {
    pub fn new() -> Self {
        Self
    }

    pub fn decompose(&self) -> bool {
        Matrix::try_decompose_transform(S_DATA).is_some()
    }
}

impl Default for MatrixBenchmarks {
    fn default() -> Self {
        Self::new()
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("visuals", "MatrixBenchmarks");
    class.benchmark("decompose", "", MatrixBenchmarks::new, |b| b.decompose()).baseline();
}

#[cfg(test)]
mod tests {
    #[test]
    fn matrix_benchmarks() {
        crate::harness::smoke_class(super::register, "MatrixBenchmarks");
    }
}
