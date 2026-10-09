use ferroui_base::media::text_formatting::unicode::Script;

/// The four letter codes of ISO 15924 of the scripts of the base library,
/// in the order of their numbers: the base library has the table
/// (`PropertyValueAliasHelper`), but not for another crate, and the font
/// collection of the system asks for a fallback by this code.
const TAGS: [[u8; 4]; 176] = [
    *b"Zzzz", *b"Zyyy", *b"Zinh", *b"Adlm", *b"Aghb", *b"Ahom", *b"Arab", *b"Armi",
    *b"Armn", *b"Avst", *b"Bali", *b"Bamu", *b"Bass", *b"Batk", *b"Beng", *b"Berf",
    *b"Bhks", *b"Bopo", *b"Brah", *b"Brai", *b"Bugi", *b"Buhd", *b"Cakm", *b"Cans",
    *b"Cari", *b"Cham", *b"Cher", *b"Chrs", *b"Copt", *b"Cpmn", *b"Cprt", *b"Cyrl",
    *b"Deva", *b"Diak", *b"Dogr", *b"Dsrt", *b"Dupl", *b"Egyp", *b"Elba", *b"Elym",
    *b"Ethi", *b"Gara", *b"Geor", *b"Glag", *b"Gong", *b"Gonm", *b"Goth", *b"Gran",
    *b"Grek", *b"Gujr", *b"Gukh", *b"Guru", *b"Hang", *b"Hani", *b"Hano", *b"Hatr",
    *b"Hebr", *b"Hira", *b"Hluw", *b"Hmng", *b"Hmnp", *b"Hrkt", *b"Hung", *b"Ital",
    *b"Java", *b"Kali", *b"Kana", *b"Kawi", *b"Khar", *b"Khmr", *b"Khoj", *b"Kits",
    *b"Knda", *b"Krai", *b"Kthi", *b"Lana", *b"Laoo", *b"Latn", *b"Lepc", *b"Limb",
    *b"Lina", *b"Linb", *b"Lisu", *b"Lyci", *b"Lydi", *b"Mahj", *b"Maka", *b"Mand",
    *b"Mani", *b"Marc", *b"Medf", *b"Mend", *b"Merc", *b"Mero", *b"Mlym", *b"Modi",
    *b"Mong", *b"Mroo", *b"Mtei", *b"Mult", *b"Mymr", *b"Nagm", *b"Nand", *b"Narb",
    *b"Nbat", *b"Newa", *b"Nkoo", *b"Nshu", *b"Ogam", *b"Olck", *b"Onao", *b"Orkh",
    *b"Orya", *b"Osge", *b"Osma", *b"Ougr", *b"Palm", *b"Pauc", *b"Perm", *b"Phag",
    *b"Phli", *b"Phlp", *b"Phnx", *b"Plrd", *b"Prti", *b"Rjng", *b"Rohg", *b"Runr",
    *b"Samr", *b"Sarb", *b"Saur", *b"Sgnw", *b"Shaw", *b"Shrd", *b"Sidd", *b"Sidt",
    *b"Sind", *b"Sinh", *b"Sogd", *b"Sogo", *b"Sora", *b"Soyo", *b"Sund", *b"Sunu",
    *b"Sylo", *b"Syrc", *b"Tagb", *b"Takr", *b"Tale", *b"Talu", *b"Taml", *b"Tang",
    *b"Tavt", *b"Tayo", *b"Telu", *b"Tfng", *b"Tglg", *b"Thaa", *b"Thai", *b"Tibt",
    *b"Tirh", *b"Tnsa", *b"Todr", *b"Tols", *b"Toto", *b"Tutg", *b"Ugar", *b"Vaii",
    *b"Vith", *b"Wara", *b"Wcho", *b"Xpeo", *b"Xsux", *b"Yezi", *b"Yiii", *b"Zanb",
];

/// The code of ISO 15924 of a script, `Zzzz` for one the table does not
/// have.
pub fn script_tag(script: Script) -> [u8; 4] {
    TAGS.get(script as i32 as usize).copied().unwrap_or(*b"Zzzz")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_codes_of_some_scripts() {
        assert_eq!(*b"Zzzz", script_tag(Script::Unknown));
        assert_eq!(*b"Zyyy", script_tag(Script::Common));
        assert_eq!(*b"Arab", script_tag(Script::Arabic));
        assert_eq!(*b"Latn", script_tag(Script::Latin));
        assert_eq!(*b"Hani", script_tag(Script::Han));
    }
}
