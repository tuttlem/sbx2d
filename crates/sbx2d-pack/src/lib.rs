//! Declarative sbx2d pack discovery and validation.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub path: PathBuf,
    pub message: String,
}
impl Diagnostic {
    fn new(path: &Path, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }
    #[must_use]
    pub fn render(&self, _source: Option<&str>) -> String {
        format!(
            "{}: error[{}]: {}",
            self.path.display(),
            self.code,
            self.message
        )
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Diagnostic {}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SceneReference {
    pub namespace: String,
    pub name: String,
}
impl fmt::Display for SceneReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.name)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest {
    pub pack_version: u32,
    pub name: String,
    pub start_scene: SceneReference,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    pub name: String,
    pub path: PathBuf,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scene {
    pub reference: SceneReference,
    pub path: PathBuf,
    pub source: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    pub id: String,
    pub path: PathBuf,
    pub source: String,
    pub inheritance: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPack {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub worlds: BTreeMap<String, World>,
    pub scenes: BTreeMap<SceneReference, Scene>,
    pub definitions: BTreeMap<String, Definition>,
    pub assets: BTreeMap<String, PathBuf>,
}

pub fn load_manifest(input: impl AsRef<Path>) -> Result<Manifest, Diagnostic> {
    let input = input.as_ref();
    let path = if input.is_dir() {
        input.join("game.lua")
    } else {
        input.into()
    };
    parse_manifest(&path, &read(&path)?)
}
pub fn parse_manifest(path: impl AsRef<Path>, source: &str) -> Result<Manifest, Diagnostic> {
    let path = path.as_ref();
    restricted(path, source)?;
    let version = field(path, source, "pack_version")?
        .parse::<u32>()
        .map_err(|_| Diagnostic::new(path, "SBX102", "pack_version must be a positive integer"))?;
    if version != 1 {
        return Err(Diagnostic::new(
            path,
            "SBX106",
            "unsupported pack format version",
        ));
    }
    let name = field(path, source, "name")?;
    identifier(path, &name)?;
    let start = field(path, source, "start_scene")?;
    let start_scene = reference(path, &start)?;
    Ok(Manifest {
        pack_version: version,
        name,
        start_scene,
    })
}
pub fn load_pack(input: impl AsRef<Path>) -> Result<ResolvedPack, Diagnostic> {
    let input = input.as_ref();
    let root = if input.is_dir() {
        input
    } else {
        input
            .parent()
            .ok_or_else(|| Diagnostic::new(input, "SBX001", "manifest has no parent directory"))?
    };
    let manifest = load_manifest(root)?;
    let mut worlds = BTreeMap::new();
    let mut scenes = BTreeMap::new();
    let mut definitions = BTreeMap::new();
    let mut assets = BTreeMap::new();
    let worlds_dir = root.join("worlds");
    if worlds_dir.exists() {
        for world_dir in dirs(&worlds_dir)? {
            let name = filename(&world_dir)?;
            identifier(&world_dir, &name)?;
            let world = world_dir.join("world.lua");
            if !world.is_file() {
                return Err(Diagnostic::new(
                    &world_dir,
                    "SBX201",
                    "world is missing world.lua",
                ));
            }
            let world_source = read(&world)?;
            restricted(&world, &world_source)?;
            assets_in(&world, &world_source, &name, &mut assets)?;
            worlds.insert(
                name.clone(),
                World {
                    name: name.clone(),
                    path: world,
                },
            );
            let scene_root = world_dir.join("scenes");
            if scene_root.exists() {
                for scene_dir in dirs(&scene_root)? {
                    let scene_name = filename(&scene_dir)?;
                    identifier(&scene_dir, &scene_name)?;
                    let file = scene_dir.join("scene.lua");
                    if !file.is_file() {
                        return Err(Diagnostic::new(
                            &scene_dir,
                            "SBX202",
                            "scene is missing scene.lua",
                        ));
                    }
                    let source = read(&file)?;
                    restricted(&file, &source)?;
                    assets_in(&file, &source, &name, &mut assets)?;
                    let r = SceneReference {
                        namespace: name.clone(),
                        name: scene_name,
                    };
                    scenes.insert(
                        r.clone(),
                        Scene {
                            reference: r,
                            path: file,
                            source,
                        },
                    );
                }
            }
            definitions_in(&world_dir, &name, &mut definitions)?;
        }
    }
    definitions_in(&root.join("common"), "common", &mut definitions)?;
    for definition in definitions.values() {
        let namespace = definition.id.split_once(':').map_or("common", |x| x.0);
        assets_in(&definition.path, &definition.source, namespace, &mut assets)?;
    }
    resolve_definition_inheritance(&mut definitions)?;
    validate_scene_inheritance(&scenes)?;
    if !scenes.contains_key(&manifest.start_scene) {
        return Err(Diagnostic::new(
            &root.join("game.lua"),
            "SBX203",
            "start_scene does not exist",
        ));
    }
    Ok(ResolvedPack {
        root: root.into(),
        manifest,
        worlds,
        scenes,
        definitions,
        assets,
    })
}
#[must_use]
pub fn to_json(pack: &ResolvedPack) -> String {
    let worlds = pack
        .worlds
        .keys()
        .map(|x| format!("\"{x}\""))
        .collect::<Vec<_>>()
        .join(",");
    let scenes = pack
        .scenes
        .keys()
        .map(|x| format!("\"{x}\""))
        .collect::<Vec<_>>()
        .join(",");
    let definitions = pack
        .definitions
        .iter()
        .map(|(id, d)| {
            format!(
                "\"{}\":{{\"source\":\"{}\",\"inheritance\":[{}]}}",
                id,
                json_string(&d.source),
                d.inheritance
                    .iter()
                    .map(|x| format!("\"{x}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let assets = pack
        .assets
        .iter()
        .map(|(id, path)| {
            format!(
                "\"{}\":\"{}\"",
                id,
                json_string(&path.display().to_string())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"name\":\"{}\",\"start_scene\":\"{}\",\"worlds\":[{}],\"scenes\":[{}],\"definitions\":{{{}}},\"assets\":{{{}}}}}",
        json_string(&pack.manifest.name),
        pack.manifest.start_scene,
        worlds,
        scenes,
        definitions,
        assets
    )
}
fn json_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
}
fn read(path: &Path) -> Result<String, Diagnostic> {
    fs::read_to_string(path)
        .map_err(|e| Diagnostic::new(path, "SBX001", format!("could not read file: {e}")))
}
fn dirs(path: &Path) -> Result<Vec<PathBuf>, Diagnostic> {
    let mut out = fs::read_dir(path)
        .map_err(|e| Diagnostic::new(path, "SBX001", e.to_string()))?
        .filter_map(Result::ok)
        .map(|x| x.path())
        .filter(|x| x.is_dir())
        .collect::<Vec<_>>();
    out.sort();
    Ok(out)
}
fn filename(path: &Path) -> Result<String, Diagnostic> {
    path.file_name()
        .and_then(|x| x.to_str())
        .map(str::to_owned)
        .ok_or_else(|| Diagnostic::new(path, "SBX001", "non-UTF8 path"))
}
fn identifier(path: &Path, s: &str) -> Result<(), Diagnostic> {
    if !s.is_empty()
        && s.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase() || (i > 0 && (b.is_ascii_digit() || b == b'-' || b == b'_'))
        })
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            path,
            "SBX107",
            format!("invalid identifier {s}"),
        ))
    }
}
fn reference(path: &Path, s: &str) -> Result<SceneReference, Diagnostic> {
    let Some((a, b)) = s.split_once(':') else {
        return Err(Diagnostic::new(
            path,
            "SBX108",
            "reference must be namespace:name",
        ));
    };
    if s.matches(':').count() != 1 {
        return Err(Diagnostic::new(
            path,
            "SBX108",
            "reference must have one colon",
        ));
    }
    identifier(path, a)?;
    identifier(path, b)?;
    Ok(SceneReference {
        namespace: a.into(),
        name: b.into(),
    })
}
fn field(path: &Path, source: &str, key: &str) -> Result<String, Diagnostic> {
    let needle = format!("{key} =");
    let Some(rest) = source.split(&needle).nth(1) else {
        return Err(Diagnostic::new(
            path,
            "SBX103",
            format!("missing required field {key}"),
        ));
    };
    let rest = rest.trim_start();
    if let Some(q) = rest
        .as_bytes()
        .first()
        .filter(|x| **x == b'\'' || **x == b'"')
    {
        let tail = &rest[1..];
        let Some(end) = tail.find(*q as char) else {
            return Err(Diagnostic::new(path, "SBX101", "unterminated string"));
        };
        Ok(tail[..end].into())
    } else {
        Ok(rest
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or_default()
            .into())
    }
}
fn restricted(path: &Path, source: &str) -> Result<(), Diagnostic> {
    let s = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<String>();
    if !s.trim_start().starts_with("return {")
        || source.contains("function")
        || source.contains("require")
        || source.contains("os.")
        || source.contains("io.")
    {
        Err(Diagnostic::new(
            path,
            "SBX101",
            "pack files must contain only returned data",
        ))
    } else {
        Ok(())
    }
}
fn assets_in(
    path: &Path,
    source: &str,
    namespace: &str,
    out: &mut BTreeMap<String, PathBuf>,
) -> Result<(), Diagnostic> {
    let Some(after) = source.split("assets = {").nth(1) else {
        return Ok(());
    };
    let body = after.split('}').next().unwrap_or_default();
    for item in body.split(',') {
        let Some((name, value)) = item.split_once('=') else {
            continue;
        };
        let name = name.trim();
        identifier(path, name)?;
        let value = value
            .trim()
            .trim_matches(char::from(34))
            .trim_matches(char::from(39));
        if value.is_empty() {
            return Err(Diagnostic::new(
                path,
                "SBX102",
                "asset value must be a path string",
            ));
        }
        let target = path.parent().expect("file").join(value);
        if !target.is_file() {
            return Err(Diagnostic::new(
                path,
                "SBX205",
                format!("asset does not exist: {value}"),
            ));
        }
        let id = format!("{namespace}:{name}");
        if out.insert(id.clone(), target).is_some() {
            return Err(Diagnostic::new(
                path,
                "SBX206",
                format!("duplicate asset identifier {id}"),
            ));
        }
    }
    Ok(())
}
fn definitions_in(
    dir: &Path,
    ns: &str,
    out: &mut BTreeMap<String, Definition>,
) -> Result<(), Diagnostic> {
    if !dir.exists() {
        return Ok(());
    }
    for p in dirs(dir)? {
        if p.file_name().is_some_and(|x| x == "scenes") {
            continue;
        }
        definitions_in(&p, ns, out)?;
    }
    for entry in fs::read_dir(dir)
        .map_err(|e| Diagnostic::new(dir, "SBX001", e.to_string()))?
        .filter_map(Result::ok)
    {
        let p = entry.path();
        if p.extension().is_some_and(|x| x == "lua")
            && p.file_name().is_some_and(|x| x != "world.lua")
        {
            let source = read(&p)?;
            restricted(&p, &source)?;
            if let Ok(id) = field(&p, &source, "id") {
                identifier(&p, &id)?;
                let q = format!("{ns}:{id}");
                if out
                    .insert(
                        q.clone(),
                        Definition {
                            id: q.clone(),
                            path: p,
                            source,
                            inheritance: vec![q],
                        },
                    )
                    .is_some()
                {
                    return Err(Diagnostic::new(
                        dir,
                        "SBX206",
                        "duplicate definition identifier",
                    ));
                }
            }
        }
    }
    Ok(())
}
fn parent(path: &Path, source: &str) -> Result<Option<String>, Diagnostic> {
    match field(path, source, "extends") {
        Ok(x) => {
            reference(path, &x)?;
            Ok(Some(x))
        }
        Err(x) if x.code == "SBX103" => Ok(None),
        Err(x) => Err(x),
    }
}
fn resolve_definition_inheritance(
    defs: &mut BTreeMap<String, Definition>,
) -> Result<(), Diagnostic> {
    fn chain(
        id: &str,
        defs: &BTreeMap<String, Definition>,
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<String>, Diagnostic> {
        let def = &defs[id];
        if !visiting.insert(id.into()) {
            return Err(Diagnostic::new(
                &def.path,
                "SBX208",
                "cyclic definition inheritance",
            ));
        }
        let mut out = if let Some(p) = parent(&def.path, &def.source)? {
            if !defs.contains_key(&p) {
                return Err(Diagnostic::new(
                    &def.path,
                    "SBX209",
                    format!("unknown definition parent {p}"),
                ));
            }
            chain(&p, defs, visiting)?
        } else {
            Vec::new()
        };
        visiting.remove(id);
        out.push(id.into());
        Ok(out)
    }
    for id in defs.keys().cloned().collect::<Vec<_>>() {
        let inheritance = chain(&id, defs, &mut BTreeSet::new())?;
        defs.get_mut(&id).expect("known id").inheritance = inheritance;
    }
    Ok(())
}
fn validate_scene_inheritance(scenes: &BTreeMap<SceneReference, Scene>) -> Result<(), Diagnostic> {
    fn visit(
        id: &SceneReference,
        scenes: &BTreeMap<SceneReference, Scene>,
        done: &mut BTreeSet<SceneReference>,
        visiting: &mut BTreeSet<SceneReference>,
    ) -> Result<(), Diagnostic> {
        if done.contains(id) {
            return Ok(());
        }
        let scene = &scenes[id];
        if !visiting.insert(id.clone()) {
            return Err(Diagnostic::new(
                &scene.path,
                "SBX208",
                "cyclic scene inheritance",
            ));
        }
        if let Some(p) = parent(&scene.path, &scene.source)? {
            let p = reference(&scene.path, &p)?;
            if !scenes.contains_key(&p) {
                return Err(Diagnostic::new(
                    &scene.path,
                    "SBX209",
                    format!("unknown scene parent {p}"),
                ));
            }
            visit(&p, scenes, done, visiting)?;
        }
        visiting.remove(id);
        done.insert(id.clone());
        Ok(())
    }
    let mut done = BTreeSet::new();
    for id in scenes.keys() {
        visit(id, scenes, &mut done, &mut BTreeSet::new())?;
    }
    Ok(())
}
