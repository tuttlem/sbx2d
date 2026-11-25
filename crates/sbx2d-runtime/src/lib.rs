//! Platform-neutral runtime shell contracts.
use sbx2d_pack::{AssetSource, ResolvedPack, SceneReference};
use std::{collections::BTreeMap, fmt, fs, path::PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sprite {
    pub asset: String,
    pub layer: i32,
    pub visible: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowSpec {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
}
impl Default for WindowSpec {
    fn default() -> Self {
        Self {
            title: "sbx2d".into(),
            width: 960,
            height: 540,
            resizable: true,
        }
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InputState {
    pub pressed: Vec<String>,
    pub pointer_x: i32,
    pub pointer_y: i32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub id: String,
    pub transform: Transform,
    pub sprite: Option<Sprite>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveScene {
    pub reference: SceneReference,
    pub entities: Vec<Entity>,
    pub camera: Camera,
}
#[derive(Clone, Debug, Default)]
pub struct AssetStore {
    pub bytes: BTreeMap<String, Vec<u8>>,
}
impl AssetStore {
    pub fn load(pack: &ResolvedPack) -> Result<Self, RuntimeError> {
        let mut store = Self::default();
        for (id, source) in &pack.assets {
            let bytes = match source {
                AssetSource::File(path) => fs::read(path)
                    .map_err(|_| RuntimeError::MissingAsset(id.clone(), path.clone()))?,
                AssetSource::Rgb(rgb) => rgb.to_vec(),
            };
            store.bytes.insert(id.clone(), bytes);
        }
        Ok(store)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderItem {
    pub entity_id: String,
    pub asset: String,
    pub layer: i32,
    pub visible: bool,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RenderFrame {
    pub items: Vec<RenderItem>,
}
pub trait Renderer {
    type Error;
    fn render(&mut self, frame: &RenderFrame) -> Result<(), Self::Error>;
}
#[derive(Clone, Debug, Default)]
pub struct HeadlessRenderer {
    pub frames: Vec<RenderFrame>,
}
impl Renderer for HeadlessRenderer {
    type Error = std::convert::Infallible;
    fn render(&mut self, frame: &RenderFrame) -> Result<(), Self::Error> {
        self.frames.push(frame.clone());
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct RasterRenderer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}
impl RasterRenderer {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }
    fn clear(&mut self) {
        self.pixels.fill(0);
    }
    fn draw_item(&mut self, index: usize, item: &RenderItem) {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        let x0 = (index * 32) % self.width.max(1);
        let y0 = (index * 24) % self.height.max(1);
        for y in y0..(y0 + 24).min(self.height) {
            for x in x0..(x0 + 24).min(self.width) {
                let offset = ((y * self.width + x) * 4) as usize;
                self.pixels[offset..offset + 4].copy_from_slice(&[
                    80,
                    160,
                    240,
                    u8::from(item.visible) * 255,
                ]);
            }
        }
    }
}
impl Renderer for RasterRenderer {
    type Error = std::convert::Infallible;
    fn render(&mut self, frame: &RenderFrame) -> Result<(), Self::Error> {
        self.clear();
        for (index, item) in frame.items.iter().enumerate() {
            self.draw_item(index, item);
        }
        Ok(())
    }
}
impl RasterRenderer {
    pub fn render_assets(&mut self, frame: &RenderFrame, assets: &AssetStore) {
        self.clear();
        for (index, item) in frame.items.iter().enumerate() {
            let color = assets
                .bytes
                .get(&item.asset)
                .map_or([80, 160, 240], |bytes| parse_color(bytes));
            self.draw_item_color(index, item, color);
        }
    }
    fn draw_item_color(&mut self, index: usize, item: &RenderItem, color: [u8; 3]) {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        let x0 = (index * 32) % self.width.max(1);
        let y0 = (index * 24) % self.height.max(1);
        for y in y0..(y0 + 24).min(self.height) {
            for x in x0..(x0 + 24).min(self.width) {
                let offset = ((y * self.width + x) * 4) as usize;
                self.pixels[offset..offset + 4].copy_from_slice(&[
                    color[0],
                    color[1],
                    color[2],
                    u8::from(item.visible) * 255,
                ]);
            }
        }
    }
}
fn parse_color(bytes: &[u8]) -> [u8; 3] {
    let values = std::str::from_utf8(bytes).ok().map(|text| {
        text.split_whitespace()
            .filter_map(|part| part.parse::<u8>().ok())
            .collect::<Vec<_>>()
    });
    match values.as_deref() {
        Some([red, green, blue]) => [*red, *green, *blue],
        _ => [
            bytes.first().copied().unwrap_or(80),
            bytes.get(1).copied().unwrap_or(160),
            bytes.get(2).copied().unwrap_or(240),
        ],
    }
}
impl RenderFrame {
    #[must_use]
    pub fn from_scene(scene: &ActiveScene) -> Self {
        let mut items = scene
            .entities
            .iter()
            .filter_map(|entity| {
                entity.sprite.as_ref().map(|sprite| RenderItem {
                    entity_id: entity.id.clone(),
                    asset: sprite.asset.clone(),
                    layer: sprite.layer,
                    visible: sprite.visible,
                })
            })
            .collect::<Vec<_>>();
        items.sort_by_key(|item| (item.layer, item.entity_id.clone()));
        Self { items }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    pub id: String,
    pub version: u32,
}
#[derive(Clone, Debug, Default)]
pub struct CapabilityRegistry {
    capabilities: BTreeMap<String, Capability>,
}
impl CapabilityRegistry {
    pub fn register(&mut self, id: impl Into<String>, version: u32) -> bool {
        let id = id.into();
        self.capabilities
            .insert(id.clone(), Capability { id, version })
            .is_none()
    }
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.capabilities.get(id)
    }
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.capabilities.keys().map(String::as_str)
    }
}
#[derive(Debug)]
pub enum RuntimeError {
    MissingScene(SceneReference),
    EmptyPack,
    MissingAsset(String, PathBuf),
}
impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingScene(s) => write!(f, "scene `{s}` is not available"),
            Self::EmptyPack => write!(f, "pack contains no scenes"),
            Self::MissingAsset(id, path) => write!(
                f,
                "asset `{id}` could not be loaded from {}",
                path.display()
            ),
        }
    }
}
impl std::error::Error for RuntimeError {}
#[derive(Clone, Debug)]
pub struct Runtime {
    pub registry: CapabilityRegistry,
    pub active_scene: Option<ActiveScene>,
}
impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
impl Runtime {
    #[must_use]
    pub fn new() -> Self {
        let mut registry = CapabilityRegistry::default();
        for id in ["transform", "sprite", "camera"] {
            registry.register(id, 1);
        }
        Self {
            registry,
            active_scene: None,
        }
    }
    pub fn load_scene(
        &mut self,
        pack: &ResolvedPack,
        reference: &SceneReference,
    ) -> Result<(), RuntimeError> {
        let scene = pack
            .scenes
            .get(reference)
            .ok_or_else(|| RuntimeError::MissingScene(reference.clone()))?;
        let entities = scene_entities(&scene.source);
        self.active_scene = Some(ActiveScene {
            reference: scene.reference.clone(),
            entities,
            camera: Camera::default(),
        });
        Ok(())
    }
    pub fn start(&mut self, pack: &ResolvedPack) -> Result<&ActiveScene, RuntimeError> {
        if pack.scenes.is_empty() {
            return Err(RuntimeError::EmptyPack);
        }
        let start = pack.manifest.start_scene.clone();
        self.load_scene(pack, &start)?;
        self.active_scene.as_ref().ok_or(RuntimeError::EmptyPack)
    }
    pub fn render_frame(&self) -> RenderFrame {
        self.active_scene
            .as_ref()
            .map_or_else(RenderFrame::default, RenderFrame::from_scene)
    }
}

fn scene_entities(source: &str) -> Vec<Entity> {
    source
        .lines()
        .enumerate()
        .filter_map(|(line, text)| {
            let asset = text
                .split("sprite =")
                .nth(1)?
                .trim()
                .trim_matches(',')
                .trim_matches('"')
                .trim_matches('\'')
                .to_owned();
            if asset.is_empty() {
                return None;
            }
            Some(Entity {
                id: format!("entity-{line}"),
                transform: Transform::default(),
                sprite: Some(Sprite {
                    asset,
                    layer: 0,
                    visible: true,
                }),
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_has_shell_capabilities() {
        assert!(Runtime::new().registry.get("sprite").is_some());
    }

    #[test]
    fn render_frame_orders_items_by_layer_then_id() {
        let scene = ActiveScene {
            reference: SceneReference {
                namespace: "test".into(),
                name: "scene".into(),
            },
            entities: vec![
                Entity {
                    id: "z".into(),
                    transform: Transform::default(),
                    sprite: Some(Sprite {
                        asset: "z".into(),
                        layer: 2,
                        visible: true,
                    }),
                },
                Entity {
                    id: "a".into(),
                    transform: Transform::default(),
                    sprite: Some(Sprite {
                        asset: "a".into(),
                        layer: 1,
                        visible: true,
                    }),
                },
            ],
            camera: Camera::default(),
        };
        let frame = RenderFrame::from_scene(&scene);
        assert_eq!(
            frame
                .items
                .iter()
                .map(|item| item.entity_id.as_str())
                .collect::<Vec<_>>(),
            ["a", "z"]
        );
    }

    #[test]
    fn window_and_input_defaults_are_portable() {
        assert_eq!(WindowSpec::default().width, 960);
        assert!(InputState::default().pressed.is_empty());
    }

    #[test]
    fn headless_renderer_records_frames() {
        let mut renderer = HeadlessRenderer::default();
        renderer
            .render(&RenderFrame::default())
            .expect("infallible renderer");
        assert_eq!(renderer.frames.len(), 1);
    }

    #[test]
    fn raster_renderer_produces_rgba_framebuffer() {
        let mut renderer = RasterRenderer::new(64, 48);
        let frame = RenderFrame {
            items: vec![RenderItem {
                entity_id: "hero".into(),
                asset: "hero".into(),
                layer: 0,
                visible: true,
            }],
        };
        renderer.render(&frame).expect("infallible renderer");
        assert_eq!(renderer.pixels.len(), 64 * 48 * 4);
        assert!(
            renderer
                .pixels
                .chunks_exact(4)
                .any(|pixel| pixel == [80, 160, 240, 255])
        );
    }
}
