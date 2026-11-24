//! `{SvgImage}` in markup with no `xmlns:` prefix, through the compiled loader and the run-time
//! loader.

// The generated code repeats branches and clones literals.
#[allow(clippy::if_same_then_else, clippy::clone_on_copy)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}

pub use generated::compiled_xaml;

#[cfg(test)]
mod tests;
