//! Point clouds and masks: shapes made of points, which particle-style
//! engines form figures, digits and words from.

use std::sync::Arc;

/// A point cloud illustration (a `.mdpc` file): points in the unit square,
/// ordered by importance, so any prefix of them is a fair sketch of the
/// whole (a group of `n` particles takes the first `n`).
///
/// ```
/// use mdeck_sdk::cloud::Cloud;
/// let c = Cloud::new("dot", vec![[0.5, 0.5]], 1.0);
/// assert_eq!(c.points.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Cloud {
    /// The name decks use (`@illustration: <name>`).
    pub name: String,
    /// Points in the unit square (x right, y down), most important first.
    pub points: Vec<[f32; 2]>,
    /// Height over width of the drawing the points came from.
    pub aspect: f32,
}

impl Cloud {
    /// A cloud named `name`.
    ///
    /// See [`Cloud`] for an example.
    pub fn new(name: impl Into<String>, points: Vec<[f32; 2]>, aspect: f32) -> Self {
        Self {
            name: name.into(),
            points,
            aspect,
        }
    }
}

/// Points sampled inside a shape (a countdown digit, the end words), in the
/// unit square of the shape's ink, shuffled so any prefix covers the whole
/// shape. Cloning shares the points.
///
/// Unlike [`Cloud::aspect`], [`Mask::aspect`] is **width over height**: a
/// mask stretched to a box of height `h` is `h * aspect` wide.
///
/// ```
/// use mdeck_sdk::cloud::Mask;
/// let m = Mask::new(vec![[0.0, 0.0], [1.0, 1.0]], 0.6);
/// assert_eq!(m.points.len(), 2);
/// assert_eq!(m.aspect, 0.6);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    /// Points in the unit square.
    pub points: Arc<Vec<[f32; 2]>>,
    /// The shape's width over its height.
    pub aspect: f32,
}

impl Mask {
    /// A mask of `points` with width / height `aspect`.
    ///
    /// See [`Mask`] for an example.
    pub fn new(points: Vec<[f32; 2]>, aspect: f32) -> Self {
        Self {
            points: Arc::new(points),
            aspect,
        }
    }

    /// Whether the mask has no points.
    ///
    /// ```
    /// assert!(mdeck_sdk::cloud::Mask::new(vec![], 1.0).is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_share_points_when_cloned() {
        let m = Mask::new(vec![[0.1, 0.2]; 10], 2.0);
        let c = m.clone();
        assert!(Arc::ptr_eq(&m.points, &c.points));
    }
}
