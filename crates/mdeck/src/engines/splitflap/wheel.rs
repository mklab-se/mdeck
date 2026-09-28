//! The wheel of characters every flap carries, and how a flap turns
//! through it.

/// Every character a flap carries, in the order the flaps turn. A cell
/// turns forward through this wheel until it shows its character.
pub const WHEEL: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÆØÜÉ0123456789.,:;!?'\"-+/()&%#@*=$€£<>_•…█";

/// A full flap: a solid colour instead of a character (countdown digits,
/// progress bars).
pub const SOLID: char = '█';

/// Position of a character on the wheel, if the flaps carry it.
pub fn wheel_index(c: char) -> Option<usize> {
    WHEEL.chars().position(|w| w == c)
}

/// The character after `c` on the wheel (wrapping round).
pub fn wheel_next(c: char) -> char {
    let n = WHEEL.chars().count();
    let i = wheel_index(c).map_or(0, |i| (i + 1) % n);
    WHEEL.chars().nth(i).unwrap_or(' ')
}

/// The character a flap turns to next on its way to `target`. A solid
/// flap is a flap of its own, one turn from anything.
pub fn step_toward(c: char, target: char) -> char {
    if target == SOLID {
        SOLID
    } else {
        wheel_next(c)
    }
}

/// How many flips turn `from` into `to`.
pub fn wheel_distance(from: char, to: char) -> usize {
    if to == SOLID {
        return usize::from(from != SOLID);
    }
    let n = WHEEL.chars().count();
    match (wheel_index(from), wheel_index(to)) {
        (Some(a), Some(b)) => (b + n - a) % n,
        _ => 0,
    }
}

/// A character as a flap shows it: capitals, typographic marks folded to
/// the plain ones the wheel carries. `None` when the flaps cannot show it.
pub(super) fn flap_chars(c: char) -> Vec<Option<char>> {
    let folded = match c {
        '\u{2018}' | '\u{2019}' | '`' => '\'',
        '\u{201C}' | '\u{201D}' | '\u{00AB}' | '\u{00BB}' => '"',
        '\u{2013}' | '\u{2014}' | '\u{2212}' => '-',
        '\u{00D7}' => 'X',
        '\t' | '\u{00A0}' => ' ',
        c => c,
    };
    folded
        .to_uppercase()
        .map(|u| wheel_index(u).map(|_| u))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheel_turns_forward_and_wraps() {
        assert_eq!(wheel_next(' '), 'A');
        assert_eq!(wheel_next('A'), 'B');
        assert_eq!(wheel_next(SOLID), ' ');
        assert_eq!(wheel_distance(' ', 'C'), 3);
        assert_eq!(wheel_distance('C', ' '), WHEEL.chars().count() - 3);
        assert_eq!(wheel_distance('Q', 'Q'), 0);
        assert_eq!(wheel_distance('Q', SOLID), 1, "solid flaps turn at once");
        assert_eq!(step_toward('Q', SOLID), SOLID);
    }
}
