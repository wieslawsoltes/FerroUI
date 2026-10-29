//! Shared by initial asset generation and runtime development documents.
pub fn replace_geometry(source: &mut String, key: &str, data: &str) -> Result<usize, String> {
    let start_tag = format!("<StreamGeometry x:Key=\"{key}\">");
    let mut from = 0;
    let mut count = 0;
    while let Some(start) = source[from..]
        .find(&start_tag)
        .map(|at| from + at + start_tag.len())
    {
        let end = start
            + source[start..]
                .find("</StreamGeometry>")
                .ok_or_else(|| format!("The geometry resource {key} is not closed"))?;
        source.replace_range(start..end, data);
        from = start + data.len();
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn substitutes_all_matching_resources_without_changing_other_markup() {
        let mut source = "<StreamGeometry x:Key=\"AppIcon\">old</StreamGeometry><TextBlock Text=\"old\"/><StreamGeometry x:Key=\"AppIcon\">other</StreamGeometry>".to_owned();
        assert_eq!(replace_geometry(&mut source, "AppIcon", "new").unwrap(), 2);
        assert!(source.contains("<TextBlock Text=\"old\"/>"));
        assert_eq!(source.matches(">new</StreamGeometry>").count(), 2);
        assert_eq!(
            replace_geometry(&mut source, "missing", "unused").unwrap(),
            0
        );
    }
    #[test]
    fn incomplete_geometry_is_rejected() {
        let mut source = "<StreamGeometry x:Key=\"AppIcon\">old".to_owned();
        assert!(replace_geometry(&mut source, "AppIcon", "new").is_err());
    }
}
