use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use rand::RngCore;
use zip::ZipArchive;

fn print_usage() {
    println!("Usage: mcmpi <url> [--redownload] [--reinstall] [--keep-zip] [--eula] [--start] [--java=<path>] [--out=<path>]");
    println!();
    println!("Options:");
    println!("--redownload:     Force downloading the file again.");
    println!("--reinstall:      Force reinstalling the modpack.");
    println!("--keep-zip:       Keep the downloaded zip file.");
    println!("--eula:           Agree to the eula.");
    println!("--start:          Start the server after installation.");
    println!("--java=<path>:    Specify the java path (experimental).");
    println!("--out=<path>:     Specify the output folder path.");
}

fn decode_url(url: &String) -> String {
    url
    .replace("%20", " ")
    .replace("%29", ")")
    .replace("%28", "(")
    .replace("%21", "!")
    .replace("%23", "#")
    .replace("%24", "$")
}

fn extract_filename(path: &String) -> Option<String> {
    match path.split('/').next_back() {
        Some(file_name) => {
            match file_name.rfind(".") {
                Some(index) => Some(file_name[..index].to_string()),
                None => Some(file_name.to_string())
            }
        },
        None => {
            println!("Failed to extract file name!");
            return None;
        }
    }
}

fn download_file(url: &String, file_path: &String) -> Result<(), Box<dyn Error>> {
    let mut res = match reqwest::blocking::get(url) {
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

    match io::copy(&mut res, &mut out_file) {
        Ok(_) => {},
        Err(_) => {
            return Err("Failed to write file!".into());
        }
    };

    Ok(())
}

fn unzip_file(zipfile_path: &String, output_path: &String) -> Result<(), Box<dyn Error>> {
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

    let _ = fs::create_dir_all(output_path);

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
    // region: read cmd args
    let mut url: String = String::new();
    let mut redownload = false;
    let mut reinstall = false;
    let mut keep_zip = false;
    let mut eula = false;
    let mut start_server = false;
    let mut output_path: Option<String> = None;

    for arg in env::args().skip(1).collect::<Vec<String>>() {
        if arg == "--redownload" { redownload = true; }
        else if arg == "--reinstall" { reinstall = true; }
        else if arg == "--keep-zip" { keep_zip = true; }
        else if arg == "--eula" { eula = true; }
        else if arg == "--start" { start_server = true; }
        else if let Some(path) = arg.strip_prefix("--out=") { output_path = Some(path.to_string()); }
        else if arg.starts_with("http://") || arg.starts_with("https://") { url = arg; }
    }

    if url.is_empty() {
        print_usage();
        return;
    }

    // fix url encoding
    url = decode_url(&url);
    // endregion

    // region: decide output paths
    let default_name = match extract_filename(&url) {
        Some(path) => path,
        None => {
            eprintln!("Failed to extract file name!");
            return;
        }
    };
    let directory_name = match output_path {
        Some(path) => path,
        None => default_name
    };
    let file_name = directory_name.clone() + ".zip";
    // endregion

    // region: download file
    println!("Downloading {}...", &file_name);
    if fs::exists(&file_name).unwrap_or(false) && !redownload {
        println!("File {} already exists, skipping download.", &file_name);
    } else {
        match download_file(&url, &file_name) {
            Ok(_) => println!("Downloaded files."),
            Err(err) => {
                println!("Failed to download file: {}", err);
                return;
            }
        };
    }
    // endregion

    // region: unzip file
    println!("Unzipping files...");

    if fs::exists(&directory_name).unwrap_or(false) && !reinstall {
        println!("Directory {} already exists, skipping unzip.", &directory_name);
    } else {        
        match unzip_file(&file_name, &directory_name) {
            Ok(_) => {
                println!("Unzipped files to {}.", &directory_name);

                if !keep_zip {
                    match fs::remove_file(&file_name) {
                        Ok(_) => println!("Deleted zip file."),
                        Err(_) => {
                            println!("Failed to delete zip file.");
                            return;
                        }
                    };
                }
            },
            Err(err) => {
                println!("Failed to unzip file: {}", err);
                return;
            }
        };
    }
    // endregion

    // region: agree to eula
    if eula {
        let _ = fs::remove_file(format!("{}/eula.txt", &directory_name));

        let mut file_out = match File::create(format!("{}/eula.txt", &directory_name)) {
            Ok(file) => file,
            Err(_) => {
                println!("Failed to create eula file.");
                return;
            }
        };
        match file_out.write_all("eula=true".as_bytes()) {
            Ok(_) => {},
            Err(_) => {
                println!("Failed to write eula file.");
                return;
            }
        };

        println!("Agreed to eula.");
    }
    // endregion

    // region: add metadata
    if !fs::exists(format!("{}/.mcmpi", &directory_name)).unwrap_or(false) {
        let mut file_out = match File::create(format!("{}/.mcmpi", &directory_name)) {
            Ok(file) => file,
            Err(_) => {
                println!("Failed to create metadata file.");
                return;
            }
        };

        let data = format!("Name: {}\nDownload: {}\nDate: {}", file_name, url, chrono::Local::now().to_rfc2822());

        match file_out.write_all(data.as_bytes()) {
            Ok(_) => {},
            Err(_) => {
                println!("Failed to write metadata file.");
                return;
            }
        };
    }
    // endregion

    if !start_server { return; }

    // region: start server
    // check screen
    let screen_exists = Command::new("screen").arg("-v").output().map(|output| output.status.success()).unwrap_or(false);
    if !screen_exists {
        println!("Screen is not installed. Please install it first.");
        return;
    }

    // run server
    let rnd = rand::rng().next_u32();

    let tmp = format!("mcmpi-{}", rnd);
    let screen_name = tmp.as_str();

    let start_file = {
        if fs::exists(format!("{}/start.sh", &directory_name)).unwrap_or(false) { "start.sh" }
        else if fs::exists(format!("{}/launch.sh", &directory_name)).unwrap_or(false) { "launch.sh" }
        else {
            println!("Failed to find launch script.");
            return;
        }
    };

    let _ = Command::new("screen").arg("-dmS").arg(screen_name).spawn().and_then(|mut c| c.wait());
    let _ = Command::new("screen").arg("-S").arg(screen_name).arg("-X").arg("stuff").arg(format!("cd {}&&chmod +x ./{}&&./{}\\n", &directory_name, start_file, start_file)).spawn().and_then(|mut c| c.wait());

    println!("Server started in screen {}.", screen_name);
    // endregion
}
