/// This module defines the `InterfaceIndex` struct and implements
/// conversions and formatting for it. The `InterfaceIndex` struct
/// is used to represent an interface index as a `u16` value.
use std::fmt;

/// A struct representing an interface index.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct InterfaceIndex(pub u16);

impl From<InterfaceIndex> for u16 {
    /// Converts an `InterfaceIndex` into a `u16` value.
    ///
    /// # Arguments
    ///
    /// * `index` - An `InterfaceIndex` instance.
    ///
    /// # Returns
    ///
    /// A `u16` value representing the interface index.
    fn from(index: InterfaceIndex) -> u16 {
        index.0
    }
}

/// Implement the `fmt::Display` trait for `InterfaceIndex` to allow formatted output.
impl fmt::Display for InterfaceIndex {
    /// Formats the `InterfaceIndex` for display.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to a `fmt::Formatter`.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating the success or failure of the formatting operation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u16() {
        let index: u16 = 42;
        let interface_index = InterfaceIndex(index);
        assert_eq!(interface_index.0, 42);
    }

    #[test]
    fn test_into_u16() {
        let interface_index = InterfaceIndex(42);
        let index: u16 = interface_index.into();
        assert_eq!(index, 42);
    }

    #[test]
    fn test_display() {
        let interface_index = InterfaceIndex(42);
        assert_eq!(format!("{}", interface_index), "42");
    }
}
