use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::process::Command;
use zip::ZipArchive;

fn download_file(url: String, file_path: String) -> Result<(), Box<dyn Error>> {
    let mut res = match reqwest::blocking::get(url.clone()) {
        Ok(res) => res,
        Err(_) => {
            return Err("Failed to request page!".into());
        }
    };

    let mut out_file = match File::create(file_path) {
        Ok(file) => file,
        Err(_) => {
            return Err("Failed to create file!".into());
        }
    };

    io::copy(&mut res, &mut out_file).unwrap();

    Ok(())
}

fn unzip_file(zipfile_path: String, output_path: String) -> Result<(), Box<dyn Error>> {
    let file = match File::open(zipfile_path) {
        Ok(file) => file,
        Err(_) => {
            return Err("Failed to open zipfile!".into());
        }
    };

    let mut archive = match ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(_) => {
            return Err("Failed to open zip file!".into());
        }
    };

    let _ = fs::create_dir_all(output_path.clone());

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(file) => file,
            Err(_) => {
                return Err("Failed to open subfile!".into());
            }
        };

        let outpath = Path::new(&output_path).join(file.name());

        if file.is_dir() {
            match fs::create_dir_all(&outpath) {
                Ok(_) => {},
                Err(_) => {
                    return Err("Failed to create directory!".into());
                }
            }
        } else {
            if let Some(p) = outpath.parent() {
                match fs::create_dir_all(p) {
                    Ok(_) => {},
                    Err(_) => {
                        return Err("Failed to create directory!".into());
                    }
                };
            }

            let mut outfile = match File::create(&outpath) {
                Ok(file) => file,
                Err(_) => {
                    return Err("Failed to create file!".into());
                }
            };

            match io::copy(&mut file, &mut outfile) {
                Ok(_) => {},
                Err(_) => {
                    return Err("Failed to write file!".into());
                }
            };
        }
    }

    Ok(())
}

fn main() {
    let mut url = String::new();
    let mut redownload = false;
    let mut reinstall = false;
    let mut keep_zip = false;
    let mut start_server = false;
    let mut output_path: Option<String> = None;

    for arg in env::args().skip(1).collect::<Vec<String>>() {
        if arg == "--redownload" { redownload = true; }
        else if arg == "--reinstall" { reinstall = true; }
        else if arg == "--keep-zip" { keep_zip = true; }
        else if arg == "--start" { start_server = true; }
        else if arg.starts_with("--out=") { output_path = Some(arg[6..].to_string()); }
        else if arg.starts_with("http://") || arg.starts_with("https://") { url = arg; }
    }

    if url.is_empty() {
        println!("Usage: mcmpi <url> [--redownload] [--reinstall] [--keep-zip] [--start] [--out=<path>]");
        println!();
        println!("Options:");
        println!("--redownload: Force downloading the file again.");
        println!("--reinstall:  Force reinstalling the modpack.");
        println!("--keep-zip:   Keep the downloaded zip file.");
        println!("--start:      Start the server after installation.");
        println!("--out=<path>: Specify the output folder path.");
        return;
    }

    // fix url encoding
    url = url
        .replace("%20", " ")
        .replace("%29", ")")
        .replace("%28", "(")
        .replace("%21", "!")
        .replace("%23", "#")
        .replace("%24", "$");

    let file_name = match url.split('/').next_back() {
        Some(file_name) => file_name,
        None => {
            println!("Failed to extract file name!");
            return;
        }
    }.to_string();

    let directory_name = {
        if let Some(path) = output_path.clone() {
            path
        } else if let Some(index) = file_name.rfind(".") {
            file_name[..index].to_string()
        } else {
            file_name.clone()
        }
    };

    println!("Downloading {}...", file_name.clone());

    // download file
    if fs::exists(file_name.clone()).unwrap() && !redownload {
        println!("File {} already exists, skipping download.", file_name.clone());
    } else {
        match download_file(url, file_name.clone()) {
            Ok(_) => println!("Downloaded files."),
            Err(err) => {
                println!("Failed to download file: {}", err);
                return;
            }
        };
    }

    // unzip file
    if fs::exists(directory_name.clone()).unwrap() && !reinstall {
        println!("Directory {} already exists, skipping unzip.", directory_name.clone());
    } else {
        match unzip_file(file_name.clone(), directory_name.clone()) {
            Ok(_) => {
                println!("Unzipped files to {}.", directory_name.clone());

                if !keep_zip {
                    fs::remove_file(file_name.clone()).unwrap();
                    println!("Deleted zip file.");
                }
            },
            Err(err) => {
                println!("Failed to unzip file: {}", err);
                return;
            }
        };
    }

    if !start_server { return; }

    // check screen
    let screen_exists = Command::new("screen").arg("-v").output().map(|output| output.status.success()).unwrap_or(false);
    if !screen_exists {
        println!("Screen is not installed. Please install it first.");
        return;
    }

    // run server
    let screen_name = format!("mcmpi-{}", directory_name.clone());
    
    let screen_file = {
        if fs::exists(format!("{}/start.sh", directory_name.clone())).unwrap_or(false) { "start.sh" }
        else if fs::exists(format!("{}/launch.sh", directory_name.clone())).unwrap_or(false) { "launch.sh" }
        else {
            println!("Failed to find launch script.");
            return;
        }
    }.to_string();
    
    let screen_command = format!("cd {} && chmod +x ./{} && ./{}", directory_name.clone(), screen_file.clone(), screen_file.clone());

    let args = [
        "-dmS", screen_name.as_str(),
        "bash", "-c",
        screen_command.as_str()
    ];

    match Command::new("screen").args(args).spawn() {
        Ok(_) => println!("Server started."),
        Err(err) => {
            println!("Failed to start screen: {}", err);
        }
    };
}
