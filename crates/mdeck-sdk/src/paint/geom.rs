//! Points, vectors, rectangles and alignment, in points (logical pixels).

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A position on the slide, in points.
///
/// ```
/// use mdeck_sdk::paint::{Pos2, Vec2};
/// let p = Pos2::new(10.0, 20.0) + Vec2::new(5.0, 0.0);
/// assert_eq!(p, Pos2::new(15.0, 20.0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pos2 {
    /// Horizontal position, growing to the right.
    pub x: f32,
    /// Vertical position, growing downward.
    pub y: f32,
}

/// A displacement or a size, in points.
///
/// ```
/// use mdeck_sdk::paint::Vec2;
/// assert_eq!(Vec2::new(3.0, 4.0).length(), 5.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    /// Horizontal component.
    pub x: f32,
    /// Vertical component.
    pub y: f32,
}

impl Pos2 {
    /// The origin.
    pub const ZERO: Pos2 = Pos2 { x: 0.0, y: 0.0 };

    /// A position at `(x, y)`.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Pos2::new(1.0, 2.0).y, 2.0);
    /// ```
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The vector from the origin to this position.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Vec2};
    /// assert_eq!(Pos2::new(1.0, 2.0).to_vec2(), Vec2::new(1.0, 2.0));
    /// ```
    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Distance to `other`.
    ///
    /// ```
    /// use mdeck_sdk::paint::Pos2;
    /// assert_eq!(Pos2::ZERO.distance(Pos2::new(3.0, 4.0)), 5.0);
    /// ```
    pub fn distance(self, other: Pos2) -> f32 {
        (self - other).length()
    }

    /// The point `t` (0..1) of the way from `self` to `other`.
    ///
    /// ```
    /// use mdeck_sdk::paint::Pos2;
    /// assert_eq!(Pos2::ZERO.lerp(Pos2::new(10.0, 0.0), 0.5), Pos2::new(5.0, 0.0));
    /// ```
    pub fn lerp(self, other: Pos2, t: f32) -> Pos2 {
        Pos2::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    /// A vector `(x, y)`.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Vec2::new(1.0, 2.0).x, 1.0);
    /// ```
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// A vector with both components `v`.
    ///
    /// ```
    /// use mdeck_sdk::paint::Vec2;
    /// assert_eq!(Vec2::splat(2.0), Vec2::new(2.0, 2.0));
    /// ```
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    /// Euclidean length.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Vec2::new(0.0, 2.0).length(), 2.0);
    /// ```
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// This vector scaled to length 1 (zero stays zero).
    ///
    /// ```
    /// use mdeck_sdk::paint::Vec2;
    /// assert_eq!(Vec2::new(0.0, 5.0).normalized(), Vec2::new(0.0, 1.0));
    /// ```
    pub fn normalized(self) -> Vec2 {
        let l = self.length();
        if l > 0.0 { self / l } else { Vec2::ZERO }
    }

    /// The vector turned a quarter clockwise on screen.
    ///
    /// ```
    /// use mdeck_sdk::paint::Vec2;
    /// assert_eq!(Vec2::new(1.0, 0.0).rot90(), Vec2::new(0.0, 1.0));
    /// ```
    pub fn rot90(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }

    /// The angle in radians from the positive x axis.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Vec2::new(1.0, 0.0).angle(), 0.0);
    /// ```
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    /// A unit vector at `angle` radians.
    ///
    /// ```
    /// use mdeck_sdk::paint::Vec2;
    /// assert!((Vec2::angled(0.0).x - 1.0).abs() < 1e-6);
    /// ```
    pub fn angled(angle: f32) -> Vec2 {
        Vec2::new(angle.cos(), angle.sin())
    }
}

impl Add<Vec2> for Pos2 {
    type Output = Pos2;
    fn add(self, v: Vec2) -> Pos2 {
        Pos2::new(self.x + v.x, self.y + v.y)
    }
}
impl AddAssign<Vec2> for Pos2 {
    fn add_assign(&mut self, v: Vec2) {
        *self = *self + v;
    }
}
impl Sub<Vec2> for Pos2 {
    type Output = Pos2;
    fn sub(self, v: Vec2) -> Pos2 {
        Pos2::new(self.x - v.x, self.y - v.y)
    }
}
impl SubAssign<Vec2> for Pos2 {
    fn sub_assign(&mut self, v: Vec2) {
        *self = *self - v;
    }
}
impl Sub for Pos2 {
    type Output = Vec2;
    fn sub(self, o: Pos2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}
impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Vec2) {
        *self = *self + o;
    }
}
impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}
impl SubAssign for Vec2 {
    fn sub_assign(&mut self, o: Vec2) {
        *self = *self - o;
    }
}
impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}
impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f32) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}
impl Mul<Vec2> for f32 {
    type Output = Vec2;
    fn mul(self, v: Vec2) -> Vec2 {
        v * self
    }
}
impl Div<f32> for Vec2 {
    type Output = Vec2;
    fn div(self, k: f32) -> Vec2 {
        Vec2::new(self.x / k, self.y / k)
    }
}

/// An axis-aligned rectangle, in points.
///
/// ```
/// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
/// let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
/// assert_eq!(r.center(), Pos2::new(960.0, 540.0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// The top-left corner.
    pub min: Pos2,
    /// The bottom-right corner.
    pub max: Pos2,
}

impl Rect {
    /// The rectangle with no area at the origin.
    pub const ZERO: Rect = Rect {
        min: Pos2::ZERO,
        max: Pos2::ZERO,
    };

    /// The rectangle between two corners.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect};
    /// let r = Rect::from_min_max(Pos2::ZERO, Pos2::new(2.0, 3.0));
    /// assert_eq!(r.height(), 3.0);
    /// ```
    pub const fn from_min_max(min: Pos2, max: Pos2) -> Self {
        Self { min, max }
    }

    /// The rectangle at `min` of `size`.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::new(1.0, 1.0), Vec2::new(2.0, 2.0));
    /// assert_eq!(r.max, Pos2::new(3.0, 3.0));
    /// ```
    pub fn from_min_size(min: Pos2, size: Vec2) -> Self {
        Self {
            min,
            max: min + size,
        }
    }

    /// The rectangle of `size` centred on `center`.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_center_size(Pos2::ZERO, Vec2::splat(2.0));
    /// assert_eq!(r.min, Pos2::new(-1.0, -1.0));
    /// ```
    pub fn from_center_size(center: Pos2, size: Vec2) -> Self {
        Self {
            min: center - size / 2.0,
            max: center + size / 2.0,
        }
    }

    /// The smallest rectangle holding two points, in any order.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect};
    /// let r = Rect::from_two_pos(Pos2::new(5.0, 0.0), Pos2::new(0.0, 5.0));
    /// assert_eq!(r.min, Pos2::ZERO);
    /// ```
    pub fn from_two_pos(a: Pos2, b: Pos2) -> Self {
        Self {
            min: Pos2::new(a.x.min(b.x), a.y.min(b.y)),
            max: Pos2::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    /// Width.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// assert_eq!(Rect::from_min_size(Pos2::ZERO, Vec2::new(4.0, 1.0)).width(), 4.0);
    /// ```
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// assert_eq!(Rect::from_min_size(Pos2::ZERO, Vec2::new(4.0, 1.0)).height(), 1.0);
    /// ```
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Width and height.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let s = Vec2::new(4.0, 1.0);
    /// assert_eq!(Rect::from_min_size(Pos2::ZERO, s).size(), s);
    /// ```
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// The centre.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(2.0));
    /// assert_eq!(r.center(), Pos2::new(1.0, 1.0));
    /// ```
    pub fn center(&self) -> Pos2 {
        self.min.lerp(self.max, 0.5)
    }

    /// Left edge.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect};
    /// assert_eq!(Rect::from_min_max(Pos2::new(1.0, 2.0), Pos2::new(3.0, 4.0)).left(), 1.0);
    /// ```
    pub fn left(&self) -> f32 {
        self.min.x
    }

    /// Right edge.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect};
    /// assert_eq!(Rect::from_min_max(Pos2::new(1.0, 2.0), Pos2::new(3.0, 4.0)).right(), 3.0);
    /// ```
    pub fn right(&self) -> f32 {
        self.max.x
    }

    /// Top edge.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect};
    /// assert_eq!(Rect::from_min_max(Pos2::new(1.0, 2.0), Pos2::new(3.0, 4.0)).top(), 2.0);
    /// ```
    pub fn top(&self) -> f32 {
        self.min.y
    }

    /// Bottom edge.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect};
    /// assert_eq!(Rect::from_min_max(Pos2::new(1.0, 2.0), Pos2::new(3.0, 4.0)).bottom(), 4.0);
    /// ```
    pub fn bottom(&self) -> f32 {
        self.max.y
    }

    /// Whether `p` lies inside (edges included).
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(2.0));
    /// assert!(r.contains(Pos2::new(1.0, 1.0)));
    /// assert!(!r.contains(Pos2::new(3.0, 1.0)));
    /// ```
    pub fn contains(&self, p: Pos2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Whether the two rectangles overlap.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let a = Rect::from_min_size(Pos2::ZERO, Vec2::splat(2.0));
    /// assert!(a.intersects(Rect::from_min_size(Pos2::new(1.0, 1.0), Vec2::splat(2.0))));
    /// ```
    pub fn intersects(&self, other: Rect) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }

    /// The rectangle grown by `margin` on every side (negative shrinks).
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(2.0)).expand(1.0);
    /// assert_eq!(r.width(), 4.0);
    /// ```
    pub fn expand(&self, margin: f32) -> Rect {
        self.expand2(Vec2::splat(margin))
    }

    /// The rectangle grown by `margin.x` left and right, `margin.y` top and bottom.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(2.0)).expand2(Vec2::new(1.0, 0.0));
    /// assert_eq!(r.size(), Vec2::new(4.0, 2.0));
    /// ```
    pub fn expand2(&self, margin: Vec2) -> Rect {
        Rect {
            min: self.min - margin,
            max: self.max + margin,
        }
    }

    /// The rectangle shrunk by `margin` on every side.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(4.0)).shrink(1.0);
    /// assert_eq!(r.width(), 2.0);
    /// ```
    pub fn shrink(&self, margin: f32) -> Rect {
        self.expand(-margin)
    }

    /// The overlap of two rectangles (zero or negative size when they do not meet).
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let a = Rect::from_min_size(Pos2::ZERO, Vec2::splat(2.0));
    /// let b = Rect::from_min_size(Pos2::new(1.0, 1.0), Vec2::splat(2.0));
    /// assert_eq!(a.intersect(b).size(), Vec2::splat(1.0));
    /// ```
    pub fn intersect(&self, other: Rect) -> Rect {
        Rect {
            min: Pos2::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y)),
            max: Pos2::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y)),
        }
    }

    /// The smallest rectangle holding both.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let a = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0));
    /// let b = Rect::from_min_size(Pos2::new(2.0, 2.0), Vec2::splat(1.0));
    /// assert_eq!(a.union(b).size(), Vec2::splat(3.0));
    /// ```
    pub fn union(&self, other: Rect) -> Rect {
        Rect {
            min: Pos2::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Pos2::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    /// The rectangle moved by `v`.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0)).translate(Vec2::new(5.0, 0.0));
    /// assert_eq!(r.left(), 5.0);
    /// ```
    pub fn translate(&self, v: Vec2) -> Rect {
        Rect {
            min: self.min + v,
            max: self.max + v,
        }
    }

    /// The point at fractions `(u, v)` of the rectangle (0,0 top left, 1,1 bottom right).
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 50.0));
    /// assert_eq!(r.lerp_inside(0.5, 1.0), Pos2::new(50.0, 50.0));
    /// ```
    pub fn lerp_inside(&self, u: f32, v: f32) -> Pos2 {
        Pos2::new(
            self.min.x + self.width() * u,
            self.min.y + self.height() * v,
        )
    }

    /// Where `p` lies as fractions of the rectangle, the inverse of [`Rect::lerp_inside`].
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 50.0));
    /// assert_eq!(r.fraction_of(Pos2::new(25.0, 25.0)), (0.25, 0.5));
    /// ```
    pub fn fraction_of(&self, p: Pos2) -> (f32, f32) {
        (
            (p.x - self.min.x) / self.width().max(f32::EPSILON),
            (p.y - self.min.y) / self.height().max(f32::EPSILON),
        )
    }

    /// The sub-rectangle at fractions `u, v` of size `w, h` (all 0..1 of this one):
    /// how a [`crate::stage::Place`] becomes points.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
    /// assert_eq!(r.sub_rect(0.5, 0.5, 0.5, 0.5).min, Pos2::new(50.0, 50.0));
    /// ```
    pub fn sub_rect(&self, u: f32, v: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(
            self.lerp_inside(u, v),
            Vec2::new(self.width() * w, self.height() * h),
        )
    }

    /// The rectangle `t` (0..1) of the way from `self` to `other`.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// let a = Rect::from_min_size(Pos2::ZERO, Vec2::splat(2.0));
    /// let b = a.translate(Vec2::new(10.0, 0.0));
    /// assert_eq!(a.lerp_towards(b, 0.5).left(), 5.0);
    /// ```
    pub fn lerp_towards(&self, other: Rect, t: f32) -> Rect {
        Rect {
            min: self.min.lerp(other.min, t),
            max: self.max.lerp(other.max, t),
        }
    }

    /// Whether the rectangle has positive width and height.
    ///
    /// ```
    /// use mdeck_sdk::paint::Rect;
    /// assert!(!Rect::ZERO.is_positive());
    /// ```
    pub fn is_positive(&self) -> bool {
        self.width() > 0.0 && self.height() > 0.0
    }
}

/// Alignment along one axis.
///
/// ```
/// use mdeck_sdk::paint::Align;
/// assert_eq!(Align::Center.factor(), 0.5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    /// Left or top.
    Min,
    /// Centre.
    Center,
    /// Right or bottom.
    Max,
}

impl Align {
    /// 0, 0.5 or 1.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::Align::Max.factor(), 1.0);
    /// ```
    pub fn factor(self) -> f32 {
        match self {
            Align::Min => 0.0,
            Align::Center => 0.5,
            Align::Max => 1.0,
        }
    }
}

/// Two-axis alignment: which point of a box sits on the anchor.
///
/// ```
/// use mdeck_sdk::paint::{Align2, Pos2, Rect, Vec2};
/// let r = Align2::CENTER_CENTER.anchor_size(Pos2::ZERO, Vec2::splat(2.0));
/// assert_eq!(r.center(), Pos2::ZERO);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Align2 {
    /// Horizontal alignment.
    pub x: Align,
    /// Vertical alignment.
    pub y: Align,
}

impl Align2 {
    /// Top left on the anchor.
    pub const LEFT_TOP: Align2 = Align2 {
        x: Align::Min,
        y: Align::Min,
    };
    /// Middle of the left edge on the anchor.
    pub const LEFT_CENTER: Align2 = Align2 {
        x: Align::Min,
        y: Align::Center,
    };
    /// Bottom left on the anchor.
    pub const LEFT_BOTTOM: Align2 = Align2 {
        x: Align::Min,
        y: Align::Max,
    };
    /// Middle of the top edge on the anchor.
    pub const CENTER_TOP: Align2 = Align2 {
        x: Align::Center,
        y: Align::Min,
    };
    /// Centre on the anchor.
    pub const CENTER_CENTER: Align2 = Align2 {
        x: Align::Center,
        y: Align::Center,
    };
    /// Middle of the bottom edge on the anchor.
    pub const CENTER_BOTTOM: Align2 = Align2 {
        x: Align::Center,
        y: Align::Max,
    };
    /// Top right on the anchor.
    pub const RIGHT_TOP: Align2 = Align2 {
        x: Align::Max,
        y: Align::Min,
    };
    /// Middle of the right edge on the anchor.
    pub const RIGHT_CENTER: Align2 = Align2 {
        x: Align::Max,
        y: Align::Center,
    };
    /// Bottom right on the anchor.
    pub const RIGHT_BOTTOM: Align2 = Align2 {
        x: Align::Max,
        y: Align::Max,
    };

    /// The box of `size` placed so that this alignment's point sits on `anchor`.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Align2, Pos2, Vec2};
    /// let r = Align2::RIGHT_BOTTOM.anchor_size(Pos2::new(10.0, 10.0), Vec2::splat(4.0));
    /// assert_eq!(r.min, Pos2::new(6.0, 6.0));
    /// ```
    pub fn anchor_size(self, anchor: Pos2, size: Vec2) -> Rect {
        let min = Pos2::new(
            anchor.x - size.x * self.x.factor(),
            anchor.y - size.y * self.y.factor(),
        );
        Rect::from_min_size(min, size)
    }
}

// egui conversions, private to the crate (EXT-24: no egui type in the
// public API, trait impls included).

/// Conversion into the egui type behind an SDK type.
pub(crate) trait ToEgui {
    type Out;
    fn eg(self) -> Self::Out;
}

/// Conversion from an egui type into its SDK type.
pub(crate) trait FromEgui<T> {
    fn sdk(self) -> T;
}

impl ToEgui for Pos2 {
    type Out = egui::Pos2;
    fn eg(self) -> egui::Pos2 {
        egui::pos2(self.x, self.y)
    }
}
impl FromEgui<Pos2> for egui::Pos2 {
    fn sdk(self) -> Pos2 {
        Pos2::new(self.x, self.y)
    }
}
impl ToEgui for Vec2 {
    type Out = egui::Vec2;
    fn eg(self) -> egui::Vec2 {
        egui::vec2(self.x, self.y)
    }
}
impl FromEgui<Vec2> for egui::Vec2 {
    fn sdk(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}
impl ToEgui for Rect {
    type Out = egui::Rect;
    fn eg(self) -> egui::Rect {
        egui::Rect::from_min_max(self.min.eg(), self.max.eg())
    }
}
impl FromEgui<Rect> for egui::Rect {
    fn sdk(self) -> Rect {
        Rect::from_min_max(self.min.sdk(), self.max.sdk())
    }
}
impl ToEgui for Align2 {
    type Out = egui::Align2;
    fn eg(self) -> egui::Align2 {
        let f = |a: Align| match a {
            Align::Min => egui::Align::Min,
            Align::Center => egui::Align::Center,
            Align::Max => egui::Align::Max,
        };
        egui::Align2([f(self.x), f(self.y)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_intersection_and_containment() {
        let a = Rect::from_min_size(Pos2::ZERO, Vec2::new(10.0, 10.0));
        let b = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::splat(5.0));
        assert!(!a.intersects(b));
        assert!(!a.intersect(b).is_positive());
        assert!(a.contains(Pos2::new(10.0, 10.0)), "edges are inside");
    }

    #[test]
    fn egui_round_trip_keeps_values() {
        let r = Rect::from_min_size(Pos2::new(1.5, 2.5), Vec2::new(3.0, 4.0));
        let back: Rect = r.eg().sdk();
        assert_eq!(back, r);
        assert_eq!(Align2::RIGHT_TOP.eg(), egui::Align2::RIGHT_TOP);
    }

    #[test]
    fn fractions_invert_lerp_inside() {
        let r = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(200.0, 100.0));
        let (u, v) = r.fraction_of(r.lerp_inside(0.3, 0.7));
        assert!((u - 0.3).abs() < 1e-5 && (v - 0.7).abs() < 1e-5);
    }
}
