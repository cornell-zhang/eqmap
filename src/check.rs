/*!

  Struct to check equivalence of two models.

*/

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
/// An enum to help facilitate model checking
pub enum Check {
    /// Two models are proven equivalent
    Equiv,
    /// Two models are proven not equivalent
    NotEquiv,
    /// Inconclusive result of checking
    Inconclusive,
}

impl Check {
    /// Unwrap a conclusive result
    pub fn unwrap(self) -> bool {
        match self {
            Check::Equiv => true,
            Check::NotEquiv => false,
            Check::Inconclusive => panic!("Inconclusive"),
        }
    }

    /// Expect a conclusive result
    pub fn expect(self, msg: &str) -> bool {
        match self {
            Check::Equiv => true,
            Check::NotEquiv => false,
            Check::Inconclusive => panic!("{}", msg),
        }
    }

    /// Returns `true`` if the check proved equivalence
    pub fn is_equiv(&self) -> bool {
        matches!(self, Check::Equiv)
    }

    /// Returns `true` if the check proved inequivalence
    pub fn is_not_equiv(&self) -> bool {
        matches!(self, Check::NotEquiv)
    }

    /// Returns `true` if the check is inconclusive
    pub fn is_inconclusive(&self) -> bool {
        matches!(self, Check::Inconclusive)
    }

    /// Returns `true` if the check is conclusive
    pub fn is_conclusive(&self) -> bool {
        !self.is_inconclusive()
    }
}

impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Create an equivalent instance
pub fn equivalent() -> Check {
    Check::Equiv
}

/// Create an inequivalent instance
pub fn not_equivalent() -> Check {
    Check::NotEquiv
}

/// Create an inconclusive instance
pub fn inconclusive() -> Check {
    Check::Inconclusive
}
