/*!

  Implement four-state logic

*/

use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq)]
/// An enum to represent four-state logic
pub enum Logic {
    /// Logical zero
    False,
    /// Logical one
    True,
    /// Don't care
    X,
    /// High-impedance state
    Z,
}

impl Logic {
    /// Get logic as bool
    pub fn unwrap(self) -> bool {
        match self {
            Logic::True => true,
            Logic::False => false,
            _ => panic!("Logic is not truthy"),
        }
    }

    /// Get logic as bool with failure `msg`
    pub fn expect(self, msg: &str) -> bool {
        match self {
            Logic::True => true,
            Logic::False => false,
            _ => panic!("{}", msg),
        }
    }

    /// Returns [true] if the logic is a don't care
    pub fn is_dont_care(&self) -> bool {
        matches!(self, Logic::X)
    }

    /// Returns the logic as a string
    pub fn as_str(&self) -> &str {
        match self {
            Logic::True => "1'b1",
            Logic::False => "1'b0",
            Logic::X => "1'bx",
            Logic::Z => "1'bz",
        }
    }

    /// Create a four-state logic element from a boolean
    pub fn from_bool(b: bool) -> Logic {
        if b { Logic::True } else { Logic::False }
    }
}

impl fmt::Display for Logic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Logic::True => write!(f, "1'b1"),
            Logic::False => write!(f, "1'b0"),
            Logic::X => write!(f, "1'bx"),
            Logic::Z => write!(f, "1'bz"),
        }
    }
}

impl From<bool> for Logic {
    fn from(value: bool) -> Self {
        if value { Logic::True } else { Logic::False }
    }
}

impl FromStr for Logic {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1'b1" | "1'h1" => Ok(Logic::True),
            "1'b0" | "1'h0" => Ok(Logic::False),
            "1'bx" | "1'hx" => Ok(Logic::X),
            "1'bz" | "1'hz" => Ok(Logic::Z),
            _ => Err(format!("Invalid logic value: {}", s)),
        }
    }
}

/// Create a [Logic::True] instance
pub fn r#true() -> Logic {
    Logic::True
}

/// Create a [Logic::False] instance
pub fn r#false() -> Logic {
    Logic::False
}

/// Create a don't care instance
pub fn dont_care() -> Logic {
    Logic::X
}
