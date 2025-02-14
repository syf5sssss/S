use exif::In;
use regex::Regex;
use sqlutil::{ DbHelper, Img };
use std::{ error::Error, fs, path::Path };

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod sqlutil;
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// src-tauri/src/main.rs
#[tauri::command]
fn get_image_path() -> String {
    let asset_url =
        "http://asset.localhost/D:/TEST/rt2/IMG_20250209_164108/IMG_20250209_124541.jpg";
    println!("{}", asset_url);
    return asset_url.to_string();
}

#[tauri::command]
fn load_dir_imgs(path: &str) -> Result<Vec<Img>, String> {
    let mut imgs = Vec::<Img>::new();
    let files = match load_dir(path) {
        Ok(files) => files,
        Err(e) => {
            return Err(format!("{}", e));
        }
    };
    for str in files {
        let fpath = Path::new(&str);
        let file = match fs::File::open(fpath) {
            Ok(file) => file,
            Err(e) => {
                return Err(format!("无法打开文件 {}: {}", fpath.display(), e));
            }
        };
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = match exifreader.read_from_container(&mut bufreader) {
            Ok(exif) => exif,
            Err(e) => {
                continue;
                // return Err(format!("无法读取 EXIF 数据 {}: {}", fpath.display(), e));
            }
        };

        let mut pic = Img::new();
        // 获取文件名
        let file_name = fpath
            .file_name()
            .and_then(|s| s.to_str()) // 将 OsStr 转换为 &str
            .ok_or_else(|| "无法获取文件名")?; // 如果失败，返回错误
        pic.name = file_name.to_string();
        pic.path = str;
        if let Some(gps_date_time) = exif.get_field(exif::Tag::DateTime, In::PRIMARY) {
            let width_str = format!("{}", gps_date_time.display_value().with_unit(&exif));
            pic.time = width_str;
            // println!("GPS DateTime: {:?}", width_str);
        }
        if let Some(gps_latitude) = exif.get_field(exif::Tag::GPSLatitude, In::PRIMARY) {
            let width_str = format!("{}", gps_latitude.display_value().with_unit(&exif));
            println!("GPS Latitude: {:?}", width_str);
            let lat = parse_dms(&width_str).expect("Failed to parse latitude");
            pic.lat = lat;
            println!("WGS84坐标:lat ({})", lat);
        }
        if let Some(gps_longitude) = exif.get_field(exif::Tag::GPSLongitude, In::PRIMARY) {
            let width_str = format!("{}", gps_longitude.display_value().with_unit(&exif));
            println!("GPS Longitude: {:?}", width_str);
            let lon = parse_dms(&width_str).expect("Failed to parse longitude");
            pic.lon = lon;
            println!("WGS84坐标:lon ({})", lon);
        }
        if pic.lat < pic.lon {
            let cah = pic.lat;
            pic.lat = pic.lon;
            pic.lon = cah;
        }
        imgs.push(pic);
    }
    // if imgs.len() > 0 {
    //     let db = match DbHelper::new("img.db") {
    //         Ok(db) => db,
    //         Err(e) => {
    //             return Err(format!("{}", e));
    //         }
    //     };
    //     match db.truncate() {
    //         Ok(_) => {}
    //         Err(e) => {
    //             return Err(format!("{}", e));
    //         }
    //     }
    //     let _ = db.insert_imgs(&imgs);
    //     let all = db.query_all();
    //     println!("all: {:?}", all);
    // }
    Ok(imgs)
}

#[tauri::command]
fn insert_imgs(imgs: Vec<Img>) -> String {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return e.to_string();
        }
    };
    let _ = db.insert_imgs(&imgs);
    return "".to_string();
}

#[tauri::command]
fn truncate() -> String {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return e.to_string();
        }
    };
    let _ = db.truncate();
    return "".to_string();
}

#[tauri::command]
fn query_all() -> Result<Vec<Img>, String> {
    let db = match DbHelper::new("img.db") {
        Ok(db) => db,
        Err(e) => {
            return Err(format!("{}", e));
        }
    };
    let all = match db.query_all() {
        Ok(all) => all,
        Err(e) => {
            return Err(format!("{}", e));
        }
    };
    Ok(all)
}

fn parse_dms(dms: &str) -> Result<f64, Box<dyn Error>> {
    let re = Regex::new(r"(\d+) deg (\d+) min ([\d.]+) sec ([NSEW])").unwrap();
    let caps = re.captures(dms).ok_or("Invalid DMS format")?;

    let degrees: f64 = caps[1].parse()?;
    let minutes: f64 = caps[2].parse()?;
    let seconds: f64 = caps[3].parse()?;
    let direction = &caps[4];

    let total_degrees = degrees + minutes / 60.0 + seconds / 3600.0;
    let sign = match direction {
        "N" | "E" => 1.0,
        "S" | "W" => -1.0,
        _ => {
            return Err("Invalid direction".into());
        }
    };

    Ok(total_degrees * sign)
}

#[tauri::command]
fn load_dir(path: &str) -> Result<Vec<String>, String> {
    // 将路径转换为 PathBuf
    let path = Path::new(path);

    // 检查路径是否存在
    if !path.exists() {
        return Err(format!("路径不存在: {}", path.display()));
    }

    // 调用递归函数获取所有文件路径
    let mut files = Vec::new();
    match collect_files(path, &mut files) {
        Ok(_) => Ok(files),
        Err(e) => Err(format!("遍历目录时出错: {}", e)),
    }
}

fn collect_files(dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
    // 读取目录内容
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(format!("无法读取目录 {}: {}", dir.display(), e));
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                return Err(format!("无法读取目录项 {}: {}", dir.display(), e));
            }
        };

        let path = entry.path();
        if path.is_dir() {
            // 如果是目录，递归调用
            collect_files(&path, files)?;
        } else if path.is_file() {
            // 如果是文件，将路径添加到列表中
            files.push(path.to_string_lossy().into_owned());
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder
        ::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(
            tauri::generate_handler![
                greet,
                get_image_path,
                load_dir,
                load_dir_imgs,
                insert_imgs,
                truncate,
                query_all
            ]
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
