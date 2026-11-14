// Copyright 2022 the SVG Types Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::stream::Stream;

/// [`paint-order`] property variants.
///
/// [`paint-order`]: https://www.w3.org/TR/SVG2/painting.html#PaintOrder
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(missing_docs)]
pub enum PaintOrderKind {
    Fill,
    Stroke,
    Markers,
}

/// Representation of the [`paint-order`] property.
///
/// [`paint-order`]: https://www.w3.org/TR/SVG2/painting.html#PaintOrder
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PaintOrder {
    /// The order.
    ///
    /// Guarantee to not have duplicates.
    pub order: [PaintOrderKind; 3],
}

impl Default for PaintOrder {
    #[inline]
    fn default() -> Self {
        Self {
            order: [
                PaintOrderKind::Fill,
                PaintOrderKind::Stroke,
                PaintOrderKind::Markers,
            ],
        }
    }
}

impl From<[PaintOrderKind; 3]> for PaintOrder {
    #[inline]
    fn from(order: [PaintOrderKind; 3]) -> Self {
        Self { order }
    }
}

impl core::str::FromStr for PaintOrder {
    type Err = ();

    /// Parses `PaintOrder` from a string.
    ///
    /// Never returns an error and fallbacks to the default value instead.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut order = Self::default().order;

        let mut s = Stream::from(text);
        let mut count = 0;
        while !s.at_end() && count < 3 {
            s.skip_spaces();
            let name = s.consume_ascii_ident();
            s.skip_spaces();
            let name = match name {
                // `normal` is the special value that should short-circuit.
                "normal" => return Ok(Self::default()),
                "fill" => PaintOrderKind::Fill,
                "stroke" => PaintOrderKind::Stroke,
                "markers" => PaintOrderKind::Markers,
                _ => return Ok(Self::default()),
            };

            let Some(index) = order[count..].iter().position(|v| *v == name) else {
                return Ok(Self::default());
            };
            order[count..=count + index].rotate_right(1);
            count += 1;
        }

        s.skip_spaces();
        if !s.at_end() {
            // Any trailing data is an error.
            return Ok(Self::default());
        }

        Ok(Self { order })
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn parse_1() {
        assert_eq!(PaintOrder::from_str("normal").unwrap(), PaintOrder::default());
    }

    #[test]
    fn parse_2() {
        assert_eq!(PaintOrder::from_str("qwe").unwrap(), PaintOrder::default());
    }

    #[test]
    fn parse_3() {
        assert_eq!(PaintOrder::from_str("").unwrap(), PaintOrder::default());
    }

    #[test]
    fn parse_4() {
        assert_eq!(PaintOrder::from_str("stroke qwe").unwrap(), PaintOrder::default());
    }

    #[test]
    fn parse_5() {
        assert_eq!(PaintOrder::from_str("stroke stroke").unwrap(), PaintOrder::default());
    }

    #[test]
    fn parse_6() {
        assert_eq!(PaintOrder::from_str("stroke").unwrap(), PaintOrder::from([
            PaintOrderKind::Stroke, PaintOrderKind::Fill, PaintOrderKind::Markers
        ]));
    }

    #[test]
    fn parse_7() {
        assert_eq!(PaintOrder::from_str("stroke markers").unwrap(), PaintOrder::from([
            PaintOrderKind::Stroke, PaintOrderKind::Markers, PaintOrderKind::Fill
        ]));
    }

    #[test]
    fn parse_8() {
        assert_eq!(PaintOrder::from_str("stroke markers fill").unwrap(), PaintOrder::from([
            PaintOrderKind::Stroke, PaintOrderKind::Markers, PaintOrderKind::Fill
        ]));
    }

    #[test]
    fn parse_9() {
        assert_eq!(PaintOrder::from_str("markers").unwrap(), PaintOrder::from([
            PaintOrderKind::Markers, PaintOrderKind::Fill, PaintOrderKind::Stroke
        ]));
    }

    #[test]
    fn parse_10() {
        assert_eq!(PaintOrder::from_str("  stroke\n").unwrap(), PaintOrder::from([
            PaintOrderKind::Stroke, PaintOrderKind::Fill, PaintOrderKind::Markers
        ]));
    }

    #[test]
    fn parse_11() {
        assert_eq!(PaintOrder::from_str("stroke stroke stroke stroke").unwrap(), PaintOrder::default());
    }

    #[test]
    fn parse_12() {
        assert_eq!(PaintOrder::from_str("stroke stroke markers").unwrap(), PaintOrder::default());
    }

    #[test]
    fn test_all_paint_order_combinations() {
        let tests_combinations = [
            (
                "fill stroke markers",
                [PaintOrderKind::Fill, PaintOrderKind::Stroke, PaintOrderKind::Markers],
            ),
            (
                "fill markers stroke",
                [PaintOrderKind::Fill, PaintOrderKind::Markers, PaintOrderKind::Stroke],
            ),
            (
                "stroke fill markers",
                [PaintOrderKind::Stroke, PaintOrderKind::Fill, PaintOrderKind::Markers],
            ),
            (
                "stroke markers fill",
                [PaintOrderKind::Stroke, PaintOrderKind::Markers, PaintOrderKind::Fill],
            ),
            (
                "markers fill stroke",
                [PaintOrderKind::Markers, PaintOrderKind::Fill, PaintOrderKind::Stroke],
            ),
            (
                "markers stroke fill",
                [PaintOrderKind::Markers, PaintOrderKind::Stroke, PaintOrderKind::Fill],
            )
        ];

        for (str_combination, expected_order) in tests_combinations {
            let actual = PaintOrder::from_str(str_combination).unwrap();
            let expected = PaintOrder::from(expected_order);
            assert_eq!(actual, expected);
        }
    }
}
