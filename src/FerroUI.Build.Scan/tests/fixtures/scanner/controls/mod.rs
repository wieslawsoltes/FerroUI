mod border;
pub mod decorator;
pub mod grid;
pub mod text_block;

pub use border::Border;
pub use grid::Grid;

#[cfg(test)]
mod tests {
    use super::*;

    ferroui_base::ferro_class!(TestOnly: Border);
}
