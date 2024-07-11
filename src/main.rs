use anyhow::{Context, Result};
use argh::FromArgs;
use resvg::{
    tiny_skia::Pixmap,
    usvg::{Options, Transform, Tree},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// Utility to render svg to png
#[derive(FromArgs, Debug)]
struct Svg2Png {
    /// input svg file or directory
    #[argh(positional)]
    input: PathBuf,

    /// output png file path or directory
    #[argh(option, short = 'o', default = "PathBuf::from(\".\")")]
    output: PathBuf,

    /// overwrite output file if it already exists
    #[argh(switch)]
    overwrite: bool,
}

fn main() -> Result<()> {
    let args: Svg2Png = argh::from_env();
    if args.input.is_dir() {
        process_directory(args.input, args.output, args.overwrite)?;
    } else {
        process_file(args.input, args.output, args.overwrite)?;
    }
    Ok(())
}

fn process_directory(input_dir: PathBuf, output_path: PathBuf, overwrite: bool) -> Result<()> {
    for entry in fs::read_dir(&input_dir)
        .with_context(|| format!("Failed to read input directory: {:?}", input_dir))?
    {
        let path = entry
            .with_context(|| format!("Failed to read directory entry in: {:?}", input_dir))?
            .path();
        if path.extension().map_or(false, |ext| ext == "svg") {
            process_file(path, output_path.clone(), overwrite)?;
        }
    }
    Ok(())
}

fn process_file(input_file: PathBuf, output_path: PathBuf, overwrite: bool) -> Result<()> {
    let data = fs::read(&input_file)
        .with_context(|| format!("Failed to read input file: {:?}", input_file))?;
    let svg = Tree::from_data(&data, &Options::default())
        .with_context(|| format!("Failed to parse SVG data in file: {:?}", input_file))?;
    let mut pixmap = Pixmap::new(svg.size().width() as u32, svg.size().height() as u32)
        .context("Failed to create pixmap: maybe svg size is zero")?;
    resvg::render(&svg, Transform::identity(), &mut pixmap.as_mut());

    let output_path = determine_output_path(&input_file, &output_path)?;
    if output_path.exists() && !overwrite {
        print!(
            "File '{}' already exists. Overwrite? [y/N] ",
            output_path.display()
        );
        std::io::stdout()
            .flush()
            .context("Failed to flush stdout")?;
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("Failed to read input")?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Skipped: {}", output_path.display());
            return Ok(());
        }
    }

    pixmap
        .save_png(output_path.to_str().unwrap())
        .with_context(|| format!("Failed to save PNG file: {:?}", output_path))?;
    println!("Saved: {}", output_path.display());
    Ok(())
}

fn determine_output_path(input_file: &Path, output_path: &Path) -> Result<PathBuf> {
    let output_file_name = input_file
        .file_name()
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to get file name from input path: {:?}", input_file)
        })?
        .to_owned();

    let output_path = if !output_path.exists() {
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create output directory: {:?}", parent))?;
            }
        }
        if output_path.extension().map_or(false, |ext| ext == "png") {
            output_path.to_path_buf()
        } else {
            fs::create_dir(output_path)
                .with_context(|| format!("Failed to create output directory: {:?}", output_path))?;
            output_path.join(&output_file_name)
        }
    } else if output_path.is_dir() {
        output_path.join(&output_file_name)
    } else {
        anyhow::bail!("File already exists: {:?}", output_path)
    };

    Ok(output_path.with_extension("png"))
}
