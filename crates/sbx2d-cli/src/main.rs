use sbx2d_runtime::Renderer;
use std::{collections::BTreeMap, env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    match (
        arguments.next(),
        arguments.next(),
        arguments.next(),
        arguments.next(),
    ) {
        (Some(command), Some(path), json, None) if command == "validate" => validate(
            Path::new(&path),
            json.as_deref().is_some_and(|x| x == "--json"),
        ),
        (Some(command), Some(path), json, None) if command == "list-worlds" => list_worlds(
            Path::new(&path),
            json.as_deref().is_some_and(|x| x == "--json"),
        ),
        (Some(command), Some(path), json, None) if command == "list-scenes" => list_scenes(
            Path::new(&path),
            json.as_deref().is_some_and(|x| x == "--json"),
        ),
        (Some(command), Some(id), Some(path), None) if command == "inspect" => {
            inspect(&id, Path::new(&path))
        }
        (Some(command), Some(path), None, None) if command == "run" => run(Path::new(&path)),
        (Some(command), Some(path), Some(window), None)
            if command == "run" && window == "--window" =>
        {
            run_window(Path::new(&path))
        }
        _ => {
            eprintln!(
                "usage:\n  sbx2d validate <pack-directory|game.lua>\n  sbx2d list-worlds <pack-directory>\n  sbx2d list-scenes <pack-directory>\n  sbx2d inspect <qualified-definition-id> <pack-directory>\n  sbx2d run <pack-directory>"
            );
            ExitCode::from(2)
        }
    }
}

fn validate(path: &Path, json: bool) -> ExitCode {
    match sbx2d_pack::load_pack(path) {
        Ok(pack) => {
            if json {
                println!("{}", sbx2d_pack::to_json(&pack));
            } else {
                println!(
                    "valid pack `{}` (format {}, start scene {}; {} worlds, {} scenes)",
                    pack.manifest.name,
                    pack.manifest.pack_version,
                    pack.manifest.start_scene,
                    pack.worlds.len(),
                    pack.scenes.len()
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            let source = std::fs::read_to_string(&error.path).ok();
            eprintln!("{}", error.render(source.as_deref()));
            ExitCode::from(1)
        }
    }
}

fn list_worlds(path: &Path, json: bool) -> ExitCode {
    match sbx2d_pack::load_pack(path) {
        Ok(pack) => {
            if json {
                println!(
                    "{{\"worlds\":[{}]}}",
                    pack.worlds
                        .keys()
                        .map(|x| format!("\"{x}\""))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            } else {
                for world in pack.worlds.keys() {
                    println!("{world}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => report(error),
    }
}

fn list_scenes(path: &Path, json: bool) -> ExitCode {
    match sbx2d_pack::load_pack(path) {
        Ok(pack) => {
            if json {
                println!(
                    "{{\"scenes\":[{}]}}",
                    pack.scenes
                        .keys()
                        .map(|x| format!("\"{x}\""))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            } else {
                for scene in pack.scenes.keys() {
                    println!("{scene}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => report(error),
    }
}

fn inspect(id: &std::ffi::OsStr, path: &Path) -> ExitCode {
    let Some(id) = id.to_str() else {
        eprintln!("inspect id must be valid UTF-8");
        return ExitCode::from(2);
    };
    match sbx2d_pack::load_pack(path) {
        Ok(pack) => {
            if let Some(definition) = pack.definitions.get(id) {
                println!(
                    "{}",
                    sbx2d_pack::to_json(&sbx2d_pack::ResolvedPack {
                        root: pack.root,
                        manifest: pack.manifest,
                        worlds: BTreeMap::default(),
                        scenes: BTreeMap::default(),
                        definitions: [(id.to_owned(), definition.clone())].into_iter().collect(),
                        assets: BTreeMap::default(),
                    })
                );
                ExitCode::SUCCESS
            } else {
                eprintln!("unknown definition `{id}`");
                ExitCode::from(1)
            }
        }
        Err(error) => report(error),
    }
}

fn report(error: sbx2d_pack::Diagnostic) -> ExitCode {
    let source = std::fs::read_to_string(&error.path).ok();
    eprintln!("{}", error.render(source.as_deref()));
    ExitCode::from(1)
}

fn run(path: &Path) -> ExitCode {
    match sbx2d_pack::load_pack(path) {
        Ok(pack) => {
            let mut runtime = sbx2d_runtime::Runtime::new();
            match runtime.start(&pack) {
                Ok(scene) => {
                    let scene_name = scene.reference.to_string();
                    let entity_count = scene.entities.len();
                    let capabilities = runtime.registry.ids().collect::<Vec<_>>().join(", ");
                    let mut renderer = sbx2d_runtime::HeadlessRenderer::default();
                    if let Err(error) = renderer.render(&runtime.render_frame()) {
                        match error {}
                    }
                    let asset_count = match sbx2d_runtime::AssetStore::load(&pack) {
                        Ok(assets) => assets.bytes.len(),
                        Err(error) => {
                            eprintln!("runtime error: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    println!(
                        "started scene {scene_name} ({entity_count} entities, {asset_count} assets, {} rendered frame, capabilities: {capabilities})",
                        renderer.frames.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("runtime error: {error}");
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => report(error),
    }
}

fn run_window(path: &Path) -> ExitCode {
    let pack = match sbx2d_pack::load_pack(path) {
        Ok(pack) => pack,
        Err(error) => return report(error),
    };
    let mut runtime = sbx2d_runtime::Runtime::new();
    if let Err(error) = runtime.start(&pack) {
        eprintln!("runtime error: {error}");
        return ExitCode::from(1);
    }
    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(loop_) => loop_,
        Err(error) => {
            eprintln!("window error: {error}");
            return ExitCode::from(1);
        }
    };
    let frame = runtime.render_frame();
    let mut app = WindowApp {
        title: format!(
            "{} — {} ({} render items)",
            pack.manifest.name,
            pack.manifest.start_scene,
            frame.items.len()
        ),
        window: None,
    };
    match event_loop.run_app(&mut app) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("window error: {error}");
            ExitCode::from(1)
        }
    }
}

struct WindowApp {
    title: String,
    window: Option<winit::window::Window>,
}
impl winit::application::ApplicationHandler for WindowApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            self.window = Some(
                event_loop
                    .create_window(
                        winit::window::Window::default_attributes()
                            .with_title(&self.title)
                            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 540.0)),
                    )
                    .expect("window creation"),
            );
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
