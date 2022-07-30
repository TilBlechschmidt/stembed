#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyPosition {
    Left(KeyColumn, KeyRow),
    Right(KeyColumn, KeyRow),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyRow {
    Above = 0,
    Top = 1,
    Bottom = 2,
    Below = 3,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyColumn {
    Pinky = 0,
    Ring = 1,
    Middle = 2,
    Index = 3,
    Thumb = 4,

    ExtraLeading = 5,
    ExtraTrailing = 6,
}

const fn equal(lhs: &[u8], rhs: &[u8]) -> bool {
    if lhs.len() != rhs.len() {
        return false;
    }
    let mut i = 0;
    while i < lhs.len() {
        if lhs[i] != rhs[i] {
            return false;
        }
        i += 1;
    }
    true
}

macro_rules! const_string_match {
    ( $expected:expr, $( $actual:literal => $output:expr ),* ) => {
        {
            let expected_bytes = $expected.as_bytes();
            $(
                if equal(expected_bytes, $actual.as_bytes()) {
                    return $output;
                }
            )*
        }
    };
}

impl KeyPosition {
    pub const fn as_str(&self) -> &'static str {
        use KeyColumn::*;
        use KeyPosition::*;
        use KeyRow::*;

        match self {
            Left(Pinky, Above) => "LP0",
            Left(Pinky, Top) => "LP1",
            Left(Pinky, Bottom) => "LP2",
            Left(Pinky, Below) => "LP3",

            Left(Ring, Above) => "LR0",
            Left(Ring, Top) => "LR1",
            Left(Ring, Bottom) => "LR2",
            Left(Ring, Below) => "LR3",

            Left(Middle, Above) => "LM0",
            Left(Middle, Top) => "LM1",
            Left(Middle, Bottom) => "LM2",
            Left(Middle, Below) => "LM3",

            Left(Index, Above) => "LI0",
            Left(Index, Top) => "LI1",
            Left(Index, Bottom) => "LI2",
            Left(Index, Below) => "LI3",

            Left(Thumb, Above) => "LT0",
            Left(Thumb, Top) => "LT1",
            Left(Thumb, Bottom) => "LT2",
            Left(Thumb, Below) => "LT3",

            Left(ExtraLeading, Above) => "LEL0",
            Left(ExtraLeading, Top) => "LEL1",
            Left(ExtraLeading, Bottom) => "LEL2",
            Left(ExtraLeading, Below) => "LEL3",

            Left(ExtraTrailing, Above) => "LET0",
            Left(ExtraTrailing, Top) => "LET1",
            Left(ExtraTrailing, Bottom) => "LET2",
            Left(ExtraTrailing, Below) => "LET3",

            Right(Pinky, Above) => "RP0",
            Right(Pinky, Top) => "RP1",
            Right(Pinky, Bottom) => "RP2",
            Right(Pinky, Below) => "RP3",

            Right(Ring, Above) => "RR0",
            Right(Ring, Top) => "RR1",
            Right(Ring, Bottom) => "RR2",
            Right(Ring, Below) => "RR3",

            Right(Middle, Above) => "RM0",
            Right(Middle, Top) => "RM1",
            Right(Middle, Bottom) => "RM2",
            Right(Middle, Below) => "RM3",

            Right(Index, Above) => "RI0",
            Right(Index, Top) => "RI1",
            Right(Index, Bottom) => "RI2",
            Right(Index, Below) => "RI3",
            Right(Thumb, Above) => "RT0",
            Right(Thumb, Top) => "RT1",
            Right(Thumb, Bottom) => "RT2",
            Right(Thumb, Below) => "RT3",

            Right(ExtraLeading, Above) => "REL0",
            Right(ExtraLeading, Top) => "REL1",
            Right(ExtraLeading, Bottom) => "REL2",
            Right(ExtraLeading, Below) => "REL3",

            Right(ExtraTrailing, Above) => "RET0",
            Right(ExtraTrailing, Top) => "RET1",
            Right(ExtraTrailing, Bottom) => "RET2",
            Right(ExtraTrailing, Below) => "RET3",
        }
    }

    pub const fn from(string: &'static str) -> Option<Self> {
        use KeyColumn::*;
        use KeyPosition::*;
        use KeyRow::*;

        const_string_match!(string,
            "---" => None,
            "----" => None,

            "LP0" => Some(Left(Pinky, Above)),
            "LP1" => Some(Left(Pinky, Top)),
            "LP2" => Some(Left(Pinky, Bottom)),
            "LP3" => Some(Left(Pinky, Below)),

            "LR0" => Some(Left(Ring, Above)),
            "LR1" => Some(Left(Ring, Top)),
            "LR2" => Some(Left(Ring, Bottom)),
            "LR3" => Some(Left(Ring, Below)),

            "LM0" => Some(Left(Middle, Above)),
            "LM1" => Some(Left(Middle, Top)),
            "LM2" => Some(Left(Middle, Bottom)),
            "LM3" => Some(Left(Middle, Below)),

            "LI0" => Some(Left(Index, Above)),
            "LI1" => Some(Left(Index, Top)),
            "LI2" => Some(Left(Index, Bottom)),
            "LI3" => Some(Left(Index, Below)),

            "LT0" => Some(Left(Thumb, Above)),
            "LT1" => Some(Left(Thumb, Top)),
            "LT2" => Some(Left(Thumb, Bottom)),
            "LT3" => Some(Left(Thumb, Below)),

            "LEL0" => Some(Left(ExtraLeading, Above)),
            "LEL1" => Some(Left(ExtraLeading, Top)),
            "LEL2" => Some(Left(ExtraLeading, Bottom)),
            "LEL3" => Some(Left(ExtraLeading, Below)),

            "LET0" => Some(Left(ExtraTrailing, Above)),
            "LET1" => Some(Left(ExtraTrailing, Top)),
            "LET2" => Some(Left(ExtraTrailing, Bottom)),
            "LET3" => Some(Left(ExtraTrailing, Below)),

            "RP0" => Some(Right(Pinky, Above)),
            "RP1" => Some(Right(Pinky, Top)),
            "RP2" => Some(Right(Pinky, Bottom)),
            "RP3" => Some(Right(Pinky, Below)),

            "RR0" => Some(Right(Ring, Above)),
            "RR1" => Some(Right(Ring, Top)),
            "RR2" => Some(Right(Ring, Bottom)),
            "RR3" => Some(Right(Ring, Below)),

            "RM0" => Some(Right(Middle, Above)),
            "RM1" => Some(Right(Middle, Top)),
            "RM2" => Some(Right(Middle, Bottom)),
            "RM3" => Some(Right(Middle, Below)),

            "RI0" => Some(Right(Index, Above)),
            "RI1" => Some(Right(Index, Top)),
            "RI2" => Some(Right(Index, Bottom)),
            "RI3" => Some(Right(Index, Below)),

            "RT0" => Some(Right(Thumb, Above)),
            "RT1" => Some(Right(Thumb, Top)),
            "RT2" => Some(Right(Thumb, Bottom)),
            "RT3" => Some(Right(Thumb, Below)),

            "REL0" => Some(Right(ExtraLeading, Above)),
            "REL1" => Some(Right(ExtraLeading, Top)),
            "REL2" => Some(Right(ExtraLeading, Bottom)),
            "REL3" => Some(Right(ExtraLeading, Below)),

            "RET0" => Some(Right(ExtraTrailing, Above)),
            "RET1" => Some(Right(ExtraTrailing, Top)),
            "RET2" => Some(Right(ExtraTrailing, Bottom)),
            "RET3" => Some(Right(ExtraTrailing, Below))
        );

        panic!("Unknown key position");
    }
}
