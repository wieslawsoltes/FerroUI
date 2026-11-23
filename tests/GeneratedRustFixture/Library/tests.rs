//! The assets the build of the crate embedded (#850).

use crate::compiled_markup::ASSETS;

/// A dictionary merged into a class is compiled into a unit of the class, and its text is not
/// embedded again as a raw asset of a directory the build embeds: neither its path nor the canary
/// comment of the file is in the assets. A file of the directory that no document consumed is
/// still embedded.
#[test]
fn a_dictionary_the_compiler_consumed_is_not_embedded_again() {
    let paths: Vec<&str> = ASSETS.iter().map(|(path, _)| *path).collect();
    assert!(!paths.contains(&"/Dictionaries/Part.xaml"), "{paths:?}");
    assert!(paths.contains(&"/Dictionaries/Loose.xaml"), "{paths:?}");
    let canary = b"The canary of #850";
    let embedded = ASSETS.iter().any(|(_, bytes)| bytes.windows(canary.len()).any(|window| window == canary));
    assert!(!embedded, "the text of the merged dictionary is embedded");
}
