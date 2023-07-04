use crate::games::go::GO_MAX_AREA;
use static_assertions::const_assert;

const_assert!(GO_MAX_AREA < u16::MAX - 1);

/// More compact version of `Option<u16>` that uses `u16::MAX` as the `None` value.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct OptionU16 {
    value: u16,
}

impl OptionU16 {
    #[allow(non_upper_case_globals)]
    pub const None: OptionU16 = OptionU16 { value: u16::MAX };

    #[allow(non_snake_case)]
    pub fn Some(value: u16) -> Self {
        debug_assert!(value != u16::MAX);
        OptionU16 { value }
    }

    pub fn to_option(self) -> Option<u16> {
        if self.value == u16::MAX {
            None
        } else {
            Some(self.value)
        }
    }

    pub fn from_option(value: Option<u16>) -> Self {
        assert_ne!(value, Some(u16::MAX));
        OptionU16 {
            value: value.unwrap_or(u16::MAX),
        }
    }

    #[must_use]
    pub fn or(self, other: Self) -> Self {
        if self.value == u16::MAX {
            other
        } else {
            self
        }
    }

    #[must_use]
    pub fn map(self, f: impl FnMut(u16) -> u16) -> Self {
        Self::from_option(self.to_option().map(f))
    }

    #[must_use]
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    pub fn is_some(self) -> bool {
        self.value != u16::MAX
    }

    pub fn is_none(self) -> bool {
        self.value == u16::MAX
    }
}

impl Default for OptionU16 {
    fn default() -> Self {
        OptionU16::None
    }
}
