//! The [`Registry`]: every engine, visual, design set, transition, theme and
//! point cloud mdeck knows, by name. mdeck fills it with its built-ins and
//! then with each extension's.

use std::collections::BTreeMap;
use std::fmt;

use crate::design::DesignSet;
use crate::engine::EngineDef;
use crate::transition::Transition;
use crate::visual::Visual;

/// Two registrations under one name.
///
/// ```
/// use mdeck_sdk::registry::Registry;
/// let mut r = Registry::new();
/// r.theme("led", "name: led\n").unwrap();
/// r.set_origin("my-pack");
/// let e = r.theme("led", "name: led\n").unwrap_err();
/// assert_eq!(e.to_string(), "theme `led` is registered twice: by mdeck and by my-pack");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RegistryError {
    /// What was registered (`engine`, `visual`, `design set`, `transition`,
    /// `theme`, `point cloud`).
    pub kind: &'static str,
    /// The name both used.
    pub name: String,
    /// Who registered it first.
    pub first: String,
    /// Who tried to register it again.
    pub second: String,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} `{}` is registered twice: by {} and by {}",
            self.kind, self.name, self.first, self.second
        )
    }
}

impl std::error::Error for RegistryError {}

/// One registration: what kind of thing, its name and who registered it.
///
/// See [`Registry::registrations`] for an example.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Registration<'a> {
    /// `engine`, `visual`, `design set`, `transition`, `theme`, `point cloud`
    /// or `font`.
    pub kind: &'static str,
    /// The name it is registered under.
    pub name: &'a str,
    /// `mdeck` for the built-ins, else the origin set with
    /// [`Registry::set_origin`].
    pub origin: &'a str,
}

/// An extension's entry point: register everything it brings.
///
/// ```
/// use mdeck_sdk::registry::{Register, Registry, RegistryError};
/// fn register(r: &mut Registry) -> Result<(), RegistryError> {
///     r.theme("midnight", "name: midnight\n")
/// }
/// let entry: Register = register;
/// let mut r = Registry::new();
/// r.set_origin("my-pack");
/// entry(&mut r).unwrap();
/// assert!(r.theme_source("midnight").is_some());
/// ```
pub type Register = fn(&mut Registry) -> Result<(), RegistryError>;

struct Entry<T> {
    origin: String,
    item: T,
}

/// Everything registered, by name. Names are unique per kind; registering
/// a name twice is an error naming both origins.
///
/// ```
/// use mdeck_sdk::registry::Registry;
/// let mut r = Registry::new();
/// r.theme("dusk", "name: dusk\n").unwrap();
/// r.set_origin("my-pack");
/// let err = r.theme("dusk", "name: dusk\n").unwrap_err();
/// assert_eq!(err.first, "mdeck");
/// assert_eq!(err.second, "my-pack");
/// ```
pub struct Registry {
    origin: String,
    engines: BTreeMap<String, Entry<&'static EngineDef>>,
    visuals: BTreeMap<String, Entry<Box<dyn Visual>>>,
    design_sets: BTreeMap<String, Entry<Box<dyn DesignSet>>>,
    transitions: BTreeMap<String, Entry<Box<dyn Transition>>>,
    themes: BTreeMap<String, Entry<&'static str>>,
    point_clouds: BTreeMap<String, Entry<&'static [u8]>>,
    fonts: BTreeMap<String, Entry<&'static [u8]>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field("engines", &self.engines.keys().collect::<Vec<_>>())
            .field("visuals", &self.visuals.keys().collect::<Vec<_>>())
            .field("design_sets", &self.design_sets.keys().collect::<Vec<_>>())
            .field("transitions", &self.transitions.keys().collect::<Vec<_>>())
            .field("themes", &self.themes.keys().collect::<Vec<_>>())
            .field(
                "point_clouds",
                &self.point_clouds.keys().collect::<Vec<_>>(),
            )
            .field("fonts", &self.fonts.keys().collect::<Vec<_>>())
            .finish()
    }
}

fn insert<T>(
    map: &mut BTreeMap<String, Entry<T>>,
    kind: &'static str,
    name: &str,
    origin: &str,
    item: T,
) -> Result<(), RegistryError> {
    if let Some(existing) = map.get(name) {
        return Err(RegistryError {
            kind,
            name: name.to_owned(),
            first: existing.origin.clone(),
            second: origin.to_owned(),
        });
    }
    map.insert(
        name.to_owned(),
        Entry {
            origin: origin.to_owned(),
            item,
        },
    );
    Ok(())
}

impl Registry {
    /// An empty registry; registrations come from `mdeck` until
    /// [`Registry::set_origin`] says otherwise.
    ///
    /// See [`Registry`] for an example.
    pub fn new() -> Self {
        Self {
            origin: "mdeck".to_owned(),
            engines: BTreeMap::new(),
            visuals: BTreeMap::new(),
            design_sets: BTreeMap::new(),
            transitions: BTreeMap::new(),
            themes: BTreeMap::new(),
            point_clouds: BTreeMap::new(),
            fonts: BTreeMap::new(),
        }
    }

    /// Name who registers what follows (an extension's crate or pack name),
    /// for duplicate-name errors.
    ///
    /// See [`Registry`] for an example.
    pub fn set_origin(&mut self, origin: &str) {
        self.origin = origin.to_owned();
    }

    /// Register an engine under [`EngineDef::name`].
    ///
    /// See [`crate::engine::EngineDef`] for defining one.
    pub fn engine(&mut self, def: &'static EngineDef) -> Result<(), RegistryError> {
        insert(&mut self.engines, "engine", def.name, &self.origin, def)
    }

    /// Register a visual under its [`Visual::tag`].
    ///
    /// See [`crate::visual::Visual`] for defining one.
    pub fn visual(&mut self, visual: Box<dyn Visual>) -> Result<(), RegistryError> {
        let tag = visual.tag().to_owned();
        insert(&mut self.visuals, "visual", &tag, &self.origin, visual)
    }

    /// Register a design set under its [`DesignSet::name`].
    ///
    /// See [`crate::design::DesignSet`] for defining one.
    pub fn design_set(&mut self, set: Box<dyn DesignSet>) -> Result<(), RegistryError> {
        let name = set.name().to_owned();
        insert(
            &mut self.design_sets,
            "design set",
            &name,
            &self.origin,
            set,
        )
    }

    /// Register a transition under its [`Transition::name`].
    ///
    /// See [`crate::transition::Transition`] for defining one.
    pub fn transition(&mut self, transition: Box<dyn Transition>) -> Result<(), RegistryError> {
        let name = transition.name().to_owned();
        insert(
            &mut self.transitions,
            "transition",
            &name,
            &self.origin,
            transition,
        )
    }

    /// Register an embedded theme: `yaml` is the theme file's text.
    ///
    /// See [`Register`] for an example.
    pub fn theme(&mut self, name: &str, yaml: &'static str) -> Result<(), RegistryError> {
        insert(&mut self.themes, "theme", name, &self.origin, yaml)
    }

    /// Register an embedded point cloud: `bytes` is the `.mdpc` file.
    ///
    /// ```
    /// let mut r = mdeck_sdk::registry::Registry::new();
    /// r.point_cloud("owl", b"mdpc").unwrap();
    /// assert_eq!(r.point_cloud_bytes("owl"), Some(&b"mdpc"[..]));
    /// ```
    pub fn point_cloud(&mut self, name: &str, bytes: &'static [u8]) -> Result<(), RegistryError> {
        insert(
            &mut self.point_clouds,
            "point cloud",
            name,
            &self.origin,
            bytes,
        )
    }

    /// Register an embedded font file: `file` is the name an embedded theme
    /// writes in its `fonts:` block (`AcmeSans-Regular.ttf`), `bytes` the
    /// `.ttf` or `.otf` file. This is how a crate's own themes use the
    /// company's fonts without a folder on disk.
    ///
    /// ```
    /// let mut r = mdeck_sdk::registry::Registry::new();
    /// r.font("AcmeSans-Regular.ttf", b"font bytes").unwrap();
    /// // The theme then says: fonts: { body: AcmeSans-Regular.ttf }
    /// assert_eq!(r.font_bytes("AcmeSans-Regular.ttf"), Some(&b"font bytes"[..]));
    /// assert!(r.font_bytes("Other.ttf").is_none());
    /// ```
    pub fn font(&mut self, file: &str, bytes: &'static [u8]) -> Result<(), RegistryError> {
        insert(&mut self.fonts, "font", file, &self.origin, bytes)
    }

    /// The bytes of the embedded font file `file`.
    ///
    /// See [`Registry::font`] for an example.
    pub fn font_bytes(&self, file: &str) -> Option<&'static [u8]> {
        self.fonts.get(file).map(|e| e.item)
    }

    /// The engine named `name`.
    ///
    /// ```
    /// assert!(mdeck_sdk::registry::Registry::new().engine_def("led").is_none());
    /// ```
    pub fn engine_def(&self, name: &str) -> Option<&'static EngineDef> {
        self.engines.get(name).map(|e| e.item)
    }

    /// Every engine, by name.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::registry::Registry::new().engines().count(), 0);
    /// ```
    pub fn engines(&self) -> impl Iterator<Item = &'static EngineDef> + '_ {
        self.engines.values().map(|e| e.item)
    }

    /// The visual with fence tag `tag` (without `@`).
    ///
    /// ```
    /// assert!(mdeck_sdk::registry::Registry::new().visual_for("meter").is_none());
    /// ```
    pub fn visual_for(&self, tag: &str) -> Option<&dyn Visual> {
        self.visuals.get(tag).map(|e| e.item.as_ref())
    }

    /// Every visual, by tag.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::registry::Registry::new().visuals().count(), 0);
    /// ```
    pub fn visuals(&self) -> impl Iterator<Item = &dyn Visual> + '_ {
        self.visuals.values().map(|e| e.item.as_ref())
    }

    /// The design set named `name`.
    ///
    /// ```
    /// assert!(mdeck_sdk::registry::Registry::new().design_set_for("board").is_none());
    /// ```
    pub fn design_set_for(&self, name: &str) -> Option<&dyn DesignSet> {
        self.design_sets.get(name).map(|e| e.item.as_ref())
    }

    /// Every design set, by name.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::registry::Registry::new().design_sets().count(), 0);
    /// ```
    pub fn design_sets(&self) -> impl Iterator<Item = &dyn DesignSet> + '_ {
        self.design_sets.values().map(|e| e.item.as_ref())
    }

    /// The transition named `name`.
    ///
    /// ```
    /// assert!(mdeck_sdk::registry::Registry::new().transition_for("drop").is_none());
    /// ```
    pub fn transition_for(&self, name: &str) -> Option<&dyn Transition> {
        self.transitions.get(name).map(|e| e.item.as_ref())
    }

    /// Every transition, by name.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::registry::Registry::new().transitions().count(), 0);
    /// ```
    pub fn transitions(&self) -> impl Iterator<Item = &dyn Transition> + '_ {
        self.transitions.values().map(|e| e.item.as_ref())
    }

    /// The text of the embedded theme `name`.
    ///
    /// See [`Register`] for an example.
    pub fn theme_source(&self, name: &str) -> Option<&'static str> {
        self.themes.get(name).map(|e| e.item)
    }

    /// The names of every embedded theme.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::registry::Registry::new().theme_names().count(), 0);
    /// ```
    pub fn theme_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.themes.keys().map(String::as_str)
    }

    /// Everything registered, kind by kind and by name within a kind, with
    /// who registered it (`mdeck extensions list`).
    ///
    /// ```
    /// let mut r = mdeck_sdk::registry::Registry::new();
    /// r.set_origin("acme");
    /// r.theme("dusk", "name: dusk\n").unwrap();
    /// let all: Vec<_> = r.registrations().collect();
    /// assert_eq!((all[0].kind, all[0].name, all[0].origin), ("theme", "dusk", "acme"));
    /// ```
    pub fn registrations(&self) -> impl Iterator<Item = Registration<'_>> + '_ {
        fn of<'a, T>(
            kind: &'static str,
            map: &'a BTreeMap<String, Entry<T>>,
        ) -> impl Iterator<Item = Registration<'a>> + 'a {
            map.iter().map(move |(name, e)| Registration {
                kind,
                name,
                origin: &e.origin,
            })
        }
        of("engine", &self.engines)
            .chain(of("visual", &self.visuals))
            .chain(of("design set", &self.design_sets))
            .chain(of("transition", &self.transitions))
            .chain(of("theme", &self.themes))
            .chain(of("point cloud", &self.point_clouds))
            .chain(of("font", &self.fonts))
    }

    /// The bytes of the embedded point cloud `name`.
    ///
    /// See [`Registry::point_cloud`] for an example.
    pub fn point_cloud_bytes(&self, name: &str) -> Option<&'static [u8]> {
        self.point_clouds.get(name).map(|e| e.item)
    }

    /// The names of every embedded point cloud.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::registry::Registry::new().point_cloud_names().count(), 0);
    /// ```
    pub fn point_cloud_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.point_clouds.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Slide;
    use crate::design::DesignCx;
    use crate::engine::Engine;
    use crate::paint::{Painter, Rect};
    use crate::stage::{Frame, Stage};
    use crate::transition::SideLook;
    use crate::visual::VisualCx;

    struct Nop;
    impl Engine for Nop {
        fn update(&mut self, _: &Frame, _: &Stage) {}
        fn paint(&mut self, _: &mut Painter, _: &Frame, _: &Stage) {}
    }
    static NOP: EngineDef = EngineDef::new("nop", "", |_| Box::new(Nop));

    struct V;
    impl Visual for V {
        fn tag(&self) -> &str {
            "v"
        }
        fn summary(&self) -> &str {
            ""
        }
        fn draw(&self, _: &mut VisualCx, _: &str, _: Rect, _: usize) -> f32 {
            0.0
        }
    }

    struct D;
    impl DesignSet for D {
        fn name(&self) -> &str {
            "d"
        }
        fn render(&self, _: &mut DesignCx, _: &Slide, _: Rect) {}
    }

    struct T;
    impl Transition for T {
        fn name(&self) -> &str {
            "t"
        }
        fn summary(&self) -> &str {
            ""
        }
        fn look(&self, _: f32, _: bool, _: Rect) -> (SideLook, SideLook) {
            (SideLook::HIDDEN, SideLook::SHOWN)
        }
    }

    #[test]
    fn every_kind_registers_and_rejects_duplicates_naming_both() {
        let mut r = Registry::new();
        r.engine(&NOP).unwrap();
        r.visual(Box::new(V)).unwrap();
        r.design_set(Box::new(D)).unwrap();
        r.transition(Box::new(T)).unwrap();
        r.point_cloud("c", b"").unwrap();
        r.font("f.ttf", b"").unwrap();
        r.set_origin("pack");
        let errs = [
            r.engine(&NOP).unwrap_err(),
            r.visual(Box::new(V)).unwrap_err(),
            r.design_set(Box::new(D)).unwrap_err(),
            r.transition(Box::new(T)).unwrap_err(),
            r.point_cloud("c", b"").unwrap_err(),
            r.font("f.ttf", b"").unwrap_err(),
        ];
        for e in &errs {
            assert_eq!((e.first.as_str(), e.second.as_str()), ("mdeck", "pack"));
        }
        assert_eq!(
            errs[1].to_string(),
            "visual `v` is registered twice: by mdeck and by pack"
        );
        assert_eq!(r.engine_def("nop").unwrap().name, "nop");
        assert!(r.visual_for("v").is_some());
        assert!(r.design_set_for("d").is_some());
        assert!(r.transition_for("t").is_some());
        assert_eq!(r.engines().count(), 1);
    }
}
