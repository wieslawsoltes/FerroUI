use super::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};

/// The PANOSE classification of a font (the ten bytes stored in the `OS/2` table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Panose {
    data: [u8; 10],
}

impl Panose {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(b0: u8, b1: u8, b2: u8, b3: u8, b4: u8, b5: u8, b6: u8, b7: u8, b8: u8, b9: u8) -> Self {
        Self { data: [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9] }
    }

    pub fn load(reader: &mut BigEndianBinaryReader<'_>) -> Result<Panose, FontTableError> {
        Ok(Panose::new(
            reader.read_byte()?,
            reader.read_byte()?,
            reader.read_byte()?,
            reader.read_byte()?,
            reader.read_byte()?,
            reader.read_byte()?,
            reader.read_byte()?,
            reader.read_byte()?,
            reader.read_byte()?,
            reader.read_byte()?,
        ))
    }

    pub fn family_kind(&self) -> PanoseFamilyKind {
        PanoseFamilyKind(self.data[0])
    }

    // Latin Text properties (when the family kind is LatinText)

    pub fn serif_style(&self) -> PanoseSerifStyle {
        PanoseSerifStyle(self.data[1])
    }

    pub fn weight(&self) -> PanoseWeight {
        PanoseWeight(self.data[2])
    }

    pub fn proportion(&self) -> PanoseProportion {
        PanoseProportion(self.data[3])
    }

    pub fn contrast(&self) -> PanoseContrast {
        PanoseContrast(self.data[4])
    }

    pub fn stroke_variation(&self) -> PanoseStrokeVariation {
        PanoseStrokeVariation(self.data[5])
    }

    pub fn arm_style(&self) -> PanoseArmStyle {
        PanoseArmStyle(self.data[6])
    }

    pub fn letterform(&self) -> PanoseLetterform {
        PanoseLetterform(self.data[7])
    }

    pub fn midline(&self) -> PanoseMidline {
        PanoseMidline(self.data[8])
    }

    pub fn x_height(&self) -> PanoseXHeight {
        PanoseXHeight(self.data[9])
    }
}


/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseFamilyKind(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseFamilyKind {
    pub const Any: PanoseFamilyKind = PanoseFamilyKind(0);
    pub const NoFit: PanoseFamilyKind = PanoseFamilyKind(1);
    pub const LatinText: PanoseFamilyKind = PanoseFamilyKind(2);
    pub const LatinHandWritten: PanoseFamilyKind = PanoseFamilyKind(3);
    pub const LatinDecorative: PanoseFamilyKind = PanoseFamilyKind(4);
    pub const LatinSymbol: PanoseFamilyKind = PanoseFamilyKind(5);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseSerifStyle(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseSerifStyle {
    pub const Any: PanoseSerifStyle = PanoseSerifStyle(0);
    pub const NoFit: PanoseSerifStyle = PanoseSerifStyle(1);
    pub const Cove: PanoseSerifStyle = PanoseSerifStyle(2);
    pub const ObtuseCove: PanoseSerifStyle = PanoseSerifStyle(3);
    pub const SquareCove: PanoseSerifStyle = PanoseSerifStyle(4);
    pub const ObtuseSquareCove: PanoseSerifStyle = PanoseSerifStyle(5);
    pub const Square: PanoseSerifStyle = PanoseSerifStyle(6);
    pub const Thin: PanoseSerifStyle = PanoseSerifStyle(7);
    pub const Oval: PanoseSerifStyle = PanoseSerifStyle(8);
    pub const Exaggerated: PanoseSerifStyle = PanoseSerifStyle(9);
    pub const Triangle: PanoseSerifStyle = PanoseSerifStyle(10);
    pub const NormalSans: PanoseSerifStyle = PanoseSerifStyle(11);
    pub const ObtuseSans: PanoseSerifStyle = PanoseSerifStyle(12);
    pub const PerpendicularSans: PanoseSerifStyle = PanoseSerifStyle(13);
    pub const Flared: PanoseSerifStyle = PanoseSerifStyle(14);
    pub const Rounded: PanoseSerifStyle = PanoseSerifStyle(15);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseWeight(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseWeight {
    pub const Any: PanoseWeight = PanoseWeight(0);
    pub const NoFit: PanoseWeight = PanoseWeight(1);
    pub const VeryLight: PanoseWeight = PanoseWeight(2);
    pub const Light: PanoseWeight = PanoseWeight(3);
    pub const Thin: PanoseWeight = PanoseWeight(4);
    pub const Book: PanoseWeight = PanoseWeight(5);
    pub const Medium: PanoseWeight = PanoseWeight(6);
    pub const Demi: PanoseWeight = PanoseWeight(7);
    pub const Bold: PanoseWeight = PanoseWeight(8);
    pub const Heavy: PanoseWeight = PanoseWeight(9);
    pub const Black: PanoseWeight = PanoseWeight(10);
    pub const ExtraBlack: PanoseWeight = PanoseWeight(11);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseProportion(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseProportion {
    pub const Any: PanoseProportion = PanoseProportion(0);
    pub const NoFit: PanoseProportion = PanoseProportion(1);
    pub const OldStyle: PanoseProportion = PanoseProportion(2);
    pub const Modern: PanoseProportion = PanoseProportion(3);
    pub const EvenWidth: PanoseProportion = PanoseProportion(4);
    pub const Extended: PanoseProportion = PanoseProportion(5);
    pub const Condensed: PanoseProportion = PanoseProportion(6);
    pub const VeryExtended: PanoseProportion = PanoseProportion(7);
    pub const VeryCondensed: PanoseProportion = PanoseProportion(8);
    pub const Monospaced: PanoseProportion = PanoseProportion(9);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseContrast(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseContrast {
    pub const Any: PanoseContrast = PanoseContrast(0);
    pub const NoFit: PanoseContrast = PanoseContrast(1);
    pub const None: PanoseContrast = PanoseContrast(2);
    pub const VeryLow: PanoseContrast = PanoseContrast(3);
    pub const Low: PanoseContrast = PanoseContrast(4);
    pub const MediumLow: PanoseContrast = PanoseContrast(5);
    pub const Medium: PanoseContrast = PanoseContrast(6);
    pub const MediumHigh: PanoseContrast = PanoseContrast(7);
    pub const High: PanoseContrast = PanoseContrast(8);
    pub const VeryHigh: PanoseContrast = PanoseContrast(9);
    pub const HorizontalLow: PanoseContrast = PanoseContrast(10);
    pub const HorizontalMedium: PanoseContrast = PanoseContrast(11);
    pub const HorizontalHigh: PanoseContrast = PanoseContrast(12);
    pub const Broken: PanoseContrast = PanoseContrast(13);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseStrokeVariation(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseStrokeVariation {
    pub const Any: PanoseStrokeVariation = PanoseStrokeVariation(0);
    pub const NoFit: PanoseStrokeVariation = PanoseStrokeVariation(1);
    pub const NoVariation: PanoseStrokeVariation = PanoseStrokeVariation(2);
    pub const GradualDiagonal: PanoseStrokeVariation = PanoseStrokeVariation(3);
    pub const GradualTransitional: PanoseStrokeVariation = PanoseStrokeVariation(4);
    pub const GradualVertical: PanoseStrokeVariation = PanoseStrokeVariation(5);
    pub const GradualHorizontal: PanoseStrokeVariation = PanoseStrokeVariation(6);
    pub const RapidVertical: PanoseStrokeVariation = PanoseStrokeVariation(7);
    pub const RapidHorizontal: PanoseStrokeVariation = PanoseStrokeVariation(8);
    pub const InstantVertical: PanoseStrokeVariation = PanoseStrokeVariation(9);
    pub const InstantHorizontal: PanoseStrokeVariation = PanoseStrokeVariation(10);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseArmStyle(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseArmStyle {
    pub const Any: PanoseArmStyle = PanoseArmStyle(0);
    pub const NoFit: PanoseArmStyle = PanoseArmStyle(1);
    pub const StraightArmsHorizontal: PanoseArmStyle = PanoseArmStyle(2);
    pub const StraightArmsWedge: PanoseArmStyle = PanoseArmStyle(3);
    pub const StraightArmsVertical: PanoseArmStyle = PanoseArmStyle(4);
    pub const StraightArmsSingleSerif: PanoseArmStyle = PanoseArmStyle(5);
    pub const StraightArmsDoubleSerif: PanoseArmStyle = PanoseArmStyle(6);
    pub const NonStraightArmsHorizontal: PanoseArmStyle = PanoseArmStyle(7);
    pub const NonStraightArmsWedge: PanoseArmStyle = PanoseArmStyle(8);
    pub const NonStraightArmsVertical: PanoseArmStyle = PanoseArmStyle(9);
    pub const NonStraightArmsSingleSerif: PanoseArmStyle = PanoseArmStyle(10);
    pub const NonStraightArmsDoubleSerif: PanoseArmStyle = PanoseArmStyle(11);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseLetterform(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseLetterform {
    pub const Any: PanoseLetterform = PanoseLetterform(0);
    pub const NoFit: PanoseLetterform = PanoseLetterform(1);
    pub const NormalContact: PanoseLetterform = PanoseLetterform(2);
    pub const NormalWeighted: PanoseLetterform = PanoseLetterform(3);
    pub const NormalBoxed: PanoseLetterform = PanoseLetterform(4);
    pub const NormalFlattened: PanoseLetterform = PanoseLetterform(5);
    pub const NormalRounded: PanoseLetterform = PanoseLetterform(6);
    pub const NormalOffCenter: PanoseLetterform = PanoseLetterform(7);
    pub const NormalSquare: PanoseLetterform = PanoseLetterform(8);
    pub const ObliqueContact: PanoseLetterform = PanoseLetterform(9);
    pub const ObliqueWeighted: PanoseLetterform = PanoseLetterform(10);
    pub const ObliqueBoxed: PanoseLetterform = PanoseLetterform(11);
    pub const ObliqueFlattened: PanoseLetterform = PanoseLetterform(12);
    pub const ObliqueRounded: PanoseLetterform = PanoseLetterform(13);
    pub const ObliqueOffCenter: PanoseLetterform = PanoseLetterform(14);
    pub const ObliqueSquare: PanoseLetterform = PanoseLetterform(15);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseMidline(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseMidline {
    pub const Any: PanoseMidline = PanoseMidline(0);
    pub const NoFit: PanoseMidline = PanoseMidline(1);
    pub const StandardTrimmed: PanoseMidline = PanoseMidline(2);
    pub const StandardPointed: PanoseMidline = PanoseMidline(3);
    pub const StandardSerifed: PanoseMidline = PanoseMidline(4);
    pub const HighTrimmed: PanoseMidline = PanoseMidline(5);
    pub const HighPointed: PanoseMidline = PanoseMidline(6);
    pub const HighSerifed: PanoseMidline = PanoseMidline(7);
    pub const ConstantTrimmed: PanoseMidline = PanoseMidline(8);
    pub const ConstantPointed: PanoseMidline = PanoseMidline(9);
    pub const ConstantSerifed: PanoseMidline = PanoseMidline(10);
    pub const LowTrimmed: PanoseMidline = PanoseMidline(11);
    pub const LowPointed: PanoseMidline = PanoseMidline(12);
    pub const LowSerifed: PanoseMidline = PanoseMidline(13);
}

/// A PANOSE digit; any byte value is representable, the named values are
/// associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PanoseXHeight(pub u8);

#[allow(non_upper_case_globals)]
impl PanoseXHeight {
    pub const Any: PanoseXHeight = PanoseXHeight(0);
    pub const NoFit: PanoseXHeight = PanoseXHeight(1);
    pub const ConstantSmall: PanoseXHeight = PanoseXHeight(2);
    pub const ConstantStandard: PanoseXHeight = PanoseXHeight(3);
    pub const ConstantLarge: PanoseXHeight = PanoseXHeight(4);
    pub const DuckingSmall: PanoseXHeight = PanoseXHeight(5);
    pub const DuckingStandard: PanoseXHeight = PanoseXHeight(6);
    pub const DuckingLarge: PanoseXHeight = PanoseXHeight(7);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_reads_ten_bytes() {
        let data = [2u8, 11, 5, 3, 0, 0, 0, 2, 0, 4];
        let mut reader = BigEndianBinaryReader::new(&data);

        let panose = Panose::load(&mut reader).unwrap();

        assert_eq!(reader.position(), 10);
        assert_eq!(panose.family_kind(), PanoseFamilyKind::LatinText);
        assert_eq!(panose.serif_style(), PanoseSerifStyle::NormalSans);
        assert_eq!(panose.weight(), PanoseWeight::Book);
        assert_eq!(panose.proportion(), PanoseProportion::Modern);
        assert_eq!(panose.contrast(), PanoseContrast::Any);
        assert_eq!(panose.stroke_variation(), PanoseStrokeVariation::Any);
        assert_eq!(panose.arm_style(), PanoseArmStyle::Any);
        assert_eq!(panose.letterform(), PanoseLetterform::NormalContact);
        assert_eq!(panose.midline(), PanoseMidline::Any);
        assert_eq!(panose.x_height(), PanoseXHeight::ConstantLarge);
    }

    #[test]
    fn load_fails_on_truncated_data() {
        let data = [2u8; 9];
        let mut reader = BigEndianBinaryReader::new(&data);

        assert!(Panose::load(&mut reader).is_err());
    }
}
