bitflags::bitflags! {
    /// Code pages a font declares support for, as recorded in the OS/2 table's
    /// `ulCodePageRange1` (low 32 bits) and `ulCodePageRange2` (high 32 bits).
    ///
    /// Each bit indicates that the font is considered functional for the
    /// corresponding code page. For fallback selection this is the most reliable
    /// declared signal of the font's *intended* locale: a CJK font that sets the
    /// `JapaneseJis` bit but not `ChineseSimplified` is saying "I'm designed for
    /// Japanese".
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct FontCodePageCoverage: u64 {
        /// No code page coverage declared.
        const None = 0;
        const Latin1 = 1 << 0;
        const Latin2EasternEurope = 1 << 1;
        const Cyrillic = 1 << 2;
        const Greek = 1 << 3;
        const Turkish = 1 << 4;
        const Hebrew = 1 << 5;
        const Arabic = 1 << 6;
        const WindowsBaltic = 1 << 7;
        const Vietnamese = 1 << 8;
        const Thai = 1 << 16;
        const JapaneseJis = 1 << 17;
        const ChineseSimplified = 1 << 18;
        const KoreanWansung = 1 << 19;
        const ChineseTraditional = 1 << 20;
        const KoreanJohab = 1 << 21;
        const MacRoman = 1 << 29;
        const Oem = 1 << 30;
        const Symbol = 1 << 31;
        const Ibm869 = 1 << 48;
        const Msdos866 = 1 << 49;
        const Msdos865 = 1 << 50;
        const Arabic864 = 1 << 51;
        const Msdos863 = 1 << 52;
        const Hebrew862 = 1 << 53;
        const Msdos861 = 1 << 54;
        const Msdos860 = 1 << 55;
        const IbmTurkish857 = 1 << 56;
        const IbmCyrillic855 = 1 << 57;
        const Latin2_852 = 1 << 58;
        const Msdos775 = 1 << 59;
        const Greek737 = 1 << 60;
        const Arabic708 = 1 << 61;
        const WeLatin1_850 = 1 << 62;
        const Us437 = 1 << 63;
    }
}
