use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    match (arguments.next(), arguments.next()) {
        (Some(command), Some(path)) if command == "validate" && arguments.next().is_none() => {
            validate(Path::new(&path))
        }
        _ => {
            eprintln!("usage: sbx2d validate <pack-directory|game.lua>");
            ExitCode::from(2)
        }
    }
}

fn validate(path: &Path) -> ExitCode {
    match sbx2d_pack::load_manifest(path) {
        Ok(manifest) => {
            println!(
                "valid pack `{}` (format {}, start scene {})",
                manifest.name, manifest.pack_version, manifest.start_scene
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            let source = std::fs::read_to_string(&error.path).ok();
            eprintln!("{}", error.render(source.as_deref()));
            ExitCode::from(1)
        }
    }
}
