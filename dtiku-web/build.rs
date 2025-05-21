use minify_html::{minify, Cfg};
use std::fs;
use std::io::Write;
use walkdir::WalkDir;

fn main() {
    let template_dir = "templates";

    let mut cfg = Cfg::new();
    cfg.minify_css = true;
    cfg.keep_closing_tags = true;

    for entry in WalkDir::new(template_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path().extension().map_or(false, |ext| ext == "jinja")
                && e.path().display().to_string().ends_with(".html.jinja")
        })
    {
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());

        match fs::read_to_string(path) {
            Ok(content) => {
                let minified = minify(content.as_bytes(), &cfg);

                // 可选：覆盖原文件，或保存为 .min.jinja
                // let out_path = path;
                let out_path = path.with_extension("min.jinja");

                let mut file = match fs::File::create(&out_path) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Failed to write file {}: {}", out_path.display(), e);
                        continue;
                    }
                };

                if let Err(e) = file.write_all(&minified) {
                    eprintln!("Failed to write file {}: {}", out_path.display(), e);
                } else {
                    println!("Minified: {}", out_path.display());
                }
            }
            Err(e) => {
                eprintln!("Failed to read file {}: {}", path.display(), e);
            }
        }
    }
}
