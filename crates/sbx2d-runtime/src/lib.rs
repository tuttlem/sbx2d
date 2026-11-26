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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, Default, PartialEq)]
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BodyType {
    Static,
    Dynamic,
    Kinematic,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub half_width: f32,
    pub half_height: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsBody {
    pub entity_id: String,
    pub body_type: BodyType,
    pub position: Vec2,
    pub previous_position: Vec2,
    pub velocity: Vec2,
    pub collider: Aabb,
    pub gravity_scale: f32,
    pub grounded: bool,
    pub layer: u32,
    pub mask: u32,
    pub sensor: bool,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contact {
    pub first: String,
    pub second: String,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugCollider {
    pub center: Vec2,
    pub half_width: f32,
    pub half_height: f32,
    pub sensor: bool,
}
#[derive(Clone, Debug)]
pub struct PhysicsWorld {
    pub gravity: Vec2,
    pub fixed_dt: f32,
    pub bodies: Vec<PhysicsBody>,
    pub contacts: Vec<Contact>,
    accumulator: f32,
}
impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: Vec2 { x: 0.0, y: 980.0 },
            fixed_dt: 1.0 / 60.0,
            bodies: Vec::new(),
            contacts: Vec::new(),
            accumulator: 0.0,
        }
    }
}
impl PhysicsWorld {
    pub fn set_fixed_dt(&mut self, fixed_dt: f32) {
        if fixed_dt.is_finite() && fixed_dt > 0.0 {
            self.fixed_dt = fixed_dt;
            self.accumulator = 0.0;
        }
    }
    pub fn advance(&mut self, dt: f32) {
        self.accumulator += dt.max(0.0);
        while self.accumulator >= self.fixed_dt {
            self.step();
            self.accumulator -= self.fixed_dt;
        }
    }
    pub fn step(&mut self) {
        for body in &mut self.bodies {
            if body.body_type == BodyType::Dynamic {
                body.previous_position = body.position;
                let resting = body.grounded && body.velocity.y == 0.0;
                body.velocity.x += self.gravity.x * body.gravity_scale * self.fixed_dt;
                if !resting {
                    body.velocity.y += self.gravity.y * body.gravity_scale * self.fixed_dt;
                }
                body.position.x += body.velocity.x * self.fixed_dt;
                body.position.y += body.velocity.y * self.fixed_dt;
                body.grounded = false;
            }
        }
        self.contacts.clear();
        for i in 0..self.bodies.len() {
            for j in (i + 1)..self.bodies.len() {
                let (a, b) = (self.bodies[i].clone(), self.bodies[j].clone());
                if overlap(&a, &b) && (a.layer & b.mask != 0) && (b.layer & a.mask != 0) {
                    self.contacts.push(Contact {
                        first: a.entity_id.clone(),
                        second: b.entity_id.clone(),
                    });
                }
                if !a.sensor
                    && !b.sensor
                    && a.body_type == BodyType::Dynamic
                    && b.body_type == BodyType::Static
                    && a.position.y + a.collider.half_height
                        >= b.position.y - b.collider.half_height
                    && a.velocity.y >= 0.0
                {
                    if !self.contacts.iter().any(|contact| {
                        contact.first == a.entity_id && contact.second == b.entity_id
                    }) {
                        self.contacts.push(Contact {
                            first: a.entity_id.clone(),
                            second: b.entity_id.clone(),
                        });
                    }
                    let floor_y = b.position.y;
                    let floor_half_height = b.collider.half_height;
                    let body = &mut self.bodies[i];
                    body.position.y = floor_y - floor_half_height - body.collider.half_height;
                    if body.velocity.y.abs() * body.restitution.max(0.0) < 3.0 {
                        body.velocity.y = 0.0;
                    } else {
                        body.velocity.y = -body.velocity.y * body.restitution.max(0.0);
                    }
                    body.velocity.x *= (1.0 - body.friction.clamp(0.0, 1.0)).max(0.0);
                    body.grounded = true;
                }
                if !a.sensor
                    && !b.sensor
                    && b.body_type == BodyType::Dynamic
                    && a.body_type == BodyType::Static
                    && b.position.y + b.collider.half_height
                        >= a.position.y - a.collider.half_height
                    && b.velocity.y >= 0.0
                {
                    self.contacts.push(Contact {
                        first: b.entity_id.clone(),
                        second: a.entity_id.clone(),
                    });
                    let floor_y = a.position.y;
                    let floor_half_height = a.collider.half_height;
                    let body = &mut self.bodies[j];
                    body.position.y = floor_y - floor_half_height - body.collider.half_height;
                    if body.velocity.y.abs() * body.restitution.max(0.0) < 3.0 {
                        body.velocity.y = 0.0;
                    } else {
                        body.velocity.y = -body.velocity.y * body.restitution.max(0.0);
                    }
                    body.velocity.x *= (1.0 - body.friction.clamp(0.0, 1.0)).max(0.0);
                    body.grounded = true;
                }
            }
        }
    }
    #[must_use]
    pub fn debug_colliders(&self) -> Vec<DebugCollider> {
        self.bodies
            .iter()
            .map(|body| DebugCollider {
                center: body.position,
                half_width: body.collider.half_width,
                half_height: body.collider.half_height,
                sensor: body.sensor,
            })
            .collect()
    }
    #[must_use]
    pub fn interpolation_alpha(&self) -> f32 {
        (self.accumulator / self.fixed_dt).clamp(0.0, 1.0)
    }
    #[must_use]
    pub fn interpolated_position(&self, body: &PhysicsBody) -> Vec2 {
        let alpha = self.interpolation_alpha();
        Vec2 {
            x: body.previous_position.x + (body.position.x - body.previous_position.x) * alpha,
            y: body.previous_position.y + (body.position.y - body.previous_position.y) * alpha,
        }
    }
}
fn overlap(a: &PhysicsBody, b: &PhysicsBody) -> bool {
    (a.position.x - b.position.x).abs() < a.collider.half_width + b.collider.half_width
        && (a.position.y - b.position.y).abs() < a.collider.half_height + b.collider.half_height
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
#[derive(Clone, Debug, PartialEq)]
pub struct RenderItem {
    pub entity_id: String,
    pub asset: String,
    pub layer: i32,
    pub visible: bool,
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Debug, Default, PartialEq)]
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
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn draw_item(&mut self, _index: usize, item: &RenderItem) {
        let x0 = (item.x.max(0.0) as u32).min(self.width.saturating_sub(1));
        let y0 = (item.y.max(0.0) as u32).min(self.height.saturating_sub(1));
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
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn draw_item_color(&mut self, _index: usize, item: &RenderItem, color: [u8; 3]) {
        let x0 = (item.x.max(0.0) as u32).min(self.width.saturating_sub(1));
        let y0 = (item.y.max(0.0) as u32).min(self.height.saturating_sub(1));
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
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn draw_debug_colliders(&mut self, colliders: &[DebugCollider]) {
        for collider in colliders {
            let center_x = collider.center.x.max(0.0) as u32;
            let center_y = collider.center.y.max(0.0) as u32;
            let min_x = center_x
                .saturating_sub(collider.half_width.max(0.0) as u32)
                .min(self.width);
            let min_y = center_y
                .saturating_sub(collider.half_height.max(0.0) as u32)
                .min(self.height);
            let max_x =
                (center_x + collider.half_width.max(0.0) as u32).min(self.width.saturating_sub(1));
            let max_y = (center_y + collider.half_height.max(0.0) as u32)
                .min(self.height.saturating_sub(1));
            let color = if collider.sensor {
                [240, 200, 40, 255]
            } else {
                [40, 240, 120, 255]
            };
            for x in min_x..=max_x {
                self.pixel(x, min_y, color);
                self.pixel(x, max_y, color);
            }
            for y in min_y..=max_y {
                self.pixel(min_x, y, color);
                self.pixel(max_x, y, color);
            }
        }
    }
    fn pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        let offset = ((y * self.width + x) * 4) as usize;
        if offset + 4 <= self.pixels.len() {
            self.pixels[offset..offset + 4].copy_from_slice(&color);
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
                    x: entity.transform.x,
                    y: entity.transform.y,
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
    pub physics: PhysicsWorld,
    pub interactions: InteractionEngine,
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
            physics: PhysicsWorld::default(),
            interactions: InteractionEngine::default(),
        }
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn load_scene(
        &mut self,
        pack: &ResolvedPack,
        reference: &SceneReference,
    ) -> Result<(), RuntimeError> {
        let scene = pack
            .scenes
            .get(reference)
            .ok_or_else(|| RuntimeError::MissingScene(reference.clone()))?;
        // Scene-level physics knobs are intentionally small and deterministic for now.
        // They provide a stable bridge until the pack schema grows typed components.
        if let Some(gravity_x) = numeric_field(&scene.source, "gravity_x") {
            self.physics.gravity.x = gravity_x;
        }
        if let Some(gravity_y) = numeric_field(&scene.source, "gravity_y") {
            self.physics.gravity.y = gravity_y;
        }
        if let Some(fixed_dt) = numeric_field(&scene.source, "fixed_dt") {
            if fixed_dt.is_finite() && fixed_dt > 0.0 {
                self.physics.set_fixed_dt(fixed_dt);
            }
        }
        let entities = scene_entities(&scene.source);
        self.physics.bodies = scene_bodies(&scene.source);
        for line in scene.source.lines() {
            if line.contains("health") {
                if let (Some(id), Some(value)) =
                    (inline_string(line, "id"), inline_number(line, "health"))
                {
                    self.interactions.health.insert(id, value as i32);
                }
            }
            if line.contains("on_contact") {
                let first = inline_string(line, "first");
                let second = inline_string(line, "second");
                if let (Some(first), Some(second), Some(score)) =
                    (first.clone(), second.clone(), inline_number(line, "score"))
                {
                    self.interactions.on_contact(
                        first,
                        second,
                        Action::AddScore {
                            amount: score as i32,
                        },
                    );
                }
                if let (Some(first), Some(second), Some(effect)) =
                    (first.clone(), second.clone(), inline_string(line, "sound"))
                {
                    self.interactions
                        .on_contact(first, second, Action::PlaySound { effect });
                }
                if let (Some(first), Some(second), Some(target), Some(amount)) = (
                    first,
                    second,
                    inline_string(line, "target"),
                    inline_number(line, "damage"),
                ) {
                    self.interactions.on_contact(
                        first,
                        second,
                        Action::Damage {
                            target,
                            amount: amount as i32,
                        },
                    );
                }
            }
        }
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
    pub fn tick(&mut self, dt: f32) {
        self.physics.advance(dt);
        if let Some(scene) = &mut self.active_scene {
            for body in &self.physics.bodies {
                if let Some(entity) = scene.entities.iter_mut().find(|e| e.id == body.entity_id) {
                    entity.transform.x = self.physics.interpolated_position(body).x;
                    entity.transform.y = self.physics.interpolated_position(body).y;
                }
            }
        }
        self.interactions.dispatch_contacts(&self.physics.contacts);
        self.interactions
            .process(&mut self.active_scene, &mut self.physics);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    Contact { first: String, second: String },
    Damage { target: String, amount: i32 },
    Sound { effect: String },
    Score { amount: i32 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Damage {
        target: String,
        amount: i32,
    },
    PlaySound {
        effect: String,
    },
    AddScore {
        amount: i32,
    },
    Despawn {
        entity: String,
    },
    SetVelocity {
        entity: String,
        velocity: Vec2,
    },
    ApplyImpulse {
        entity: String,
        impulse: Vec2,
    },
    AreaDamage {
        center: Vec2,
        radius: f32,
        amount: i32,
    },
    Transition {
        scene: SceneReference,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionTrace {
    pub event: Event,
    pub action: Action,
}

#[derive(Clone, Debug, Default)]
pub struct InteractionEngine {
    pub health: BTreeMap<String, i32>,
    pub score: i32,
    pub queue: Vec<Action>,
    pub trace: Vec<ActionTrace>,
    pub sounds: Vec<String>,
    pub next_scene: Option<SceneReference>,
    pub contact_rules: BTreeMap<(String, String), Vec<Action>>,
}
impl InteractionEngine {
    pub fn on_contact(
        &mut self,
        first: impl Into<String>,
        second: impl Into<String>,
        action: Action,
    ) {
        self.contact_rules
            .entry((first.into(), second.into()))
            .or_default()
            .push(action);
    }
    pub fn dispatch_contacts(&mut self, contacts: &[Contact]) {
        for contact in contacts {
            let key = (contact.first.clone(), contact.second.clone());
            if let Some(actions) = self.contact_rules.get(&key).cloned() {
                for action in actions {
                    self.emit(
                        Event::Contact {
                            first: contact.first.clone(),
                            second: contact.second.clone(),
                        },
                        action,
                    );
                }
            }
        }
    }
    pub fn emit(&mut self, event: Event, action: Action) {
        self.trace.push(ActionTrace {
            event,
            action: action.clone(),
        });
        self.queue.push(action);
    }
    pub fn process(&mut self, scene: &mut Option<ActiveScene>, physics: &mut PhysicsWorld) {
        let actions = std::mem::take(&mut self.queue);
        for action in actions {
            match action {
                Action::Damage { target, amount } => {
                    let health = self.health.entry(target.clone()).or_insert(0);
                    *health -= amount;
                    if *health <= 0 {
                        self.queue.push(Action::Despawn { entity: target });
                    }
                }
                Action::PlaySound { effect } => self.sounds.push(effect),
                Action::AddScore { amount } => self.score += amount,
                Action::Despawn { entity } => {
                    if let Some(active) = scene {
                        active.entities.retain(|item| item.id != entity);
                    }
                    physics.bodies.retain(|body| body.entity_id != entity);
                }
                Action::SetVelocity { entity, velocity } => {
                    if let Some(body) = physics
                        .bodies
                        .iter_mut()
                        .find(|body| body.entity_id == entity)
                    {
                        body.velocity = velocity;
                    }
                }
                Action::ApplyImpulse { entity, impulse } => {
                    if let Some(body) = physics
                        .bodies
                        .iter_mut()
                        .find(|body| body.entity_id == entity)
                    {
                        body.velocity.x += impulse.x / body.mass.max(0.001);
                        body.velocity.y += impulse.y / body.mass.max(0.001);
                    }
                }
                Action::AreaDamage {
                    center,
                    radius,
                    amount,
                } => {
                    let radius_sq = radius.max(0.0).powi(2);
                    for body in &physics.bodies {
                        let dx = body.position.x - center.x;
                        let dy = body.position.y - center.y;
                        if dx * dx + dy * dy <= radius_sq {
                            let health = self.health.entry(body.entity_id.clone()).or_insert(0);
                            *health -= amount;
                            if *health <= 0 {
                                self.queue.push(Action::Despawn {
                                    entity: body.entity_id.clone(),
                                });
                            }
                        }
                    }
                }
                Action::Transition { scene: target } => self.next_scene = Some(target),
            }
        }
        if !self.queue.is_empty() {
            self.process(scene, physics);
        }
    }
}

fn numeric_field(source: &str, name: &str) -> Option<f32> {
    let needle = format!("{name} =");
    let value = source.lines().find_map(|line| {
        let (_, rest) = line.split_once(&needle)?;
        rest.trim().trim_end_matches(',').trim().parse::<f32>().ok()
    })?;
    value.is_finite().then_some(value)
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
                id: asset
                    .rsplit(':')
                    .next()
                    .map_or_else(|| format!("entity-{line}"), str::to_owned),
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

/// Reads the deliberately small, data-only body syntax used by scene packs:
/// `body = { id = "hero", type = "dynamic", x = 0, y = 0, width = 1, height = 1 }`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unnecessary_filter_map
)]
fn scene_bodies(source: &str) -> Vec<PhysicsBody> {
    source
        .lines()
        .filter(|line| line.contains('{') && line.contains("type") && line.contains("id"))
        .enumerate()
        .filter_map(|(index, line)| {
            let id = inline_string(line, "id").unwrap_or_else(|| format!("body-{index}"));
            let body_type = match inline_string(line, "type").as_deref() {
                Some("static") => BodyType::Static,
                Some("kinematic") => BodyType::Kinematic,
                _ => BodyType::Dynamic,
            };
            let x = inline_number(line, "x").unwrap_or(0.0);
            let y = inline_number(line, "y").unwrap_or(0.0);
            let width = inline_number(line, "width").unwrap_or(0.5).abs();
            let height = inline_number(line, "height").unwrap_or(0.5).abs();
            Some(PhysicsBody {
                entity_id: id,
                body_type,
                position: Vec2 { x, y },
                previous_position: Vec2 { x, y },
                velocity: Vec2 { x: 0.0, y: 0.0 },
                collider: Aabb {
                    half_width: width,
                    half_height: height,
                },
                gravity_scale: inline_number(line, "gravity_scale").unwrap_or(1.0),
                grounded: false,
                layer: inline_number(line, "layer").unwrap_or(1.0) as u32,
                mask: u32::MAX,
                sensor: inline_string(line, "sensor").is_some_and(|v| v == "true"),
                mass: 1.0,
                friction: inline_number(line, "friction").unwrap_or(0.0).max(0.0),
                restitution: inline_number(line, "restitution")
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0),
            })
        })
        .collect()
}

fn inline_number(line: &str, name: &str) -> Option<f32> {
    let (_, value) = line.split_once(&format!(" {name} ="))?;
    value.split([',', '}']).next()?.trim().parse().ok()
}

fn inline_string(line: &str, name: &str) -> Option<String> {
    let (_, value) = line.split_once(&format!(" {name} ="))?;
    let value = value.trim().trim_start_matches(['"', '\'']);
    Some(value.split(['"', '\'', ',', '}']).next()?.trim().to_owned())
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
    fn fixed_step_body_falls_and_contacts_floor() {
        let mut world = PhysicsWorld::default();
        world.bodies.push(PhysicsBody {
            entity_id: "floor".into(),
            body_type: BodyType::Static,
            position: Vec2 { x: 0.0, y: 10.0 },
            previous_position: Vec2 { x: 0.0, y: 10.0 },
            velocity: Vec2 { x: 0.0, y: 0.0 },
            collider: Aabb {
                half_width: 10.0,
                half_height: 1.0,
            },
            gravity_scale: 0.0,
            grounded: false,
            layer: 1,
            mask: u32::MAX,
            sensor: false,
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
        });
        world.bodies.push(PhysicsBody {
            entity_id: "crate".into(),
            body_type: BodyType::Dynamic,
            position: Vec2 { x: 0.0, y: 2.0 },
            previous_position: Vec2 { x: 0.0, y: 2.0 },
            velocity: Vec2 { x: 0.0, y: 0.0 },
            collider: Aabb {
                half_width: 1.0,
                half_height: 1.0,
            },
            gravity_scale: 1.0,
            grounded: false,
            layer: 1,
            mask: u32::MAX,
            sensor: false,
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
        });
        world.advance(1.0);
        assert!(world.bodies[1].position.y > 0.0);
        assert!(!world.contacts.is_empty());
    }

    #[test]
    fn fixed_step_is_independent_of_render_batching() {
        let body = |id: &str| PhysicsBody {
            entity_id: id.into(),
            body_type: BodyType::Dynamic,
            position: Vec2 { x: 0.0, y: 0.0 },
            previous_position: Vec2 { x: 0.0, y: 0.0 },
            velocity: Vec2 { x: 0.0, y: 0.0 },
            collider: Aabb {
                half_width: 1.0,
                half_height: 1.0,
            },
            gravity_scale: 1.0,
            grounded: false,
            layer: 1,
            mask: u32::MAX,
            sensor: false,
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
        };
        let mut batched = PhysicsWorld::default();
        batched.bodies.push(body("a"));
        batched.advance(1.0 / 30.0);
        let mut split = PhysicsWorld::default();
        split.bodies.push(body("b"));
        split.advance(1.0 / 60.0);
        split.advance(1.0 / 60.0);
        assert_eq!(batched.bodies[0].position, split.bodies[0].position);
    }

    #[test]
    fn restitution_reverses_downward_velocity_on_contact() {
        let mut world = PhysicsWorld::default();
        world.bodies.push(PhysicsBody {
            entity_id: "floor".into(),
            body_type: BodyType::Static,
            position: Vec2 { x: 0.0, y: 1.0 },
            previous_position: Vec2 { x: 0.0, y: 1.0 },
            velocity: Vec2 { x: 0.0, y: 0.0 },
            collider: Aabb {
                half_width: 10.0,
                half_height: 1.0,
            },
            gravity_scale: 0.0,
            grounded: false,
            layer: 1,
            mask: u32::MAX,
            sensor: false,
            mass: 0.0,
            friction: 0.0,
            restitution: 0.0,
        });
        world.bodies.push(PhysicsBody {
            entity_id: "ball".into(),
            body_type: BodyType::Dynamic,
            position: Vec2 { x: 0.0, y: 2.0 },
            previous_position: Vec2 { x: 0.0, y: 2.0 },
            velocity: Vec2 { x: 0.0, y: 10.0 },
            collider: Aabb {
                half_width: 1.0,
                half_height: 1.0,
            },
            gravity_scale: 0.0,
            grounded: false,
            layer: 1,
            mask: u32::MAX,
            sensor: false,
            mass: 1.0,
            friction: 0.0,
            restitution: 1.0,
        });
        world.step();
        assert!(world.bodies[1].velocity.y < 0.0);
    }

    #[test]
    fn sensor_contacts_do_not_resolve_position() {
        let mut world = PhysicsWorld::default();
        world.bodies.push(PhysicsBody {
            entity_id: "sensor".into(),
            body_type: BodyType::Static,
            position: Vec2 { x: 0.0, y: 1.0 },
            previous_position: Vec2 { x: 0.0, y: 1.0 },
            velocity: Vec2 { x: 0.0, y: 0.0 },
            collider: Aabb {
                half_width: 10.0,
                half_height: 1.0,
            },
            gravity_scale: 0.0,
            grounded: false,
            layer: 1,
            mask: u32::MAX,
            sensor: true,
            mass: 0.0,
            friction: 0.0,
            restitution: 0.0,
        });
        world.bodies.push(PhysicsBody {
            entity_id: "body".into(),
            body_type: BodyType::Dynamic,
            position: Vec2 { x: 0.0, y: 2.0 },
            previous_position: Vec2 { x: 0.0, y: 2.0 },
            velocity: Vec2 { x: 0.0, y: 0.0 },
            collider: Aabb {
                half_width: 1.0,
                half_height: 1.0,
            },
            gravity_scale: 0.0,
            grounded: false,
            layer: 1,
            mask: u32::MAX,
            sensor: false,
            mass: 1.0,
            friction: 0.0,
            restitution: 0.0,
        });
        world.step();
        assert!((world.bodies[1].position.y - 2.0).abs() < f32::EPSILON);
        assert_eq!(world.contacts.len(), 1);
    }

    #[test]
    fn declarative_scene_bodies_are_instantiated() {
        let bodies = scene_bodies(
            r#"return { body = { id = "floor", type = "static", x = 4, y = 8, width = 3, height = 1 } }"#,
        );
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].entity_id, "floor");
        assert_eq!(bodies[0].body_type, BodyType::Static);
        assert_eq!(bodies[0].position, Vec2 { x: 4.0, y: 8.0 });
    }

    #[test]
    fn interaction_actions_damage_score_sound_and_despawn() {
        let mut engine = InteractionEngine::default();
        let mut scene = Some(ActiveScene {
            reference: SceneReference {
                namespace: "w".into(),
                name: "s".into(),
            },
            entities: vec![Entity {
                id: "crate".into(),
                transform: Transform::default(),
                sprite: None,
            }],
            camera: Camera::default(),
        });
        let mut physics = PhysicsWorld::default();
        engine.health.insert("crate".into(), 3);
        engine.emit(
            Event::Damage {
                target: "crate".into(),
                amount: 3,
            },
            Action::Damage {
                target: "crate".into(),
                amount: 3,
            },
        );
        engine.emit(Event::Score { amount: 10 }, Action::AddScore { amount: 10 });
        engine.emit(
            Event::Sound {
                effect: "pop".into(),
            },
            Action::PlaySound {
                effect: "pop".into(),
            },
        );
        engine.process(&mut scene, &mut physics);
        assert_eq!(engine.score, 10);
        assert_eq!(engine.sounds, vec!["pop"]);
        assert!(scene.unwrap().entities.is_empty());
        assert_eq!(engine.trace.len(), 3);
    }

    #[test]
    fn contact_rules_dispatch_damage_and_despawn() {
        let mut engine = InteractionEngine::default();
        engine.health.insert("crate".into(), 1);
        engine.on_contact(
            "hero",
            "crate",
            Action::Damage {
                target: "crate".into(),
                amount: 1,
            },
        );
        engine.dispatch_contacts(&[Contact {
            first: "hero".into(),
            second: "crate".into(),
        }]);
        assert_eq!(engine.trace.len(), 1);
        let mut scene = Some(ActiveScene {
            reference: SceneReference {
                namespace: "w".into(),
                name: "s".into(),
            },
            entities: vec![Entity {
                id: "crate".into(),
                transform: Transform::default(),
                sprite: None,
            }],
            camera: Camera::default(),
        });
        let mut physics = PhysicsWorld::default();
        engine.process(&mut scene, &mut physics);
        assert!(scene.unwrap().entities.is_empty());
    }

    #[test]
    fn area_damage_affects_bodies_in_radius() {
        let mut engine = InteractionEngine::default();
        engine.health.insert("crate".into(), 2);
        engine.emit(
            Event::Damage {
                target: "area".into(),
                amount: 1,
            },
            Action::AreaDamage {
                center: Vec2 { x: 0.0, y: 0.0 },
                radius: 5.0,
                amount: 2,
            },
        );
        let mut scene = Some(ActiveScene {
            reference: SceneReference {
                namespace: "w".into(),
                name: "s".into(),
            },
            entities: vec![Entity {
                id: "crate".into(),
                transform: Transform::default(),
                sprite: None,
            }],
            camera: Camera::default(),
        });
        let mut physics = PhysicsWorld::default();
        physics.bodies.push(PhysicsBody {
            entity_id: "crate".into(),
            body_type: BodyType::Dynamic,
            position: Vec2 { x: 1.0, y: 1.0 },
            previous_position: Vec2 { x: 1.0, y: 1.0 },
            velocity: Vec2 { x: 0.0, y: 0.0 },
            collider: Aabb {
                half_width: 1.0,
                half_height: 1.0,
            },
            gravity_scale: 1.0,
            grounded: false,
            layer: 1,
            mask: u32::MAX,
            sensor: false,
            mass: 1.0,
            friction: 0.0,
            restitution: 0.0,
        });
        engine.process(&mut scene, &mut physics);
        assert!(scene.unwrap().entities.is_empty());
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
                x: 0.0,
                y: 0.0,
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
