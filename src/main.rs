use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs::{remove_dir_all, DirEntry};
use std::hash::Hash;
use std::io;
use std::io::Write;
use std::path::Path;
use std::{env, fs};

use dotenv::dotenv;

fn main() -> io::Result<()> {
    dotenv().ok();
    let args = env::args().collect::<Vec<String>>();
    let input_folder = env::var("RENAMER_INPUT_FOLDER").unwrap_or("./input".to_string());
    let output_folder = env::var("RENAMER_OUTPUT_FOLDER").unwrap_or("./output".to_string());

    let series_folders = fs::read_dir(&input_folder)?;
    let extraction_methods = Methods::default();
    let default_method = if let Some(v) =
        extraction_methods.get(args.get(1).cloned().unwrap_or_default().as_str())
    {
        v.name.to_string()
    } else {
        "default".to_string()
    };
    for series_entry in series_folders {
        if let Ok(series_dir) = series_entry {
            if series_dir.file_type()?.is_dir() {
                series_processing(
                    &output_folder,
                    series_dir,
                    &extraction_methods,
                    &default_method,
                )?;
            }
        }
    }
    remove_empty_folders(&input_folder)?;

    Ok(())
}

fn remove_empty_folders(input_folder: &str) -> io::Result<()> {
    let series_folders = fs::read_dir(input_folder)?;
    for series_entry in series_folders {
        if let Ok(series_dir) = series_entry {
            if series_dir.file_type()?.is_dir() {
                let series_files = fs::read_dir(&series_dir.path())?;
                if series_files.count() == 0 {
                    remove_dir_all(&series_dir.path())?;
                }
            }
        }
    }
    Ok(())
}

fn clear_terminal() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().expect("Failed to flush stdout");
}

fn file_processing<'a>(
    output: &str,
    file: DirEntry,
    series_name_str: &Cow<str>,
    extraction_methods: &Methods,
    default_method: &str,
) -> io::Result<()> {
    let file_name = file.file_name();
    let file_name_str = file_name.to_string_lossy();
    let ext = Path::new(&file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    let (mut season, mut episode) = (extraction_methods
        .get(default_method)
        .expect("Cant fail")
        .func)(&file_name_str);

    let mut extraction_attempted = HashSet::new();
    extraction_attempted.insert(default_method.to_string());

    let (season, episode) = loop {
        match (season, episode) {
            (Some(s), Some(e)) => break (s, e),
            _ => {
                let show = extraction_methods
                    .clone()
                    .data
                    .into_iter()
                    .filter(|v| !extraction_attempted.contains(v.name))
                    .enumerate()
                    .collect::<HashMap<_, _>>();
                let (s, e) = if show.is_empty() {
                    prompt_for_season_episode(&file_name_str)
                } else {
                    let mut items = show
                        .iter()
                        .map(|(num, v)| format!("{}:{}", num + 2, v.pattern))
                        .collect::<Vec<_>>();
                    items.insert(0, "0:Skip".to_string());
                    items.insert(1, "1:Custom".to_string());
                    clear_terminal();
                    println!("How do you want to process: {}", file_name_str);
                    println!("Options: {}", items.join(", "));
                    println!();
                    print!("Please insert number: ");
                    let _ = io::stdout().flush();
                    let mut user_input = String::new();
                    let _ = io::stdin().read_line(&mut user_input);
                    if let Ok(parsed) = user_input.replace("\n", "").parse::<usize>() {
                        if parsed == 0 {
                            return Ok(());
                        } else if parsed == 1 {
                            prompt_for_season_episode(&file_name_str)
                        } else {
                            let func = show.get(&(parsed - 2));
                            if let Some(v) = func {
                                extraction_attempted.insert(v.name.to_string());
                                (v.func)(&file_name_str)
                            } else {
                                println!("Invalid input");
                                (None, None)
                            }
                        }
                    } else {
                        println!("Invlaid selection");
                        (None, None)
                    }
                };
                season = s;
                episode = e;
            }
        }
    };
    save(output, file, ext, series_name_str, season, episode)
}

fn save(
    output: &str,
    file: DirEntry,
    ext: &str,
    series_name_str: impl Display,
    season: i32,
    episode: i32,
) -> io::Result<()> {
    // Create the output directory structure
    let output_dir = format!("{}/{}/Season {:02}", output, series_name_str, season,);

    // Create the necessary directories
    fs::create_dir_all(&output_dir)?;

    // Move the file to the output directory
    let new_file_path =
        Path::new(&output_dir).join(format!("Episode S{:02}E{:02}.{}", season, episode, ext));
    fs::rename(file.path(), new_file_path)
}

fn series_processing(
    output: &str,
    series_dir: DirEntry,
    extraction_methods: &Methods,
    default_method: &str,
) -> io::Result<()> {
    let series_name = series_dir.file_name();
    let series_name_str = series_name.to_string_lossy();

    // Read the files in the series folder
    let series_files = fs::read_dir(&series_dir.path())?;

    for file_entry in series_files {
        if let Ok(file) = file_entry {
            if file.file_type()?.is_file() {
                file_processing(
                    output,
                    file,
                    &series_name_str,
                    extraction_methods,
                    default_method,
                )?;
            }
        }
    }
    Ok(())
}

#[derive(Eq, PartialEq, Hash, Clone)]
struct Method<'a> {
    name: &'a str,
    pattern: &'a str,
    func: fn(&str) -> (Option<i32>, Option<i32>),
}

#[derive(Clone)]
struct Methods<'a> {
    data: HashSet<Method<'a>>,
}

impl<'a> Methods<'a> {
    pub fn get(&'a self, name: &str) -> Option<&'a Method<'a>> {
        self.data.iter().find(|method| method.name == name)
    }
}

fn prompt_for_season_episode(filename: &str) -> (Option<i32>, Option<i32>) {
    clear_terminal();
    println!("Enter season and episode for file '{}'", filename);
    let mut season_input = String::new();
    let mut episode_input = String::new();
    print!("Enter Season: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut season_input);
    print!("Enter Episode: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut episode_input);

    let season = season_input.trim().parse().ok();
    let episode = episode_input.trim().parse().ok();
    if season.is_none() || episode.is_none() {
        return (None, None);
    }
    (season, episode)
}

fn extract_season_episode(filename: &str) -> (Option<i32>, Option<i32>) {
    let re = regex::Regex::new(r"S(\d+)E(\d+)").expect("Regex");
    if let Some(captures) = re.captures(filename) {
        let season = captures[1].parse().ok();
        let episode = captures[2].parse().ok();
        (season, episode)
    } else {
        (None, None)
    }
}

fn extract_season_episode_dash(filename: &str) -> (Option<i32>, Option<i32>) {
    let re = regex::Regex::new(r" - (\d+)").expect("Regex");
    if let Some(captures) = re.captures(filename) {
        let season = Some(1); // Fixed season value
        let episode = captures[1].parse().ok();
        (season, episode)
    } else {
        (None, None)
    }
}

impl<'a> Default for Methods<'a> {
    fn default() -> Self {
        Self {
            data: vec![
                Method {
                    name: "dash",
                    pattern: " - {int}",
                    func: extract_season_episode_dash,
                },
                Method {
                    name: "default",
                    pattern: "S{int}E{int}",
                    func: extract_season_episode,
                },
            ]
            .into_iter()
            .collect(),
        }
    }
}
