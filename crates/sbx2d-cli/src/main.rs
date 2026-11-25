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
        _ => {
            eprintln!(
                "usage:\n  sbx2d validate <pack-directory|game.lua>\n  sbx2d list-worlds <pack-directory>\n  sbx2d list-scenes <pack-directory>\n  sbx2d inspect <qualified-definition-id> <pack-directory>"
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
                        .map(|x| format!("\"{}\"", x))
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
                        .map(|x| format!("\"{}\"", x))
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
